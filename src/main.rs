//! # LtHash Custom Modulo & Solinas Compression Suite (M = 2^16 - 15)
//!
//! This module provides high-performance, branchless modular arithmetic for **LtHash**
//! (Lattice-based Homomorphic Hash) operating over the Solinas prime field $Z_M$ where
//! $M = 2^{16} - 15 = 65,521$.
//!
//! ## Architecture Overview
//!
//! 1. **Solinas $u32 \to u16$ Reduction (`compress_u32_to_u16`):**
//!    Leverages the identity $2^{16} \equiv 15 \pmod M$ to reduce raw 32-bit integers into
//!    strictly reduced 16-bit elements in $[0, 65,520]$ without hardware division.
//!    - **Fold 1:** $(x \gg 16) \times 15 + (x \ \& \ \text{0xFFFF})$ (max $1,048,560$, $\sim 20$ bits).
//!    - **Fold 2:** $(\text{sum}_1 \gg 16) \times 15 + (\text{sum}_1 \ \& \ \text{0xFFFF})$ (max $65,760$).
//!    - **Branchless Normalization:** Subtracts $65,521$ if $\text{sum}_2 \ge 65,521$ using bitwise selection.
//!
//! 2. **Modular Arithmetic (`custom_mod_add_u16_single` / `custom_mod_sub_u16_single`):**
//!    Fully branchless vector-ready operations with preconditioning for raw inputs.
//!    When fed by `compress_u32_to_u16`, the preconditioning mask evaluates to $0$, enabling LLVM
//!    to eliminate unused instructions during inlining.

pub const LANES: usize = 1024;
pub const LANE_COUNT: usize = LANES;
pub const MODULUS_16: u16 = 65_521; // Solinas prime M = 2^16 - 15

/// Branchless Solinas reduction mapping any 32-bit unsigned integer into $Z_M$ ($M = 65,521$).
///
/// Guarantees the output $y \in [0, 65,520]$.
#[inline(always)]
pub fn compress_u32_to_u16(x: u32) -> u16 {
    // Fold 1: Multiply top 16 bits by 15 and add bottom 16 bits (max: 1,048,560)
    let sum1 = (x & 0xFFFF) + 15 * (x >> 16);

    // Fold 2: Repeat on intermediate 20-bit output (max: 65,760)
    let sum2 = (sum1 & 0xFFFF) + 15 * (sum1 >> 16);

    // Single branchless normalization step since sum2 <= 65,760 < 2 * MODULUS_16
    let mask = (sum2 >= MODULUS_16 as u32) as u32;
    (sum2 - (MODULUS_16 as u32 & mask.wrapping_neg())) as u16
}

/// Fully branchless modular addition on a single u16 lane in $Z_M$.
///
/// Accepts both reduced elements in $[0, 65,520]$ and unreduced raw inputs in $[0, 65,535]$.
#[inline(always)]
pub fn custom_mod_add_u16_single(a: u16, b: u16) -> u16 {
    // 1. Precondition `b`: subtract 65,521 if b >= 65,521
    let mask_b = (b >= MODULUS_16) as u16;
    let b_red = b.wrapping_sub(MODULUS_16 & mask_b.wrapping_neg());

    // 2. Perform u16 addition with overflow detection
    let (sum, overflow) = a.overflowing_add(b_red);

    // 3. Modular adjustment: subtract 65,521 if overflow || sum >= 65,521
    let mask_sum = (overflow | (sum >= MODULUS_16)) as u16;
    sum.wrapping_sub(MODULUS_16 & mask_sum.wrapping_neg())
}

/// Fully branchless modular subtraction on a single u16 lane in $Z_M$.
///
/// Accepts both reduced elements in $[0, 65,520]$ and unreduced raw inputs in $[0, 65,535]$.
#[inline(always)]
pub fn custom_mod_sub_u16_single(a: u16, b: u16) -> u16 {
    // 1. Precondition `b`: subtract 65,521 if b >= 65,521
    let mask_b = (b >= MODULUS_16) as u16;
    let b_red = b.wrapping_sub(MODULUS_16 & mask_b.wrapping_neg());

    // 2. Perform u16 subtraction with underflow detection
    let (diff, underflow) = a.overflowing_sub(b_red);

    // 3. Modular adjustment: add 65,521 if underflow (a < b_red)
    let mask_underflow = underflow as u16;
    diff.wrapping_add(MODULUS_16 & mask_underflow.wrapping_neg())
}

/// State container holding a 1,024-lane vector of 16-bit field elements in $Z_M$.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LtHashState {
    pub lanes: [u16; LANES],
}

