use bitcoin::bip32::Xpriv;
use bitcoin::script::ScriptBuf;
use bitcoin::secp256k1::{All, PublicKey, Secp256k1, SecretKey};
use bitcoin::Network;

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
}

// KEY MANAGEMENT STRUCTURES
// ============================================================================

pub struct KeysManager {
    pub secp_ctx: Secp256k1<All>,
    pub master_key: Xpriv,
    pub network: Network,
}

/// Manages cryptographic operations for Lightning channel.
pub struct ChannelKeyManager {
    /// Secret key used to sign commitment transactions
    pub funding_key: SecretKey,
    /// Base secret used to derive per-commitment revocation keys
    pub revocation_basepoint_secret: SecretKey,
    /// Secret key for immediately spendable balance
    pub payment_basepoint_secret: SecretKey,
    /// Base secret used to derive per-commitment delayed payment key
    pub delayed_payment_basepoint_secret: SecretKey,
    /// Base secret used to derive per-commitment HTLC key
    pub htlc_basepoint_secret: SecretKey,
    /// Seed used to generate per-commitment points
    pub commitment_seed: [u8; 32],
    /// Secp256k1 context for cryptographic operations
    pub secp_ctx: Secp256k1<All>,
}

/// Channel public keys which do not change over the life of a channel.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ChannelPublicKeys {
    /// Public key used to sign commitment transactions
    pub funding_pubkey: PublicKey,
    /// Base point used to derive per-commitment revocation keys
    pub revocation_basepoint: PublicKey,
    /// Public key for immediately spendable balance
    pub payment_basepoint: PublicKey,
    /// Base point used to derive per-commitment delayed payment key
    pub delayed_payment_basepoint: PublicKey,
    /// Base point used to derive per-commitment HTLC key
    pub htlc_basepoint: PublicKey,
}

// COMMITMENT KEYS STRUCTURE)
// ============================================================================

/// The set of public keys which are used in the creation of one commitment transaction.
/// These are derived from the channel base keys and per-commitment point.
#[derive(Clone, Debug)]
pub struct CommitmentKeys {
    /// The per-commitment point used to derive the other keys
    pub per_commitment_point: PublicKey,

    /// The revocation key which allows the broadcaster's counterparty to punish
    /// them if they broadcast an old state
    pub revocation_key: PublicKey,

    /// Local party's HTLC key (derived from local_htlc_basepoint)
    pub local_htlc_key: PublicKey,

    /// Remote party's HTLC key (derived from remote_htlc_basepoint)
    pub remote_htlc_key: PublicKey,

    /// Local party's delayed payment key (for to_local output)
    pub local_delayed_payment_key: PublicKey,
}


// OUTPUT SORTING STRUCTURES
// ============================================================================

#[derive(Debug, Clone)]
pub struct OutputWithMetadata {
    pub value: u64,
    pub script: ScriptBuf,
    pub cltv_expiry: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct HTLCOutput {
    /// Amount in satoshis
    pub amount_sat: u64,
    /// Payment hash for this HTLC
    pub payment_hash: [u8; 32],
    /// CLTV expiry height
    pub cltv_expiry: u32,
}

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
