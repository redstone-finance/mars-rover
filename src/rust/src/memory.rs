use std::{cell::RefCell, collections::BTreeMap, fmt, rc::Rc};

use anyhow::{anyhow, Result};
use napi::Error;
use soroban_env_common::xdr::LedgerEntryData;
use soroban_env_host::{
    storage::{EntryWithLiveUntil, SnapshotSource},
    xdr::{AccountEntry, LedgerEntry, LedgerKey},
    HostError,
};

type StorageMap = BTreeMap<Rc<LedgerKey>, EntryWithLiveUntil>;

#[derive(Default, Clone)]
pub struct Memory {
    memory: RefCell<StorageMap>,
}

impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let memory = self.memory.borrow();

        let mut map = f.debug_map();
        for (key, (entry, ttl)) in memory.iter() {
            // Format key and entry more concisely
            let key_str = format!("{:?}", key);
            let entry_str = format!("Entry({:?}), TTL: {:?}", entry.data, ttl);
            map.entry(&key_str, &entry_str);
        }
        map.finish()
    }
}

impl Memory {
    pub fn insert(&self, entry: LedgerEntry) {
        self.insert_with_ttl(entry, None);
    }

    pub fn insert_with_ttl(&self, entry: LedgerEntry, ttl: Option<u32>) {
        self.memory
            .borrow_mut()
            .insert(Rc::new(entry.to_key()), (Rc::new(entry), ttl));
    }

    pub fn update_ttl(&self, key: &Rc<LedgerKey>, new_ttl: Option<u32>) {
        self.memory
            .borrow_mut()
            .entry(key.clone())
            .and_modify(|(_, ttl)| *ttl = new_ttl);
    }

    pub fn remove(&self, key: &Rc<LedgerKey>) {
        self.memory.borrow_mut().remove(key);
    }

    pub fn get_account(&self, key: Rc<LedgerKey>) -> Result<Option<AccountEntry>> {
        let entry = self
            .get(&key)
            .map_err(|e| Error::from_reason(format!("memory access error: {:?}", e)))?;

        let entry = match entry {
            Some((entry, _)) => entry,
            _ => return Ok(None),
        };

        match &entry.data {
            LedgerEntryData::Account(account_entry) => Ok(Some(account_entry.clone())),
            _ => Err(anyhow!("account not found")),
        }
    }
}

impl SnapshotSource for Memory {
    fn get(&self, key: &Rc<LedgerKey>) -> Result<Option<EntryWithLiveUntil>, HostError> {
        let entry = self.memory.borrow().get(key).cloned();

        Ok(entry)
    }
}
