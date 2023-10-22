use std::{
    mem::drop,
    sync::{Arc, RwLock}, 
    thread,
    time::Duration
};

use crate::{
    builder::BlockBuilder,
    db::AccountsDB,
    structures::{
        Account, 
        StakeTransaction,
        Transaction,
        TransferTransaction, 
        TransactionSign,
        UserAccount,
        ValidatorAccount,
    }, 
    pool::Mempool, 
};

fn setup_accounts(db: &AccountsDB) -> (UserAccount, UserAccount) {
    let account1 = UserAccount::new();
    let account2 = UserAccount::new();
    db.add_account(account1.public_key, account1.clone());
    db.add_account(account2.public_key, account2.clone());
    (account1, account2)
}

fn setup_validators() -> (ValidatorAccount, ValidatorAccount, Arc<RwLock<AccountsDB>>, Arc<RwLock<Mempool>>) {
    let mempool = Arc::new(RwLock::new(Mempool::new()));
    let db = Arc::new(RwLock::new(AccountsDB::new()));
    let builder1 = BlockBuilder::new(Arc::clone(&mempool), Arc::clone(&db));
    let builder2 = BlockBuilder::new(Arc::clone(&mempool), Arc::clone(&db));
    let validator1 = ValidatorAccount::new(builder1);
    let validator2 = ValidatorAccount::new(builder2);
    let db_lock = db.write().unwrap();

    db_lock.add_validator(validator1.public_key, validator1.clone());
    db_lock.add_validator(validator2.public_key, validator2.clone());
    (validator1, validator2, Arc::clone(&db), Arc::clone(&mempool))
}

#[test]
fn test_account_creation() {
    let db = AccountsDB::new();
    let (account1, account2) = setup_accounts(&db);

    assert!(db.get_account(&account1.public_key).is_some(), "Account 1 should exist");
    assert!(db.get_account(&account2.public_key).is_some(), "Account 2 should exist");
}

#[test]
fn test_account_balance() {
    let db = AccountsDB::new();
    let (account1, _) = setup_accounts(&db);

    let increase_res = db.increase_account_balance(&account1.public_key, 1000);
    assert!(increase_res.is_ok(), "Increasing balance should succeed");

    let fetched_account1 = db.get_account(&account1.public_key).expect("Account 1 should exist");
    assert_eq!(fetched_account1.balance, 1000, "Balance should be 1000");
}

#[test]
fn test_transfer_transaction_validation() {
    let db = AccountsDB::new();
    let (account1, account2) = setup_accounts(&db);

    let _ = db.increase_account_balance(&account1.public_key, 1000);

    let mut tx = TransferTransaction::new(account2.public_key, account1.public_key, 500, account1.nonce);

    tx.sign(&Account::UserAccount(account1));

    assert!(tx.validate(&db)); 
}

#[test]
fn test_validator_creation() {
    let (validator1, validator2, db, _) = setup_validators();

    let db_lock = db.read().unwrap();

    assert!(db_lock.is_validator(&validator1.public_key), "Validator 1 should exist");
    assert!(db_lock.is_validator(&validator2.public_key), "Validator 2 should exist");
}

#[test]
fn test_stake_transaction_validation() {
    let (validator1, _v, db, _) = setup_validators();
    let db_lock = db.write().unwrap();

    let account1 = setup_accounts(&db_lock).0;

    let _ = db_lock.increase_account_balance(&account1.public_key, 1000);

    let mut tx = StakeTransaction::new(validator1.public_key, account1.public_key, 500, account1.nonce);

    tx.sign(&Account::UserAccount(account1));

    assert!(tx.validate(&db_lock));
}

#[test]
fn test_send_transaction() {
    let mempool = Mempool::new();
    let db = AccountsDB::new();

    let (account1, account2) = setup_accounts(&db);

    let _ = db.increase_account_balance(&account1.public_key, 1000);

    let mut tx = TransferTransaction::new(account2.public_key, account1.public_key, 500, account1.nonce);

    tx.sign(&Account::UserAccount(account1));

    assert!(tx.validate(&db), "Transaction validation failed");

    let sig = mempool.send_transaction(Transaction::Transfer(tx));

    assert!(sig.is_ok(), "Transaction send failed");
}

