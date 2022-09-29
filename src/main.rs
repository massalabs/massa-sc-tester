mod execution_context;
mod interface_impl;
mod step_config;

use crate::execution_context::AsyncMessage;
use anyhow::{bail, Result};
use execution_context::{CallItem, ExecutionContext, Slot};
use indexmap::IndexMap;
use json::{object, JsonValue};
use massa_sc_runtime::{run_function, run_main};
use serde::Deserialize;
use std::{
    cmp::Ordering,
    collections::{BTreeSet, VecDeque},
    fs,
    path::Path,
};
use step_config::StepConfig;
use structopt::StructOpt;

fn execute_step(
    exec_context: &mut ExecutionContext,
    slot: Slot,
    config_step: StepConfig,
) -> Result<JsonValue> {
    // init trace
    let mut trace = JsonValue::new_array();

    // match the config step
    match config_step {
        StepConfig::ExecuteSC {
            path,
            function,
            parameter,
            gas,
            call_stack,
        } => {
            // init the context for this step
            exec_context.reset_addresses()?;
            if let Some(stack) = call_stack {
                for call_item in stack {
                    exec_context.call_stack_push(call_item)?;
                }
            } else {
                // TODO: MAKE CALL STACK MANDATORY
                exec_context.call_stack_push(CallItem {
                    address: "sender".to_string(),
                    coins: 0,
                })?;
            }
            exec_context.execution_slot = slot;

            // read the wasm file
            let sc_path = Path::new(&path);
            if !sc_path.is_file() {
                bail!("{} isn't a file", path)
            }
            let extension = sc_path.extension().unwrap_or_default();
            if extension != "wasm" {
                bail!("{} extension should be .wasm", path)
            }
            let module = fs::read(sc_path)?;

            // run the function
            let (remaining_gas, function_name) = if let Some(function) = function {
                (
                    run_function(
                        &module,
                        gas,
                        &function,
                        &parameter.unwrap_or_default(),
                        exec_context,
                    )?,
                    function,
                )
            } else {
                (run_main(&module, gas, exec_context)?, "main".to_string())
            };

            // push the function trace
            let json = object!(
                execute_function: {
                    name: function_name,
                    remaining_gas: remaining_gas,
                    output: exec_context.take_execution_trace()?,
                }
            );
            trace.push(json)?;
        }
        // TODO: HANLDE OTHER STEPS TYPES
        _ => (),
    }

    // run the asynchronous messages
    for AsyncMessage {
        sender_address,
        target_address,
        target_handler,
        gas,
        coins,
        data,
    } in exec_context.get_async_messages_to_execute()?
    {
        let bytecode = exec_context.get_entry(&target_address)?.get_bytecode()?;
        exec_context.reset_addresses()?;
        exec_context.call_stack_push(CallItem {
            address: sender_address,
            coins,
        })?;
        exec_context.call_stack_push(CallItem {
            address: target_address,
            coins,
        })?;
        let remaining_gas = run_function(
            &bytecode,
            gas,
            &target_handler,
            std::str::from_utf8(&data)?,
            exec_context,
        )?;
        let json = object!(
            execute_message_function: {
                name: target_handler,
                remaining_gas: remaining_gas,
                output: exec_context.take_execution_trace()?,
            }
        );
        trace.push(json)?;
    }

    // save the ledger
    exec_context.save()?;
    Ok(trace)
}

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

    // print the trace
    let mut file = fs::File::create("trace.json")?;
    trace.write_pretty(&mut file, 4)?;
    Ok(())
}
