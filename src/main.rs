#![feature(btree_drain_filter)]

mod execution_context;
mod interface_impl;
mod step;
mod step_config;

use crate::step::execute_step;
use anyhow::{bail, Result};
use execution_context::{ExecutionContext, Slot};
use json::{object, JsonValue};
use serde::Deserialize;
use std::{
    cmp::Ordering,
    collections::{BTreeSet, VecDeque},
    fs,
    path::Path,
};
use step_config::StepConfig;
use structopt::StructOpt;

// TODO: add WASM target support
// TODO: update read steps output formatting
// TODO: improve README.md
// TODO: add step info on execution config error
// TODO: implement storage costs
// TODO: use massa-node cryptography

#[derive(Debug, Deserialize)]
struct Step {
    name: String,
    config: StepConfig,
}

#[derive(Debug, Deserialize)]
struct SlotExecutionSteps {
    slot: Slot,
    execution_steps: VecDeque<Step>,
}

impl PartialOrd for SlotExecutionSteps {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.slot.partial_cmp(&other.slot)
    }
}

impl Ord for SlotExecutionSteps {
    fn cmp(&self, other: &Self) -> Ordering {
        self.slot.cmp(&other.slot)
    }
}

impl PartialEq for SlotExecutionSteps {
    fn eq(&self, other: &Self) -> bool {
        self.slot.eq(&other.slot)
    }
}

impl Eq for SlotExecutionSteps {}

#[derive(StructOpt)]
struct CommandArguments {
    /// Path to the execution config
    config_path: String,
}

#[paw::main]
fn main(args: CommandArguments) -> Result<()> {
    // create the context
    let mut exec_context = ExecutionContext::new()?;

    // parse the config file
    let path = Path::new(&args.config_path);
    if !path.is_file() {
        bail!("{} isn't a file", args.config_path)
    }
    let extension = path.extension().unwrap_or_default();
    if extension != "json" {
        bail!("{} extension should be .json", args.config_path)
    }
    let config_slice = fs::read(path)?;
    let executions_config: BTreeSet<SlotExecutionSteps> = serde_json::from_slice(&config_slice)?;

    // execute the steps
    let mut trace = JsonValue::new_array();
    for SlotExecutionSteps {
        slot,
        execution_steps,
    } in executions_config
    {
        let mut slot_trace = JsonValue::new_array();
        for Step { name, config } in execution_steps {
            let step_trace = execute_step(&mut exec_context, slot, config)?;
            slot_trace.push(object!(
                execute_step: {
                    name: name,
                    output: step_trace
                }
            ))?;
        }
        trace.push(object!(
            execute_slot: {
                execution_slot: {
                    period: slot.period,
                    thread: slot.thread
                },
                output: slot_trace
            }
        ))?;
    }

    // write the trace
    let mut file = fs::File::create("trace.json")?;
    trace.write_pretty(&mut file, 4)?;
    Ok(())
}
