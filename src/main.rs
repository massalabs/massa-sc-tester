#![warn(unused_crate_dependencies)]
#![feature(btree_drain_filter)]
#![allow(clippy::from_over_into)]

mod constants;
mod execution_context;
mod interface_impl;
mod step_config;
mod step_manager;

use crate::step_manager::execute_step;
use anyhow::{bail, Result};
use constants::TRACE_PATH;
use execution_context::ExecutionContext;
use json::{object, JsonValue};
use std::{collections::BTreeSet, fs, path::Path};
use step_config::{SlotExecutionSteps, Step};
use structopt::StructOpt;

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
    let config_slice = fs::read(path)?;
    let executions_config: BTreeSet<SlotExecutionSteps> = match extension.to_str() {
        Some("yaml") | Some("yml") => serde_yaml::from_slice(&config_slice)?,
        Some("json") => serde_json::from_slice(&config_slice)?,
        _ => bail!(
            "{} extension should be .yaml, .yml or .json",
            args.config_path
        ),
    };

    // execute the steps
    let mut trace = JsonValue::new_array();
    for SlotExecutionSteps {
        slot,
        execution_steps,
    } in executions_config
    {
        exec_context.execution_slot = slot;
        let mut slot_trace = JsonValue::new_array();
        for Step { name, config } in execution_steps {
            let step_trace = execute_step(&mut exec_context, config)?;
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
    let mut file = fs::File::create(TRACE_PATH)?;
    trace.write_pretty(&mut file, 4)?;
    Ok(())
}
