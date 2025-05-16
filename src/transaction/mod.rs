use std::fmt::Error;

use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;

// Pump protocol related constants
pub const GLOBAL_ACCOUNT: Pubkey =
    solana_sdk::pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const FEE_RECIPIENT: Pubkey =
    solana_sdk::pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");
pub const EVENT_AUTHORITY: Pubkey = solana_sdk::pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const PUMP_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PROXY_PROGRAM: Pubkey = solana_sdk::pubkey!("AmXoSVCLjsfKrwCUqvkMFXYcDzZ4FeoMYs7SAhGyfMGy");

// System accounts
pub const SYSVAR_RENT_PUBKEY: Pubkey = solana_sdk::pubkey!("SysvarRent111111111111111111111111111111111");

// Instruction discriminators
pub const PUMP_BUY_SELECTOR: &[u8; 8] = &[82, 225, 119, 231, 78, 29, 45, 70]; // Internal buy discriminator
pub const PUMP_SELL_SELECTOR: &[u8; 8] = &[83, 225, 119, 231, 78, 29, 45, 70]; // Internal sell discriminator
pub const ATA_SELECTOR: &[u8; 8] = &[22, 51, 53, 97, 247, 184, 54, 78]; // Create ATA discriminator

const BONDING_CURVE_SEED: &[u8] = b"bonding-curve";

