use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey, All};
use bitcoin::bip32::{Xpriv, DerivationPath};
use bitcoin::hashes::{Hash, sha256};
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::HashEngine;
use bitcoin::Network;
use std::str::FromStr;

use crate::types::{KeyFamily, KeysManager, ChannelKeys};

// ============================================================================
// SECTION 1: BIP32 KEY DERIVATION & KEYS MANAGER
// ============================================================================
// These exercises teach how to derive keys using BIP32 hierarchical
// deterministic key derivation for Lightning channels.

/// Exercise 1: Create a new KeysManager from a seed
pub fn new_keys_manager(seed: [u8; 32], network: Network) -> KeysManager {
    let secp_ctx = Secp256k1::new();
    let master_key = Xpriv::new_master(network, &seed).expect("Valid seed");
    
    KeysManager {
        secp_ctx,
        master_key,
        network,
    }
}

/// Exercise 2: Derive a key from a specific key family and index
impl KeysManager {
    pub fn derive_key(&self, key_family: KeyFamily, index: u32) -> SecretKey {
        // Path: m/1017'/0'/<key_family>'/0/<index>
        let path_str = format!("m/1017'/0'/{}'/0/{}", key_family as u32, index);
        let path = DerivationPath::from_str(&path_str).expect("Valid derivation path");
        
        let derived = self.master_key
            .derive_priv(&self.secp_ctx, &path)
            .expect("Valid derivation");
        
        derived.private_key
    }
}

/// Exercise 3: Get the node's identity secret key
impl KeysManager {
    pub fn get_node_secret(&self) -> SecretKey {
        self.derive_key(KeyFamily::NodeKey, 0)
    }
}

/// Exercise 4: Generate a deterministic channel seed
/// This seed is used to derive per-commitment secrets for the channel
pub fn generate_channel_seed(
    channel_value_satoshis: u64,
    local_pubkey: &PublicKey,
    remote_pubkey: &PublicKey,
    nonce: [u8; 32],
) -> [u8; 32] {
    let mut engine = Sha256::engine();
    
    // Add channel value
    engine.input(&channel_value_satoshis.to_be_bytes());
    
    // Sort pubkeys for determinism
    let (first, second) = if local_pubkey.serialize() < remote_pubkey.serialize() {
        (local_pubkey, remote_pubkey)
    } else {
        (remote_pubkey, local_pubkey)
    };
    
    engine.input(&first.serialize());
    engine.input(&second.serialize());
    engine.input(&nonce);
    
    Sha256::from_engine(engine).to_byte_array()
}

/// Exercise 5: Derive all base keys needed for a channel
/// These base keys will be used with per-commitment points to create
/// commitment-specific keys for each channel state
impl KeysManager {
    pub fn derive_channel_keys(&self, channel_index: u32, channel_seed: [u8; 32]) -> ChannelKeys {
        
        // Use derive_key for each key family
        let funding_key = self.derive_key(KeyFamily::MultiSig, channel_index);
        let revocation_base_key = self.derive_key(KeyFamily::RevocationBase, channel_index);
        let payment_base_key = self.derive_key(KeyFamily::PaymentBase, channel_index);
        let delayed_payment_base_key = self.derive_key(KeyFamily::DelayBase, channel_index);
        let htlc_base_key = self.derive_key(KeyFamily::HtlcBase, channel_index);
        
        // Use channel_seed as commitment_seed
        let mut commitment_seed = [0u8; 32];
        commitment_seed.copy_from_slice(&channel_seed);
        
        ChannelKeys {
            funding_key,
            revocation_base_key,
            payment_base_key,
            delayed_payment_base_key,
            htlc_base_key,
            commitment_seed,
            secp_ctx: self.secp_ctx.clone(),
        }
    }
}
