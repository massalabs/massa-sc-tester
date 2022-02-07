const LEDGER_PATH: &str = "./ledger.json";

use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use massa_sc_runtime::{Address, Bytecode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize, Serialize)]
pub(crate) struct Entry {
    pub(crate) database: std::collections::HashMap<String, Bytecode>,
    pub(crate) bytecode: Option<Bytecode>,
    pub(crate) balance: u64,
}

impl Entry {
    pub(crate) fn get_bytecode(&self) -> Result<Bytecode> {
        match &self.bytecode {
            Some(bytecode) => Ok(bytecode.clone()),
            _ => bail!("Error bytecode not found"),
        }
    }
    pub(crate) fn get_data(&self, key: &str) -> Result<Bytecode> {
        match self.database.get(key) {
            Some(bytecode) => Ok(bytecode.clone()),
            _ => Ok(Bytecode::default()),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub(crate) struct Ledger(std::collections::BTreeMap<Address, Entry>);

impl Ledger {
    pub(crate) fn get(&self, address: &Address) -> Result<Entry> {
        match self.0.get(address) {
            Some(entry) => Ok(entry.clone()),
            _ => bail!("Entry {} not found", address),
        }
    }
    pub(crate) fn set_module(&mut self, address: &Address, module: &Bytecode) {
        let mut entry = match self.get(address) {
            Ok(entry) => entry,
            _ => Entry::default(),
        };
        entry.bytecode = Some(module.clone());
        self.0.insert(address.to_owned(), entry);
    }
    pub(crate) fn set_data_entry(&self, address: &Address, key: String, value: Bytecode) {
        let mut entry = match self.get(address) {
            Ok(entry) => entry,
            _ => Entry::default(),
        };
        entry.database.insert(key, value);
    }
    pub(crate) fn sub(&mut self, address: &Address, amount: u64) -> Result<()> {
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
        self.0.insert(address.clone(), entry);
        Ok(())
    }
    pub(crate) fn add(&mut self, address: &Address, amount: u64) -> Result<()> {
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
        self.0.insert(address.clone(), entry);
        Ok(())
    }
}

#[derive(Clone, Default)]
pub(crate) struct InterfaceImpl {
    ledger: Arc<Mutex<Ledger>>,
    call_stack: Arc<Mutex<std::collections::VecDeque<Address>>>,
    owned: Arc<Mutex<std::collections::VecDeque<Address>>>,
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
    pub(crate) fn get_entry(&self, address: &Address) -> Result<Entry> {
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
    pub(crate) fn call_stack_push(&self, address: Address) -> Result<()> {
        match self.call_stack.lock() {
            Ok(mut cs) => {
                cs.push_back(address);
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
    pub(crate) fn call_stack_peek(&self) -> Result<Address> {
        match self.call_stack.lock() {
            Ok(cs) => match cs.back() {
                Some(address) => Ok(address.clone()),
                None => bail!("Call stack err:\npeek failed"),
            },
            Err(err) => bail!("Call stack err:\n{}", err),
        }
    }
    pub(crate) fn set_data_entry(
        &self,
        address: &Address,
        key: &str,
        value: Bytecode,
    ) -> Result<()> {
        match self.ledger.lock() {
            Ok(ledger) => {
                ledger.set_data_entry(address, key.to_string(), value);
                Ok(())
            }
            Err(err) => bail!("Ledger data insertion err:\n{}", err),
        }
    }
    pub(crate) fn get(&self, address: &Address) -> Result<Entry> {
        match self.ledger.lock() {
            Ok(ledger) => ledger.get(address),
            Err(err) => bail!("Data get err:\n{}", err),
        }
    }
    pub(crate) fn set_module(&self, address: &Address, module: &Bytecode) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => {
                ledger.set_module(address, module);
                Ok(())
            }
            Err(err) => bail!("Data get err:\n{}", err),
        }
    }
    pub(crate) fn sub(&self, address: &Address, amount: u64) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => ledger.sub(address, amount),
            Err(err) => bail!("Balance sub err:\n{}", err),
        }
    }
    pub(crate) fn add(&self, address: &Address, amount: u64) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => Ok(ledger.add(address, amount)?),
            Err(err) => bail!("Balance add err:\n{}", err),
        }
    }
    pub(crate) fn callstack_to_vec(&self) -> Result<Vec<Address>> {
        match self.call_stack.lock() {
            Ok(cs) => Ok(cs.clone().into()),
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn owned_to_vec(&self) -> Result<Vec<Address>> {
        match self.owned.lock() {
            Ok(owned) => Ok(owned.clone().into()),
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn own(&self, address: &Address) -> Result<bool> {
        match self.owned.lock() {
            Ok(owned) => Ok(owned.contains(address)),
            Err(err) => bail!("Call sack err:\n{}", err),
        }
    }
    pub(crate) fn own_insert(&self, address: &Address) -> Result<()> {
        match self.owned.lock() {
            Ok(mut owned) => {
                owned.push_back(address.clone());
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
                call_stack.push_back("sender".to_string());
            }
            Err(err) => bail!("Call sack err:\n{}", err),
        };
        Ok(())
    }
}
