use std::{
    time::SystemTime,
    sync::{Arc, RwLock}
};

use ed25519_dalek::{
    Keypair, 
    PublicKey, 
    PUBLIC_KEY_LENGTH, 
    SECRET_KEY_LENGTH, 
    SecretKey,
    Signature, 
    Signer as DalekSigner,
    Verifier
};
use hex;
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};

use crate::{
    builder::BlockBuilder,
    db::AccountsDB,
    pool::Mempool
};

// Primitives for accounts / blocks / transactions
pub type Blockhash = [u8; 32];
pub type Pubkey = [u8; PUBLIC_KEY_LENGTH];
pub type Seckey = [u8; SECRET_KEY_LENGTH];
pub type Address = String;

const DEFAULT_SIGNATURE_BYTES: [u8; Signature::BYTE_SIZE] = [0; Signature::BYTE_SIZE];

// Enums defining types of accounts & transactions
pub enum Account {
    UserAccount(UserAccount),
    ValidatorAccount(ValidatorAccount),
}

pub trait Signer {
    fn public_key(&self) -> &Pubkey;
    fn secret_key(&self) -> &Pubkey;
}

impl Signer for Account {
    fn public_key(&self) -> &Pubkey {
        match self {
            Account::UserAccount(user) => user.public_key(),
            Account::ValidatorAccount(validator) => validator.public_key(),
        }
    }

