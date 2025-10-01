use serde::{Deserialize, Serialize};
use soroban_env_common::xdr::{TransactionEvent, TransactionResult};
use soroban_env_host::xdr::{
    ContractEvent, DiagnosticEvent, LedgerEntry, LedgerEntryChangeType, LedgerKey,
};

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum SimulateTransactionResponse {
    Success(SimulateTransactionSuccessResponse),
    Error(SimulateTransactionErrorResponse),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulateTransactionSuccessResponse {
    pub id: String,
    pub latest_ledger: u32,
    pub events: Vec<DiagnosticEvent>,
    #[serde(rename = "_parsed")]
    pub parsed: bool,
    pub transaction_data: String,
    pub min_resource_fee: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<SimulateHostFunctionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_changes: Option<Vec<LedgerEntryChange>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulateTransactionErrorResponse {
    pub id: String,
    pub latest_ledger: u32,
    pub events: Vec<DiagnosticEvent>,
    #[serde(rename = "_parsed")]
    pub parsed: bool,
    pub error: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntryChange {
    #[serde(rename = "type")]
    pub change_type: LedgerEntryChangeType,
    pub key: LedgerKey,
    pub before: Option<LedgerEntry>,
    pub after: Option<LedgerEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct SimulateHostFunctionResult {
    pub auth: Vec<String>,
    pub retval: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntryResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified_ledger_seq: Option<u32>,
    pub key: String,
    pub val: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_until_ledger_seq: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct LedgerInfo {
    pub protocol_version: u32,
    pub sequence_number: u32,
    pub timestamp: u64,
    pub network_id: [u8; 32],
    pub base_reserve: u32,
    pub min_temp_entry_ttl: u32,
    pub min_persistent_entry_ttl: u32,
    pub max_entry_ttl: u32,
}

impl From<soroban_env_host::LedgerInfo> for LedgerInfo {
    fn from(value: soroban_env_host::LedgerInfo) -> Self {
        Self {
            protocol_version: value.protocol_version,
            sequence_number: value.sequence_number,
            timestamp: value.timestamp,
            network_id: value.network_id,
            base_reserve: value.base_reserve,
            min_temp_entry_ttl: value.min_temp_entry_ttl,
            min_persistent_entry_ttl: value.min_persistent_entry_ttl,
            max_entry_ttl: value.max_entry_ttl,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Durability {
    Temporary,
    Persistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SendTransactionStatus {
    Pending,
    Duplicate,
    TryAgainLater,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseSendTransactionResponse {
    pub status: SendTransactionStatus,
    pub hash: String,
    pub latest_ledger: u32,
    pub latest_ledger_close_time: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTransactionResponse {
    #[serde(flatten)]
    pub base: BaseSendTransactionResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_result: Option<TransactionResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_events: Option<Vec<DiagnosticEvent>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GetTransactionStatus {
    Success,
    NotFound,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMissingTransactionResponse {
    pub tx_hash: String,
    pub latest_ledger: u32,
    pub latest_ledger_close_time: u64,
    pub oldest_ledger: u32,
    pub oldest_ledger_close_time: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFailedTransactionResponse {
    pub tx_hash: String,
    pub latest_ledger: u32,
    pub latest_ledger_close_time: u64,
    pub oldest_ledger: u32,
    pub oldest_ledger_close_time: u64,
    pub ledger: u32,
    pub created_at: u64,
    pub application_order: u32,
    pub fee_bump: bool,
    pub envelope_xdr: String,
    pub result_xdr: String,
    pub result_meta_xdr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_events_xdr: Option<Vec<DiagnosticEvent>>,
    pub events: TransactionEvents,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSuccessfulTransactionResponse {
    pub tx_hash: String,
    pub latest_ledger: u32,
    pub latest_ledger_close_time: u64,
    pub oldest_ledger: u32,
    pub oldest_ledger_close_time: u64,
    pub ledger: u32,
    pub created_at: u64,
    pub application_order: u32,
    pub fee_bump: bool,
    pub envelope_xdr: String,
    pub result_xdr: String,
    pub result_meta_xdr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_events_xdr: Option<Vec<DiagnosticEvent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_value: Option<Vec<u8>>,
    pub events: TransactionEvents,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum GetTransactionResponse {
    #[serde(rename = "NOT_FOUND")]
    NotFound(GetMissingTransactionResponse),
    #[serde(rename = "FAILED")]
    Failed(GetFailedTransactionResponse),
    #[serde(rename = "SUCCESS")]
    Success(GetSuccessfulTransactionResponse),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionEvents {
    pub transaction_events_xdr: Vec<TransactionEvent>,
    pub contract_events_xdr: Vec<Vec<ContractEvent>>,
}
