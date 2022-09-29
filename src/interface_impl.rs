use crate::execution_context::{AsyncMessage, ExecutionContext, Slot};

use anyhow::{bail, Result};
use json::object;
use massa_sc_runtime::{Interface, InterfaceClone};
use std::hash::Hasher;
use wyhash::WyHash;

impl InterfaceClone for ExecutionContext {
    fn clone_box(&self) -> Box<dyn Interface> {
        Box::new(self.clone())
    }
}

impl Interface for ExecutionContext {
    fn print(&self, message: &str) -> Result<()> {
        let json = object!(
            print: {
                message: message
            }
        );
        self.update_execution_trace(json)?;
        Ok(())
    }

    fn init_call(&self, address: &str, raw_coins: u64) -> Result<Vec<u8>> {
        let entry = self.get_entry(address)?;
        let from_address = self.call_stack_peek()?.address;
        if raw_coins > 0 {
            self.transfer_coins_for(&from_address, address, raw_coins)?
        }
        self.call_stack_push(crate::execution_context::CallItem {
            address: address.to_owned(),
            coins: raw_coins,
        })?;
        entry.get_bytecode()
    }

    /// Returns zero as a default if address not found.
    fn get_balance(&self) -> Result<u64> {
        let address = &self.call_stack_peek()?.address;
        let balance = self.get_entry(address)?.balance;
        let json = object!(
            get_balance: {
                return_value: balance
            }
        );
        self.update_execution_trace(json)?;
        Ok(balance)
    }

    /// Returns zero as a default if address not found.
    fn get_balance_for(&self, address: &str) -> Result<u64> {
        let balance = self.get_entry(address)?.balance;
        let json = object!(
            get_balance_for: {
                address: address,
                return_value: balance
            }
        );
        self.update_execution_trace(json)?;
        Ok(balance)
    }

    /// Pops the last element of the call stack
    fn finish_call(&self) -> Result<()> {
        self.call_stack_pop()
    }

    /// Requires a new address that contains the sent bytecode.
    ///
    /// Generate a new address with a concatenation of the block_id hash, the
    /// operation index in the block and the index of address owned in context.
    ///
    /// Insert in the ledger the given bytecode in the generated address
    fn create_module(&self, module: &[u8]) -> Result<String> {
        let mut gen = WyHash::with_seed(rand::random());
        gen.write(&[rand::random(), rand::random(), rand::random()]);
        let address = base64::encode(gen.finish().to_be_bytes());
        self.set_module(&address, module)?;
        self.own_insert(&address)?;
        let json = object!(
            create_module: {
                module: module,
                return_value: address.clone()
            }
        );
        self.update_execution_trace(json)?;
        Ok(address)
    }

    /// Requires the data at the address
    fn raw_get_data_for(&self, address: &str, key: &str) -> Result<Vec<u8>> {
        let data = self.get(address)?.get_data(key)?;
        let json = object!(
            raw_get_data_for: {
                address: address,
                key: key,
                return_value: data.clone(),
            }
        );
        self.update_execution_trace(json)?;
        Ok(data)
    }

    /// Requires to replace the data in the current address
    ///
    /// Note:
    /// The execution lib will allways use the current context address for the update
    fn raw_set_data_for(&self, address: &str, key: &str, value: &[u8]) -> Result<()> {
        let curr_address = self.call_stack_peek()?.address;
        let json = object!(
            raw_set_data_for: {
                address: address,
                key: key,
                value: value,
            }
        );
        self.update_execution_trace(json)?;
        if self.own(address)? || *address == curr_address {
            self.set_data_entry(address, key, value.to_vec())?;
            Ok(())
        } else {
            bail!("You don't have the write access to this entry")
        }
    }

    fn raw_get_data(&self, key: &str) -> Result<Vec<u8>> {
        let data = self.get(&self.call_stack_peek()?.address)?.get_data(key)?;
        let json = object!(
            raw_get_data: {
                key: key,
                return_value: data.clone()
            }
        );
        self.update_execution_trace(json)?;
        Ok(data)
    }

    fn raw_set_data(&self, key: &str, value: &[u8]) -> Result<()> {
        let json = object!(
            raw_set_data: {
                key: key,
                value: value
            }
        );
        self.update_execution_trace(json)?;
        self.set_data_entry(&self.call_stack_peek()?.address, key, value.to_vec())
    }

    /// Transfer coins from the current address to a target address
    /// to_address: target address
    /// raw_amount: amount to transfer (in raw u64)
    fn transfer_coins(&self, to_address: &str, raw_amount: u64) -> Result<()> {
        let json = object!(
            transfer_coins: {
                to_address: to_address,
                raw_amount: raw_amount
            }
        );
        self.update_execution_trace(json)?;
        let from_address = self.call_stack_peek()?.address;
        self.transfer_coins_for(&from_address, to_address, raw_amount)
    }

