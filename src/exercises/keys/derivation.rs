use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::HashEngine;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{All, PublicKey, Secp256k1, SecretKey};
use bitcoin::Network;
use std::str::FromStr;

use crate::types::{ChannelKeyManager, KeyFamily, KeysManager};

/// Exercise 1: Create a new KeysManager from a seed
pub fn new_keys_manager(seed: [u8; 32], network: Network) -> KeysManager {
    
    unimplemented!();

    // Initialize secp256k1 context

    // Create extended master key from seed

    // Return KeysManager
    
}

/// Exercise 2: Derive a key from a specific key family and channel_id
impl KeysManager {
    pub fn derive_key(&self, key_family: KeyFamily, channel_id_index: u32) -> SecretKey {
        
        unimplemented!();

        // Build derivation path: m/1017'/0'/<key_family>'/0/<channel_id_index>

        // Parse string into DerivationPath struct

        // Derive child private key at the specified path

        // Extract and return the secret key

    }
}

/// Exercise 3: Derive all base keys needed for a channel
impl KeysManager {
    pub fn derive_channel_keys(&self, channel_id_index: u32) -> ChannelKeyManager {
        
        unimplemented!();

        // Derive each key using the appropriate KeyFamily

        // Commitment seed is stored as raw bytes (not as SecretKey type)

        // Return ChannelKeyManager

    }
}
