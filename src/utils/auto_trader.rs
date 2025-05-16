use std::error::Error;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio::task::JoinHandle;
use crate::utils::redis::RedisClient;
use crate::transaction::{pump_buy, pump_sell};
use crate::utils::blockhash_cache::BlockhashCache;
use redis::RedisError;

pub struct AutoTrader {
    redis_client: Arc<RedisClient>,
    rpc_url: String,
    private_key: String,
    running: bool,
    min_sol_price: u64,
    max_sol_price: u64,
    buy_amount: u64,     // Buy amount (lamports)
    sell_delay_ms: u64,  // Sell delay time (milliseconds)
    blockhash_cache: Arc<BlockhashCache>, // Add blockhash cache
}

impl AutoTrader {
    // Create a new auto trader, now requires asynchronous initialization
    pub async fn new(
        redis_client: Arc<RedisClient>,
        rpc_url: String,
        private_key: String,
    ) -> Self {
        // Default settings
        let min_sol_price = 500_000_000; // 0.5 SOL
        let max_sol_price = 1_000_000_000; // 1 SOL
        let buy_amount = 100_000_000; // 0.1 SOL
        let sell_delay_ms = 5000; // Auto sell after 5 seconds

        // Create blockhash cache, reduce cache time to 500ms to keep blockhash updated without frequent requests
        let blockhash_cache = Arc::new(BlockhashCache::new(&rpc_url, 500));

        Self {
            redis_client,
            rpc_url,
            private_key,
            running: false,
            min_sol_price,
            max_sol_price,
            buy_amount,
            sell_delay_ms,
            blockhash_cache,
        }
    }

    // Set price range
    pub async fn set_price_range(&mut self, min_sol_price: u64, max_sol_price: u64) {
        self.min_sol_price = min_sol_price;
        self.max_sol_price = max_sol_price;
        println!("Set sniping price range: {} - {} SOL",
                 min_sol_price as f64 / 1_000_000_000.0,
                 max_sol_price as f64 / 1_000_000_000.0);
    }

    // Set buy amount
    pub async fn set_buy_amount(&mut self, buy_amount: u64) {
        self.buy_amount = buy_amount;
        println!("Set sniping buy amount: {} SOL", buy_amount as f64 / 1_000_000_000.0);
    }

    // Set sell delay time
    pub async fn set_sell_delay(&mut self, sell_delay_ms: u64) {
        self.sell_delay_ms = sell_delay_ms;
        println!("Set auto sell delay: {}ms", sell_delay_ms);
    }

    // Start the auto trading background task
    pub fn start(&mut self) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
        self.running = true;
        let rpc_url = self.rpc_url.clone();
        let private_key = self.private_key.clone();
        let redis_client = self.redis_client.clone();
        let blockhash_cache = self.blockhash_cache.clone(); // Clone cache reference

        println!("Starting auto trading background task");

