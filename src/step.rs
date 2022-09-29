use crate::execution_context::Slot;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StepType {
    ExecuteSC {
        /// Path to the smart contract
        path: String,
        /// Function of the smart contract to be tested, default is 'main'
        function: Option<String>,
        /// Parameter of the given function
        parameter: Option<String>,
        /// Caller address
        caller_address: Option<String>,
        /// Gas for execution
        gas: u64,
        /// Raw coins sent by the caller, default is '0', 1 raw_coin = 1e-9 coin
        coins: Option<u64>,
        /// Execution slot
        slot: Slot,
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
        /// Raw coins sent by the caller, default is '0', 1 raw_coin = 1e-9 coin
        coins: Option<u64>,
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
    // WriteAsyncMessages {
    //     /// Emitting address
    //     emitter_address: Option<String>,
    //     target_address: &str,
    //     target_handler: &str,
    //     validity_start: (u64, u8),
    //     validity_end: (u64, u8),
    //     max_gas: u64,
    //     gas_price: u64,
    //     coins: u64,
    //     data: &[u8],
    // },
}
