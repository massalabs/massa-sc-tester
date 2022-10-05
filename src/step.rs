use crate::execution_context::{AsyncMessage, CallItem, Entry, ExecutionContext, Slot};
use crate::step_config::StepConfig;
use anyhow::{bail, Result};
use json::{object, JsonValue};
use massa_sc_runtime::{run_function, run_main};
use std::{fs, path::Path};

pub(crate) fn execute_step(
    exec_context: &mut ExecutionContext,
    slot: Slot,
    config_step: StepConfig,
) -> Result<JsonValue> {
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
            // init the context
            exec_context.reset_addresses()?;
            for call_item in call_stack {
                exec_context.call_stack_push(call_item)?;
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

            // execute the function
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
                execute_sc: {
                    name: function_name,
                    remaining_gas: remaining_gas,
                    output: exec_context.take_execution_trace()?,
                }
            );
            trace.push(json)?;
        }
        StepConfig::CallSC {
            address,
            function,
            parameter,
            gas,
            call_stack,
        } => {
            // init the context
            exec_context.reset_addresses()?;
            for call_item in call_stack {
                exec_context.call_stack_push(call_item)?;
            }
            exec_context.execution_slot = slot;

            // read the bytecode
            let bytecode = exec_context.get_entry(&address)?.get_bytecode();

            // execute the function
            let (remaining_gas, function_name) = if let Some(function) = function {
                (
                    run_function(
                        &bytecode,
                        gas,
                        &function,
                        &parameter.unwrap_or_default(),
                        exec_context,
                    )?,
                    function,
                )
            } else {
                (run_main(&bytecode, gas, exec_context)?, "main".to_string())
            };

            // push the function trace
            let json = object!(
                call_sc: {
                    name: function_name,
                    remaining_gas: remaining_gas,
                    output: exec_context.take_execution_trace()?,
                }
            );
            trace.push(json)?;
        }
        StepConfig::ReadEvents { start, end } => {
            let events = exec_context.get_events_in(start, end)?;
            let json = object!(read_events: JsonValue::from(serde_json::to_string(&events)?));
            trace.push(json)?;
        }
        StepConfig::ReadLedgerEntry { address } => {
            let entry = exec_context.get_entry(&address)?;
            let json = object!(read_ledger_entry: JsonValue::from(serde_json::to_string(&entry)?));
            trace.push(json)?;
        }
        StepConfig::WriteLedgerEntry {
            address,
            balance,
            bytecode,
            datastore,
        } => {
            exec_context.create_new_entry(
                address,
                Entry {
                    balance: balance.unwrap_or_default(),
                    bytecode: bytecode.unwrap_or_default(),
                    datastore: datastore.unwrap_or_default(),
                },
            )?;
        }
        StepConfig::ReadAsyncMessages { start, end } => {
            let msgs = exec_context.get_async_messages_in(start, end)?;
            let json = object!(read_async_messages: JsonValue::from(serde_json::to_string(&msgs)?));
            trace.push(json)?;
        }
        StepConfig::WriteAsyncMessage {
            sender_address: emitter_address,
            target_address,
            target_handler,
            execution_slot,
            gas,
            coins,
            data,
        } => exec_context.push_async_message(
            execution_slot,
            AsyncMessage {
                sender_address: emitter_address,
                target_address,
                target_handler,
                gas,
                coins,
                data: data.into_bytes(),
            },
        )?,
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
        // set the call stack
        exec_context.reset_addresses()?;
        exec_context.call_stack_push(CallItem {
            address: sender_address,
            coins,
        })?;
        exec_context.call_stack_push(CallItem {
            address: target_address.clone(),
            coins,
        })?;

        // read the bytecode
        let bytecode = exec_context.get_entry(&target_address)?.get_bytecode();

        // execute the function
        let remaining_gas = run_function(
            &bytecode,
            gas,
            &target_handler,
            std::str::from_utf8(&data)?,
            exec_context,
        )?;

        // push the message trace
        let json = object!(
            execute_async_message: {
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
