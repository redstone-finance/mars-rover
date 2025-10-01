use std::{collections::HashSet, rc::Rc};

use anyhow::{ensure, Context, Result};
use soroban_env_host::{
    budget::Budget,
    e2e_invoke::{self, InvokeHostFunctionResult, LedgerEntryChange, RecordingInvocationAuthMode},
    storage::SnapshotSource,
    xdr::{
        AccountId, ContractCostParamEntry, ContractCostParams, ContractEvent, DiagnosticEvent,
        ExtensionPoint, HostFunction, LedgerEntry, LedgerEntryData, LedgerKey,
        LedgerKeyContractCode, LedgerKeyContractData, Limits, OperationBody, ReadXdr,
        SorobanAuthorizationEntry, SorobanResources, SorobanTransactionDataExt, TransactionExt,
        TransactionResultResult, TransactionV1Envelope, WriteXdr,
    },
    HostError, LedgerInfo,
};
use soroban_simulation::simulation::{
    simulate_invoke_host_function_op, SimulationAdjustmentConfig,
};

use crate::{
    memory::Memory,
    model::{
        SimulateHostFunctionResult, SimulateTransactionErrorResponse, SimulateTransactionResponse,
        SimulateTransactionSuccessResponse,
    },
    network_config::default_network_config,
    utils::{build_module_cache_for_entries, changes_from_simulation, failed_result, ttl_entry},
};

pub struct ExecutionResult {
    pub error: Option<TransactionResultResult>,
    pub fee_charges: i64,
    pub result: Result<Vec<u8>, HostError>,
    pub events: Vec<DiagnosticEvent>,
}

pub struct Executor {
    memory: Rc<Memory>,
}

impl Executor {
    pub fn new(memory: Rc<Memory>) -> Self {
        Self { memory }
    }

    pub fn simulate_transaction(
        &self,
        transaction_envelope: TransactionV1Envelope,
        ledger_info: &LedgerInfo,
    ) -> Result<SimulateTransactionResponse> {
        let host_function_op = match &transaction_envelope.tx.operations[0].body {
            OperationBody::InvokeHostFunction(host) => host,
            _ => return Err(anyhow::anyhow!("Expected InvokeHostFunction operation")),
        };

        let network_config = default_network_config()?;
        let simulation = simulate_invoke_host_function_op(
            self.memory.clone(),
            &network_config,
            &SimulationAdjustmentConfig::no_adjustments(),
            ledger_info,
            host_function_op.host_function.clone(),
            RecordingInvocationAuthMode::Recording(true),
            &transaction_envelope.tx.source_account.account_id(),
            [1; 32],
            true,
        )
        .context("Failed to simulate invoke host function operation")?;

        if simulation.invoke_result.is_err() {
            let err = simulation.invoke_result.unwrap_err();
            return Ok(SimulateTransactionResponse::Error(
                SimulateTransactionErrorResponse {
                    error: err.to_string(),
                    id: "1".into(),
                    parsed: true,
                    events: simulation.diagnostic_events,
                    latest_ledger: ledger_info.sequence_number,
                },
            ));
        }

        let changes = changes_from_simulation(simulation.modified_entries);
        let tx_data = simulation
            .transaction_data
            .ok_or_else(|| anyhow::anyhow!("Transaction data missing from simulation"))?;

        let response = SimulateTransactionResponse::Success(SimulateTransactionSuccessResponse {
            id: "1".into(),
            latest_ledger: ledger_info.sequence_number,
            events: simulation.diagnostic_events,
            min_resource_fee: tx_data.resource_fee.to_string(),
            parsed: true,
            result: Some(SimulateHostFunctionResult {
                retval: simulation.invoke_result?.to_xdr_base64(Limits::none())?,
                auth: simulation
                    .auth
                    .into_iter()
                    .map(|auth| auth.to_xdr_base64(Limits::none()))
                    .collect::<Result<Vec<_>, _>>()
                    .context("Failed to convert auth to XDR base64")?,
            }),
            state_changes: Some(changes),
            transaction_data: tx_data
                .to_xdr_base64(Limits::none())
                .context("Failed to convert transaction data to XDR base64")?,
        });

        Ok(response)
    }

