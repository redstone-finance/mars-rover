use std::rc::Rc;

use anyhow::{anyhow, bail, Context, Result};
use napi::Error;
use soroban_env_common::xdr::{
    AccountEntry, AccountEntryExt, AccountId, LedgerEntry, LedgerEntryData, LedgerKey,
    LedgerKeyAccount, Limits, OperationResultTr, ReadXdr, SequenceNumber, String32, Thresholds,
    TransactionEnvelope, TransactionResultResult, TransactionV1Envelope,
};
use soroban_env_host::{
    e2e_testutils::ledger_entry,
    storage::SnapshotSource,
    xdr::{
        ContractDataDurability, Hash, InvokeHostFunctionResult, LedgerKeyContractData,
        OperationResult, ScAddress, ScVal, TransactionResult, WriteXdr,
    },
    LedgerInfo,
};

use crate::{
    executor::{ExecutionResult, Executor},
    ledger_info::{get_initial_ledger_info, NETWORK_PASSPHRASE},
    memory::Memory,
    model::{
        BaseSendTransactionResponse, GetFailedTransactionResponse, GetMissingTransactionResponse,
        GetSuccessfulTransactionResponse, GetTransactionResponse, LedgerEntryResult,
        SendTransactionResponse, SendTransactionStatus, TransactionEvents,
    },
    tx_storage::{TransactionInfo, TxStorage},
    utils::{failed_result, tx_hash},
    validation::TxValidation,
    NetworkInfo,
};

pub struct Sandbox {
    memory: Rc<Memory>,
    ledger_info: LedgerInfo,
    executor: Executor,
    validator: TxValidation,
    tx_storage: TxStorage,
}

impl Sandbox {
    pub fn new() -> Self {
        let memory = Rc::new(Memory::default());
        let ledger_info = get_initial_ledger_info();
        let executor = Executor::new(memory.clone());
        let validator = TxValidation::new(memory.clone());

        Self {
            memory,
            ledger_info,
            executor,
            validator,
            tx_storage: TxStorage::default(),
        }
    }

