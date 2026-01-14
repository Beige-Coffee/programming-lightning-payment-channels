pub mod derivation;
pub mod commitment;
pub mod channel_key_manager;

// Re-export commonly used items
pub use derivation::{new_keys_manager};
pub use commitment::{
    derive_public_key, 
    derive_private_key,
    derive_revocation_public_key,
    derive_revocation_private_key,
};

// Re-export channel_key_manager items
// Note: The ChannelKeyManager struct itself is in types.rs,
// but all its methods are implemented in channel_key_manager.rs
