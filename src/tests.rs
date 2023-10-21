use crate::{
    db::AccountsDB,
    structures::{
        Account, 
        TransferTransaction, 
        Transaction,
        UserAccount,
        ValidatorAccount,
    }
};

fn setup_accounts(db: &AccountsDB) -> (UserAccount, UserAccount) {
    let account1 = UserAccount::new();
    let account2 = UserAccount::new();
    db.add_account(account1.public_key, account1.clone());
    db.add_account(account2.public_key, account2.clone());
    (account1, account2)
}

fn setup_validators(db: &AccountsDB) -> (ValidatorAccount, ValidatorAccount) {
    let validator1 = ValidatorAccount::new();
    let validator2 = ValidatorAccount::new();
    db.add_validator(validator1.public_key, validator1.clone());
    db.add_validator(validator2.public_key, validator2.clone());
    (validator1, validator2)
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
fn test_transaction_validation() {
    let db = AccountsDB::new();
    let (account1, account2) = setup_accounts(&db);

    let _ = db.increase_account_balance(&account1.public_key, 1000);

    let mut tx = TransferTransaction::new(account2.public_key, account1.public_key, 500, account1.nonce);

    tx.sign(&Account::UserAccount(account1));

    assert!(tx.validate(&db)); 
}

#[test]
fn test_validator_creation() {
    let db = AccountsDB::new();
    let (validator1, validator2) = setup_validators(&db);

    assert!(db.is_validator(&validator1.public_key), "Validator 1 should exist");
    assert!(db.is_validator(&validator2.public_key), "Validator 2 should exist");
}