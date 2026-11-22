// ---------------------------------------------------------------------------
// Oracle Sanity Engine — Math & Precision Normalization (Soroban-native)
//
// This module provides standardized precision handling for Stellar/Soroban:
// - STROOP precision: 10^7 (matches Stellar Classic native asset units)
// - Derivative precision: 10^14 (for derivative asset comparisons)
//
// All price valuations should be normalized using these constants to ensure
// consistency across different oracle feeds and prevent precision-related bugs.
// ---------------------------------------------------------------------------

use crate::error::OracleError;

/// Stellar STROOP precision (10^7).
/// This matches the native precision of Stellar Classic assets (XLM, custom assets).
/// Example: $20.00 = 200_000_000 (20 * 10^7)
pub const STROOP_PRECISION: i128 = 10_000_000;

/// Derivative asset precision (10^14).
/// Used for derivative asset comparisons where higher precision is needed.
/// Example: $20.00 = 2_000_000_000_000_000 (20 * 10^14)
pub const DERIVATIVE_PRECISION: i128 = 100_000_000_000_000;

/// Normalizes a price to STROOP precision (10^7).
///
/// # Arguments
/// - `price` - The raw price value
/// - `current_precision` - The current precision of the price
///
/// # Returns
/// The price normalized to STROOP precision
pub fn normalize_to_stroop(price: i128, current_precision: i128) -> i128 {
    if current_precision == STROOP_PRECISION {
        return price;
    }
    if current_precision > STROOP_PRECISION {
        price / (current_precision / STROOP_PRECISION)
    } else {
        price * (STROOP_PRECISION / current_precision)
    }
}

/// Normalizes a price to derivative precision (10^14).
///
/// # Arguments
/// - `price` - The raw price value
/// - `current_precision` - The current precision of the price
///
/// # Returns
/// The price normalized to derivative precision
pub fn normalize_to_derivative(price: i128, current_precision: i128) -> i128 {
    if current_precision == DERIVATIVE_PRECISION {
        return price;
    }
    if current_precision > DERIVATIVE_PRECISION {
        price / (current_precision / DERIVATIVE_PRECISION)
    } else {
        price * (DERIVATIVE_PRECISION / current_precision)
    }
}

/// Computes deviation in basis points between two prices.
///
/// deviation_bps = (|price_a - price_b| * 10_000) / price_a
///
/// # Arguments
/// - `price_a` - First price (typically the primary oracle price)
/// - `price_b` - Second price (typically the fallback oracle price)
///
/// # Returns
/// Deviation in basis points (1 bp = 0.01%), or an `OracleError` if either
/// price is non-positive or the intermediate arithmetic overflows `i128`.
///
/// # Errors
/// - `OracleError::InvalidPrice` if either price is zero or negative.
/// - `OracleError::ArithmeticOverflow` if scaling the difference overflows.
pub fn compute_deviation_bps(price_a: i128, price_b: i128) -> Result<i128, OracleError> {
    if price_a <= 0 || price_b <= 0 {
        return Err(OracleError::InvalidPrice);
    }

    let diff = if price_a >= price_b {
        price_a.checked_sub(price_b)
    } else {
        price_b.checked_sub(price_a)
    }
    .ok_or(OracleError::ArithmeticOverflow)?;

    diff.checked_mul(10_000)
        .and_then(|scaled| scaled.checked_div(price_a))
        .ok_or(OracleError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stroop_precision_constant() {
        assert_eq!(STROOP_PRECISION, 10_000_000);
    }

    #[test]
    fn test_derivative_precision_constant() {
        assert_eq!(DERIVATIVE_PRECISION, 100_000_000_000_000);
    }

    #[test]
    fn test_normalize_to_stroop_same_precision() {
        let price = 200_000_000;
        assert_eq!(normalize_to_stroop(price, STROOP_PRECISION), price);
    }

    #[test]
    fn test_normalize_to_stroop_from_derivative() {
        // $20.00 in derivative precision = 2_000_000_000_000_000
        // Should normalize to 200_000_000 in STROOP
        let price = 2_000_000_000_000_000;
        let normalized = normalize_to_stroop(price, DERIVATIVE_PRECISION);
        assert_eq!(normalized, 200_000_000);
    }

    #[test]
    fn test_compute_deviation_bps_identical() {
        let deviation = compute_deviation_bps(1_000_000_000, 1_000_000_000);
        assert_eq!(deviation, Ok(0));
    }

    #[test]
    fn test_compute_deviation_bps_one_percent() {
        // 1% deviation = 100 bps
        let deviation = compute_deviation_bps(1_000_000_000, 990_000_000);
        assert_eq!(deviation, Ok(100));
    }

    #[test]
    fn test_compute_deviation_bps_five_percent() {
        // 5% deviation = 500 bps
        let deviation = compute_deviation_bps(1_000_000_000, 950_000_000);
        assert_eq!(deviation, Ok(500));
    }

    #[test]
    fn test_compute_deviation_bps_zero_price() {
        assert_eq!(
            compute_deviation_bps(0, 1_000_000_000),
            Err(OracleError::InvalidPrice)
        );
        assert_eq!(
            compute_deviation_bps(1_000_000_000, 0),
            Err(OracleError::InvalidPrice)
        );
    }

    #[test]
    fn test_compute_deviation_bps_negative_price() {
        assert_eq!(
            compute_deviation_bps(-1, 1_000_000_000),
            Err(OracleError::InvalidPrice)
        );
    }

    #[test]
    fn test_compute_deviation_bps_overflow_is_reported() {
        // |MAX - 1| * 10_000 cannot be represented in i128.
        assert_eq!(
            compute_deviation_bps(i128::MAX, 1),
            Err(OracleError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_compute_deviation_bps_large_but_consistent_prices() {
        // 1e30 and 9.9e29 are large yet their scaled difference fits in i128,
        // so a 1% deviation must still be reported as 100 bps.
        let primary = 1_000_000_000_000_000_000_000_000_000_000_i128;
        let fallback = 990_000_000_000_000_000_000_000_000_000_i128;
        assert_eq!(compute_deviation_bps(primary, fallback), Ok(100));
    }
}
