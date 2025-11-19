// ============================================================================
// SECTION 5: FEE CALCULATIONS
// ============================================================================
// These exercises teach how to calculate transaction fees for Lightning
// commitment and HTLC transactions.

/// Exercise 18: Calculate commitment transaction fee
/// 
/// Fee calculation: (feerate_per_kw * weight) / 1000
/// Weight = 724 + (172 * num_untrimmed_htlcs)
pub fn calculate_commitment_tx_fee(
    feerate_per_kw: u64,
    num_untrimmed_htlcs: usize,
) -> u64 {
    let weight = 724 + (172 * num_untrimmed_htlcs);
    (feerate_per_kw * weight as u64) / 1000
}

pub fn calculate_htlc_timeout_tx_fee(feerate_per_kw: u64) -> u64 {
    const HTLC_TX_WEIGHT: u64 = 663;
    (feerate_per_kw * HTLC_TX_WEIGHT) / 1000
}

pub fn calculate_htlc_success_tx_fee(feerate_per_kw: u64) -> u64 {
    const HTLC_TX_WEIGHT: u64 = 703;
    (feerate_per_kw * HTLC_TX_WEIGHT) / 1000
}

/// Exercise 20: Check if an HTLC amount is below the dust limit
/// 
/// An HTLC is considered "dust" if its amount is less than the dust limit
/// plus the fee required to claim it. Dust HTLCs are trimmed (not included)
/// in the commitment transaction.
pub fn is_htlc_dust(
    htlc_amount_sat: u64,
    dust_limit_satoshis: u64,
    feerate_per_kw: u64,
    outbound_htlc: bool,
) -> bool {
    
    
    let htlc_tx_fee = if outbound_htlc {
       calculate_htlc_timeout_tx_fee(feerate_per_kw)
    } else {
        calculate_htlc_success_tx_fee(feerate_per_kw)
    };
        
    htlc_amount_sat < dust_limit_satoshis + htlc_tx_fee
}
