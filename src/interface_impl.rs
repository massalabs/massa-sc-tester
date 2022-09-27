use crate::ledger_interface::InterfaceImpl;

use anyhow::{bail, Result};
use massa_sc_runtime::{Interface, InterfaceClone};
use std::hash::Hasher;
use wyhash::WyHash;

impl InterfaceClone for InterfaceImpl {
    fn clone_box(&self) -> Box<dyn Interface> {
        Box::new(self.clone())
    }
}

impl Interface for InterfaceImpl {
    fn print(&self, message: &str) -> Result<()> {
        println!("SC print: {}", message);
        Ok(())
    }

    fn init_call(&self, address: &str, raw_coins: u64) -> Result<Vec<u8>> {
        let entry = self.get_entry(address)?;
        let from_address = self.call_stack_peek()?.address;
        if raw_coins > 0 {
            self.transfer_coins_for(&from_address, address, raw_coins)?
        }
        self.call_stack_push(crate::ledger_interface::CallItem {
            address: address.to_owned(),
            coins: raw_coins,
        })?;
        entry.get_bytecode()
    }

    /// Returns zero as a default if address not found.
    fn get_balance(&self) -> Result<u64> {
        let address = &self.call_stack_peek()?.address;
        Ok(self.get_entry(address)?.balance)
    }

    /// Returns zero as a default if address not found.
    fn get_balance_for(&self, address: &str) -> Result<u64> {
        Ok(self.get_entry(address)?.balance)
    }

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
        Ok(address)
    }

    /// Requires the data at the address
    fn raw_get_data_for(&self, address: &str, key: &str) -> Result<Vec<u8>> {
        self.get(address)?.get_data(key)
    }

    /// Requires to replace the data in the current address
    ///
    /// Note:
    /// The execution lib will allways use the current context address for the update
    fn raw_set_data_for(&self, address: &str, key: &str, value: &[u8]) -> Result<()> {
        let curr_address = self.call_stack_peek()?.address;
        if self.own(address)? || *address == curr_address {
            self.set_data_entry(address, key, value.to_vec())?;
            Ok(())
        } else {
            bail!("You don't have the write access to this entry")
        }
    }

    fn raw_get_data(&self, key: &str) -> Result<Vec<u8>> {
        self.get(&self.call_stack_peek()?.address)?.get_data(key)
    }

    fn raw_set_data(&self, key: &str, value: &[u8]) -> Result<()> {
        self.set_data_entry(&self.call_stack_peek()?.address, key, value.to_vec())
    }

    /// Transfer coins from the current address to a target address
    /// to_address: target address
    /// raw_amount: amount to transfer (in raw u64)
    fn transfer_coins(&self, to_address: &str, raw_amount: u64) -> Result<()> {
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
        Ok(())
    }

    /// Return the list of owned adresses of a given SC user
    fn get_owned_addresses(&self) -> Result<Vec<String>> {
        self.owned_to_vec()
    }

    fn get_call_stack(&self) -> Result<Vec<String>> {
        self.callstack_to_vec()
    }

    fn generate_event(&self, data: String) -> Result<()> {
        println!("Event sent: {}", data);
        Ok(())
    }

    fn get_call_coins(&self) -> Result<u64> {
        Ok(self.call_stack_peek()?.coins)
    }

    fn has_data(&self, key: &str) -> Result<bool> {
        Ok(self.get(&self.call_stack_peek()?.address)?.has_data(key))
    }

    fn hash(&self, key: &[u8]) -> Result<String> {
        println!("Info: hashing will produce a different value than the real node.");
        Ok(String::from_utf8(key.to_vec())?)
    }

    fn raw_set_bytecode_for(&self, address: &str, bytecode: &[u8]) -> Result<()> {
        self.set_module(address, bytecode)?;
        Ok(())
    }

    fn raw_set_bytecode(&self, bytecode: &[u8]) -> Result<()> {
        self.set_module(&self.call_stack_peek()?.address, bytecode)?;
        Ok(())
    }

    fn unsafe_random(&self) -> Result<i64> {
        Ok(rand::random())
    }

    fn get_current_period(&self) -> Result<u64> {
        Ok(self.execution_slot.period)
    }

    fn get_current_thread(&self) -> Result<u8> {
        Ok(self.execution_slot.thread)
    }

    fn send_message(
        &self,
        _target_address: &str,
        _target_handler: &str,
        _validity_start: (u64, u8),
        _validity_end: (u64, u8),
        _max_gas: u64,
        _gas_price: u64,
        _coins: u64,
        data: &[u8],
    ) -> Result<()> {
        println!("Sent message data: {:?}", data);
        Ok(())
    }
}
