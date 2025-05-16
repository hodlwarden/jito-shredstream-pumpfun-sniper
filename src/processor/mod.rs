use chrono::Local;
use solana_sdk::{message::VersionedMessage, pubkey::Pubkey, transaction::VersionedTransaction};
use solana_entry::entry::Entry;
use crate::instruction::parse_instruction_data;
use std::error::Error;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::utils::auto_trader::AutoTrader;

// Used to store virtual reserve information for tokens
struct TokenReserves {
    virtual_sol_reserves: u64,    // Virtual SOL reserves
    virtual_token_reserves: u64,  // Virtual token reserves
}

pub struct TransactionProcessor {
    token_creator_pubkey: Pubkey,
    // Use HashMap to track virtual reserve states for various tokens
    token_reserves: HashMap<String, TokenReserves>,
    // Auto trader
    auto_trader: Option<Arc<Mutex<AutoTrader>>>,
}

impl TransactionProcessor {
    pub fn new(token_creator_pubkey: Pubkey) -> Self {
        Self { 
            token_creator_pubkey,
            token_reserves: HashMap::new(),
            auto_trader: None,
        }
    }
    
    // Set up the auto trader
    pub fn set_auto_trader(&mut self, auto_trader: Arc<Mutex<AutoTrader>>) {
        self.auto_trader = Some(auto_trader);
        println!("Auto trader has been set up");
    }

    pub fn process_entries(&mut self, entries: Vec<Entry>, slot: u64) -> Result<(), Box<dyn Error>> {
        for entry in entries {
            for tx_data in entry.transactions {
                let transaction = tx_data;
                
                match &transaction.message {
                    VersionedMessage::V0(message) => self.process_message_v0(message, &transaction, slot)?,
                    VersionedMessage::Legacy(message) => self.process_message_legacy(message, &transaction, slot)?,
                }
            }
        }
        Ok(())
    }

