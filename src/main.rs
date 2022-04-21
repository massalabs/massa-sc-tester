use anyhow::{bail, Result};
use massa_sc_runtime::run_main;
use std::{env, fs, path::Path};

mod interface_impl;
mod ledger_interface;
mod types;

pub struct Arguments {
    filename: String,
    module: Vec<u8>,
    function: Option<String>,
    caller: Option<ledger_interface::CallItem>,
}

fn read_arguments() -> Result<Arguments> {
    let args: Vec<String> = env::args().collect();
    let len = args.len();
    println!("{}", len);
    if !(2..=5).contains(&len) {
        bail!("invalid number of arguments")
    }
    // parse the file
    let name = args[1].clone();
    let path = Path::new(&name);
    if !path.is_file() {
        bail!("{} isn't file", name)
    }
    let extension = path.extension().unwrap_or_default();
    if extension != "wasm" {
        bail!("{} should be .wasm", name)
    }
    let bin = fs::read(path)?;
    Ok(Arguments {
        filename: path.to_str().unwrap().to_string(),
        module: bin,
        function: None,
        caller: None,
    })
}

fn main() -> Result<()> {
    let args: Arguments = read_arguments()?;
    let ledger_context = ledger_interface::InterfaceImpl::new()?;
    ledger_context.reset_addresses()?;
    println!("run {}", args.filename);
    println!(
        "remaining points: {}",
        run_main(&args.module, 1_000_000_000_000, &ledger_context)?
    );
    ledger_context.save()?;
    Ok(())
}