impl LtHashState {
    /// Creates a new identity state (all lanes initialized to zero).
    pub fn new() -> Self {
        Self { lanes: [0u16; LANES] }
    }

    /// Fully branchless modular addition in $Z_M$ using 16-bit input state.
    #[inline(always)]
    pub fn add(&mut self, other: &Self) {
        for (a, &b) in self.lanes.iter_mut().zip(other.lanes.iter()) {
            *a = custom_mod_add_u16_single(*a, b);
        }
    }

    /// Fully branchless modular subtraction in $Z_M$ using 16-bit input state.
    #[inline(always)]
    pub fn sub(&mut self, other: &Self) {
        for (a, &b) in self.lanes.iter_mut().zip(other.lanes.iter()) {
            *a = custom_mod_sub_u16_single(*a, b);
        }
    }

    /// Solinas compresses a raw 32-bit state vector into $Z_M$ and adds it to the accumulator.
    #[inline(always)]
    pub fn add_u32(&mut self, rhs_u32: &[u32; LANES]) {
        for (a, &b) in self.lanes.iter_mut().zip(rhs_u32.iter()) {
            let b_red = compress_u32_to_u16(b);
            *a = custom_mod_add_u16_single(*a, b_red);
        }
    }

    /// Solinas compresses a raw 32-bit state vector into $Z_M$ and subtracts it from the accumulator.
    #[inline(always)]
    pub fn sub_u32(&mut self, rhs_u32: &[u32; LANES]) {
        for (a, &b) in self.lanes.iter_mut().zip(rhs_u32.iter()) {
            let b_red = compress_u32_to_u16(b);
            *a = custom_mod_sub_u16_single(*a, b_red);
        }
    }

    /// Dummy hash expansion generating 1,024 16-bit lanes for testing and benchmark harnesses.
    pub fn hash_element(data: &[u8]) -> Self {
        let mut lanes = [0u16; LANES];
        for (i, lane) in lanes.iter_mut().enumerate() {
            let mut acc: u16 = (i as u16).wrapping_mul(31);
            for &byte in data {
                acc = acc.wrapping_mul(37).wrapping_add(byte as u16);
            }
            *lane = acc;
        }
        Self { lanes }
    }

    /// Dummy hash expansion generating 1,024 raw 32-bit lanes for testing `add_u32` / `sub_u32`.
    pub fn hash_element_u32(data: &[u8]) -> [u32; LANES] {
        let mut lanes = [0u32; LANES];
        for (i, lane) in lanes.iter_mut().enumerate() {
            let mut acc: u32 = (i as u32).wrapping_mul(1_664_525);
            for &byte in data {
                acc = acc.wrapping_mul(37).wrapping_add(byte as u32);
            }
            *lane = acc;
        }
        lanes
    }
}

impl Default for LtHashState {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("LtHash 1,024-lane Solinas benchmarking suite (M = 65,521)");

    let mut state = LtHashState::new();
    let elem_u32 = LtHashState::hash_element_u32(b"kaspa_utxo_32bit_test_payload");

    state.add_u32(&elem_u32);
    println!("First lane sample (after add_u32): {}", state.lanes[0]);

