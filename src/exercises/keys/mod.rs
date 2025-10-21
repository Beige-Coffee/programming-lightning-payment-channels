pub mod derivation;
pub mod commitment;
pub mod sign;

// Re-export commonly used items
pub use derivation::{new_keys_manager};
pub use commitment::{
    derive_public_key, 
    derive_private_key,
    derive_revocation_public_key,
    derive_revocation_private_key,
};

// Re-export all signing functions for backwards compatibility
pub use sign::*;