    fn process_message_v0(&mut self, message: &solana_sdk::message::v0::Message, transaction: &VersionedTransaction, slot: u64) -> Result<(), Box<dyn Error>> {
        if message.account_keys.contains(&self.token_creator_pubkey) {
            println!("\n{}", "-".repeat(80));
            println!("[{}] Pumpfun internal token creation event:", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"));
            println!("Slot: {}", slot);
            println!("Signatures: {}", transaction.signatures[0]);
            
            // Extract key account addresses
            let mint_address = message.account_keys[1].to_string();
            let bonding_curve = message.account_keys[2].to_string();
            
            println!("Mint: {}", mint_address);
            println!("Bonding_Curve: {}", bonding_curve);

            // Check all instructions in the transaction
            for instruction in &message.instructions {
                let program_id = message.account_keys[instruction.program_id_index as usize].to_string();
                
                // If the instruction is for the target program
                if program_id == self.token_creator_pubkey.to_string() || program_id == "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" {
                    // Attempt to parse the instruction
                    if let Ok((instruction_type, create_event, buy_event)) = parse_instruction_data(&instruction.data) {
                        match instruction_type.as_str() {
                            "CreateEvent" => {
                                if let Some(event) = create_event {
                                    println!("Token_Metadata:");
                                    println!("  Name: {}", event.name);
                                    println!("  Symbol: {}", event.symbol);
                                    println!("  URI: {}", event.uri);
                                    println!("  Creator: {}", event.user);
                                    
                                    // Initialize virtual reserves for the new token
                                    if !self.token_reserves.contains_key(&mint_address) {
                                        // Initialize virtual reserve values - adjusted based on transaction records for more accurate values
                                        let virtual_sol_reserves = 30_000_000_000;             // 30 SOL (lamports)
                                        let virtual_token_reserves = 1_073_000_000_000_000;    // Approximately 1.073 billion tokens (6 decimal precision)
                                        
                                        self.token_reserves.insert(mint_address.clone(), TokenReserves {
                                            virtual_sol_reserves,
                                            virtual_token_reserves,
                                        });
                                    }
                                }
                            }
                            "Buy" => {
                                if let Some(event) = buy_event {
                                    // Use raw values directly, preserving precision
                                    let token_amount = event.amount;
                                    let sol_amount = event.max_sol_cost;
                                    
                                    // Simplified display output
                                    let token_amount_display = token_amount as f64 / 1_000_000.0; // Considering 6 decimal places
                                    let sol_amount_display = sol_amount as f64 / 1_000_000_000.0;
                                    
                                    println!("Buy_Event:");
                                    println!("  User: {}", message.account_keys[0]);
                                    println!("  SOL_Amount: {:.6}", sol_amount_display);
                                    println!("  Token_Amount: {:.6}", token_amount_display);
                                    
                                    // Check if snipe conditions are met
                                    if let Some(auto_trader) = &self.auto_trader {
                                        // Clone mint_address and auto_trader for use in async closure
                                        let mint = mint_address.clone();
                                        let trader_clone = Arc::clone(auto_trader);
                                        
                                        // Use tokio::spawn to start an async task to check if sniping is needed
                                        let sol_amount_copy = sol_amount;
                                        let sol_display = sol_amount_display;
                                        
                                        // Get current token price
                                        let token_price = if let Some(reserves) = self.token_reserves.get(&mint_address) {
                                            let virtual_sol = reserves.virtual_sol_reserves as f64 / 1_000_000_000.0;
                                            let virtual_token = reserves.virtual_token_reserves as f64 / 1_000_000.0;
                                            virtual_sol / virtual_token
                                        } else {
                                            0.000000033 // Default estimated value if actual price cannot be obtained
                                        };
                                        
                                        // Pass slot to be used for getting an appropriate block hash
                                        let current_slot = slot;
                                        
                                        // Use tokio::spawn to execute async code
                                        tokio::spawn(async move {
                                            // Record start time for monitoring processing delay
                                            let start_time = std::time::Instant::now();
                                            
                                            let should_snipe = {
                                                let trader = trader_clone.lock().await;
                                                trader.should_snipe(sol_amount_copy)
                                            };
                                            
                                            if should_snipe {
                                                println!("Detected eligible purchase, preparing to snipe: {} SOL", sol_display);
                                                println!("Using slot: {}, current time: {}", current_slot, Local::now().format("%H:%M:%S%.3f"));
                                                println!("Delay from detection to snipe preparation: {:.3}ms", start_time.elapsed().as_millis());
                                                
                                                // Acquire lock to execute snipe, passing slot
                                                let trader = trader_clone.lock().await;
                                                if let Err(e) = trader.snipe_token(&mint, token_price, Some(current_slot)).await {
                                                    println!("Snipe failed: {:?}", e);
                                                }
                                            }
                                        });
                                    }
                                    
                                    // Update virtual reserves (for internal calculation only, not displayed as real values)
                                    if let Some(reserves) = self.token_reserves.get_mut(&mint_address) {
                                        // State before update
                                        let old_virtual_token = reserves.virtual_token_reserves;
                                        
                                        // Update virtual reserves, adding overflow check
                                        reserves.virtual_sol_reserves = reserves.virtual_sol_reserves.saturating_add(sol_amount);
                                        
                                        // Use saturating_sub to avoid overflow
                                        if token_amount <= reserves.virtual_token_reserves {
                                            reserves.virtual_token_reserves = reserves.virtual_token_reserves.saturating_sub(token_amount);
                                        }
                                        
                                        // Calculate price (using virtual reserves)
                                        let virtual_sol = reserves.virtual_sol_reserves as f64 / 1_000_000_000.0;
                                        let virtual_token = reserves.virtual_token_reserves as f64 / 1_000_000.0;
                                        let price = virtual_sol / virtual_token;
                                        
                                        // realSolReserves and realTokenReserves are actually just data extracted from the transaction, not real reserve states
                                        // realSolReserves is usually the SOL invested in the transaction
                                        let real_sol_reserves = sol_amount_display;
                                        
                                        // realTokenReserves is based on the token reserve before the transaction minus the tokens obtained
                                        // Use checked_sub to avoid overflow, display 0 if overflow occurs
                                        let real_token_reserves = if old_virtual_token >= token_amount {
                                            (old_virtual_token - token_amount) as f64 / 1_000_000.0
                                        } else {
                                            0.0 // Display 0 if overflow occurs
                                        };
                                        
                                        println!("  realSolReserves: {:.6}", real_sol_reserves);
                                        println!("  realTokenReserves: {:.6}", real_token_reserves);
                                        println!("  Price: {:.9}", price);
                                    }
                                }
                            }
                            _ => {
                                // Other instruction types are not processed for now
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn process_message_legacy(&mut self, message: &solana_sdk::message::Message, transaction: &VersionedTransaction, slot: u64) -> Result<(), Box<dyn Error>> {
        if message.account_keys.contains(&self.token_creator_pubkey) {
            println!("\n{}", "-".repeat(80));
            println!("[{}] Pumpfun internal token creation event:", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"));
            println!("Slot: {}", slot);
            println!("Signatures: {}", transaction.signatures[0]);
            
            // Extract key account addresses
            let mint_address = message.account_keys[1].to_string();
            let bonding_curve = message.account_keys[2].to_string();
            
            println!("Mint: {}", mint_address);
            println!("Bonding_Curve: {}", bonding_curve);

            // Check all instructions in the transaction
            for instruction in &message.instructions {
                let program_id = message.account_keys[instruction.program_id_index as usize].to_string();
                
                // If the instruction is for the target program
                if program_id == self.token_creator_pubkey.to_string() || program_id == "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" {
                    // Attempt to parse the instruction
                    if let Ok((instruction_type, create_event, buy_event)) = parse_instruction_data(&instruction.data) {
                        match instruction_type.as_str() {
                            "CreateEvent" => {
                                if let Some(event) = create_event {
                                    println!("Token_Metadata:");
                                    println!("  Name: {}", event.name);
                                    println!("  Symbol: {}", event.symbol);
                                    println!("  URI: {}", event.uri);
                                    println!("  Creator: {}", event.user);
                                    
                                    // Initialize virtual reserves for the new token
                                    if !self.token_reserves.contains_key(&mint_address) {
                                        // Initialize virtual reserve values - adjusted based on transaction records for more accurate values
                                        let virtual_sol_reserves = 30_000_000_000;             // 30 SOL (lamports)
                                        let virtual_token_reserves = 1_073_000_000_000_000;    // Approximately 1.073 billion tokens (6 decimal precision)
                                        
                                        self.token_reserves.insert(mint_address.clone(), TokenReserves {
                                            virtual_sol_reserves,
                                            virtual_token_reserves,
                                        });
                                    }
                                }
                            }
                            "Buy" => {
                                if let Some(event) = buy_event {
                                    // Use raw values directly, preserving precision
                                    let token_amount = event.amount;
                                    let sol_amount = event.max_sol_cost;
                                    
                                    // Simplified display output
                                    let token_amount_display = token_amount as f64 / 1_000_000.0; // Considering 6 decimal places
                                    let sol_amount_display = sol_amount as f64 / 1_000_000_000.0;
                                    
                                    println!("Buy_Event:");
                                    println!("  User: {}", message.account_keys[0]);
                                    println!("  SOL_Amount: {:.6} SOL", sol_amount_display);
                                    println!("  Token_Amount: {:.6} ", token_amount_display);
                                    
                                    // Check if snipe conditions are met
                                    if let Some(auto_trader) = &self.auto_trader {
                                        // Clone mint_address and auto_trader for use in async closure
                                        let mint = mint_address.clone();
                                        let trader_clone = Arc::clone(auto_trader);
                                        
                                        // Use tokio::spawn to start an async task to check if sniping is needed
                                        let sol_amount_copy = sol_amount;
                                        let sol_display = sol_amount_display;
                                        
                                        // Get current token price
                                        let token_price = if let Some(reserves) = self.token_reserves.get(&mint_address) {
                                            let virtual_sol = reserves.virtual_sol_reserves as f64 / 1_000_000_000.0;
                                            let virtual_token = reserves.virtual_token_reserves as f64 / 1_000_000.0;
                                            virtual_sol / virtual_token
                                        } else {
                                            0.000000033 // Default estimated value if actual price cannot be obtained
                                        };
                                        
                                        // Pass slot to be used for getting an appropriate block hash
                                        let current_slot = slot;
                                        
                                        // Use tokio::spawn to execute async code
                                        tokio::spawn(async move {
                                            // Record start time for monitoring processing delay
                                            let start_time = std::time::Instant::now();
                                            
                                            let should_snipe = {
                                                let trader = trader_clone.lock().await;
                                                trader.should_snipe(sol_amount_copy)
                                            };
                                            
                                            if should_snipe {
                                                println!("Detected eligible purchase, preparing to snipe: {} SOL", sol_display);
                                                println!("Using slot: {}, current time: {}", current_slot, Local::now().format("%H:%M:%S%.3f"));
                                                println!("Delay from detection to snipe preparation: {:.3}ms", start_time.elapsed().as_millis());
                                                
                                                // Acquire lock to execute snipe, passing slot
                                                let trader = trader_clone.lock().await;
                                                if let Err(e) = trader.snipe_token(&mint, token_price, Some(current_slot)).await {
                                                    println!("Snipe failed: {:?}", e);
                                                }
                                            }
                                        });
                                    }
                                    
                                    // Update virtual reserves (for internal calculation only, not displayed as real values)
                                    if let Some(reserves) = self.token_reserves.get_mut(&mint_address) {
                                        // State before update
                                        let old_virtual_token = reserves.virtual_token_reserves;
                                        
                                        // Update virtual reserves, adding overflow check
                                        reserves.virtual_sol_reserves = reserves.virtual_sol_reserves.saturating_add(sol_amount);
                                        
                                        // Use saturating_sub to avoid overflow
                                        if token_amount <= reserves.virtual_token_reserves {
                                            reserves.virtual_token_reserves = reserves.virtual_token_reserves.saturating_sub(token_amount);
                                        }
                                        
                                        // Calculate price (using virtual reserves)
                                        let virtual_sol = reserves.virtual_sol_reserves as f64 / 1_000_000_000.0;
                                        let virtual_token = reserves.virtual_token_reserves as f64 / 1_000_000.0;
                                        let price = virtual_sol / virtual_token;
                                        
                                        // realSolReserves and realTokenReserves are actually just data extracted from the transaction, not real reserve states
                                        // realSolReserves is usually the SOL invested in the transaction
                                        let real_sol_reserves = sol_amount_display;
                                        
                                        // realTokenReserves is based on the token reserve before the transaction minus the tokens obtained
                                        // Use checked_sub to avoid overflow, display 0 if overflow occurs
                                        let real_token_reserves = if old_virtual_token >= token_amount {
                                            (old_virtual_token - token_amount) as f64 / 1_000_000.0
                                        } else {
                                            0.0 // Display 0 if overflow occurs
                                        };
                                        
                                        println!("  realSolReserves: {:.6}", real_sol_reserves);
                                        println!("  realTokenReserves: {:.6}", real_token_reserves);
                                        println!("  Price: {:.9}", price);
                                    }
                                }
                            }
                            _ => {
                                // Other instruction types are not processed for now
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
} 
