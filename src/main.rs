mod interface_impl;
mod ledger_interface;
mod types;

use anyhow::{bail, Result};
use ledger_interface::{CallItem, InterfaceImpl};
use massa_sc_runtime::{run_function, run_main};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};
use structopt::StructOpt;

#[derive(Deserialize)]
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
}

fn execute_step(args: StepArguments) -> Result<()> {
    // init the context
    let ledger_context = InterfaceImpl::new()?;
    ledger_context.reset_addresses()?;
    if let Some(address) = args.address {
        ledger_context.call_stack_push(CallItem {
            address,
            coins: args.coins.unwrap_or_default(),
        })?;
    }

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
    println!("run {}", args.path);

    // run the function
    println!(
        "remaining points: {}",
        if let Some(function) = args.function {
            run_function(
                &module,
                args.gas,
                &function,
                &args.parameter.unwrap_or_default(),
                &ledger_context,
            )?
        } else {
            run_main(&module, args.gas, &ledger_context)?
        }
    );

    // save the ledger
    ledger_context.save()?;
    Ok(())
}

#[derive(StructOpt)]
struct CommandArguments {
    /// Path to the execution config
    config_path: String,
}

#[paw::main]
fn main(args: CommandArguments) -> Result<()> {
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
    let executions_steps: HashMap<String, StepArguments> =
        serde_json::from_slice(&config_slice).unwrap();

    // execute the steps
    for (_step_name, step) in executions_steps {
        execute_step(step)?;
    }
    Ok(())
}