/// Pump protocol token buy transaction
///
/// # Arguments
///
/// * `rpc_url` - RPC node URL
/// * `private_key` - User's private key
/// * `token_mint` - Token Mint address
/// * `token_amount` - Amount of tokens to buy
/// * `max_sol_cost` - Maximum SOL cost (in lamports)
/// * `slot` - Optional slot number for logging
/// * `cached_blockhash` - Optional cached blockhash, if provided, RPC will not be queried
pub async fn pump_buy(
    rpc_url: &str,
    private_key: &str,
    token_mint: Pubkey,
    token_amount: u64,
    max_sol_cost: u64,
    slot: Option<u64>,
    cached_blockhash: Option<Hash>,
) -> Result<String, Error> {
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Construct buy instruction data
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(PUMP_BUY_SELECTOR);
    data.extend_from_slice(&token_amount.to_le_bytes());
    data.extend_from_slice(&max_sol_cost.to_le_bytes());

    let signer = solana_sdk::signature::Keypair::from_base58_string(private_key);

    // Calculate Bonding Curve address
    let bonding_curve_address =
        Pubkey::find_program_address(&[BONDING_CURVE_SEED, token_mint.as_ref()], &PUMP_PROGRAM_ID);

    // User's associated token account
    let associated_user = get_associated_token_address(&signer.pubkey(), &token_mint);

    // Bonding Curve's associated token account
    let associated_bonding_curve =
        get_associated_token_address(&bonding_curve_address.0, &token_mint);

    // Construct buy instruction
    let buy_instruction = Instruction::new_with_bytes(
        PROXY_PROGRAM,
        &data,
        vec![
            AccountMeta::new_readonly(GLOBAL_ACCOUNT, false),
            AccountMeta::new(FEE_RECIPIENT, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(bonding_curve_address.0, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new(associated_user, false),
            AccountMeta::new(signer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(SYSVAR_RENT_PUBKEY, false), // Correct Rent Sysvar address
            AccountMeta::new_readonly(EVENT_AUTHORITY, false),
            AccountMeta::new_readonly(PUMP_PROGRAM_ID, false),
        ],
    );

    // Create ATA instruction data
    let mut ata_data = Vec::with_capacity(9);
    ata_data.extend_from_slice(ATA_SELECTOR);
    ata_data.extend_from_slice(&[0]);

    // Construct create ATA instruction
    let ata_instruction = Instruction::new_with_bytes(
        PROXY_PROGRAM,
        &ata_data,
        vec![
            AccountMeta::new(signer.pubkey(), true),
            AccountMeta::new(associated_user, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        ],
    );

    // Add priority fee instructions - Increase priority fee to 200000 for faster processing
    let compute_unit_price_ix = solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(200000);

    // Increase maximum compute units to ensure the transaction doesn't fail due to insufficient compute resources
    let compute_unit_limit_ix = solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(200000);

    // Get blockhash
    let blockhash = if let Some(hash) = cached_blockhash {
        // Use the provided cached blockhash
        if let Some(slot_num) = slot {
            println!("Buy using related slot: {} and cached blockhash", slot_num);
        } else {
            println!("Buy using cached blockhash");
        }
        hash
    } else {
        // Get the latest blockhash directly
        if let Some(slot_num) = slot {
            println!("Buy using related slot: {} and newly fetched blockhash", slot_num);
        }

        rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            })
            .await
            .unwrap()
            .0
    };

    // Create transaction
    let transaction = Transaction::new_signed_with_payer(
        &[compute_unit_price_ix, compute_unit_limit_ix, ata_instruction, buy_instruction], // Add two priority instructions
        Some(&signer.pubkey()),
        &[&signer],
        blockhash,
    );

    // Send transaction - Use optimal transaction settings
    match rpc_client
        .send_transaction_with_config(
            &transaction,
            RpcSendTransactionConfig {
                skip_preflight: true,
                preflight_commitment: Some(CommitmentLevel::Processed), // Use Processed level for fastest return
                max_retries: Some(0), // No retries, as we need to know the result immediately
                ..Default::default()
            },
        )
        .await
    {
        Ok(signature) => {
            println!("Buy transaction submitted: {}", signature);
            Ok(signature.to_string())
        }
        Err(e) => {
            println!("Buy transaction failed: {:?}", e);
            Err(Error)
        }
    }
}

/// Pump protocol token sell transaction
///
/// # Arguments
///
/// * `rpc_url` - RPC node URL
/// * `private_key` - User's private key
/// * `token_mint` - Token Mint address
/// * `token_amount` - Amount of tokens to sell
/// * `min_sol_receive` - Minimum SOL to receive (in lamports)
/// * `slot` - Optional slot number for logging
/// * `cached_blockhash` - Optional cached blockhash, if provided, RPC will not be queried
pub async fn pump_sell(
    rpc_url: &str,
    private_key: &str,
    token_mint: Pubkey,
    token_amount: u64,
    min_sol_receive: u64,
    slot: Option<u64>,
    cached_blockhash: Option<Hash>,
) -> Result<String, Error> {
    let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Construct sell instruction data
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(PUMP_SELL_SELECTOR); // Use internal selector PUMPFUN_SELL_SELECTOR
    data.extend_from_slice(&token_amount.to_le_bytes());
    data.extend_from_slice(&min_sol_receive.to_le_bytes());

    let signer = solana_sdk::signature::Keypair::from_base58_string(private_key);

    // Calculate Bonding Curve address
    let bonding_curve_address =
        Pubkey::find_program_address(&[BONDING_CURVE_SEED, token_mint.as_ref()], &PUMP_PROGRAM_ID);

    // User's associated token account
    let associated_user = get_associated_token_address(&signer.pubkey(), &token_mint);

    // Bonding Curve's associated token account
    let associated_bonding_curve =
        get_associated_token_address(&bonding_curve_address.0, &token_mint);

    // Add priority fee instructions - Increase priority fee to 200000 for faster processing
    let compute_unit_price_ix = solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(200000);

    // Increase maximum compute units to ensure the transaction doesn't fail due to insufficient compute resources
    let compute_unit_limit_ix = solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(200000);

    // Construct sell instruction
    let sell_instruction = Instruction::new_with_bytes(
        PROXY_PROGRAM,
        &data,
        vec![
            AccountMeta::new_readonly(GLOBAL_ACCOUNT, false),
            AccountMeta::new(FEE_RECIPIENT, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(bonding_curve_address.0, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new(associated_user, false),
            AccountMeta::new(signer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(EVENT_AUTHORITY, false),
            AccountMeta::new_readonly(PUMP_PROGRAM_ID, false),
        ],
    );

    // Get blockhash
    let blockhash = if let Some(hash) = cached_blockhash {
        // Use the provided cached blockhash
        if let Some(slot_num) = slot {
            println!("Sell using related slot: {} and cached blockhash", slot_num);
        } else {
            println!("Sell using cached blockhash");
        }
        hash
    } else {
        // Get the latest blockhash directly
        if let Some(slot_num) = slot {
            println!("Sell using related slot: {} and newly fetched blockhash", slot_num);
        }

        rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            })
            .await
            .unwrap()
            .0
    };

    // Create transaction
    let transaction = Transaction::new_signed_with_payer(
        &[compute_unit_price_ix, compute_unit_limit_ix, sell_instruction], // Add two priority instructions
        Some(&signer.pubkey()),
        &[&signer],
        blockhash,
    );

    // Send transaction - Use optimal transaction settings
    match rpc_client
        .send_transaction_with_config(
            &transaction,
            RpcSendTransactionConfig {
                skip_preflight: true,
                preflight_commitment: Some(CommitmentLevel::Processed), // Use Processed level for fastest return
                max_retries: Some(0), // No retries, as we need to know the result immediately
                ..Default::default()
            },
        )
        .await
    {
        Ok(signature) => {
            println!("Sell transaction submitted: {}", signature);
            Ok(signature.to_string())
        }
        Err(e) => {
            println!("Sell transaction failed: {:?}", e);
            Err(Error)
        }
    }
}
