const LEDGER_PATH: &str = "./ledger.json";

use anyhow::{bail, Result};
use json::JsonValue;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    ops::Bound,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default, Deserialize, Serialize)]
pub(crate) struct Entry {
    pub(crate) balance: u64,
    pub(crate) bytecode: Vec<u8>,
    pub(crate) datastore: BTreeMap<String, Vec<u8>>,
}

impl Entry {
    pub(crate) fn get_bytecode(&self) -> Vec<u8> {
        self.bytecode.clone()
    }
    pub(crate) fn get_data(&self, key: &str) -> Vec<u8> {
        self.datastore.get(key).cloned().unwrap_or_default()
    }
    pub(crate) fn has_data(&self, key: &str) -> bool {
        self.datastore.contains_key(key)
    }
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub(crate) struct Ledger(pub BTreeMap<String, Entry>);

impl Ledger {
    pub(crate) fn get(&self, address: &str) -> Result<Entry> {
        match self.0.get(address) {
            Some(entry) => Ok(entry.clone()),
            _ => bail!("ledger entry {} not found", address),
        }
    }
    pub(crate) fn set_module(&mut self, address: &str, module: &[u8]) {
        self.0
            .entry(address.to_string())
            .and_modify(|entry| entry.bytecode = module.to_vec())
            .or_insert_with(|| Entry {
                bytecode: module.to_vec(),
                ..Default::default()
            });
    }
    pub(crate) fn set_data_entry(&mut self, address: &str, key: String, value: Vec<u8>) {
        self.0
            .entry(address.to_string())
            .and_modify(|entry| {
                entry.datastore.insert(key.clone(), value.clone());
            })
            .or_insert_with(|| {
                let mut datastore = BTreeMap::new();
                datastore.insert(key, value);
                Entry {
                    datastore,
                    ..Default::default()
                }
            });
    }
    pub(crate) fn sub(&mut self, address: &str, amount: u64) -> Result<()> {
        let entry = match self.0.get_mut(address) {
            Some(entry) => entry,
            None => bail!("cannot find {} in the ledger", address),
        };
        if let Some(balance) = entry.balance.checked_sub(amount) {
            entry.balance = balance;
        } else {
            bail!(
                "cannot sub {} coins to {}, balance is too low",
                amount,
                address,
            )
        }
        Ok(())
    }
    pub(crate) fn add(&mut self, address: &str, amount: u64) -> Result<()> {
        let entry = match self.0.get_mut(address) {
            Some(entry) => entry,
            None => bail!("cannot find {} in the ledger", address),
        };
        if let Some(balance) = entry.balance.checked_add(amount) {
            entry.balance = balance;
        } else {
            bail!(
                "cannot add {} coins to {}, it would overflow",
                amount,
                address,
            )
        }
        Ok(())
    }
}

#[derive(Clone, Deserialize, Debug, Default)]
pub(crate) struct CallItem {
    /// Adress called
    pub address: String,
    /// Raw coins sent by the caller, default is '0', 1 raw_coin = 1e-9 coin
    pub coins: u64,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize)]
pub struct AsyncMessage {
    pub sender_address: String,
    pub target_address: String,
    pub target_handler: String,
    pub gas: u64,
    pub coins: u64,
    pub data: Vec<u8>,
}

type AsyncPool = BTreeMap<Slot, Vec<AsyncMessage>>;

#[derive(Clone, Debug, Serialize)]
pub struct Event {
    sender_address: String,
    data: String,
}

type EventPool = BTreeMap<Slot, Vec<Event>>;

#[derive(Clone)]
pub(crate) struct ExecutionContext {
    ledger: Arc<Mutex<Ledger>>,
    call_stack: Arc<Mutex<std::collections::VecDeque<CallItem>>>,
    owned: Arc<Mutex<std::collections::VecDeque<String>>>,
    async_pool: Arc<Mutex<AsyncPool>>,
    event_pool: Arc<Mutex<EventPool>>,
    execution_trace: Arc<Mutex<JsonValue>>,
    pub execution_slot: Slot,
}

