use anyhow::{bail, Result};
use std::{env, fs, path::Path};
use assembly_simulator::run;

mod ledger_interface;
mod types;
mod interface_impl;

fn read_files() -> Result<Vec<(String, Vec<u8>)>> {
    // TODO: should be or use a read_files(filename: Path) -> String
    let args: Vec<String> = env::args().collect();
    let mut ret = vec![];
    #[allow(clippy::needless_range_loop)]
    for i in 1..args.len() {
        let name = args[i].clone();
        let path = Path::new(&name);
        if !path.is_file() {
            bail!("{} isn't file", name)
        }
        // TODO: should also handle binary WASM file?!
        let extention = path.extension().unwrap_or_default();
        if extention != "wat" && extention != "wasm" {
            bail!("{} should be in webassembly", name)
        }
        ret.push((path.to_str().unwrap().to_string(), fs::read(path)?));
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
            run(&module, 20000, &ledger_context)?
        );
    }
    ledger_context.save()?;
    Ok(())
}