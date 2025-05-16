mod config;
mod client;
mod processor;
mod utils;
mod instruction;
mod transaction;

use config::Config;
use client::ShredstreamClient;
use processor::TransactionProcessor;
use utils::deserialize_entries;
use utils::redis::RedisClient;
use utils::auto_trader::AutoTrader;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::env;
use dotenvy::dotenv;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();

    // Get configuration
    let config = Config::new();
    let client_result = ShredstreamClient::new(config.clone()).await;
    let mut client = match client_result {
        Ok(client) => client,
        Err(e) => {
            println!("Failed to create client: {:?}", e);
            return;
        }
    };

    let mut processor = TransactionProcessor::new(config.token_creator_pubkey);

    // Get Redis configuration
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    println!("Connecting to Redis: {}", redis_url);

    // Get RPC and private key
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let private_key = match env::var("PRIVATE_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("PRIVATE_KEY environment variable must be set");
            return;
        }
    };

    // Initialize Redis client
    let redis_client_result = RedisClient::new(&redis_url).await;
    let redis_client = match redis_client_result {
        Ok(client) => {
            println!("Redis connection successful");
            Arc::new(client)
        },
        Err(e) => {
            println!("Redis connection failed: {:?}, auto-trading function will not be used", e);
            return;
        }
    };

    // Initialize AutoTrader
    let auto_trader = AutoTrader::new(
        redis_client.clone(),
        rpc_url.clone(),
        private_key.clone()
    ).await;

    // Read sniping price range from environment variables
    let min_sol_str = env::var("MIN_SOL_PRICE").unwrap_or_else(|_| "0.5".to_string());
    let max_sol_str = env::var("MAX_SOL_PRICE").unwrap_or_else(|_| "3.0".to_string());
    let buy_sol_str = env::var("BUY_SOL_AMOUNT").unwrap_or_else(|_| "0.1".to_string());
    let sell_delay_ms = env::var("SELL_DELAY_MS").unwrap_or_else(|_| "5000".to_string());

    // Convert floating-point SOL values to integer lamports
    let min_sol = (min_sol_str.parse::<f64>().unwrap_or(0.5) * 1_000_000_000.0) as u64;
    let max_sol = (max_sol_str.parse::<f64>().unwrap_or(3.0) * 1_000_000_000.0) as u64;
    let buy_sol = (buy_sol_str.parse::<f64>().unwrap_or(0.1) * 1_000_000_000.0) as u64;
    let sell_delay = sell_delay_ms.parse::<u64>().unwrap_or(5000);

    // Create a mutex for the AutoTrader
    let auto_trader = Arc::new(Mutex::new(auto_trader));

    // Set trader parameters and start
    {
        let mut trader = auto_trader.lock().await;
        trader.set_price_range(min_sol, max_sol).await;
        trader.set_buy_amount(buy_sol).await;
        trader.set_sell_delay(sell_delay).await;
        trader.start();
    }

    // Set the AutoTrader for the processor
    processor.set_auto_trader(Arc::clone(&auto_trader));

    println!("Starting to listen for Jito Shredstream data...");
    println!("Will automatically snipe new tokens with a price between {} - {} SOL", min_sol_str, max_sol_str);
    println!("Will invest {} SOL for each purchase", buy_sol_str);
    println!("Will automatically sell after {}ms", sell_delay);
    println!("---------------------------");

    // Main loop - continuously listen for Shredstream data
    loop {
        match client.subscribe_entries().await {
            Ok(mut stream) => {
                let process_result = async {
                    while let Some(entry) = match stream.message().await {
                        Ok(entry) => entry,
                        Err(e) => {
                            println!("Failed to get message: {:?}", e);
                            return Ok(());
                        }
                    } {
                        match deserialize_entries(&entry.entries) {
                            Ok(entries) => {
                                if let Err(e) = processor.process_entries(entries, entry.slot) {
                                    println!("Failed to process entries: {:?}", e);
                                }
                            },
                            Err(e) => {
                                println!("Deserialization failed: {e}");
                            }
                        }
                    }
                    Ok::<(), ()>(())
                }.await;

                if let Err(_) = process_result {
                    println!("Fatal error occurred in message processing loop");
                }
            }
            Err(e) => {
                println!("Connection lost: {e}");
                println!("Reconnecting in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}
