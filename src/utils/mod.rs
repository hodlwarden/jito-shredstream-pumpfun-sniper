use solana_entry::entry::Entry;
use bincode::Error as BincodeError;

pub mod redis;
pub mod auto_trader;
pub mod blockhash_cache;

pub fn deserialize_entries(data: &[u8]) -> Result<Vec<Entry>, BincodeError> {
    bincode::deserialize::<Vec<Entry>>(data)
} 