    fn secret_key(&self) -> &Pubkey {
        match self {
            Account::UserAccount(user) => user.secret_key(),
            Account::ValidatorAccount(validator) => validator.secret_key(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Transaction {
    Stake(StakeTransaction),
    Transfer(TransferTransaction),
}

impl Transaction {
    pub fn get_signer(&self) -> Pubkey {
        match self {
            Transaction::Stake(tx) => tx.staker,
            Transaction::Transfer(tx) => tx.from
        }
    }
}

impl TransactionSign for Transaction {
    fn get_signature(&self) -> &Signature {
        match self {
            Transaction::Stake(tx) => &tx.signature,
            Transaction::Transfer(tx) => &tx.signature
        }
    }

    fn get_mut_signature(&mut self) -> &mut Signature {
        match self {
            Transaction::Stake(tx) => &mut tx.signature,
            Transaction::Transfer(tx) => &mut tx.signature
        }
    }

    fn validate(&self, db: &AccountsDB) -> bool {
        match self {
            Transaction::Stake(tx) => tx.validate(db),
            Transaction::Transfer(tx) => tx.validate(db),
        }
    }

    fn serialize(&self) -> Vec<u8> {
        match self {
            Transaction::Stake(tx) => tx.serialize(),
            Transaction::Transfer(tx) => tx.serialize(),
        }
    }
}


pub trait TransactionSign {
    fn get_signature(&self) -> &Signature;
    fn get_mut_signature(&mut self) -> &mut Signature;
    fn validate(&self, db: &AccountsDB) -> bool;
    fn serialize(&self) -> Vec<u8>;

    fn sign(&mut self, signer: &Account) {
        let keypair = Keypair {
            public: PublicKey::from_bytes(signer.public_key()).expect("Invalid public key"),
            secret: SecretKey::from_bytes(signer.secret_key()).expect("Invalid secret key"),
        };

        let tx_data = self.serialize();
        let sig = keypair.sign(&tx_data);

        *self.get_mut_signature() = sig;
    }

    fn verify_signature(&self, signer: &Pubkey) -> bool {
        let public_key = PublicKey::from_bytes(signer).expect("Invalid public key");
        let tx_data = self.serialize();

        match public_key.verify(&tx_data, self.get_signature()) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

pub struct Block {
    pub transactions: Vec<Transaction>,
    pub hash: Blockhash,
    pub prev_hash: Blockhash,
    timestamp: SystemTime,
}

impl Block {
    pub fn new(transactions: Vec<Transaction>, prev_hash: Blockhash) -> Self {
        let mut block = Block {
            transactions,
            hash: [0; 32],
            prev_hash,
            timestamp: SystemTime::now(),
        };
        // Derive the hash for the new block
        block.hash = block.get_hash(prev_hash);
        block
    }

    pub fn create_genesis() -> Self {
        Self {
            transactions: vec![],
            hash: [1; 32],
            prev_hash: [1; 32],
            timestamp: SystemTime::now(),
        }
    }

    pub fn get_hash(&self, prev_hash: Blockhash) -> Blockhash {
        let mut hasher = Sha256::new();

        // Hash the previous blockhash
        hasher.update(prev_hash);

        // Hash the timestamp
        if let Ok(duration) = self.timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            let timestamp = duration.as_secs();
            hasher.update(&timestamp.to_le_bytes());
        }

        // Hash all the transactions in the block
        for tx in &self.transactions {
            let tx_data = tx.serialize();
            hasher.update(tx_data);
        }

        let hash = hasher.finalize();

        let mut new_hash = [0u8; 32];

        new_hash.copy_from_slice(hash.as_slice());

        new_hash
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct UserAccount {
    pub address: Address, // Derived from public key to string
    pub public_key: Pubkey, // Derived from secret key
    pub balance: u64,
    pub nonce: u64,
    secret_key: Seckey,
}

impl UserAccount {
    pub fn new() -> Self {
        // Generate a new random keypair for our new account
        let mut csprng = OsRng;
        let keypair: Keypair = Keypair::generate(&mut csprng);

        let public_key: Pubkey = keypair.public.to_bytes();
        let secret_key: Seckey = keypair.secret.to_bytes();

        let address = hex::encode(&public_key);

        UserAccount {
            address,
            balance: 0,
            nonce: 0,
            public_key,
            secret_key,
        }
    }

    pub fn sign_transaction(&self, transaction: &mut TransferTransaction) {
        let keypair = Keypair {
            public: PublicKey::from_bytes(&self.public_key).expect("Invalid public key"),
            secret: SecretKey::from_bytes(&self.secret_key).expect("Invalid secret key")
        };

        let tx_data = transaction.serialize();

        let sig = keypair.sign(&tx_data);

        transaction.signature = sig;
    }
}

impl Signer for UserAccount {
    fn public_key(&self) -> &Pubkey {
        &self.public_key
    }

    fn secret_key(&self) -> &Pubkey {
        &self.secret_key
    }
}

#[derive(Default, Debug, Clone)]
pub struct ValidatorAccount {
    pub address: Address,
    pub public_key: Pubkey,
    pub stake: u64,
    pub builder: BlockBuilder,
    secret_key: Seckey,
}

impl ValidatorAccount {
    pub fn new(builder: BlockBuilder) -> Self {
        let mut csprng = OsRng;
        let keypair = Keypair::generate(&mut csprng);

        let public_key: Pubkey = keypair.public.to_bytes();
        let secret_key: Seckey = keypair.secret.to_bytes();

        let address = hex::encode(&public_key);

        ValidatorAccount {
            address,
            public_key,
            stake: 0,
            builder,
            secret_key,
        }
    }
}

impl Signer for ValidatorAccount {
    fn public_key(&self) -> &Pubkey {
        &self.public_key
    }

    fn secret_key(&self) -> &Pubkey {
        &self.secret_key
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StakeTransaction {
    pub validator: Pubkey,
    pub staker: Pubkey,
    pub amt: u64,
    nonce: u64,
    signature: Signature
}

impl StakeTransaction {
    pub fn new(validator: Pubkey, staker: Pubkey, amt: u64, nonce: u64) -> Self {
        StakeTransaction {
            validator,
            staker,
            amt,
            nonce,
            signature: Signature::from_bytes(&DEFAULT_SIGNATURE_BYTES).unwrap()
        }
    }
}

impl TransactionSign for StakeTransaction {
    fn get_signature(&self) -> &Signature {
        &self.signature
    }

    fn get_mut_signature(&mut self) -> &mut Signature {
        &mut self.signature
    }

    fn validate(&self, db: &AccountsDB) -> bool {
        // Make sure `validator`` is a validator
        if !db.is_validator(&self.validator) {
            return false
        }

        let staker = match db.get_account(&self.staker) {
            Some(account) => account,
            None => return false,
        };

        if !self.verify_signature(&staker.public_key()) {
            return false
        }

        if staker.balance.lt(&self.amt) {
            return false
        }

        true
    }

    fn serialize(&self) -> Vec<u8> {
        let mut data = vec![];

        data.extend(&self.validator.to_vec());
        data.extend(&self.staker.to_vec());
        data.extend(&self.nonce.to_le_bytes());
        data.extend(&self.amt.to_le_bytes());

        data
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferTransaction {
    pub to: Pubkey,
    pub from: Pubkey,
    pub amt: u64,
    nonce: u64,
    signature: Signature,
}

impl TransferTransaction {
    pub fn new(to: Pubkey, from: Pubkey, amt: u64, nonce: u64) -> Self {
        TransferTransaction {
            to,
            from,
            amt,
            nonce,
            signature: Signature::from_bytes(&DEFAULT_SIGNATURE_BYTES).unwrap(),
        }
    }
}

impl TransactionSign for TransferTransaction {
    fn get_signature(&self) -> &Signature {
        &self.signature
    }

    fn get_mut_signature(&mut self) -> &mut Signature {
        &mut self.signature
    }

    fn serialize(&self) -> Vec<u8> {
        let mut data = vec![];

        data.extend(&self.to.to_vec());
        data.extend(&self.from.to_vec());
        data.extend(&self.nonce.to_le_bytes());
        data.extend(&self.amt.to_le_bytes());

        data
    }

    fn validate(&self, db: &AccountsDB) -> bool {
        // First we'll make sure that `to` and `from` actually exist
        let from = match db.get_account(&self.from) {
            Some(account) => account,
            None => return false,
        };

        let _to = match db.get_account(&self.to) {
            Some(account) => account,
            None => return false,
        };

        // Now we'll go ahead and make sure that the `from` account is actually the signer 
        if !self.verify_signature(from.public_key()) {
            return false;
        }

        // "Simulate" the transaction
        if from.balance.lt(&self.amt) {
            return false;
        }

        // Now we can say that for our purposes, the transaction is valid (`from` has balance gte amt & tx is signed by `from`)
        true
    }
}

