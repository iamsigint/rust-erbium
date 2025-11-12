// src/bridges/chains/bitcoin/transaction_builder.rs
use super::types::*;
use crate::bridges::chains::bitcoin::BitcoinAdapter;
use bitcoin::{Address, Transaction, TxOut, TxIn, OutPoint, Script};
use std::str::FromStr;

pub struct BitcoinTransactionBuilder<'a> {
    _adapter: &'a BitcoinAdapter,
    utxos: Vec<BitcoinUtxo>,
    change_address: Option<String>,
    fee_rate: f64, // satoshis per byte
}

impl<'a> BitcoinTransactionBuilder<'a> {
    pub fn new(adapter: &'a BitcoinAdapter) -> Self {
        Self {
            _adapter: adapter,
            utxos: Vec::new(),
            change_address: None,
            fee_rate: 1.0,
        }
    }
    
    pub fn with_utxos(mut self, utxos: Vec<BitcoinUtxo>) -> Self {
        self.utxos = utxos;
        self
    }
    
    pub fn with_change_address(mut self, address: String) -> Self {
        self.change_address = Some(address);
        self
    }
    
    pub fn with_fee_rate(mut self, fee_rate: f64) -> Self {
        self.fee_rate = fee_rate;
        self
    }
    
    pub fn build_transaction(
        &self,
        recipient: &str,
        amount: u64,
    ) -> Result<UnsignedBitcoinTransaction, BitcoinError> {
        let _recipient_address = Address::from_str(recipient)
            .map_err(|e| BitcoinError::InvalidAddress(e.to_string()))?;
        
        // Calculate total available from UTXOs
        let total_input: u64 = self.utxos.iter().map(|utxo| utxo.amount).sum();
        
        if total_input < amount {
            return Err(BitcoinError::InsufficientFunds);
        }
        
        // Estimate transaction size and fee
        let estimated_size = self.estimate_transaction_size();
        let fee = (estimated_size as f64 * self.fee_rate) as u64;
        
        let _change_amount = total_input - amount - fee;
        
        // Validate we have enough for fee
        // change_amount cannot be negative in u64; check underflow via ordering
        if total_input < amount + fee {
            return Err(BitcoinError::InsufficientFunds);
        }
        
        Ok(UnsignedBitcoinTransaction {
            recipient: recipient.to_string(),
            amount,
            fee_rate: self.fee_rate,
            inputs: self.utxos.clone(),
            change_address: self.change_address.clone(),
        })
    }
    
    pub fn create_transaction_from_utxos(
        &self,
        recipient: &str,
        amount: u64,
        utxos: Vec<BitcoinUtxo>,
    ) -> Result<Transaction, BitcoinError> {
        let recipient_address = Address::from_str(recipient)
            .map_err(|e| BitcoinError::InvalidAddress(e.to_string()))?;
        
        let total_input: u64 = utxos.iter().map(|utxo| utxo.amount).sum();
        
        if total_input < amount {
            return Err(BitcoinError::InsufficientFunds);
        }
        
        // Create transaction inputs
        let inputs: Vec<TxIn> = utxos.iter()
            .map(|utxo| {
                let outpoint = OutPoint::from_str(&format!("{}:{}", utxo.txid, utxo.vout))
                    .unwrap(); // In production, handle error properly
                TxIn {
                    previous_output: outpoint,
                    script_sig: Script::new().into(), // Empty for unsigned
                    sequence: bitcoin::Sequence(0xFFFFFFFF), // Maximum sequence number
                    witness: bitcoin::Witness::new(),
                }
            })
            .collect();
        
        // Create transaction outputs
        let mut outputs = Vec::new();
        
        // Recipient output
        let recipient_checked = recipient_address.require_network(bitcoin::Network::Bitcoin)
            .map_err(|e| BitcoinError::InvalidAddress(e.to_string()))?;
        let recipient_output = TxOut {
            value: bitcoin::Amount::from_sat(amount),
            script_pubkey: recipient_checked.script_pubkey(),
        };
        outputs.push(recipient_output);
        
        // Change output if needed
        let estimated_size = self.estimate_transaction_size_with_utxos(&utxos);
        let fee = (estimated_size as f64 * self.fee_rate) as u64;
        let change_amount = total_input - amount - fee;
        
        if change_amount > 0 {
            if let Some(change_addr) = &self.change_address {
                let change_address = Address::from_str(change_addr)
                    .map_err(|e| BitcoinError::InvalidAddress(e.to_string()))?;
                
                let change_checked = change_address.require_network(bitcoin::Network::Bitcoin)
                    .map_err(|e| BitcoinError::InvalidAddress(e.to_string()))?;
                let change_output = TxOut {
                    value: bitcoin::Amount::from_sat(change_amount),
                    script_pubkey: change_checked.script_pubkey(),
                };
                outputs.push(change_output);
            }
        }
        
        // Create transaction
        let transaction = Transaction {
            version: bitcoin::transaction::Version(2),
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: inputs,
            output: outputs,
        };
        
        Ok(transaction)
    }
    
    fn estimate_transaction_size(&self) -> usize {
        // Basic size estimation
        // 10 bytes per input (simplified)
        // 34 bytes per output (simplified)
        let input_size = self.utxos.len() * 10;
        let output_size = 34 * 2; // recipient + change
        
        10 + input_size + output_size // Base + inputs + outputs
    }
    
    fn estimate_transaction_size_with_utxos(&self, utxos: &[BitcoinUtxo]) -> usize {
        let input_size = utxos.len() * 10;
        let output_size = 34 * 2; // recipient + change
        
        10 + input_size + output_size
    }
}
