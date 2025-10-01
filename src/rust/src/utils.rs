use std::collections::HashSet;

use anyhow::Context;
use sha2::{Digest, Sha256};
use soroban_env_common::xdr::{
    ContractCodeEntryExt, ContractCostType, Hash, InvokeHostFunctionResult, LedgerEntry,
    LedgerEntryChangeType, LedgerEntryData, LedgerKey, Limits, OperationResult, OperationResultTr,
    TransactionResultResult, TransactionSignaturePayload,
    TransactionSignaturePayloadTaggedTransaction, TransactionV1Envelope, TtlEntry,
};
use soroban_env_host::{
    budget::AsBudget, vm::VersionedContractCodeCostInputs, xdr::WriteXdr, LedgerInfo, ModuleCache,
};
use soroban_simulation::simulation::LedgerEntryDiff;

use crate::{model::LedgerEntryChange, module_cache::new_module_cache};

pub fn tx_hash(
    envelope: &TransactionV1Envelope,
    ledger_info: &LedgerInfo,
) -> anyhow::Result<[u8; 32]> {
    let payload = TransactionSignaturePayload {
        network_id: Hash(ledger_info.network_id),
        tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(envelope.tx.clone()),
    };

    let payload = payload.to_xdr(Limits::none())?;

    Ok(Sha256::digest(&payload).into())
}

pub fn compute_key_hash(key: &LedgerKey) -> Vec<u8> {
    let key_xdr = key.to_xdr(Limits::none()).unwrap();
    let hash: [u8; 32] = Sha256::digest(&key_xdr).into();
    hash.to_vec()
}

pub fn sha256_hash_from_bytes_raw(bytes: &[u8], budget: impl AsBudget) -> anyhow::Result<[u8; 32]> {
    budget
        .as_budget()
        .charge(
            ContractCostType::ComputeSha256Hash,
            Some(bytes.len() as u64),
        )
        .context("Failed to charge budget for SHA256 hash")?;
    Ok(Sha256::digest(bytes).into())
}

pub fn ttl_entry(key: &LedgerKey, ttl: u32) -> TtlEntry {
    TtlEntry {
        key_hash: compute_key_hash(key).try_into().unwrap(),
        live_until_ledger_seq: ttl,
    }
}

pub fn build_module_cache_for_entries(
    ledger_info: &LedgerInfo,
    ledger_entries_with_ttl: Vec<(LedgerEntry, Option<u32>)>,
    restored_contracts: &HashSet<Hash>,
) -> anyhow::Result<ModuleCache> {
    let (cache, ctx) = new_module_cache().context("Failed to create new module cache")?;

    for (e, _) in ledger_entries_with_ttl.iter() {
        if let LedgerEntryData::ContractCode(cd) = &e.data {
            let contract_id = Hash(sha256_hash_from_bytes_raw(&cd.code, ctx.as_budget())?);
            if restored_contracts.contains(&contract_id) {
                continue;
            }
            let code_cost_inputs = match &cd.ext {
                ContractCodeEntryExt::V0 => VersionedContractCodeCostInputs::V0 {
                    wasm_bytes: cd.code.len(),
                },
                ContractCodeEntryExt::V1(v1) => {
                    VersionedContractCodeCostInputs::V1(v1.cost_inputs.clone())
                },
            };
            cache
                .parse_and_cache_module(
                    &ctx,
                    ledger_info.protocol_version,
                    &contract_id,
                    &cd.code,
                    code_cost_inputs,
                )
                .context("Failed to parse and cache module")?;
        }
    }
    Ok(cache)
}

pub fn changes_from_simulation(changes: Vec<LedgerEntryDiff>) -> Vec<LedgerEntryChange> {
    changes
        .into_iter()
        .map(|diff| {
            let change_type = match (diff.state_before.is_some(), diff.state_after.is_some()) {
                (true, true) => LedgerEntryChangeType::Updated,
                (true, false) => LedgerEntryChangeType::Removed,
                (false, true) => LedgerEntryChangeType::Updated,
                _ => panic!("unexpected"),
            };

            let key = match (&diff.state_after, &diff.state_before) {
                (Some(entry), _) => entry.to_key(),
                (_, Some(entry)) => entry.to_key(),
                _ => panic!("unexpected"),
            };

            LedgerEntryChange {
                change_type,
                key,
                before: diff.state_before,
                after: diff.state_after,
            }
        })
        .collect()
}

pub fn failed_result() -> anyhow::Result<TransactionResultResult> {
    Ok(TransactionResultResult::TxFailed(
        vec![OperationResult::OpInner(
            OperationResultTr::InvokeHostFunction(InvokeHostFunctionResult::Trapped),
        )]
        .try_into()?,
    ))
}
