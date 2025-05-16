use jito_protos::shredstream::{
    shredstream_proxy_client::ShredstreamProxyClient, SubscribeEntriesRequest, Entry,
};
use tonic::Streaming;
use crate::config::Config;
use std::time::Duration;
use tokio::time::sleep;

pub struct ShredstreamClient {
    client: ShredstreamProxyClient<tonic::transport::Channel>,
    config: Config,
}

impl ShredstreamClient {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Self::connect(&config).await?;
        Ok(Self { client, config })
    }

    async fn connect(config: &Config) -> Result<ShredstreamProxyClient<tonic::transport::Channel>, Box<dyn std::error::Error>> {
        let mut retries = 0;
        let max_retries = 5;
        let base_delay = Duration::from_secs(1);

        loop {
            match ShredstreamProxyClient::connect(config.server_url.clone()).await {
                Ok(client) => return Ok(client),
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        return Err(Box::new(e));
                    }
                    let delay = base_delay * retries;
                    println!("Connection failed, retrying in {} seconds (attempt {})...", delay.as_secs(), retries);
                    sleep(delay).await;
                }
            }
        }
    }

    pub async fn subscribe_entries(&mut self) -> Result<Streaming<Entry>, Box<dyn std::error::Error>> {
        let mut retries = 0;
        let max_retries = 5;
        let base_delay = Duration::from_secs(1);

        loop {
            match self.client
                .subscribe_entries(SubscribeEntriesRequest {})
                .await
            {
                Ok(response) => return Ok(response.into_inner()),
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        return Err(Box::new(e));
                    }
                    let delay = base_delay * retries;
                    println!("Subscription failed, retrying in {} seconds (attempt {})...", delay.as_secs(), retries);
                    sleep(delay).await;
                    
                    // Attempt to reconnect
                    self.client = Self::connect(&self.config).await?;
                }
            }
        }
    }
}
