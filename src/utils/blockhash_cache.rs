use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::{CommitmentConfig, CommitmentLevel}, hash::Hash};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Blockhash cache to reduce RPC calls
pub struct BlockhashCache {
    rpc_client: RpcClient,
    cached_blockhash: Arc<Mutex<Option<(Hash, Instant)>>>,
    max_age: Duration,
}

impl BlockhashCache {
    /// Creates a new blockhash cache
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - RPC node URL
    /// * `max_age_ms` - Maximum cache validity period (milliseconds)
    pub fn new(rpc_url: &str, max_age_ms: u64) -> Self {
        Self {
            rpc_client: RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed()),
            cached_blockhash: Arc::new(Mutex::new(None)),
            max_age: Duration::from_millis(max_age_ms),
        }
    }

    /// Gets the latest blockhash, fetching from cache if valid
    pub async fn get_latest_blockhash(&self) -> Result<Hash, Box<dyn std::error::Error + Send + Sync>> {
        let mut cache = self.cached_blockhash.lock().await;

        // Check if cache is valid
        if let Some((hash, timestamp)) = &*cache {
            if timestamp.elapsed() < self.max_age {
                println!("Using cached blockhash");
                return Ok(*hash);
            }
        }

        // Cache is missing or expired, fetch from RPC
        println!("Fetching new blockhash");
        let blockhash = self.rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            })
            .await?
            .0;

        // Update cache
        *cache = Some((blockhash, Instant::now()));

        Ok(blockhash)
    }
}
