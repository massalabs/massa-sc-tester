use crate::execution_context::{CallItem, Slot};
use serde::Deserialize;
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum StepConfig {
    #[serde(rename = "execute_sc")]
    ExecuteSC {
        /// Path to the smart contract
        path: String,
        /// Function of the smart contract to be tested, default is 'main'
        function: Option<String>,
        /// Parameter of the given function
        parameter: Option<String>,
        /// Gas for execution
        gas: u64,
        /// ExecuteSC callstack
        call_stack: VecDeque<CallItem>,
    },
    #[serde(rename = "call_sc")]
    CallSC {
        /// Address of the smart contract
        address: String,
        /// Function of the smart contract to be tested, default is 'main'
        function: Option<String>,
        /// Parameter of the given function
        parameter: Option<String>,
        /// Gas for execution
        gas: u64,
        /// CallSC callstack
        call_stack: VecDeque<CallItem>,
    },
    ReadEvents {
        /// Start slot
        start: Option<Slot>,
        /// End slot
        end: Option<Slot>,
    },
    ReadLedgerEntry {
        /// Entry address
        address: String,
    },
    WriteLedgerEntry {
        /// Entry address
        address: String,
        /// Entry balance
        balance: Option<u64>,
        /// Entry bytecode
        bytecode: Option<Vec<u8>>,
        /// Entry datastore
        datastore: Option<BTreeMap<String, Vec<u8>>>,
    },
    ReadAsyncMessages {
        /// Start slot
        start: Option<Slot>,
        /// End slot
        end: Option<Slot>,
    },
    WriteAsyncMessage {
        sender_address: String,
        target_address: String,
        target_handler: String,
        execution_slot: Slot,
        gas: u64,
        coins: u64,
        data: String,
    },
}
