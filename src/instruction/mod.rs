use solana_sdk::pubkey::Pubkey;
use std::error::Error;
use borsh::BorshDeserialize;

// Define CreateEvent arguments structure for Borsh deserialization
#[derive(BorshDeserialize, Debug)]
struct CreateArgs {
    name: String,
    symbol: String,
    uri: String,
}

// Define Buy arguments structure for Borsh deserialization
#[derive(BorshDeserialize, Debug)]
struct BuyArgs {
    amount: u64,
    max_sol_cost: u64,
}

#[derive(Debug)]
pub struct CreateEventInstruction {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub user: Pubkey,
}

#[derive(Debug)]
pub struct BuyInstruction {
    pub amount: u64,
    pub max_sol_cost: u64,
}

// Instruction discriminator bytes for CreateEvent
const CREATE_EVENT_DISCRIMINATOR: [u8; 8] = [0x18, 0x1e, 0xc8, 0x28, 0x05, 0x1c, 0x07, 0x77];
// Instruction discriminator bytes for BuyEvent
const BUY_EVENT_DISCRIMINATOR: [u8; 8] = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];

pub fn parse_instruction_data(data: &[u8]) -> Result<(String, Option<CreateEventInstruction>, Option<BuyInstruction>), Box<dyn Error>> {
    if data.len() < 8 {
        return Err("Instruction data too short".into());
    }

    let discriminator = &data[0..8];
    
    match discriminator {
        // CreateEvent instruction [0x18, 0x1e, 0xc8, 0x28, 0x05, 0x1c, 0x07, 0x77]
        discriminator if discriminator == CREATE_EVENT_DISCRIMINATOR => {
            // Try parsing with Borsh structure
            if let Ok(args) = CreateArgs::try_from_slice(&data[8..]) {
                // Extract user pubkey from the end of instruction data
                let user_offset = data.len() - 32;
                let user = if user_offset >= 8 && user_offset + 32 <= data.len() {
                    Pubkey::new_from_array(data[user_offset..user_offset + 32].try_into().unwrap())
                } else {
                    Pubkey::default()
                };

                let instruction = CreateEventInstruction {
                    name: args.name,
                    symbol: args.symbol,
                    uri: args.uri,
                    user,
                };
                return Ok(("CreateEvent".to_string(), Some(instruction), None));
            }

            // Fallback to manual parsing if Borsh deserialization fails
            let mut offset = 8;
            
            // Parse name
            if offset + 4 > data.len() {
                return Err("Insufficient data for name length".into());
            }
            let name_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            
            if offset + name_len > data.len() {
                return Err("Insufficient data for name".into());
            }
            let name = String::from_utf8(data[offset..offset + name_len].to_vec())?;
            offset += name_len;

            // Parse symbol
            if offset + 4 > data.len() {
                return Err("Insufficient data for symbol length".into());
            }
            let symbol_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            
            if offset + symbol_len > data.len() {
                return Err("Insufficient data for symbol".into());
            }
            let symbol = String::from_utf8(data[offset..offset + symbol_len].to_vec())?;
            offset += symbol_len;

            // Parse URI
            if offset + 4 > data.len() {
                return Err("Insufficient data for URI length".into());
            }
            let uri_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            
            if offset + uri_len > data.len() {
                return Err("Insufficient data for URI".into());
            }
            let uri = String::from_utf8(data[offset..offset + uri_len].to_vec())?;
            offset += uri_len;

            // Parse user pubkey
            if offset + 32 > data.len() {
                return Err("Insufficient data for user pubkey".into());
            }
            let user = Pubkey::new_from_array(data[offset..offset + 32].try_into().unwrap());

            let instruction = CreateEventInstruction { name, symbol, uri, user };
            Ok(("CreateEvent".to_string(), Some(instruction), None))
        }

        // Buy instruction [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]
        discriminator if discriminator == BUY_EVENT_DISCRIMINATOR => {
            // Try parsing with Borsh structure
            if let Ok(args) = BuyArgs::try_from_slice(&data[8..]) {
                let instruction = BuyInstruction {
                    amount: args.amount,
                    max_sol_cost: args.max_sol_cost,
                };
                return Ok(("Buy".to_string(), None, Some(instruction)));
            }

            // Fallback to manual parsing if Borsh deserialization fails
            if data.len() < 24 {
                return Err("Insufficient data for Buy instruction".into());
            }
            
            let amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let max_sol_cost = u64::from_le_bytes(data[16..24].try_into().unwrap());

            let instruction = BuyInstruction { amount, max_sol_cost };
            Ok(("Buy".to_string(), None, Some(instruction)))
        }

        // Alternative possible instruction format for token creation
        // For CreateCoin instruction (24)
        [24, _, _, _, _, _, _, _] => {
            // Try parsing CreateCoin instruction
            let mut offset = 8;
            
            // Parse name
            if offset + 4 > data.len() {
                return Err("Insufficient data for name length".into());
            }
            let name_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            
            if offset + name_len > data.len() {
                return Err("Insufficient data for name".into());
            }
            let name = String::from_utf8(data[offset..offset + name_len].to_vec())?;
            offset += name_len;

            // Parse symbol
            if offset + 4 > data.len() {
                return Err("Insufficient data for symbol length".into());
            }
            let symbol_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            
            if offset + symbol_len > data.len() {
                return Err("Insufficient data for symbol".into());
            }
            let symbol = String::from_utf8(data[offset..offset + symbol_len].to_vec())?;
            offset += symbol_len;

            // Parse URI
            if offset + 4 > data.len() {
                return Err("Insufficient data for URI length".into());
            }
            let uri_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            
            if offset + uri_len > data.len() {
                return Err("Insufficient data for URI".into());
            }
            let uri = String::from_utf8(data[offset..offset + uri_len].to_vec())?;

            // Use default user pubkey, will be replaced with actual value from account list in processor
            let user = Pubkey::default();

            let instruction = CreateEventInstruction { name, symbol, uri, user };
            Ok(("CreateEvent".to_string(), Some(instruction), None))
        }

        // BuyTokens instruction (102)
        [102, _, _, _, _, _, _, _] => {
            if data.len() < 24 {
                return Err("Insufficient data for BuyTokens instruction".into());
            }
            
            let amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let max_sol_cost = u64::from_le_bytes(data[16..24].try_into().unwrap());

            let instruction = BuyInstruction { amount, max_sol_cost };
            Ok(("Buy".to_string(), None, Some(instruction)))
        }

        _ => Err("Unknown instruction data".into()),
    }
}
