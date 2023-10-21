use dashmap::DashMap;
use crate::structures::{Account, Pubkey, UserAccount, Blockhash, ValidatorAccount};

pub struct AccountsDB {
    latest_blockhash: Blockhash,
    accounts: DashMap<Pubkey, UserAccount>,
    validators: DashMap<Pubkey, ValidatorAccount>,
}
   
impl AccountsDB {
    pub fn new() -> Self {
        Self {
            latest_blockhash: Blockhash::default(),
            accounts: DashMap::new(),
            validators: DashMap::new(),
        }
    }

    pub fn add_account(&self, pubkey: Pubkey, account: UserAccount) {
        self.accounts.insert(pubkey, account);
    }

    pub fn get_account(&self, pubkey: &Pubkey) -> Option<UserAccount> {
        self.accounts.get(pubkey).map(|acc| acc.clone())
    }

    pub fn increase_account_balance(&self, pubkey: &Pubkey, delta: u64) -> Result<(), &'static str> {
        if let Some(mut account) = self.accounts.get_mut(pubkey) {
            account.balance = account.balance.saturating_add(delta);
            Ok(())
        } else {
            Err("Account not found.")
        }
    }

    pub fn decrease_account_balance(&self, pubkey: &Pubkey, delta: u64) -> Result<(), &'static str> {
        if let Some(mut account) = self.accounts.get_mut(pubkey) {
            if account.balance.gt(&delta) {
                account.balance = account.balance.saturating_sub(delta);
                Ok(())
            } else {
                Err("Insufficient balance.")
            }
        } else {
            Err("Account not found.")
        }
    }

    pub fn add_validator(&self, pubkey: Pubkey, validator: ValidatorAccount) {
        self.validators.insert(pubkey, validator);
    }

    pub fn is_validator(&self, pubkey: &Pubkey) -> bool {
        self.validators.contains_key(pubkey)
    }

    pub fn get_validator(&self, pubkey: &Pubkey) -> Option<ValidatorAccount> {
        self.validators.get(pubkey).map(|val| val.clone())
    }

    pub fn increase_validator_stake(&self, pubkey: &Pubkey, amt: u64) -> Result<(), &'static str> {
        if let Some(mut validator) = self.validators.get_mut(pubkey) {
            validator.stake = validator.stake.saturating_add(amt);
            Ok(())
        } else {
            Err("Validator not found.")
        }
    }
}