    pub fn send_transaction(
        &self,
        transaction_envelope: &TransactionV1Envelope,
        ledger_info: &LedgerInfo,
    ) -> Result<ExecutionResult> {
        ensure!(
            transaction_envelope.tx.operations.len() == 1,
            "Only single operation is supported"
        );

        let host_function_op = match &transaction_envelope.tx.operations[0].body {
            OperationBody::InvokeHostFunction(host) => host,
            _ => return Err(anyhow::anyhow!("Expected InvokeHostFunction operation")),
        };

        let soroban_data = match &transaction_envelope.tx.ext {
            TransactionExt::V1(ext) => ext.clone(),
            _ => return Err(anyhow::anyhow!("Expected transaction extension V1")),
        };

        let resources = &soroban_data.resources;

        let restored_entry_indices = match soroban_data.ext {
            SorobanTransactionDataExt::V1(ext) => ext.archived_soroban_entries.into_vec(),
            _ => vec![],
        };

        let result = self.invoke_host_function(
            &host_function_op.host_function,
            resources,
            &transaction_envelope.tx.source_account.clone().account_id(),
            host_function_op.auth.clone().into_vec(),
            &restored_entry_indices,
            [0; 32],
            true,
            ledger_info,
        )?;

        self.apply_ledger_changes(result.ledger_changes)?;

        let error = result
            .encoded_invoke_result
            .is_err()
            .then_some(failed_result()?);
        let out = result.encoded_invoke_result.clone();

        let events = result
            .encoded_contract_events
            .iter()
            .map(|encoded| ContractEvent::from_xdr(encoded, Limits::none()).unwrap())
            .map(|e| DiagnosticEvent {
                in_successful_contract_call: error.is_none(),
                event: e,
            })
            .collect();

        let result = ExecutionResult {
            error,
            fee_charges: transaction_envelope.tx.fee as i64,
            result: out,
            events,
        };

        Ok(result)
    }

