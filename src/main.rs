use anyhow::{bail, Result};
use assembly_simulator::run;
use std::{env, fs, path::Path};

mod interface_impl;
mod ledger_interface;
mod types;

fn read_files() -> Result<Vec<(String, Vec<u8>)>> {
    let args: Vec<String> = env::args().collect();
    let mut ret = vec![];
    #[allow(clippy::needless_range_loop)]
    for i in 1..args.len() {
        let name = args[i].clone();
        let path = Path::new(&name);
        if !path.is_file() {
            bail!("{} isn't file", name)
        }
        let extention = path.extension().unwrap_or_default();
        if extention != "wasm" {
            bail!("{} should be .wasm", name)
        }
        let bin = fs::read(path)?;
        ret.push((path.to_str().unwrap().to_string(), bin));
    }
    Ok(ret)
}

fn main() -> Result<()> {
    let modules = read_files()?;
    let ledger_context = ledger_interface::InterfaceImpl::new()?;
    for (filename, module) in modules.into_iter() {
        ledger_context.reset_addresses()?;
        println!("run {}", filename);
        println!(
            "remaining points: {}",
            run(&module, 1_000_000_000_000, &ledger_context)?
        );
    }
    ledger_context.save()?;
    Ok(())
}
