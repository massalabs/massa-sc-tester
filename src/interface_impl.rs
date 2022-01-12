use crate::ledger_interface::InterfaceImpl;
use anyhow::{bail, Result};
use assembly_simulator::{Address, Bytecode, Interface, InterfaceClone};
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

    fn get_module(&self, address: &String) -> Result<Vec<u8>> {
        let entry = self.get_entry(address)?;
        self.call_stack_push(address.to_owned())?;
        entry.get_bytecode()
    }

    /// Returns zero as a default if address not found.
    fn get_balance(&self) -> Result<u64> {
        let address = &self.call_stack_peek()?;
        Ok(self.get_entry(address)?.balance)
    }

    /// Returns zero as a default if address not found.
    fn get_balance_for(&self, address: &String) -> Result<u64> {
        Ok(self.get_entry(address)?.balance)
    }

    fn exit_success(&self) -> Result<()> {
        self.call_stack_pop()
    }

    /// Requires a new address that contains the sent bytecode.
    ///
    /// Generate a new address with a concatenation of the block_id hash, the
    /// operation index in the block and the index of address owned in context.
    ///
    /// Insert in the ledger the given bytecode in the generated address
    fn create_module(&self, module: &Bytecode) -> Result<assembly_simulator::Address> {
        let mut gen = WyHash::with_seed(rand::random());
        gen.write(&[rand::random(), rand::random(), rand::random()]);
        let address = base64::encode(gen.finish().to_be_bytes());
        self.set_module(&address, module)?;
        self.own_insert(&address)?;
        Ok(address)
    }

    /// Requires the data at the address
    fn get_data_for(&self, address: &Address, key: &str) -> Result<Bytecode> {
        self.get(address)?.get_data(key)
    }

    /// Requires to replace the data in the current address
    ///
    /// Note:
    /// The execution lib will allways use the current context address for the update
    fn set_data_for(&self, address: &Address, key: &str, value: &Bytecode) -> Result<()> {
        let curr_address = self.call_stack_peek()?;
        if self.own(address)? || *address == curr_address {
            self.set_data_entry(address, key, value.clone())?;
            Ok(())
        } else {
            bail!("You don't have the write access to this entry")
        }
    }

    fn get_data(&self, key: &str) -> Result<Bytecode> {
        self.get(&self.call_stack_peek()?)?.get_data(key)
    }

    fn set_data(&self, key: &str, value: &Bytecode) -> Result<()> {
        self.set_data_entry(&self.call_stack_peek()?, key, value.clone())
    }

    /// Transfer coins from the current address to a target address
    /// to_address: target address
    /// raw_amount: amount to transfer (in raw u64)
    fn transfer_coins(&self, to_address: &String, raw_amount: u64) -> Result<()> {
        let from_address = self.call_stack_peek()?;
        self.transfer_coins_for(&from_address, to_address, raw_amount)
    }

    /// Transfer coins from the current address to a target address
    /// from_address: source address
    /// to_address: target address
    /// raw_amount: amount to transfer (in raw u64)
    fn transfer_coins_for(
        &self,
        from_address: &String,
        to_address: &String,
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
    fn get_owned_addresses(&self) -> Result<Vec<assembly_simulator::Address>> {
        self.owned_to_vec()
    }

    fn get_call_stack(&self) -> Result<Vec<assembly_simulator::Address>> {
        self.callstack_to_vec()
    }

    fn generate_event(&self, _data: String) -> Result<()> {
        // TODO store the event somewhere
        Ok(())
    }
}
