pub const LANES: usize = 1024;
pub const LANE_COUNT: usize = LANES;
pub const MODULUS_16: u16 = 65_521; // M = 2^16 - 15

/// Fully branchless modular addition on a single u16 lane in Z_M.
#[inline(always)]
pub fn custom_mod_add_u16_single(a: u16, b: u16) -> u16 {
    // 1. Precondition `b`: subtract 65,521 if b >= 65,521
    let mask_b = (b >= MODULUS_16) as u16;
    let b_red = b.wrapping_sub(MODULUS_16 & mask_b.wrapping_neg());

    // 2. Perform u16 addition
    let (sum, overflow) = a.overflowing_add(b_red);

    // 3. Modular reduction: subtract 65,521 if overflow || sum >= 65,521
    let mask_sum = (overflow | (sum >= MODULUS_16)) as u16;
    sum.wrapping_sub(MODULUS_16 & mask_sum.wrapping_neg())
}

/// Fully branchless modular subtraction on a single u16 lane in Z_M.
#[inline(always)]
pub fn custom_mod_sub_u16_single(a: u16, b: u16) -> u16 {
    // 1. Precondition `b`: subtract 65,521 if b >= 65,521
    let mask_b = (b >= MODULUS_16) as u16;
    let b_red = b.wrapping_sub(MODULUS_16 & mask_b.wrapping_neg());

    // 2. Perform u16 subtraction with underflow check
    let (diff, underflow) = a.overflowing_sub(b_red);

    // 3. Modular adjustment: add 65,521 if underflow (a < b_red)
    let mask_underflow = underflow as u16;
    diff.wrapping_add(MODULUS_16 & mask_underflow.wrapping_neg())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LtHashState {
    pub lanes: [u16; LANES],
}

impl LtHashState {
    /// Creates a new identity state (all lanes set to zero).
    pub fn new() -> Self {
        Self { lanes: [0u16; LANES] }
    }

    /// Fully branchless modular addition in Z_M.
    #[inline(always)]
    pub fn add(&mut self, other: &Self) {
        for (a, &b) in self.lanes.iter_mut().zip(other.lanes.iter()) {
            *a = custom_mod_add_u16_single(*a, b);
        }
    }

    /// Fully branchless modular subtraction in Z_M.
    #[inline(always)]
    pub fn sub(&mut self, other: &Self) {
        for (a, &b) in self.lanes.iter_mut().zip(other.lanes.iter()) {
            *a = custom_mod_sub_u16_single(*a, b);
        }
    }

    /// Dummy hash expansion generating 1024 u16 lanes for benchmark/test harness.
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
}

impl Default for LtHashState {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("LtHash 1,024-lane u16 benchmarking suite (M = 65,521)");
    let mut state = LtHashState::new();
    let elem = LtHashState::hash_element(b"kaspa_utxo_test_payload");
    state.add(&elem);
    println!("First lane sample: {}", state.lanes[0]);
    state.sub(&elem);
    println!("After subtraction (net zero check): {}", state.lanes[0]);
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
        // Test that unreduced inputs (b >= 65,521) behave identically to pre-reduced ones
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
        state.lanes[0] = 65_520; // Max reduced element

        let mut elem = LtHashState::new();
        elem.lanes[0] = 1; // 65,520 + 1 = 65,521 = 0 mod 65,521

        state.add(&elem);
        assert_eq!(state.lanes[0], 0);

        let mut elem2 = LtHashState::new();
        elem2.lanes[0] = 65_520; // 0 + 65,520 = 65,520
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
        elem_unreduced.lanes[0] = 65_535; // 65,520 - (65,535 mod 65,521 = 14) = 65,506
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

    /// Exhaustive single-lane validation testing all possible valid `a` against all u16 `b`.
    #[test]
    #[ignore]
    fn test_exhaustive_single_lane_correctness() {
        let m = MODULUS_16;

        for a in 0..MODULUS_16 {
            for b in 0..=u16::MAX {
                // Fast reference math without u32 modulo calls in debug mode
                let b_red = if b >= m { b - m } else { b };

                let sum = a as u32 + b_red as u32;
                let expected_add = (if sum >= m as u32 { sum - m as u32 } else { sum }) as u16;

                let diff = (a as u32 + m as u32) - b_red as u32;
                let expected_sub = (if diff >= m as u32 { diff - m as u32 } else { diff }) as u16;

                let actual_add = custom_mod_add_u16_single(a, b);
                let actual_sub = custom_mod_sub_u16_single(a, b);

                if actual_add != expected_add || actual_sub != expected_sub {
                    assert_eq!(actual_add, expected_add, "Add failed for a = {}, b = {}", a, b);
                    assert_eq!(actual_sub, expected_sub, "Sub failed for a = {}, b = {}", a, b);
                }
            }
        }
    }
}