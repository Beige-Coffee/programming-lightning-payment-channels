// DEPRECATED: This file is deprecated and should be deleted.
// All signing functionality has been moved to src/exercises/keys/sign.rs
// and is now implemented as methods on the InMemorySigner struct.
//
// This file is kept temporarily to prevent compilation errors during migration.
// It should be removed along with any imports of it.
//
// Migration instructions:
// 1. Replace `use crate::signing::*` with `use crate::types::InMemorySigner`
// 2. Create an InMemorySigner instance
// 3. Call methods on the instance instead of standalone functions
//
// Example:
// OLD: sign_transaction_input(&tx, 0, &script, amount, &key, &secp)
// NEW: signer.sign_transaction_input(&tx, 0, &script, amount, &key)
