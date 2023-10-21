use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;
use crate::structures::{Transaction, Pubkey, TransactionSign};

const MAX_TRANSACTIONS_PER_BLOCK: usize = 2;

#[derive(Default, Debug)]
pub struct Mempool {
    pool: DashMap<u64, Transaction>,
    counter: AtomicU64,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            pool: DashMap::new(),
            counter: AtomicU64::new(0)
        }
    }

    pub fn send_transaction(&self, tx: Transaction) -> Result<u64, &'static str> {
        let signer: Pubkey = tx.get_signer();

        if !tx.verify_signature(&signer) {
           return Err("Signature invalid.")
        }

        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        self.pool.insert(id, tx);
        Ok(id)
    }

    pub fn get_transaction(&self, id: &u64) -> Option<Transaction> {
        self.pool.get(id).map(|tx| tx.clone())
    }

    pub fn cancel_transaction(&self, id: &u64) {
        self.pool.remove(id);
    }

    pub fn get_transactions_for_block(&self) -> Vec<Transaction> {
        self.pool.iter().take(MAX_TRANSACTIONS_PER_BLOCK).map(|tx| tx.clone()).collect()
    }
}