#[test]
fn test_build_block() {
    let (validator1, _v, db, mempool) = setup_validators();
    let db_lock = db.write().unwrap();
    let mempool_lock = mempool.write().unwrap();

    let (account1, account2) = setup_accounts(&db_lock);

    let genesis_block = validator1.builder.build_genesis();

    assert!(genesis_block.transactions.is_empty(), "Genesis block should have no transactions");
    assert_eq!(genesis_block.hash, [1; 32], "Genesis block hash should be predefined");

    let _ = db_lock.increase_account_balance(&account1.public_key, 1000);

    let mut transfer_tx = TransferTransaction::new(account2.public_key, account1.public_key, 500, account1.nonce);
    let mut stake_tx = StakeTransaction::new(validator1.public_key, account1.public_key, 500, account1.nonce);

    transfer_tx.sign(&Account::UserAccount(account1.clone()));
    stake_tx.sign(&Account::UserAccount(account1.clone()));

    let signed_transfer_tx = Transaction::Transfer(transfer_tx.clone());
    let signed_stake_tx: Transaction = Transaction::Stake(stake_tx.clone());

    let transfer_sig = mempool_lock.send_transaction(signed_transfer_tx.clone());
    let stake_sig = mempool_lock.send_transaction(signed_stake_tx.clone());

    assert!(transfer_sig.is_ok(), "Transaction send failed");
    assert!(stake_sig.is_ok(), "Transaction send failed");

    drop(db_lock);
    drop(mempool_lock);

    let new_block = validator1.builder.build(genesis_block.hash);

    let block = new_block.unwrap();

    assert!(block.transactions.contains(&signed_transfer_tx), "Block should contain the transfer transaction");
    assert!(block.transactions.contains(&signed_stake_tx), "Block should contain the stake transaction");

    assert!(validator1.builder.validate_block(&block).is_ok(), "New block should be valid");
}

#[test]
fn test_run_blockchain() {
    let (validator1, validator2, db, mempool) = setup_validators();
    let db_lock = db.write().unwrap();
    let mempool_lock = mempool.write().unwrap();

    let (account1, account2) = setup_accounts(&db_lock);

    let genesis_block = validator1.builder.build_genesis();

    assert!(genesis_block.transactions.is_empty(), "Genesis block should have no transactions");
    assert_eq!(genesis_block.hash, [1; 32], "Genesis block hash should be predefined");

    let _ = db_lock.increase_account_balance(&account1.public_key, 10000);
    let _ = db_lock.increase_account_balance(&account2.public_key, 10000);

    let mut transfer_tx1 = TransferTransaction::new(account2.public_key, account1.public_key, 1500, account1.nonce);
    let mut transfer_tx2 = TransferTransaction::new(account1.public_key, account2.public_key, 2000, account2.nonce);

    let mut stake_tx1 = StakeTransaction::new(validator1.public_key, account1.public_key, 500, account1.nonce);
    let mut stake_tx2 = StakeTransaction::new(validator2.public_key, account2.public_key, 750, account2.nonce);

    transfer_tx1.sign(&Account::UserAccount(account1.clone()));
    transfer_tx2.sign(&Account::UserAccount(account2.clone()));

    stake_tx1.sign(&Account::UserAccount(account1.clone()));
    stake_tx2.sign(&Account::UserAccount(account2.clone()));

    let signed_transfer1 = Transaction::Transfer(transfer_tx1);
    let signed_transfer2 = Transaction::Transfer(transfer_tx2);

    let signed_stake1 = Transaction::Stake(stake_tx1);
    let signed_stake2 = Transaction::Stake(stake_tx2);

    let transfer1_sig = mempool_lock.send_transaction(signed_transfer1);
    let transfer2_sig = mempool_lock.send_transaction(signed_transfer2);

    assert!(transfer1_sig.is_ok(), "Transfer 1 send failed.");
    assert!(transfer2_sig.is_ok(), "Transfer 2 send failed.");

    let stake1_sig = mempool_lock.send_transaction(signed_stake1);
    let stake2_sig = mempool_lock.send_transaction(signed_stake2);

    assert!(stake1_sig.is_ok(), "Stake 1 send failed.");
    assert!(stake2_sig.is_ok(), "Stake 2 send failed.");

    drop(db_lock);
    drop(mempool_lock);

    let validator1_handle = thread::spawn(move || {
        let _ = validator1.start(Duration::from_millis(100));
    });

    let validator2_handle = thread::spawn(move || {
        let _ = validator2.start(Duration::from_millis(100));
    });

    validator1_handle.join().unwrap();
    validator2_handle.join().unwrap();
    
    let mempool_lock = mempool.read().unwrap();

    assert_eq!(mempool_lock.pool.len(), 0, "Leftover transactions in mempool");
}