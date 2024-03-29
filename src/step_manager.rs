use crate::execution_context::{AsyncMessage, CallItem, Entry, ExecutionContext};
use crate::step_config::StepConfig;
use anyhow::{bail, Result};
use json::{object, JsonValue};
use massa_sc_runtime::{run_function, run_main, Compiler, Response, RuntimeModule};
use std::{fs, path::Path};

pub(crate) fn execute_step(
    exec_context: &mut ExecutionContext,
    config_step: StepConfig,
) -> Result<JsonValue> {
    let mut trace = JsonValue::new_array();

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
        let module = RuntimeModule::new(
            &exec_context.get_entry(&target_address)?.get_bytecode(),
            gas,
            exec_context.gas_costs.clone(),
            Compiler::CL,
        )?;

        // execute the function
        let Response { remaining_gas, .. } = run_function(
            exec_context,
            module,
            &target_handler,
            &data,
            gas,
            exec_context.gas_costs.clone(),
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

            // read the wasm file
            let sc_path = Path::new(&path);
            if !sc_path.is_file() {
                bail!("{} isn't a file", path)
            }
            let extension = sc_path.extension().unwrap_or_default();
            if extension != "wasm" {
                bail!("{} extension should be .wasm", path)
            }
            let bytecode = fs::read(sc_path)?;
            let module =
                RuntimeModule::new(&bytecode, gas, exec_context.gas_costs.clone(), Compiler::CL)?;

            // execute the function
            let (Response { remaining_gas, .. }, function_name) = if let Some(function) = function {
                (
                    run_function(
                        exec_context,
                        module,
                        &function,
                        &parameter.unwrap_or_default(),
                        gas,
                        exec_context.gas_costs.clone(),
                    )?,
                    function,
                )
            } else {
                (
                    run_main(exec_context, module, gas, exec_context.gas_costs.clone())?,
                    "main".to_string(),
                )
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

            // read the bytecode
            let module = RuntimeModule::new(
                &exec_context.get_entry(&address)?.get_bytecode(),
                gas,
                exec_context.gas_costs.clone(),
                Compiler::CL,
            )?;

            // execute the function
            let (Response { remaining_gas, .. }, function_name) = if let Some(function) = function {
                (
                    run_function(
                        exec_context,
                        module,
                        &function,
                        &parameter.unwrap_or_default(),
                        gas,
                        exec_context.gas_costs.clone(),
                    )?,
                    function,
                )
            } else {
                (
                    run_main(exec_context, module, gas, exec_context.gas_costs.clone())?,
                    "main".to_string(),
                )
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
            let json = object!(read_events: JsonValue::from(events));
            trace.push(json)?;
        }
        StepConfig::ReadLedgerEntry { address } => {
            let entry = exec_context.get_entry(&address)?;
            let json = object!(read_ledger_entry: JsonValue::from(Some(entry)));
            trace.push(json)?;
        }
        StepConfig::WriteLedgerEntry {
            address,
            balance,
            bytecode,
            datastore,
        } => {
            let bytecode_ = match bytecode {
                Some(bytecode) => fs::read(bytecode).ok(),
                None => None,
            };

            exec_context.create_new_entry(
                address,
                Entry {
                    balance: balance.unwrap_or_default(),
                    bytecode: bytecode_.unwrap_or_default(),
                    datastore: datastore.unwrap_or_default(),
                },
            )?;
        }
        StepConfig::ReadAsyncMessages { start, end } => {
            let msgs = exec_context.get_async_messages_in(start, end)?;
            let json = object!(read_async_messages: JsonValue::from(msgs));
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
                data,
            },
        )?,
    }

    // save the ledger
    exec_context.save()?;
    Ok(trace)
}
