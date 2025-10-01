use sha2::{Digest, Sha256};
use soroban_env_host::{e2e_testutils::default_ledger_info, LedgerInfo};

pub const NETWORK_PASSPHRASE: &str = "mars-rover; sandbox environment";

pub fn get_initial_ledger_info() -> LedgerInfo {
    let mut li = default_ledger_info();

    let hash = Sha256::digest(NETWORK_PASSPHRASE.as_bytes());
    li.network_id = hash.into();

    li
}