    state.sub_u32(&elem_u32);
    println!("After sub_u32 (net zero check): {}", state.lanes[0]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_bounds_and_no_dual_zero() {
        let mut state = LtHashState::new();
        for &lane in state.lanes.iter() {
            assert!(lane < MODULUS_16);
        }

        let elem = LtHashState::hash_element(b"test_bounds");
        state.add(&elem);
        for &lane in state.lanes.iter() {
            assert!(lane < MODULUS_16);
        }
    }

    #[test]
    fn test_unreduced_input_preconditioning() {
        let mut state1 = LtHashState::new();
        let mut state2 = LtHashState::new();

        state1.lanes[0] = 100;
        state2.lanes[0] = 100;

        let mut b_unreduced = LtHashState::new();
        let mut b_reduced = LtHashState::new();

        b_unreduced.lanes[0] = 65_535; // 65,535 mod 65,521 = 14
        b_reduced.lanes[0] = 14;

        state1.add(&b_unreduced);
        state2.add(&b_reduced);

        assert_eq!(state1.lanes[0], state2.lanes[0]);
        assert_eq!(state1.lanes[0], 114);
    }

    #[test]
    fn test_addition_modular_wrap_exact_boundary() {
        let mut state = LtHashState::new();
        state.lanes[0] = 65_520;

        let mut elem = LtHashState::new();
        elem.lanes[0] = 1; // 65,520 + 1 = 65,521 = 0 mod 65,521

        state.add(&elem);
        assert_eq!(state.lanes[0], 0);

        let mut elem2 = LtHashState::new();
        elem2.lanes[0] = 65_520;
        state.add(&elem2);
        assert_eq!(state.lanes[0], 65_520);
    }

    #[test]
    fn test_subtraction_underflow_wrap() {
        let mut state = LtHashState::new();
        state.lanes[0] = 0;

        let mut elem = LtHashState::new();
        elem.lanes[0] = 1; // 0 - 1 mod 65,521 = 65,520

        state.sub(&elem);
        assert_eq!(state.lanes[0], 65_520);

        let mut elem_unreduced = LtHashState::new();
        elem_unreduced.lanes[0] = 65_535;
        state.sub(&elem_unreduced);
        assert_eq!(state.lanes[0], 65_506);
    }

    #[test]
    fn test_homomorphic_addition_order_independence() {
        let h_a = LtHashState::hash_element(b"item_A");
        let h_b = LtHashState::hash_element(b"item_B");

        let mut sum1 = LtHashState::new();
        sum1.add(&h_a);
        sum1.add(&h_b);

        let mut sum2 = LtHashState::new();
        sum2.add(&h_b);
        sum2.add(&h_a);

        assert_eq!(sum1, sum2);
    }

    #[test]
    fn test_full_vector_net_zero() {
        let h = LtHashState::hash_element(b"net_zero_test");
        let mut state = LtHashState::new();

        state.add(&h);
        state.sub(&h);

        assert_eq!(state, LtHashState::new());
    }

    #[test]
    fn test_u32_edge_cases() {
        let cases: &[(u32, u16)] = &[
            (0, 0),
            (65_520, 65_520),
            (65_521, 0),
            (65_522, 1),
            (65_535, 14),
            (65_536, 15),
            (1_048_560, (1_048_560 % MODULUS_16 as u32) as u16),
            (0x7FFF_FFFF, (0x7FFF_FFFF % MODULUS_16 as u32) as u16),
            (u32::MAX, (u32::MAX % MODULUS_16 as u32) as u16),
        ];

        for &(input, expected) in cases {
            let actual = compress_u32_to_u16(input);
            assert_eq!(
                actual, expected,
                "Failed edge case for u32 input: {}",
                input
            );
        }
    }

    #[test]
    fn test_u32_vector_net_zero() {
        let raw_u32 = LtHashState::hash_element_u32(b"u32_vector_net_zero");
        let mut state = LtHashState::new();

        state.add_u32(&raw_u32);
        state.sub_u32(&raw_u32);

        assert_eq!(state, LtHashState::new());
    }

    /// Exhaustive single-lane u16 validation testing all 4.29 billion pairs.
    ///
    /// Marked `#[ignore]` so daily `cargo test` runs remain instant.
    /// Run with: `cargo test --release -- --ignored`
    #[test]
    #[ignore]
    fn test_exhaustive_single_lane_correctness() {
        let m = MODULUS_16;

        for a in 0..MODULUS_16 {
            for b in 0..=u16::MAX {
                let b_red = if b >= m { b - m } else { b };

                let sum = a as u32 + b_red as u32;
                let expected_add = (if sum >= m as u32 { sum - m as u32 } else { sum }) as u16;

                let diff = (a as u32 + m as u32) - b_red as u32;
                let expected_sub = (if diff >= m as u32 { diff - m as u32 } else { diff }) as u16;

                let actual_add = custom_mod_add_u16_single(a, b);
                let actual_sub = custom_mod_sub_u16_single(a, b);

                assert_eq!(actual_add, expected_add, "Add failed for a = {}, b = {}", a, b);
                assert_eq!(actual_sub, expected_sub, "Sub failed for a = {}, b = {}", a, b);
            }
        }
    }

    /// Exhaustive 32-bit reduction sweep across all 4,294,967,296 possible u32 inputs.
    ///
    /// Marked `#[ignore]` so standard `cargo test` does not stall debug builds.
    /// Run with: `cargo test --release -- --ignored` (~0.5s runtime).
    #[test]
    #[ignore]
    fn test_exhaustive_u32_reduction_sweep() {
        for x in 0..=u32::MAX {
            let expected = (x % MODULUS_16 as u32) as u16;
            let actual = compress_u32_to_u16(x);
            assert_eq!(actual, expected, "Exhaustive mismatch at x = {}", x);
        }
    }
}