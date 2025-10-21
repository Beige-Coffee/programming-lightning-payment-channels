use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::hashes::sha256::Hash as Sha256;
use bitcoin::hashes::HashEngine;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{All, PublicKey, Secp256k1, SecretKey};
use bitcoin::Network;
use std::str::FromStr;

use crate::types::{ChannelKeys, KeyFamily, KeysManager, ChannelPublicKeys};

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
    pub fn derive_public_key(&self, key_family: KeyFamily, channel_id_index: u32) -> PublicKey {
        // Path: m/1017'/0'/<key_family>'/0/<channel_id_index>
        let path_str = format!("m/1017'/0'/{}'/0/{}", key_family as u32, channel_id_index);
        let path = DerivationPath::from_str(&path_str).expect("Valid derivation path");

        let derived = self
            .master_key
            .derive_priv(&self.secp_ctx, &path)
            .expect("Valid derivation");

        derived.private_key.public_key(&self.secp_ctx)
    }
}

/// Exercise 3: Get the node's identity secret key
impl KeysManager {
    pub fn get_node_secret(&self) -> SecretKey {
        self.derive_key(KeyFamily::NodeKey, 0)
    }
}

/// Exercise 5: Derive all base keys needed for a channel
/// These base keys will be used with per-commitment points to create
/// commitment-specific keys for each channel state
impl KeysManager {
    pub fn derive_channel_keys(&self, channel_id_index: u32) -> ChannelKeys {
        // Use derive_key for each key family
        let funding_key = self.derive_key(KeyFamily::MultiSig, channel_id_index);
        let revocation_base_key = self.derive_key(KeyFamily::RevocationBase, channel_id_index);
        let payment_base_key = self.derive_key(KeyFamily::PaymentBase, channel_id_index);
        let delayed_payment_base_key = self.derive_key(KeyFamily::DelayBase, channel_id_index);
        let htlc_base_key = self.derive_key(KeyFamily::HtlcBase, channel_id_index);
        let commitment_seed = self
            .derive_key(KeyFamily::CommitmentSeed, channel_id_index)
            .secret_bytes();

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
    pub fn derive_channel_public_keys(&self, channel_id_index: u32) -> ChannelPublicKeys {
        // Use derive_key for each key family
        let funding_pubkey = self.derive_public_key(KeyFamily::MultiSig, channel_id_index);
        let revocation_basepoint = self.derive_public_key(KeyFamily::RevocationBase, channel_id_index);
        let payment_point = self.derive_public_key(KeyFamily::PaymentBase, channel_id_index);
        let delayed_payment_basepoint = self.derive_public_key(KeyFamily::DelayBase, channel_id_index);
        let htlc_basepoint = self.derive_public_key(KeyFamily::HtlcBase, channel_id_index);

        ChannelPublicKeys {
            funding_pubkey,
            revocation_basepoint,
            payment_point,
            delayed_payment_basepoint,
            htlc_basepoint,
        }
    }
}