use crate::execution_context::{CallItem, Slot};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum StepConfig {
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
        call_stack: Option<VecDeque<CallItem>>,
    },
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
        call_stack: Option<VecDeque<CallItem>>,
    },
    ReadEvents {
        /// Emitting address
        emitter_address: Option<String>,
        /// Start slot
        start: Option<Slot>,
        /// End slot
        end: Option<Slot>,
    },
    ReadLedgerEntry {
        /// Entry address
        address: String,
        /// Entry bytecode
        bytecode: bool,
        /// Entry datastore
        datastore_key: Option<BTreeSet<Vec<u8>>>,
    },
    WriteLedgerEntry {
        /// Entry address
        address: String,
        /// Entry bytecode
        bytecode: Option<Vec<u8>>,
        /// Entry datastore
        datastore: Option<BTreeMap<Vec<u8>, Vec<u8>>>,
    },
    ReadAsyncMessages {
        /// Emitting address
        emitter_address: Option<String>,
        /// Start slot
        start: Option<Slot>,
        /// End slot
        end: Option<Slot>,
    },
    WriteAsyncMessages {
        /// Emitting address
        emitter_address: String,
        target_address: String,
        target_handler: String,
        validity_start: Slot,
        validity_end: Slot,
        max_gas: u64,
        gas_price: u64,
        coins: u64,
        data: Vec<u8>,
    },
}
