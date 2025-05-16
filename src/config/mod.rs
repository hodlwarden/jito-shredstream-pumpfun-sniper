use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::env;
use dotenvy::dotenv;

#[derive(Clone)]
pub struct Config {
    pub server_url: String,
    pub token_creator_pubkey: Pubkey,
}

impl Config {
    pub fn new() -> Self {
        // Load environment variables
        dotenv().ok();
        
        // Get server URL from environment variables, panic if not set
        let server_url = env::var("SERVER_URL").expect("Environment variable SERVER_URL not set");
        
        Self {
            server_url,
            token_creator_pubkey: Pubkey::from_str("TSLvdd1pWpHVjahSpsvCXUbgwsL3JAcvokwaKt1eokM").unwrap(),
        }
    }
}
