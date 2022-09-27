mod execution_context;
mod interface_impl;

use anyhow::{bail, Result};
use execution_context::{CallItem, ExecutionContext, Slot};
use indexmap::IndexMap;
use massa_sc_runtime::{run_function, run_main};
use serde::Deserialize;
use std::{fs, path::Path};
use structopt::StructOpt;

use crate::execution_context::AsyncMessage;

#[derive(Debug, Deserialize)]
struct StepArguments {
    /// Path to the smart contract
    path: String,
    /// Function of the smart contract to be tested, default is 'main'
    function: Option<String>,
    /// Parameter of the given function
    parameter: Option<String>,
    /// Address called
    address: Option<String>,
    /// Gas for execution
    gas: u64,
    /// Raw coins sent by the caller, default is '0', 1 raw_coin = 1e-9 coin
    coins: Option<u64>,
    /// Execution slot
    slot: Slot,
}

fn execute_step(exec_context: &mut ExecutionContext, args: StepArguments) -> Result<()> {
    // init the context for this step
    exec_context.reset_addresses()?;
    if let Some(address) = args.address {
        exec_context.call_stack_push(CallItem {
            address,
            coins: args.coins.unwrap_or_default(),
        })?;
    }
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
    println!(
        "remaining points: {}",
        if let Some(function) = args.function {
            run_function(
                &module,
                args.gas,
                &function,
                &args.parameter.unwrap_or_default(),
                exec_context,
            )?
        } else {
            run_main(&module, args.gas, exec_context)?
        }
    );

    // run the asynchronous messages
    for AsyncMessage {
        target_address,
        target_handler,
        gas,
        coins,
        data,
    } in exec_context.get_async_messages_to_execute()?
    {
        let bytecode = exec_context.get_entry(&target_address)?.get_bytecode()?;
        // TODO: setup context with coins use
        run_function(
            &bytecode,
            gas,
            &target_handler,
            std::str::from_utf8(&data)?,
            exec_context,
        )?;
    }

    // save the ledger
    exec_context.save()?;
    Ok(())
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
    for (step_name, step) in executions_steps {
        println!("start {} execution", step_name);
        execute_step(&mut exec_context, step)?;
        println!("{} execution was successful", step_name)
    }
    Ok(())
}
