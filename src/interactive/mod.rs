// Interactive CLI modules for Programming Lightning workshop
// These modules provide user-friendly interfaces for creating and managing
// Lightning Network transactions through the command line.

pub mod commitment;
pub mod funding;
pub mod htlc;

// Re-export commonly used functions for convenience
pub use funding::run as funding_run;
