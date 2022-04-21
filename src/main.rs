use anyhow::{bail, Result};
use massa_sc_runtime::{run_function, run_main};
use std::{collections::HashMap, env, fs, path::Path};

mod interface_impl;
mod ledger_interface;
mod types;

use ledger_interface::{CallItem, InterfaceImpl};

pub struct Arguments {
    filename: String,
    module: Vec<u8>,
    function: Option<(String, String)>,
    caller: Option<CallItem>,
}

fn parse_arguments() -> Result<Arguments> {
    // collect the arguments
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

    // parse the configuration parameters
    let p_list: [&str; 4] = ["function", "param", "addr", "coins"];
    let mut p: HashMap<String, String> = HashMap::new();
    for v in args.iter().skip(2) {
        let s: Vec<&str> = v.split('=').collect();
        if s.len() == 2 && p_list.contains(&s[0]) {
            p.insert(s[0].to_string(), s[1].to_string());
        } else {
            bail!("invalid parameter");
        }
    }

    // return parsed arguments
    Ok(Arguments {
        filename: path.to_str().unwrap().to_string(),
        module: bin,
        function: match (
            p.get_key_value("function").map(|x| x.1.clone()),
            p.get_key_value("param").map(|x| x.1.clone()),
        ) {
            (Some(function), Some(param)) => Some((function, param)),
            (Some(function), None) => Some((function, "".to_string())),
            _ => None,
        },
        caller: match (
            p.get_key_value("addr").map(|x| x.1.clone()),
            p.get_key_value("coins").map(|x| x.1.clone()),
        ) {
            (Some(address), Some(coins)) => Some(CallItem {
                address,
                coins: if let Ok(coins) = coins.parse::<u64>() {
                    coins
                } else {
                    println!("invalid coins, will be set to 0");
                    0
                },
            }),
            (Some(address), None) => Some(CallItem { address, coins: 0 }),
            _ => None,
        },
    })
}

fn main() -> Result<()> {
    let args: Arguments = parse_arguments()?;
    let ledger_context = InterfaceImpl::new()?;
    ledger_context.reset_addresses()?;
    if let Some(caller) = args.caller {
        ledger_context.call_stack_push(caller)?;
    }
    println!("run {}", args.filename);
    println!(
        "remaining points: {}",
        if let Some((name, param)) = args.function {
            run_function(
                &args.module,
                1_000_000_000_000,
                &name,
                &param,
                &ledger_context,
            )?
        } else {
            run_main(&args.module, 1_000_000_000_000, &ledger_context)?
        }
    );
    ledger_context.save()?;
    Ok(())
}
