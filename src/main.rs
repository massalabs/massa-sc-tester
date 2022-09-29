mod execution_context;
mod interface_impl;
mod step;

use crate::execution_context::AsyncMessage;
use anyhow::{bail, Result};
use execution_context::{CallItem, ExecutionContext, Slot};
use indexmap::IndexMap;
use json::{object, JsonValue};
use massa_sc_runtime::{run_function, run_main};
use serde::Deserialize;
use std::{fs, fs::File, path::Path};
use structopt::StructOpt;

#[derive(Debug, Deserialize)]
struct StepArguments {
    /// Path to the smart contract
    path: String,
    /// Function of the smart contract to be tested, default is 'main'
    function: Option<String>,
    /// Parameter of the given function
    parameter: Option<String>,
    /// Caller address
    address: Option<String>,
    /// Gas for execution
    gas: u64,
    /// Raw coins sent by the caller, default is '0', 1 raw_coin = 1e-9 coin
    coins: Option<u64>,
    /// Execution slot
    slot: Slot,
}

fn execute_step(exec_context: &mut ExecutionContext, args: StepArguments) -> Result<JsonValue> {
    // init trace
    let mut trace = JsonValue::new_array();

    // init the context for this step
    exec_context.reset_addresses()?;
    exec_context.call_stack_push(CallItem {
        address: args.address.unwrap_or("default_addr".to_string()),
        coins: args.coins.unwrap_or_default(),
    })?;
    exec_context.execution_slot = args.slot;

    // read the wasm file
    let path = Path::new(&args.path);
    if !path.is_file() {
        bail!("{} isn't a file", args.path)
    }
    let extension = path.extension().unwrap_or_default();
    if extension != "wasm" {
        bail!("{} extension should be .wasm", args.path)
    }
    let module = fs::read(path)?;

    // run the function
    let (remaining_gas, function_name) = if let Some(function) = args.function {
        (
            run_function(
                &module,
                args.gas,
                &function,
                &args.parameter.unwrap_or_default(),
                exec_context,
            )?,
            function,
        )
    } else {
        (
            run_main(&module, args.gas, exec_context)?,
            "main".to_string(),
        )
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
    let executions_steps: IndexMap<String, StepArguments> = serde_json::from_slice(&config_slice)?;

    // execute the steps
    let mut trace = JsonValue::new_array();
    for (step_name, step) in executions_steps {
        let step_trace = execute_step(&mut exec_context, step)?;
        let json = object!(
            execute_step: {
                name: step_name,
                output: step_trace
            }
        );
        trace.push(json)?;
    }

    // print the trace
    let mut file = File::create("trace.json")?;
    trace.write_pretty(&mut file, 4)?;
    Ok(())
}
