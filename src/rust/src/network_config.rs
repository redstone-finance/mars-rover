use std::rc::Rc;

use soroban_env_host::{
    e2e_testutils::ledger_entry,
    fees::{FeeConfiguration, RentFeeConfiguration},
    xdr::{
        ConfigSettingContractBandwidthV0, ConfigSettingContractComputeV0,
        ConfigSettingContractEventsV0, ConfigSettingContractHistoricalDataV0,
        ConfigSettingContractLedgerCostExtV0, ConfigSettingContractLedgerCostV0,
        ConfigSettingEntry, ContractCostParamEntry, ContractCostParams, ExtensionPoint,
        LedgerEntry, LedgerEntryData, StateArchivalSettings,
    },
};
use soroban_simulation::NetworkConfig;

use crate::{ledger_info::get_initial_ledger_info, memory::Memory};

fn _config_entry(entry: ConfigSettingEntry) -> (LedgerEntry, Option<u32>) {
    (ledger_entry(LedgerEntryData::ConfigSetting(entry)), None)
}
pub fn default_network_config() -> anyhow::Result<NetworkConfig> {
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

    let ledger_info = get_initial_ledger_info();

    Ok(NetworkConfig {
        fee_configuration: FeeConfiguration {
            fee_per_instruction_increment: 10,
            fee_per_disk_read_entry: 20,
            fee_per_write_entry: 30,
            fee_per_disk_read_1kb: 40,
            fee_per_write_1kb: 50,
            fee_per_historical_1kb: 60,
            fee_per_contract_event_1kb: 70,
            fee_per_transaction_size_1kb: 80,
        },
        rent_fee_configuration: RentFeeConfiguration {
            fee_per_rent_1kb: 100,
            fee_per_write_1kb: 50,
            fee_per_write_entry: 30,
            persistent_rent_rate_denominator: 100,
            temporary_rent_rate_denominator: 1000,
        },
        tx_max_instructions: 100_000_000,
        tx_memory_limit: 40_000_000,
        cpu_cost_params: ContractCostParams(cpu_cost_params.into()),
        memory_cost_params: ContractCostParams(mem_cost_params.into()),
        min_temp_entry_ttl: ledger_info.min_temp_entry_ttl,
        min_persistent_entry_ttl: ledger_info.min_persistent_entry_ttl,
        max_entry_ttl: ledger_info.max_entry_ttl,
    })
}

pub fn _populate_memory_with_config_entries(memory: Rc<Memory>) {
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
        .try_into()
        .unwrap(),
    );
    let memory_cost_params = ContractCostParams(
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
        .try_into()
        .unwrap(),
    );
    let entries = vec![
        _config_entry(ConfigSettingEntry::ContractComputeV0(
            ConfigSettingContractComputeV0 {
                ledger_max_instructions: 1,
                tx_max_instructions: 2,
                fee_rate_per_instructions_increment: 3,
                tx_memory_limit: 4,
            },
        )),
        _config_entry(ConfigSettingEntry::ContractLedgerCostV0(
            ConfigSettingContractLedgerCostV0 {
                ledger_max_disk_read_entries: 5,
                ledger_max_disk_read_bytes: 6,
                ledger_max_write_ledger_entries: 7,
                ledger_max_write_bytes: 8,
                tx_max_disk_read_entries: 9,
                tx_max_disk_read_bytes: 10,
                tx_max_write_ledger_entries: 11,
                tx_max_write_bytes: 12,
                fee_disk_read_ledger_entry: 13,
                fee_write_ledger_entry: 14,
                fee_disk_read1_kb: 15,
                // From tests/resources `test_compute_write_fee`
                soroban_state_target_size_bytes: 100_000_000_000_000,
                rent_fee1_kb_soroban_state_size_low: 1_000_000,
                rent_fee1_kb_soroban_state_size_high: 1_000_000_000,
                soroban_state_rent_fee_growth_factor: 50,
            },
        )),
        _config_entry(ConfigSettingEntry::ContractLedgerCostExtV0(
            ConfigSettingContractLedgerCostExtV0 {
                tx_max_footprint_entries: 16,
                fee_write1_kb: 17,
            },
        )),
        _config_entry(ConfigSettingEntry::ContractHistoricalDataV0(
            ConfigSettingContractHistoricalDataV0 {
                fee_historical1_kb: 20,
            },
        )),
        _config_entry(ConfigSettingEntry::ContractEventsV0(
            ConfigSettingContractEventsV0 {
                tx_max_contract_events_size_bytes: 21,
                fee_contract_events1_kb: 22,
            },
        )),
        _config_entry(ConfigSettingEntry::ContractBandwidthV0(
            ConfigSettingContractBandwidthV0 {
                ledger_max_txs_size_bytes: 23,
                tx_max_size_bytes: 24,
                fee_tx_size1_kb: 25,
            },
        )),
        _config_entry(ConfigSettingEntry::StateArchival(StateArchivalSettings {
            max_entry_ttl: 26,
            min_temporary_ttl: 27,
            min_persistent_ttl: 28,
            persistent_rent_rate_denominator: 29,
            temp_rent_rate_denominator: 30,
            max_entries_to_archive: 31,
            live_soroban_state_size_window_sample_size: 32,
            live_soroban_state_size_window_sample_period: 33,
            eviction_scan_size: 34,
            starting_eviction_scan_level: 35,
        })),
        _config_entry(ConfigSettingEntry::ContractCostParamsCpuInstructions(
            cpu_cost_params.clone(),
        )),
        _config_entry(ConfigSettingEntry::ContractCostParamsMemoryBytes(
            memory_cost_params.clone(),
        )),
    ];

    for (entry, ttl) in entries {
        memory.insert_with_ttl(entry, ttl);
    }
}
