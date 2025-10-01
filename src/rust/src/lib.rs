use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Serialize;

use crate::{ledger_info::NETWORK_PASSPHRASE, sandbox::Sandbox};

mod executor;
mod ledger_info;
mod memory;
mod model;
mod module_cache;
mod network_config;
mod sandbox;
mod tx_storage;
mod utils;
mod validation;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkInfo {
    passphrase: String,
    protocol_version: String,
}

#[napi]
pub struct MarsRover {
    sandbox: Sandbox,
}

impl Default for MarsRover {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl MarsRover {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            sandbox: Sandbox::new(),
        }
    }

    #[napi]
    pub fn set_time(&mut self, time: i64) {
        self.sandbox.set_time(time);
    }

    #[napi]
    pub fn set_sequence(&mut self, seq: u32) {
        self.sandbox.set_sequence(seq);
    }

    #[napi]
    pub fn get_ledger_info(&self) -> Result<String> {
        let info: crate::model::LedgerInfo = self.sandbox.get_ledger_info().clone().into();

        serde_json::to_string(&info).map_err(|err| Error::from_reason(err.to_string()))
    }

    #[napi]
    pub fn fund_account(&self, account: String, balance: i64) -> Result<()> {
        self.sandbox
            .fund_account(account, balance)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn get_account(&self, account: String) -> Result<String> {
        self.sandbox
            .get_account(account)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn get_balance(&self, account: String) -> Result<String> {
        self.sandbox
            .get_balance(account)
            .map(|balance| balance.to_string())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn simulate_tx(&self, transaction_envelope: String) -> Result<String> {
        self.sandbox
            .simulate_tx(transaction_envelope)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn send_transaction(&mut self, transaction_envelope: String) -> Result<String> {
        let res = self
            .sandbox
            .send_transaction(transaction_envelope)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        serde_json::to_string(&res).map_err(|err| Error::from_reason(err.to_string()))
    }

    #[napi]
    pub fn network_passphrase(&self) -> String {
        NETWORK_PASSPHRASE.to_string()
    }

    #[napi]
    pub fn get_network_info(&self) -> Result<String> {
        self.sandbox
            .get_network_info()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn get_contract_data(
        &self,
        contract_address: String,
        key: String,
        durability: String,
    ) -> Result<String> {
        let response = self
            .sandbox
            .get_contract_data(contract_address, key, durability)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        serde_json::to_string(&response).map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn get_transaction(&self, hash: String) -> Result<String> {
        let response = self
            .sandbox
            .get_transaction(hash)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        serde_json::to_string(&response).map_err(|e| Error::from_reason(e.to_string()))
    }
}
