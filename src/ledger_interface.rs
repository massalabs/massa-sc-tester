const LEDGER_PATH: &str = "./ledger.json";

use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
pub(crate) struct Entry {
    pub(crate) database: std::collections::HashMap<String, Vec<u8>>,
    pub(crate) bytecode: Option<Vec<u8>>,
    pub(crate) balance: u64,
}

impl Entry {
    pub(crate) fn get_bytecode(&self) -> Result<Vec<u8>> {
        match &self.bytecode {
            Some(bytecode) => Ok(bytecode.clone()),
            _ => bail!("Error bytecode not found"),
        }
    }
    pub(crate) fn get_data(&self, key: &str) -> Result<Vec<u8>> {
        match self.database.get(key) {
            Some(bytecode) => Ok(bytecode.clone()),
            _ => Ok(vec![]),
        }
    }
    pub(crate) fn has_data(&self, key: &str) -> bool {
        self.database.contains_key(key)
    }
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub(crate) struct Ledger(std::collections::BTreeMap<String, Entry>);

impl Ledger {
    pub(crate) fn get(&self, address: &str) -> Result<Entry> {
        match self.0.get(address) {
            Some(entry) => Ok(entry.clone()),
            _ => bail!("Entry {} not found", address),
        }
    }
    pub(crate) fn set_module(&mut self, address: &str, module: &[u8]) {
        let mut entry = match self.get(address) {
            Ok(entry) => entry,
            _ => Entry::default(),
        };
        entry.bytecode = Some(module.to_vec());
        self.0.insert(address.to_owned(), entry);
    }
    pub(crate) fn set_data_entry(&mut self, address: &str, key: String, value: Vec<u8>) {
        let mut entry = match self.get(address) {
            Ok(entry) => entry,
            _ => Entry::default(),
        };
        entry.database.insert(key, value);
        self.0.insert(address.to_owned(), entry);
    }
    pub(crate) fn sub(&mut self, address: &str, amount: u64) -> Result<()> {
        let entry = match self.get(address) {
            Ok(entry) => entry,
            _ => bail!("Cannot find address {} in the ledger", address),
        };
        if entry.balance.checked_sub(amount).is_none() {
            bail!(
                "Fail to set balance substraction for {} of amount {} in the ledger",
                address,
                amount
            )
        }
        self.0.insert(address.to_string(), entry);
        Ok(())
    }
    pub(crate) fn add(&mut self, address: &str, amount: u64) -> Result<()> {
        let entry = match self.get(address) {
            Ok(entry) => entry,
            _ => bail!("Cannot find address {} in the ledger", address),
        };
        if entry.balance.checked_add(amount).is_none() {
            bail!(
                "Fail to set balance substraction for {} of amount {} in the ledger",
                address,
                amount
            )
        }
        self.0.insert(address.to_string(), entry);
        Ok(())
    }
}

/// The call item contains the address called and the amount sent by the caller
#[derive(Clone, Default)]
pub(crate) struct CallItem {
    pub address: String,
    pub coins: u64,
}

impl CallItem {
    pub(crate) fn address(address: &str) -> Self {
        Self {
            address: address.to_string(),
            coins: 0,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct InterfaceImpl {
    ledger: Arc<Mutex<Ledger>>,
    /// Stack of call items ordered
    call_stack: Arc<Mutex<std::collections::VecDeque<CallItem>>>,
    owned: Arc<Mutex<std::collections::VecDeque<String>>>,
}

impl InterfaceImpl {
    pub(crate) fn new() -> Result<InterfaceImpl> {
        let mut ret = InterfaceImpl::default();
        if let Ok(file) = std::fs::File::open("./ledger.json") {
            let reader = std::io::BufReader::new(file);
            ret.ledger = serde_json::from_reader(reader)?;
        }
        Ok(ret)
    }
    pub(crate) fn get_entry(&self, address: &str) -> Result<Entry> {
        match self.ledger.lock() {
            Ok(ledger) => ledger.get(address),
            Err(err) => bail!("Interface get entry:\n{}", err),
        }
    }
    pub(crate) fn save(&self) -> Result<()> {
        let str = serde_json::to_string_pretty(&self.ledger)?;
        match std::fs::write(LEDGER_PATH, str) {
            Err(error) => bail!("Error ledger:\n{}", error),
            _ => Ok(()),
        }
    }
    pub(crate) fn call_stack_push(&self, item: CallItem) -> Result<()> {
        match self.call_stack.lock() {
            Ok(mut cs) => {
                cs.push_back(item);
                Ok(())
            }
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn call_stack_pop(&self) -> Result<()> {
        match self.call_stack.lock() {
            Ok(mut cs) => {
                if cs.pop_back().is_none() {
                    bail!("Call sack err:\npop failed")
                }
                Ok(())
            }
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn call_stack_peek(&self) -> Result<CallItem> {
        match self.call_stack.lock() {
            Ok(cs) => match cs.back() {
                Some(item) => Ok(item.clone()),
                None => bail!("Call stack err:\npeek failed"),
            },
            Err(err) => bail!("Call stack err:\n{}", err),
        }
    }
    pub(crate) fn set_data_entry(&self, address: &str, key: &str, value: Vec<u8>) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => {
                ledger.set_data_entry(address, key.to_string(), value);
                Ok(())
            }
            Err(err) => bail!("Ledger data insertion err:\n{}", err),
        }
    }
    pub(crate) fn get(&self, address: &str) -> Result<Entry> {
        match self.ledger.lock() {
            Ok(ledger) => ledger.get(address),
            Err(err) => bail!("Data get err:\n{}", err),
        }
    }
    pub(crate) fn set_module(&self, address: &str, module: &[u8]) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => {
                ledger.set_module(address, module);
                Ok(())
            }
            Err(err) => bail!("Data get err:\n{}", err),
        }
    }
    pub(crate) fn sub(&self, address: &str, amount: u64) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => ledger.sub(address, amount),
            Err(err) => bail!("Balance sub err:\n{}", err),
        }
    }
    pub(crate) fn add(&self, address: &str, amount: u64) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => Ok(ledger.add(address, amount)?),
            Err(err) => bail!("Balance add err:\n{}", err),
        }
    }
    pub(crate) fn callstack_to_vec(&self) -> Result<Vec<String>> {
        match self.call_stack.lock() {
            Ok(cs) => Ok(cs.iter().map(|item| item.address.to_owned()).collect()),
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn owned_to_vec(&self) -> Result<Vec<String>> {
        match self.owned.lock() {
            Ok(owned) => Ok(owned.clone().into()),
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn own(&self, address: &str) -> Result<bool> {
        match self.owned.lock() {
            Ok(owned) => Ok(owned.contains(&address.to_owned())),
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn own_insert(&self, address: &str) -> Result<()> {
        match self.owned.lock() {
            Ok(mut owned) => {
                owned.push_back(address.to_string());
                Ok(())
            }
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn reset_addresses(&self) -> Result<()> {
        match self.owned.lock() {
            Ok(mut owned) => {
                owned.clear();
                owned.push_back("sender".to_string());
            }
            Err(err) => bail!("Call sack err:\n{}", err),
        };
        match self.call_stack.lock() {
            Ok(mut call_stack) => {
                call_stack.clear();
                call_stack.push_back(CallItem::address("sender"));
            }
            Err(err) => bail!("Call sack err:\n{}", err),
        };
        Ok(())
    }
}