    pub fn get_ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }

    pub fn set_time(&mut self, time: i64) {
        self.ledger_info.timestamp = time as u64;
    }

    pub fn set_sequence(&mut self, seq: u32) {
        self.ledger_info.sequence_number = seq;
    }

    pub fn fund_account(&self, account: String, balance: i64) -> Result<()> {
        let account_id = AccountId::from_xdr_base64(account, Limits::none())?;

        let signers = vec![].try_into()?;

        let account_entry = AccountEntry {
            account_id,
            balance,
            seq_num: SequenceNumber::from(0),
            inflation_dest: None,
            ext: AccountEntryExt::V0,
            flags: 0,
            home_domain: String32::default(),
            thresholds: Thresholds([1, 0, 0, 0]),
            signers,
            num_sub_entries: 0,
        };

        let entry = ledger_entry(LedgerEntryData::Account(account_entry));
        self.memory.insert(entry);

        Ok(())
    }

    pub fn get_account(&self, account: String) -> Result<String> {
        let account = self.get_account_from_string(account)?;

        Ok(serde_json::to_string(&account)?)
    }

    pub fn get_balance(&self, account: String) -> Result<i64> {
        self.get_account_from_string(account).map(|x| x.balance)
    }

    fn get_account_from_string(&self, account: String) -> Result<AccountEntry> {
        let account_id = AccountId::from_xdr_base64(account.clone(), Limits::none())?;
        let key = LedgerKey::from(LedgerKeyAccount { account_id });

        self.memory
            .get_account(Rc::new(key))
            .map_err(|e| anyhow!("memory access error: {:?}", e))?
            .ok_or_else(|| anyhow!("account not found"))
    }

    pub fn simulate_tx(&self, transaction_envelope: String) -> Result<String> {
        let te = TransactionEnvelope::from_xdr_base64(&transaction_envelope, Limits::none())?;

        let envelope = match te {
            TransactionEnvelope::Tx(envelope) => envelope,
            _ => bail!("Unsupported transaction type"),
        };

        let response = self
            .executor
            .simulate_transaction(envelope, &self.ledger_info)?;

        Ok(serde_json::to_string(&response)?)
    }

    fn apply_account_changes(&self, account_id: AccountId) -> Result<()> {
        let key = Rc::new(LedgerKey::from(LedgerKeyAccount { account_id }));

        let (entry, ttl) = self.memory.get(&key)?.ok_or(anyhow!("No entry"))?;

        let mut account = match &entry.data {
            LedgerEntryData::Account(account_entry) => account_entry.clone(),
            _ => bail!("account not found"),
        };

        account.seq_num = SequenceNumber(account.seq_num.0 + 1);

        let entry = LedgerEntry {
            data: LedgerEntryData::Account(account),
            last_modified_ledger_seq: self.ledger_info.sequence_number,
            ext: entry.ext.clone(),
        };

        self.memory.insert_with_ttl(entry, ttl);

        Ok(())
    }

    pub fn get_network_info(&self) -> napi::Result<String> {
        let network_info = NetworkInfo {
            passphrase: NETWORK_PASSPHRASE.to_string(),
            protocol_version: self.ledger_info.protocol_version.to_string(),
        };

        serde_json::to_string(&network_info)
            .map_err(|e| Error::from_reason(format!("network info serialization failed: {}", e)))
    }

    pub fn send_transaction(
        &mut self,
        transaction_envelope: String,
    ) -> Result<SendTransactionResponse> {
        let te = TransactionEnvelope::from_xdr_base64(&transaction_envelope, Limits::none())
            .map_err(|e| Error::from_reason(format!("invalid transaction envelope: {}", e)))?;

        let envelope = match te {
            TransactionEnvelope::Tx(envelope) => envelope,
            _ => bail!("Unsupported transaction type"),
        };

        let result = self.send_transaction_inner(&envelope);

        let account_id = envelope.tx.source_account.clone().account_id();
        self.apply_account_changes(account_id)?;

        let hash = tx_hash(&envelope, &self.ledger_info)?;
        let hash = hex::encode(hash);

        let result = match result {
            Ok(result) => result,
            Err(e) => {
                self.tx_storage.insert(
                    hash,
                    TransactionInfo {
                        envelope,
                        result: Err(e.to_string()),
                        events: vec![],
                        ledger_info: self.ledger_info.clone(),
                    },
                );

                return Err(e);
            },
        };

        let status = match &result.result {
            Ok(_) => SendTransactionStatus::Pending,
            _ => SendTransactionStatus::Error,
        };

        let response = SendTransactionResponse {
            base: BaseSendTransactionResponse {
                status,
                hash: hash.clone(),
                latest_ledger: self.ledger_info.sequence_number,
                latest_ledger_close_time: self.ledger_info.timestamp,
            },
            error_result: result.error.clone().map(|error| TransactionResult {
                fee_charged: result.fee_charges,
                result: error,
                ext: Default::default(),
            }),
            diagnostic_events: result.error.is_some().then_some(result.events.clone()),
        };

        self.tx_storage.insert(
            hash,
            TransactionInfo {
                envelope,
                result: result.result.map_err(|e| e.to_string()),
                events: result.events,
                ledger_info: self.ledger_info.clone(),
            },
        );

        Ok(response)
    }

    pub fn send_transaction_inner(
        &self,
        envelope: &TransactionV1Envelope,
    ) -> Result<ExecutionResult> {
        self.validator.validate(envelope, &self.ledger_info)?;

        let result = self
            .executor
            .send_transaction(envelope, &self.ledger_info)
            .map_err(|e| anyhow!("transaction execution failed: {:?}", e))?;

        Ok(result)
    }

    pub fn get_contract_data(
        &self,
        contract_address: String,
        key: String,
        durability: String,
    ) -> Result<LedgerEntryResult> {
        let contract = ScAddress::from_xdr_base64(contract_address, Limits::none())
            .context("Invalid contract address XDR")?;
        let key = ScVal::from_xdr_base64(key, Limits::none()).context("Invalid key XDR")?;

        let durability = match durability.as_str() {
            "persistent" => ContractDataDurability::Persistent,
            "temporary" => ContractDataDurability::Temporary,
            _ => bail!("Invalid durability: {}", durability),
        };

        let key = LedgerKey::from(LedgerKeyContractData {
            contract: contract.clone(),
            key,
            durability,
        });

        let (entry, ttl) = self.memory.get(&Rc::new(key.clone()))?.ok_or(anyhow!(
            "No data for contract {contract} under key: {key:?}"
        ))?;

        Ok(LedgerEntryResult {
            last_modified_ledger_seq: Some(entry.last_modified_ledger_seq),
            key: key.to_xdr_base64(Limits::none())?,
            val: entry.data.to_xdr_base64(Limits::none())?,
            live_until_ledger_seq: ttl,
        })
    }

    pub fn get_transaction(&self, hash: String) -> Result<GetTransactionResponse> {
        let ti = match self.tx_storage.get(&hash) {
            Some(ti) => ti,
            None => {
                return Ok(GetTransactionResponse::NotFound(
                    GetMissingTransactionResponse {
                        tx_hash: hash,
                        latest_ledger: self.ledger_info.sequence_number,
                        latest_ledger_close_time: self.ledger_info.timestamp,
                        oldest_ledger: 0,
                        oldest_ledger_close_time: 0,
                    },
                ))
            },
        };

        match &ti.result {
            Ok(result) => Ok(GetTransactionResponse::Success(
                GetSuccessfulTransactionResponse {
                    tx_hash: hash.clone(),
                    latest_ledger: self.ledger_info.sequence_number,
                    latest_ledger_close_time: self.ledger_info.timestamp,
                    oldest_ledger: 0,
                    oldest_ledger_close_time: 0,
                    ledger: ti.ledger_info.sequence_number,
                    created_at: ti.ledger_info.timestamp,
                    application_order: 0,
                    fee_bump: false,
                    envelope_xdr: TransactionEnvelope::Tx(ti.envelope.clone())
                        .to_xdr_base64(Limits::none())?,
                    result_xdr: TransactionResult {
                        fee_charged: ti.envelope.tx.fee as i64,
                        result: TransactionResultResult::TxSuccess(
                            vec![OperationResult::OpInner(
                                OperationResultTr::InvokeHostFunction(
                                    InvokeHostFunctionResult::Success(Hash(
                                        hex::decode(hash)?
                                            .try_into()
                                            .map_err(|e| anyhow!("coudl not decode {e:?}"))?,
                                    )),
                                ),
                            )]
                            .try_into()?,
                        ),
                        ext: Default::default(),
                    }
                    .to_xdr_base64(Limits::none())?,
                    result_meta_xdr: Default::default(),
                    diagnostic_events_xdr: None,
                    return_value: Some(result.clone()),
                    events: TransactionEvents {
                        transaction_events_xdr: vec![],
                        contract_events_xdr: vec![ti
                            .events
                            .iter()
                            .map(|e| e.event.clone())
                            .collect()],
                    },
                },
            )),
            Err(_) => Ok(GetTransactionResponse::Failed(
                GetFailedTransactionResponse {
                    tx_hash: hash.clone(),
                    latest_ledger: self.ledger_info.sequence_number,
                    latest_ledger_close_time: self.ledger_info.timestamp,
                    oldest_ledger: 0,
                    oldest_ledger_close_time: 0,
                    ledger: ti.ledger_info.sequence_number,
                    created_at: ti.ledger_info.timestamp,
                    application_order: 0,
                    fee_bump: false,
                    envelope_xdr: TransactionEnvelope::Tx(ti.envelope.clone())
                        .to_xdr_base64(Limits::none())?,
                    result_xdr: TransactionResult {
                        fee_charged: ti.envelope.tx.fee as i64,
                        result: failed_result()?,
                        ext: Default::default(),
                    }
                    .to_xdr_base64(Limits::none())?,
                    result_meta_xdr: Default::default(),
                    diagnostic_events_xdr: None,
                    events: TransactionEvents {
                        transaction_events_xdr: vec![],
                        contract_events_xdr: vec![ti
                            .events
                            .iter()
                            .map(|e| e.event.clone())
                            .collect()],
                    },
                },
            )),
        }
    }
}
