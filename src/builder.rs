use std::sync::{Arc, RwLock};
use crate::{
    db::AccountsDB,
    structures::{Block, Blockhash, Pubkey, ValidatorAccount, TransactionSign},
    pool::{Mempool, MAX_TRANSACTIONS_PER_BLOCK},
};

#[derive(Default, Debug, Clone)]
pub struct BlockBuilder {
    pub mempool: Arc<RwLock<Mempool>>,
    pub db: Arc<RwLock<AccountsDB>>,
}

impl BlockBuilder {
    pub fn new(mempool: Arc<RwLock<Mempool>>, db: Arc<RwLock<AccountsDB>>) -> Self {
        Self { mempool, db }
    }

    pub fn build_genesis(&self) -> Block {
        Block::create_genesis()
    }

    pub fn build(&self, prev_hash: Blockhash) -> Result<Block, &'static str> {
        // Acquire locks on mempool & accountsdb
        let mempool_lock = self.mempool.read().unwrap();
        let db_lock = self.db.read().unwrap();

        if mempool_lock.pool.len() >= MAX_TRANSACTIONS_PER_BLOCK {
            let transactions = mempool_lock.get_transactions_for_block();

            for tx in &transactions {
                let signer: Pubkey = tx.get_signer();
                if !tx.verify_signature(&signer) {
                    return Err("Invalid transaction signature");
                }
                if !tx.validate(&*db_lock) {
                    return Err("Invalid transaction in block building");
                }
            }
    
            let block = Block::new(transactions, prev_hash);
    
            Ok(block)

        } else {
            Ok(self.build_genesis())
        }
    }

    pub fn get_leader(&self) -> ValidatorAccount {
        let db_lock = self.db.read().unwrap();
        db_lock.validators
            .iter()
            .max_by_key(|validator| validator.stake)
            .map(|entry| entry.clone())
            .unwrap()
    }

    pub fn validate_block(&self, block: &Block) -> Result<(), &'static str> {
        let db_lock = self.db.read().unwrap();

        for tx in &block.transactions {
            let signer: Pubkey = tx.get_signer();
            if !tx.verify_signature(&signer) {
                return Err("Invalid transaction signature");
            }
            if !tx.validate(&*db_lock) {
                return Err("Invalid transaction in block validation");
            }
        }
        
        Ok(())
    }
}
