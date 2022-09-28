mod execution_context;
mod interface_impl;

use crate::execution_context::AsyncMessage;
use anyhow::{bail, Result};
use colored::Colorize;
use execution_context::{CallItem, ExecutionContext, Slot};
use indexmap::IndexMap;
use massa_sc_runtime::{run_function, run_main};
use serde::Deserialize;
use std::{fs, path::Path, time::Instant};
use structopt::StructOpt;

macro_rules! step_runner {
    ($($arg:tt)+) => {
        print!("{} ", "STEP RUNNER".bold().yellow());
        println!($($arg)+);
    };
}

macro_rules! sc_runner {
    ($($arg:tt)+) => {
        print!("{} ", "SC RUNNER".bold().green());
        println!($($arg)+);
    };
}

macro_rules! message_runner {
    ($($arg:tt)+) => {
        print!("{} ", "MESSAGE RUNNER".bold().blue());
        println!($($arg)+);
    };
}

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
    if let Some(function) = args.function {
        sc_runner!("execute {}", function);
        let remaining_gas = run_function(
            &module,
            args.gas,
            &function,
            &args.parameter.unwrap_or_default(),
            exec_context,
        )?;
        sc_runner!(
            "{} execution was successful, remaining gas is {}",
            function,
            remaining_gas
        );
    } else {
        sc_runner!("execute {}", "main");
        let remaining_gas = run_main(&module, args.gas, exec_context)?;
        sc_runner!(
            "{} execution was successful, remaining gas is {}",
            "main",
            remaining_gas
        );
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
        exec_context.call_stack_push(CallItem {
            address: sender_address,
            coins,
        })?;
        exec_context.call_stack_push(CallItem {
            address: target_address,
            coins,
        })?;
        message_runner!("execute {}", target_handler);
        let remaining_gas = run_function(&bytecode, gas, &target_handler, &data, exec_context)?;
        message_runner!(
            "{} execution was successful, remaining gas is {}",
            target_handler,
            remaining_gas
        );
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
        step_runner!("execute {}", step_name);
        let start = Instant::now();
        execute_step(&mut exec_context, step)?;
        let duration = start.elapsed();
        step_runner!(
            "{} was successful, execution time is {} ms",
            step_name,
            duration.as_millis()
        );
    }
    Ok(())
}
