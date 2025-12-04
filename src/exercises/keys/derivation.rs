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
    let secp_ctx = Secp256k1::new();
    let master_key = Xpriv::new_master(network, &seed).unwrap();

    KeysManager {
        secp_ctx,
        master_key,
        network,
    }
}

/// Exercise 2: Derive a key from a specific key family and channel_id
impl KeysManager {
    pub fn derive_key(&self, key_family: KeyFamily, channel_id_index: u32) -> SecretKey {
        // Path: m/1017'/0'/<key_family>'/0/<channel_id_index>
        let path_str = format!("m/1017'/0'/{}'/0/{}", key_family as u32, channel_id_index);
        let path = DerivationPath::from_str(&path_str).expect("Valid derivation path");

        let derived = self
            .master_key
            .derive_priv(&self.secp_ctx, &path)
            .expect("Valid derivation");

        derived.private_key
    }
}

/// Exercise 3: Derive all base keys needed for a channel
impl KeysManager {
    pub fn derive_channel_keys(&self, channel_id_index: u32) -> ChannelKeyManager {
        // Use derive_key for each key family
        let funding_key = self.derive_key(KeyFamily::MultiSig, channel_id_index);
        let revocation_basepoint_secret = self.derive_key(KeyFamily::RevocationBase, channel_id_index);
        let payment_basepoint_secret = self.derive_key(KeyFamily::PaymentBase, channel_id_index);
        let delayed_payment_basepoint_secret = self.derive_key(KeyFamily::DelayBase, channel_id_index);
        let htlc_basepoint_secret = self.derive_key(KeyFamily::HtlcBase, channel_id_index);
        let commitment_seed = self
            .derive_key(KeyFamily::CommitmentSeed, channel_id_index)
            .secret_bytes();

        ChannelKeyManager {
            funding_key,
            revocation_basepoint_secret,
            payment_basepoint_secret,
            delayed_payment_basepoint_secret,
            htlc_basepoint_secret,
            commitment_seed,
            secp_ctx: self.secp_ctx.clone(),
        }
    }
}
