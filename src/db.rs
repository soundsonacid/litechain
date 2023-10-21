use dashmap::DashMap;
use crate::{structures::{Pubkey, UserAccount, Blockhash}, ValidatorsAccount};

pub struct AccountsDB {
    latest_blockhash: Blockhash,
    accounts: DashMap<Pubkey, UserAccount>,
    validators: ValidatorsAccount,
}
   
impl AccountsDB {
    pub fn new() -> Self {
        Self {
            latest_blockhash: Blockhash::default(),
            accounts: DashMap::new(),
            validators: ValidatorsAccount::new(),
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

    pub fn add_validator(&mut self, validator: &Pubkey) {
        self.validators.validators.insert(*validator);
    }

    pub fn is_validator(&self, validator: &Pubkey) -> bool {
        self.validators.validators.contains(validator)
    }
}