    pub fn apply_ledger_changes(&self, changes: Vec<LedgerEntryChange>) -> Result<()> {
        for change in changes {
            let key = LedgerKey::from_xdr(change.encoded_key, Limits::none())
                .context("Failed to decode ledger key from XDR")?;

            let ttl = change.ttl_change.map(|ttl| ttl.new_live_until_ledger);

            match change.encoded_new_value {
                Some(encoded_entry) => {
                    let entry = LedgerEntry::from_xdr(encoded_entry, Limits::none())
                        .context("Failed to decode ledger entry from XDR")?;
                    self.memory.insert_with_ttl(entry, ttl);
                },
                None if !change.read_only => {
                    self.memory.remove(&Rc::new(key));
                },
                _ => {
                    self.memory.update_ttl(&Rc::new(key), ttl);
                },
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn invoke_host_function(
        &self,
        host_fn: &HostFunction,
        resources: &SorobanResources,
        source_account: &AccountId,
        auth_entries: Vec<SorobanAuthorizationEntry>,
        restored_entry_indices: &[u32],
        prng_seed: [u8; 32],
        enable_diagnostics: bool,
        ledger_info: &LedgerInfo,
    ) -> Result<InvokeHostFunctionResult> {
        let limits = Limits::none();

        let encoded_host_fn = host_fn
            .to_xdr(limits.clone())
            .context("Failed to encode host function to XDR")?;
        let encoded_resources = resources
            .to_xdr(limits.clone())
            .context("Failed to encode resources to XDR")?;
        let encoded_source_account = source_account
            .to_xdr(limits.clone())
            .context("Failed to encode source account to XDR")?;

        let encoded_auth_entries: Result<Vec<Vec<u8>>> = auth_entries
            .iter()
            .map(|entry| {
                entry
                    .to_xdr(limits.clone())
                    .context("Failed to encode auth entry to XDR")
            })
            .collect();
        let encoded_auth_entries = encoded_auth_entries?;

        let mut entries_with_ttl = Vec::new();
        let all_keys = resources
            .footprint
            .read_only
            .iter()
            .chain(resources.footprint.read_write.iter());

        for key in all_keys {
            if let Some((entry_rc, ttl)) = self
                .memory
                .get(&Rc::new(key.clone()))
                .context("Failed to get entry from memory")?
            {
                entries_with_ttl.push((entry_rc, ttl));
            }
        }

        let encoded_ledger_entries: Result<Vec<Vec<u8>>> = entries_with_ttl
            .iter()
            .map(|(entry, _)| {
                entry
                    .to_xdr(limits.clone())
                    .context("Failed to encode ledger entry to XDR")
            })
            .collect();
        let encoded_ledger_entries = encoded_ledger_entries?;

        let encoded_ttl_entries: Result<Vec<Vec<u8>>, _> = entries_with_ttl
            .iter()
            .filter_map(|(entry, ttl)| {
                let key = match &entry.data {
                    LedgerEntryData::ContractData(cd) => {
                        Some(LedgerKey::ContractData(LedgerKeyContractData {
                            contract: cd.contract.clone(),
                            key: cd.key.clone(),
                            durability: cd.durability,
                        }))
                    },
                    LedgerEntryData::ContractCode(code) => {
                        Some(LedgerKey::ContractCode(LedgerKeyContractCode {
                            hash: code.hash.clone(),
                        }))
                    },
                    _ => None,
                };

                key.and_then(|k| {
                    ttl.map(|ttl_value| ttl_entry(&k, ttl_value).to_xdr(limits.clone()))
                })
            })
            .collect();

        let encoded_ttl_entries = encoded_ttl_entries?;

        let mut restored_contracts = HashSet::new();
        for index in restored_entry_indices {
            if let Some(LedgerKey::ContractCode(code)) =
                resources.footprint.read_write.get(*index as usize)
            {
                restored_contracts.insert(code.hash.clone());
            }
        }

        let ledger_entries_for_cache: Vec<(LedgerEntry, Option<u32>)> = entries_with_ttl
            .iter()
            .map(|(entry_rc, ttl)| ((**entry_rc).clone(), *ttl))
            .collect();

        let module_cache = build_module_cache_for_entries(
            ledger_info,
            ledger_entries_for_cache,
            &restored_contracts,
        )?;

        let cpu_cost_params = ContractCostParams(
            vec![
                ContractCostParamEntry {
                    ext: ExtensionPoint::V0,
                    const_term: 35,
                    linear_term: 36,
                },
                ContractCostParamEntry {
                    ext: ExtensionPoint::V0,
                    const_term: 37,
                    linear_term: 38,
                },
            ]
            .try_into()?,
        );
        let mem_cost_params = ContractCostParams(
            vec![
                ContractCostParamEntry {
                    ext: ExtensionPoint::V0,
                    const_term: 39,
                    linear_term: 40,
                },
                ContractCostParamEntry {
                    ext: ExtensionPoint::V0,
                    const_term: 41,
                    linear_term: 42,
                },
                ContractCostParamEntry {
                    ext: ExtensionPoint::V0,
                    const_term: 43,
                    linear_term: 44,
                },
            ]
            .try_into()?,
        );

        let budget =
            Budget::try_from_configs(u64::MAX, u64::MAX, cpu_cost_params, mem_cost_params)?;

        let mut diagnostic_events = Vec::new();

        let result = e2e_invoke::invoke_host_function(
            &budget,
            enable_diagnostics,
            encoded_host_fn,
            encoded_resources,
            restored_entry_indices,
            encoded_source_account,
            encoded_auth_entries.into_iter(),
            ledger_info.clone(),
            encoded_ledger_entries.into_iter(),
            encoded_ttl_entries.into_iter(),
            prng_seed.to_vec(),
            &mut diagnostic_events,
            None,
            Some(module_cache),
        )
        .context("Failed to invoke host function")?;

        Ok(result)
    }
}
