use redis::{AsyncCommands, Client, RedisError, aio::Connection as AsyncConnection};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct RedisClient {
    client: Client,
    connection: Arc<Mutex<AsyncConnection>>,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        let connection = Arc::new(Mutex::new(client.get_async_connection().await?));

        Ok(Self {
            client,
            connection,
        })
    }

    // Store Mint address in Redis as an automatic trading queue, with a specified delay time
    pub async fn store_mint_data(&self, mint: &str, delay_ms: u64) -> Result<(), RedisError> {
        let mut conn = self.connection.lock().await;

        // Get the current timestamp as the score and add the specified delay time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let sell_time = now + delay_ms; // Sell after the specified time

        // Add the mint address to the sorted set, with the score being the sell time
        conn.zadd("mints_to_sell", mint, sell_time).await?;

        println!("Token {} added to the sell queue, will be sold after {}ms", mint, delay_ms);

        Ok(())
    }

    // Store Mint address and corresponding token amount, and set the automatic sell time
    pub async fn store_mint_with_amount(&self, mint: &str, amount: u64, delay_ms: u64) -> Result<(), RedisError> {
        let mut conn = self.connection.lock().await;

        // Get the current timestamp as the score and add the specified delay time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let sell_time = now + delay_ms; // Sell after the specified time

        // Add the mint address to the sorted set, with the score being the sell time
        conn.zadd("mints_to_sell", mint, sell_time).await?;

        // Also save the token amount to another hash table
        conn.hset("mint_amounts", mint, amount.to_string()).await?;

        println!("Token {} (amount: {}) added to the sell queue, will be sold after {}ms", mint, amount, delay_ms);

        Ok(())
    }

    // Get the amount of a specified token
    pub async fn get_mint_amount(&self, mint: &str) -> Result<Option<u64>, RedisError> {
        let mut conn = self.connection.lock().await;

        // Get the token amount from the hash table
        let amount: Option<String> = conn.hget("mint_amounts", mint).await?;

        // Convert the string to u64
        match amount {
            Some(amount_str) => {
                match amount_str.parse::<u64>() {
                    Ok(amount) => Ok(Some(amount)),
                    Err(_) => Ok(None)
                }
            },
            None => Ok(None)
        }
    }

    // Get the list of tokens that need to be sold upon expiration
    pub async fn get_mints_to_sell(&self) -> Result<Vec<String>, RedisError> {
        let mut conn = self.connection.lock().await;

        // Get the current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Query all mint addresses with a score less than or equal to the current time
        let mints_to_sell: Vec<String> = conn.zrangebyscore("mints_to_sell", 0, now).await?;

        Ok(mints_to_sell)
    }

    // Remove sold tokens from Redis
    pub async fn remove_sold_mint(&self, mint: &str) -> Result<(), RedisError> {
        let mut conn = self.connection.lock().await;

        // Remove the specified mint address from the sorted set
        conn.zrem("mints_to_sell", mint).await?;

        // Also delete the token amount record
        conn.hdel("mint_amounts", mint).await?;

        println!("Removed token from sell queue: {}", mint);

        Ok(())
    }

    // Get and remove all tokens that need to be sold
    pub async fn get_and_remove_mints_to_sell(&self) -> Result<Vec<String>, RedisError> {
        // First get the tokens to be sold
        let mints_to_sell = self.get_mints_to_sell().await?;

        if mints_to_sell.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = self.connection.lock().await;

        // Get the current timestamp
        let _now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Remove all obtained tokens using the ZREM command
        // Note: The redis-rs library might not have a direct zremrangebyscore method, use zrem instead
        for mint in &mints_to_sell {
            conn.zrem("mints_to_sell", mint).await?;
        }

        Ok(mints_to_sell)
    }
}
