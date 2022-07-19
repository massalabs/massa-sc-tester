mod interface_impl;
mod ledger_interface;
mod types;

use anyhow::{bail, Result};
use ledger_interface::{CallItem, InterfaceImpl};
use massa_sc_runtime::{run_function, run_main};
use std::{fs, path::Path};
use structopt::StructOpt;

#[derive(StructOpt)]
struct Arguments {
    /// Path to the smart contract
    path: String,
    /// Function of the smart contract to be tested, default is 'main'
    #[structopt(short = "f", long = "function")]
    function: Option<String>,
    /// Parameter of the given function
    #[structopt(short = "p", long = "parameter")]
    parameter: Option<String>,
    /// Address called
    #[structopt(short = "a", long = "address")]
    address: Option<String>,
    /// Raw coins sent by the caller, default is '0', 1 raw_coin = 1e-9 coin
    #[structopt(short = "c", long = "coins")]
    coins: Option<u64>,
}

#[paw::main]
fn main(args: Arguments) -> Result<()> {
    // init the context
    let ledger_context = InterfaceImpl::new()?;
    ledger_context.reset_addresses()?;
    if let Some(address) = args.address {
        ledger_context.call_stack_push(CallItem {
            address,
            coins: args.coins.unwrap_or_default(),
        })?;
    }

    // parse the file
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

    // launch the tester
    println!(
        "remaining points: {}",
        if let Some(function) = args.function {
            run_function(
                &module,
                1_000_000_000_000,
                &function,
                &args.parameter.unwrap_or_default(),
                &ledger_context,
            )?
        } else {
            run_main(&module, 1_000_000_000_000, &ledger_context)?
        }
    );

    // save the ledger
    ledger_context.save()?;
    Ok(())
}
