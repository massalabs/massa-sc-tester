const LEDGER_PATH: &str = "./ledger.json";

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

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
                "Failed to set balance substraction for {} of amount {} in the ledger",
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
                "Failed to set balance substraction for {} of amount {} in the ledger",
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

#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Slot {
    pub period: u64,
    pub thread: u8,
}

impl PartialOrd for Slot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.period, self.thread).partial_cmp(&(other.period, other.thread))
    }
}

impl Ord for Slot {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.period, self.thread).cmp(&(other.period, other.thread))
    }
}

#[derive(Clone, Debug)]
pub struct AsyncMessage {
    pub sender_address: String,
    pub target_address: String,
    pub target_handler: String,
    pub gas: u64,
    pub coins: u64,
    pub data: Vec<u8>,
}

type AsyncPool = BTreeMap<Slot, Vec<AsyncMessage>>;

#[derive(Clone, Default)]
pub(crate) struct ExecutionContext {
    ledger: Arc<Mutex<Ledger>>,
    call_stack: Arc<Mutex<std::collections::VecDeque<CallItem>>>,
    owned: Arc<Mutex<std::collections::VecDeque<String>>>,
    async_pool: Arc<Mutex<AsyncPool>>,
    pub execution_slot: Slot,
}

impl ExecutionContext {
    pub(crate) fn new() -> Result<ExecutionContext> {
        let mut ret = ExecutionContext::default();
        if let Ok(file) = std::fs::File::open(LEDGER_PATH) {
            let reader = std::io::BufReader::new(file);
            ret.ledger = serde_json::from_reader(reader)?;
        }
        Ok(ret)
    }
    pub(crate) fn get_entry(&self, address: &str) -> Result<Entry> {
        match self.ledger.lock() {
            Ok(ledger) => ledger.get(address),
            Err(err) => bail!("Get entry error: {}", err),
        }
    }
    pub(crate) fn save(&self) -> Result<()> {
        let str = serde_json::to_string_pretty(&self.ledger)?;
        match std::fs::write(LEDGER_PATH, str) {
            Err(error) => bail!("Ledger saving error:\n{}", error),
            _ => Ok(()),
        }
    }
    pub(crate) fn call_stack_push(&self, item: CallItem) -> Result<()> {
        match self.call_stack.lock() {
            Ok(mut cs) => {
                cs.push_back(item);
                Ok(())
            }
            Err(err) => bail!("Call stack error: {}", err),
        }
    }
    pub(crate) fn call_stack_pop(&self) -> Result<()> {
        match self.call_stack.lock() {
            Ok(mut cs) => {
                if cs.pop_back().is_none() {
                    bail!("Call stack error: pop failed")
                }
                Ok(())
            }
            Err(err) => bail!("Call stack error: {}", err),
        }
    }
    pub(crate) fn call_stack_peek(&self) -> Result<CallItem> {
        match self.call_stack.lock() {
            Ok(cs) => match cs.back() {
                Some(item) => Ok(item.clone()),
                None => bail!("Call stack error: peek failed"),
            },
            Err(err) => bail!("Call stack error: {}", err),
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
            Err(err) => bail!("Call stack error: {}", err),
        }
    }
    pub(crate) fn owned_to_vec(&self) -> Result<Vec<String>> {
        match self.owned.lock() {
            Ok(owned) => Ok(owned.clone().into()),
            Err(err) => bail!("Call stack error: {}", err),
        }
    }
    pub(crate) fn own(&self, address: &str) -> Result<bool> {
        match self.owned.lock() {
            Ok(owned) => Ok(owned.contains(&address.to_owned())),
            Err(err) => bail!("Call stack error: {}", err),
        }
    }
    pub(crate) fn own_insert(&self, address: &str) -> Result<()> {
        match self.owned.lock() {
            Ok(mut owned) => {
                owned.push_back(address.to_string());
                Ok(())
            }
            Err(err) => bail!("Call stack error: {}", err),
        }
    }
    pub(crate) fn reset_addresses(&self) -> Result<()> {
        match self.owned.lock() {
            Ok(mut owned) => {
                owned.clear();
                owned.push_back("sender".to_string());
            }
            Err(err) => bail!("Call stack error: {}", err),
        };
        match self.call_stack.lock() {
            Ok(mut call_stack) => {
                call_stack.clear();
                call_stack.push_back(CallItem::address("sender"));
            }
            Err(err) => bail!("Call stack error: {}", err),
        };
        Ok(())
    }
    pub(crate) fn push_async_message(&self, slot: Slot, mut message: AsyncMessage) -> Result<()> {
        message.sender_address = self.call_stack_peek()?.address;
        match self.async_pool.lock() {
            Ok(mut async_pool) => {
                async_pool
                    .entry(slot)
                    .and_modify(|list| list.push(message.clone()))
                    .or_insert_with(|| vec![message]);
            }
            Err(err) => bail!("Async pool error: {}", err),
        }
        Ok(())
    }
    pub(crate) fn get_async_messages_to_execute(&self) -> Result<Vec<AsyncMessage>> {
        match self.async_pool.lock() {
            Ok(async_pool) => Ok(async_pool
                .iter()
                .filter_map(|(&slot, list)| {
                    if slot <= self.execution_slot {
                        Some(list.clone())
                    } else {
                        None
                    }
                })
                .flatten()
                .collect()),
            Err(err) => bail!("Async pool error: {}", err),
        }
    }
}