impl ExecutionContext {
    pub(crate) fn new() -> Result<ExecutionContext> {
        Ok(ExecutionContext {
            ledger: if let Ok(file) = std::fs::File::open(LEDGER_PATH) {
                let reader = std::io::BufReader::new(file);
                serde_json::from_reader(reader)?
            } else {
                Default::default()
            },
            call_stack: Default::default(),
            owned: Default::default(),
            async_pool: Default::default(),
            execution_slot: Default::default(),
            event_pool: Default::default(),
            execution_trace: Arc::new(Mutex::new(JsonValue::new_array())),
        })
    }
    pub(crate) fn create_new_entry(&self, address: String, entry: Entry) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => ledger.0.insert(address, entry),
            Err(err) => bail!("create_entry lock error: {}", err),
        };
        Ok(())
    }
    pub(crate) fn get_entry(&self, address: &str) -> Result<Entry> {
        match self.ledger.lock() {
            Ok(ledger) => ledger.get(address),
            Err(err) => bail!("get_entry lock error: {}", err),
        }
    }
    pub(crate) fn save(&self) -> Result<()> {
        let str = serde_json::to_string_pretty(&self.ledger)?;
        match std::fs::write(LEDGER_PATH, str) {
            Err(error) => bail!("save lock error: {}", error),
            _ => Ok(()),
        }
    }
    pub(crate) fn call_stack_push(&self, item: CallItem) -> Result<()> {
        self.sub(&item.address, item.coins)?;
        println!("{} | {}", item.address, item.coins);
        match self.call_stack.lock() {
            Ok(mut cs) => {
                cs.push_back(item);
                Ok(())
            }
            Err(err) => bail!("call_stack_push lock error: {}", err),
        }
    }
    pub(crate) fn call_stack_pop(&self) -> Result<()> {
        match self.call_stack.lock() {
            Ok(mut cs) => {
                if cs.pop_back().is_none() {
                    bail!("call_stack_pop failed")
                }
                Ok(())
            }
            Err(err) => bail!("call_stack_pop lock error: {}", err),
        }
    }
    pub(crate) fn call_stack_peek(&self) -> Result<CallItem> {
        match self.call_stack.lock() {
            Ok(cs) => match cs.back() {
                Some(item) => Ok(item.clone()),
                None => bail!("call_stack_peek failed"),
            },
            Err(err) => bail!("call_stack_peek lock error: {}", err),
        }
    }
    pub(crate) fn set_data_entry(&self, address: &str, key: &str, value: Vec<u8>) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => {
                ledger.set_data_entry(address, key.to_string(), value);
                Ok(())
            }
            Err(err) => bail!("set_data_entry lock error: {}", err),
        }
    }
    pub(crate) fn get(&self, address: &str) -> Result<Entry> {
        match self.ledger.lock() {
            Ok(ledger) => ledger.get(address),
            Err(err) => bail!("get lock error: {}", err),
        }
    }
    pub(crate) fn set_module(&self, address: &str, module: &[u8]) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => {
                ledger.set_module(address, module);
                Ok(())
            }
            Err(err) => bail!("set_module lock error: {}", err),
        }
    }
    pub(crate) fn sub(&self, address: &str, amount: u64) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => ledger.sub(address, amount),
            Err(err) => bail!("sub lock error: {}", err),
        }
    }
    pub(crate) fn add(&self, address: &str, amount: u64) -> Result<()> {
        match self.ledger.lock() {
            Ok(mut ledger) => Ok(ledger.add(address, amount)?),
            Err(err) => bail!("add lock error: {}", err),
        }
    }
    pub(crate) fn callstack_to_vec(&self) -> Result<Vec<String>> {
        match self.call_stack.lock() {
            Ok(cs) => Ok(cs.iter().map(|item| item.address.to_owned()).collect()),
            Err(err) => bail!("callstack_to_vec lock error: {}", err),
        }
    }
    pub(crate) fn owned_to_vec(&self) -> Result<Vec<String>> {
        match self.owned.lock() {
            Ok(owned) => Ok(owned.clone().into()),
            Err(err) => bail!("owned_to_vec lock error: {}", err),
        }
    }
    pub(crate) fn own(&self, address: &str) -> Result<bool> {
        match self.owned.lock() {
            Ok(owned) => Ok(owned.contains(&address.to_owned())),
            Err(err) => bail!("own lock error: {}", err),
        }
    }
    pub(crate) fn own_insert(&self, address: &str) -> Result<()> {
        match self.owned.lock() {
            Ok(mut owned) => {
                owned.push_back(address.to_string());
                Ok(())
            }
            Err(err) => bail!("own_insert lock error: {}", err),
        }
    }
    pub(crate) fn reset_addresses(&self) -> Result<()> {
        match self.owned.lock() {
            Ok(mut owned) => {
                owned.clear();
            }
            Err(err) => bail!("reset_addresses lock error: {}", err),
        };
        match self.call_stack.lock() {
            Ok(mut call_stack) => {
                call_stack.clear();
            }
            Err(err) => bail!("reset_addresses lock error: {}", err),
        };
        Ok(())
    }
    pub(crate) fn push_async_message(&self, slot: Slot, message: AsyncMessage) -> Result<()> {
        match self.async_pool.lock() {
            Ok(mut async_pool) => async_pool
                .entry(slot)
                .and_modify(|list| list.push(message.clone()))
                .or_insert_with(|| vec![message]),
            Err(err) => bail!("push_async_message lock error: {}", err),
        };
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
            Err(err) => bail!("get_async_messages_to_execute lock error: {}", err),
        }
    }
    pub(crate) fn get_async_messages_in(
        &self,
        start: Option<Slot>,
        end: Option<Slot>,
    ) -> Result<Vec<AsyncMessage>> {
        match self.async_pool.lock() {
            Ok(async_pool) => {
                let start_bound = if let Some(start) = start {
                    Bound::Included(start)
                } else {
                    Bound::Unbounded
                };
                let end_bound = if let Some(end) = end {
                    Bound::Excluded(end)
                } else {
                    Bound::Unbounded
                };
                Ok(async_pool
                    .range((start_bound, end_bound))
                    .flat_map(|(_, messages)| messages.clone())
                    .collect())
            }
            Err(err) => bail!("get_async_messages_to_execute lock error: {}", err),
        }
    }
    pub(crate) fn update_execution_trace(&self, json: JsonValue) -> Result<()> {
        match self.execution_trace.lock() {
            Ok(mut trace) => {
                if let Err(err) = trace.push(json) {
                    bail!("update_execution_trace json error: {}", err)
                }
                Ok(())
            }
            Err(err) => bail!("update_execution_trace lock error: {}", err),
        }
    }
    pub(crate) fn push_event(&self, slot: Slot, addr: String, data: String) -> Result<()> {
        match self.event_pool.lock() {
            Ok(mut event_pool) => {
                let event = Event {
                    sender_address: addr,
                    data,
                };
                event_pool
                    .entry(slot)
                    .and_modify(|list| list.push(event.clone()))
                    .or_insert_with(|| vec![event]);
            }
            Err(err) => bail!("push_event lock error: {}", err),
        };
        Ok(())
    }
    pub(crate) fn get_events_in(
        &self,
        start: Option<Slot>,
        end: Option<Slot>,
    ) -> Result<Vec<Event>> {
        match self.event_pool.lock() {
            Ok(event_pool) => {
                let start_bound = if let Some(start) = start {
                    Bound::Included(start)
                } else {
                    Bound::Unbounded
                };
                let end_bound = if let Some(end) = end {
                    Bound::Excluded(end)
                } else {
                    Bound::Unbounded
                };
                Ok(event_pool
                    .range((start_bound, end_bound))
                    .flat_map(|(_, events)| events.clone())
                    .collect())
            }
            Err(err) => bail!("get_events_in lock error: {}", err),
        }
    }
    pub(crate) fn take_execution_trace(&self) -> Result<JsonValue> {
        match self.execution_trace.lock() {
            Ok(mut trace) => {
                let ret_trace = trace.clone();
                trace.clear();
                Ok(ret_trace)
            }
            Err(err) => bail!("take_execution_trace lock error: {}", err),
        }
    }
}
