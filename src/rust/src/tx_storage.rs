use std::collections::HashMap;

use soroban_env_common::xdr::DiagnosticEvent;
use soroban_env_host::{xdr::TransactionV1Envelope, LedgerInfo};

pub struct TransactionInfo {
    pub envelope: TransactionV1Envelope,
    pub result: Result<Vec<u8>, String>,
    pub ledger_info: LedgerInfo,
    pub events: Vec<DiagnosticEvent>,
}

#[derive(Default)]
pub struct TxStorage {
    storage: HashMap<String, TransactionInfo>,
}

impl TxStorage {
    pub fn insert(&mut self, tx_hash: String, transaction_info: TransactionInfo) {
        self.storage.insert(tx_hash, transaction_info);
    }

    pub fn get(&self, tx_hash: &str) -> Option<&TransactionInfo> {
        self.storage.get(tx_hash)
    }
}
