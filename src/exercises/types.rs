use bitcoin::secp256k1::{Secp256k1, SecretKey, PublicKey, All};
use bitcoin::bip32::Xpriv;
use bitcoin::script::ScriptBuf;
use bitcoin::Network;

// ============================================================================
// KEY FAMILY ENUM
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyFamily {
    MultiSig = 0,
    RevocationBase = 1,
    HtlcBase = 2,
    PaymentBase = 3,
    DelayBase = 4,
    CommitmentSeed = 5,
    NodeKey = 6,
}

// ============================================================================
// KEY MANAGEMENT STRUCTURES
// ============================================================================

pub struct KeysManager {
    pub secp_ctx: Secp256k1<All>,
    pub master_key: Xpriv,
    pub network: Network,
}

pub struct ChannelKeys {
    pub funding_key: SecretKey,
    pub revocation_base_key: SecretKey,
    pub payment_base_key: SecretKey,
    pub delayed_payment_base_key: SecretKey,
    pub htlc_base_key: SecretKey,
    pub commitment_seed: [u8; 32],
    pub secp_ctx: Secp256k1<All>,
}

// ============================================================================
// OUTPUT SORTING STRUCTURES
// ============================================================================

#[derive(Debug, Clone)]
pub struct OutputWithMetadata {
    pub value: u64,
    pub script: ScriptBuf,
    pub cltv_expiry: Option<u32>,
}

// ============================================================================
// TEST VECTOR STRUCTURES
// ============================================================================

pub struct Bolt3TestVector {
    pub funding_txid: [u8; 32],
    pub funding_output_index: u32,
    pub funding_amount_satoshi: u64,
    pub funding_witness_script: Vec<u8>,
    pub commitment_number: u64,
    pub local_delay: u16,
    pub local_dust_limit_satoshi: u64,
    pub feerate_per_kw: u64,
    pub to_local_msat: u64,
    pub to_remote_msat: u64,
    pub local_funding_output_signature: Vec<u8>,
    pub remote_funding_output_signature: Vec<u8>,
    pub local_funding_privkey: SecretKey,
    pub remote_funding_pubkey: PublicKey,
    pub local_revocation_basepoint_secret: SecretKey,
    pub local_payment_basepoint_secret: SecretKey,
    pub local_delayed_payment_basepoint_secret: SecretKey,
    pub local_delayedpubkey: PublicKey,
    pub local_htlcpubkey: PublicKey,
    pub remote_htlcpubkey: PublicKey,
    pub local_htlc_basepoint_secret: SecretKey,
    pub local_htlc_basepoint: PublicKey,
    pub local_revocation_pubkey: PublicKey,
    pub remote_payment_basepoint: PublicKey,
    pub local_payment_basepoint: PublicKey,
    pub remote_delayed_payment_basepoint: PublicKey,
    pub remote_htlc_basepoint: PublicKey,
    pub commitment_seed: [u8; 32],
}

pub struct Bolt3Htlc {
    pub direction: HtlcDirection,
    pub amount_msat: u64,
    pub payment_hash: [u8; 32],
    pub cltv_expiry: u32,
}

pub enum HtlcDirection {
    Offered,
    Received,
}