    /// Transfer coins from the current address to a target address
    /// from_address: source address
    /// to_address: target address
    /// raw_amount: amount to transfer (in raw u64)
    fn transfer_coins_for(
        &self,
        from_address: &str,
        to_address: &str,
        raw_amount: u64,
    ) -> Result<()> {
        // debit
        self.sub(from_address, raw_amount)?;
        // credit
        if let Err(err) = self.add(to_address, raw_amount) {
            // cancel debit
            self.add(from_address, raw_amount)
                .expect("credit failed after same-amount debit succeeded");
            bail!("Error crediting destination balance: {}", err);
        }
        let json = object!(
            transfer_coins_for: {
                from_address: from_address,
                to_address: to_address,
                raw_amount: raw_amount
            }
        );
        self.update_execution_trace(json)?;
        Ok(())
    }

    /// Return the list of owned adresses of a given SC user
    fn get_owned_addresses(&self) -> Result<Vec<String>> {
        let owned = self.owned_to_vec()?;
        let json = object!(
            get_owned_addresses: {
                return_value: owned.clone()
            }
        );
        self.update_execution_trace(json)?;
        Ok(owned)
    }

    fn get_call_stack(&self) -> Result<Vec<String>> {
        let callstack = self.callstack_to_vec()?;
        let json = object!(
            get_call_stack: {
                return_value: callstack.clone()
            }
        );
        self.update_execution_trace(json)?;
        Ok(callstack)
    }

    fn generate_event(&self, data: String) -> Result<()> {
        let json = object!(
            generate_event: {
                return_value: data
            }
        );
        self.update_execution_trace(json)?;
        Ok(())
    }

    fn get_call_coins(&self) -> Result<u64> {
        let coins = self.call_stack_peek()?.coins;
        let json = object!(
            get_call_coins: {
                return_value: coins
            }
        );
        self.update_execution_trace(json)?;
        Ok(coins)
    }

    fn has_data(&self, key: &str) -> Result<bool> {
        let ret_bool = self.get(&self.call_stack_peek()?.address)?.has_data(key);
        let json = object!(
            has_data: {
                key: key,
                return_value: ret_bool
            }
        );
        self.update_execution_trace(json)?;
        Ok(ret_bool)
    }

    fn hash(&self, key: &[u8]) -> Result<String> {
        let hash = String::from_utf8(key.to_vec())?;
        let json = object!(
            hash: {
                key: key,
                return_value: hash.clone()
            }
        );
        self.update_execution_trace(json)?;
        Ok(hash)
    }

    fn raw_set_bytecode_for(&self, address: &str, bytecode: &[u8]) -> Result<()> {
        self.set_module(address, bytecode)?;
        let json = object!(
            raw_set_bytecode_for: {
                address: address,
                return_value: bytecode
            }
        );
        self.update_execution_trace(json)?;
        Ok(())
    }

    fn raw_set_bytecode(&self, bytecode: &[u8]) -> Result<()> {
        self.set_module(&self.call_stack_peek()?.address, bytecode)?;
        let json = object!(
            raw_set_bytecode: {
                return_value: bytecode
            }
        );
        self.update_execution_trace(json)?;
        Ok(())
    }

    fn unsafe_random(&self) -> Result<i64> {
        let rnbr: i64 = rand::random();
        let json = object!(
            unsafe_random: {
                return_value: rnbr
            }
        );
        self.update_execution_trace(json)?;
        Ok(rnbr)
    }

    fn get_current_period(&self) -> Result<u64> {
        let json = object!(
            get_current_period: {
                return_value:  self.execution_slot.period
            }
        );
        self.update_execution_trace(json)?;
        Ok(self.execution_slot.period)
    }

    fn get_current_thread(&self) -> Result<u8> {
        let json = object!(
            get_current_thread: {
                return_value:  self.execution_slot.thread
            }
        );
        self.update_execution_trace(json)?;
        Ok(self.execution_slot.thread)
    }

    fn send_message(
        &self,
        target_address: &str,
        target_handler: &str,
        validity_start: (u64, u8),
        validity_end: (u64, u8),
        max_gas: u64,
        gas_price: u64,
        coins: u64,
        data: &[u8],
    ) -> Result<()> {
        self.push_async_message(
            Slot {
                period: validity_start.0,
                thread: validity_start.1,
            },
            AsyncMessage {
                sender_address: "".to_string(),
                target_address: target_address.to_string(),
                target_handler: target_handler.to_string(),
                gas: max_gas,
                coins,
                data: data.to_vec(),
            },
        )?;
        let json = object!(
            send_message: {
                target_address: target_address,
                target_handler: target_handler,
                validity_start_period: validity_start.0,
                validity_start_thread: validity_start.1,
                validity_end_period: validity_end.0,
                validity_end_thread: validity_end.1,
                max_gas: max_gas,
                gas_price: gas_price,
                coins: coins,
                data: data,
            }
        );
        self.update_execution_trace(json)?;
        Ok(())
    }
}