        // Create background task to handle auto sell logic
        tokio::spawn(async move {
            // Auto sell check task
            let sell_task = tokio::spawn({
                let redis_client = redis_client.clone();
                let rpc_url = rpc_url.clone();
                let private_key = private_key.clone();
                let blockhash_cache = blockhash_cache.clone(); // Clone cache reference for internal task

                async move {
                    println!("Starting auto sell check");

                    loop {
                        // Get and remove all tokens to sell - asynchronous version
                        match redis_client.get_and_remove_mints_to_sell().await {
                            Ok(mints) => {
                                if !mints.is_empty() {
                                    // If there are tokens to sell, get blockhash once beforehand
                                    // This reduces the number of individual hash requests per transaction
                                    let blockhash = match blockhash_cache.get_latest_blockhash().await {
                                        Ok(hash) => Some(hash),
                                        Err(e) => {
                                            println!("Failed to get blockhash: {:?}", e);
                                            None
                                        }
                                    };

                                    for mint in mints {
                                        // Perform auto sell operation
                                        match Pubkey::from_str(&mint) {
                                            Ok(mint_pubkey) => {
                                                println!("Executing auto sell for: {}", mint);

                                                // Get the stored token amount
                                                match redis_client.get_mint_amount(&mint).await {
                                                    Ok(Some(token_amount)) => {
                                                        println!("Attempting to sell: {} tokens", token_amount);

                                                        if let Err(e) = pump_sell(
                                                            &rpc_url,
                                                            &private_key,
                                                            mint_pubkey,
                                                            token_amount, // Use the stored token amount
                                                            0, // Minimum receive 0 SOL
                                                            None, // Do not use a specific slot
                                                            blockhash.clone() // Use the cached blockhash
                                                        ).await {
                                                            println!("Auto sell failed: {:?}", e);
                                                        }
                                                    },
                                                    Ok(None) => {
                                                        // If stored amount is not found, use an estimated amount
                                                        // This should rarely happen as we store the amount on buy
                                                        let buy_sol = 100_000_000; // 0.1 SOL in lamports

                                                        // Use a default price estimate
                                                        let default_price = 0.000000033;

                                                        // Convert SOL to actual units
                                                        let buy_sol_f64 = buy_sol as f64 / 1_000_000_000.0;

                                                        // Calculate token amount without precision
                                                        let token_amount_no_precision = buy_sol_f64 / default_price;

                                                        // Reduce amount by 15% to avoid slippage errors
                                                        let reduced_amount = token_amount_no_precision * 0.85;

                                                        // Precision factor is 10^6
                                                        let precision_factor = 1_000_000.0;

                                                        // Calculate token amount with precision, floor
                                                        let token_amount = (reduced_amount * precision_factor).floor() as u64;

                                                        println!("Stored token amount not found, using estimated value: {} tokens (with precision)", token_amount);

                                                        if let Err(e) = pump_sell(
                                                            &rpc_url,
                                                            &private_key,
                                                            mint_pubkey,
                                                            token_amount,
                                                            0, // Minimum receive 0 SOL
                                                            None, // Do not use a specific slot
                                                            blockhash.clone() // Use the cached blockhash
                                                        ).await {
                                                            println!("Auto sell failed: {:?}", e);
                                                        }
                                                    },
                                                    Err(e) => {
                                                        println!("Failed to get token amount: {:?}", e);
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                println!("Invalid token address: {} - {:?}", mint, e);
                                            }
                                        }
                                    }
                                }
                            },
                            Err(e) => println!("Failed to get tokens to sell: {:?}", e)
                        }

                        // Check every second
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            });

            // Wait for the sell task to complete (theoretically won't complete unless error)
            if let Err(e) = sell_task.await {
                println!("Auto sell task terminated unexpectedly: {:?}", e);
            }

            Ok(())
        })
    }

    // Snipe a specific token
    pub async fn snipe_token(&self, token_mint: &str, token_price: f64, slot: Option<u64>) -> Result<(), Box<dyn Error>> {
        // Convert token address to Pubkey
        let mint_pubkey = Pubkey::from_str(token_mint)?;

        // Use the configured buy amount
        let buy_sol = self.buy_amount;

        // Convert buy_sol to SOL units (from lamports)
        let buy_sol_f64 = buy_sol as f64 / 1_000_000_000.0;

        // Ensure price is not zero to avoid division by zero
        if token_price <= 0.0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid token price: {}", token_price)
            )));
        }

        // Calculate token amount without precision
        let token_amount_no_precision = buy_sol_f64 / token_price;

        // Precision factor is 10^6
        let precision_factor = 1_000_000.0;

        // Calculate token amount with precision, floor
        // Reduce buy amount by 15% to avoid slippage errors
        let reduced_amount = token_amount_no_precision * 0.85;
        let token_amount = (reduced_amount * precision_factor).floor() as u64;

        // Record the timestamp when sniping starts
        let start_time = std::time::Instant::now();

        println!("Starting to snipe token {} (slot: {:?})", token_mint, slot);
        println!("Investment: {} SOL", buy_sol_f64);
        println!("Actual price: {} SOL/token", token_price);
        println!("Calculated token amount: {:.2} (no precision)", token_amount_no_precision);
        println!("Reduced amount: {:.2} (no precision)", reduced_amount);
        println!("Attempting to buy: {} tokens (with precision)", token_amount);

        // Get cached blockhash, prioritize fast path
        let blockhash = match self.blockhash_cache.get_latest_blockhash().await {
            Ok(hash) => Some(hash),
            Err(e) => {
                println!("Failed to get blockhash: {:?}", e);
                None
            }
        };

        // Buy the token, using the cached blockhash
        match pump_buy(
            &self.rpc_url,
            &self.private_key,
            mint_pubkey,
            token_amount,
            buy_sol,
            slot,
            blockhash
        ).await {
            Ok(signature) => {
                let elapsed = start_time.elapsed();
                println!("Snipe successful! Transaction signature: {}", signature);
                println!("Total snipe time: {:.3}ms", elapsed.as_millis());

                // After successful buy, store token address and purchased amount in Redis, set for auto sell after delay
                self.redis_client.store_mint_with_amount(token_mint, token_amount, self.sell_delay_ms).await?;

                Ok(())
            },
            Err(e) => {
                let elapsed = start_time.elapsed();
                println!("Snipe failed: {:?}", e);
                println!("Failed time: {:.3}ms", elapsed.as_millis());
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Snipe failed: {:?}", e))))
            }
        }
    }

    // Determine if sniping should occur
    pub fn should_snipe(&self, sol_amount: u64) -> bool {
        sol_amount >= self.min_sol_price && sol_amount <= self.max_sol_price
    }
}
