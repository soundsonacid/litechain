use std::{
    collections::HashSet,
    time::SystemTime
};

use ed25519_dalek::{
    Keypair, 
    PublicKey, 
    PUBLIC_KEY_LENGTH, 
    SECRET_KEY_LENGTH, 
    SecretKey,
    Signature, 
    Signer,
    Verifier
};
use hex;
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};

use crate::AccountsDB;

// Primitives for accounts / blocks / transactions
pub type Blockhash = [u8; 32];
pub type Pubkey = [u8; PUBLIC_KEY_LENGTH];
pub type Seckey = [u8; SECRET_KEY_LENGTH];

const DEFAULT_SIGNATURE_BYTES: [u8; Signature::BYTE_SIZE] = [0; Signature::BYTE_SIZE];

// Enums defining types of accounts & transactions
pub enum Account {
    UserAccount(UserAccount),
    ValidatorsAccount(ValidatorsAccount),
}

pub struct Block {
    transactions: Vec<Transaction>,
    hash: Blockhash,
    prev_hash: Blockhash,
    timestamp: SystemTime,
}

impl Block {
    pub fn new(transactions: Vec<Transaction>, prev_hash: Blockhash) -> Self {
        // Create new Block with placeholder for the hash
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
    pub address: String, // Derived from public key to string
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

    pub fn sign_transaction(&self, transaction: &mut Transaction) {
        let keypair = Keypair {
            public: PublicKey::from_bytes(&self.public_key).expect("Invalid public key"),
            secret: SecretKey::from_bytes(&self.secret_key).expect("Invalid secret key")
        };

        let tx_data = transaction.serialize();

        let sig = keypair.sign(&tx_data);

        transaction.signature = sig;
    }
}
#[derive(Default, Debug, Clone)]
pub struct ValidatorsAccount {
    pub validators: HashSet<Pubkey>,
}

impl ValidatorsAccount {
    pub fn new() -> Self {
        Self {
            validators: HashSet::new(),
        }
    }
}

pub struct Transaction {
    pub to: Pubkey,
    pub from: Pubkey,
    pub amt: u64,
    nonce: u64,
    signature: Signature,
}

impl Transaction {
    pub fn new(to: Pubkey, from: Pubkey, amt: u64, nonce: u64) -> Self {
        Transaction {
            to,
            from,
            amt,
            nonce,
            signature: Signature::from_bytes(&DEFAULT_SIGNATURE_BYTES).unwrap(),
        }
    }
    // To sign a transaction we need to first serialize it into a Vec<u8>
    // The nonce ensures that signatures won't be replicated and that we can tell two otherwise identical tx apart
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = vec![];

        data.extend(&self.to.to_vec());
        data.extend(&self.from.to_vec());
        data.extend(&self.nonce.to_le_bytes());
        data.extend(&self.amt.to_le_bytes());

        data
    }

    pub fn validate(&self, db: &AccountsDB) -> bool {
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
        if !self.verify_signature(&from) {
            return false;
        }

        // "Simulate" the transaction
        if from.balance.lt(&self.amt) {
            return false;
        }

        // Now we can say that for our purposes, the transaction is valid (`from` has balance gte amt & tx is signed by `from`)
        true
    }

    pub fn verify_signature(&self, from: &UserAccount) -> bool {
        let public_key = PublicKey::from_bytes(&from.public_key).expect("Invalid public key");

        let tx_data = self.serialize();

        // We will "re-sign" the transaction to make sure that the signature matches our public key
        match public_key.verify(&tx_data, &self.signature) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}