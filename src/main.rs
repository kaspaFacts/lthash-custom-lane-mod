pub const LANES: usize = 1024;
pub const LANE_COUNT: usize = LANES;
pub const MODULUS_16: u16 = 65_521; // M = 2^16 - 15

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LtHashState {
    pub lanes: [u16; LANES],
}

impl LtHashState {
    /// Creates a new identity state (all lanes set to zero).
    pub fn new() -> Self {
        Self { lanes: [0u16; LANES] }
    }

    /// Adds another state in-place: (a + b) mod (2^16 - 15)
    /// Operates strictly on u16 without upcasting.
    #[inline(always)]
    pub fn add(&mut self, other: &Self) {
        for (a, &b) in self.lanes.iter_mut().zip(other.lanes.iter()) {
            let (sum, overflow) = a.overflowing_add(b);
            *a = if overflow || sum >= MODULUS_16 {
                sum.wrapping_sub(MODULUS_16)
            } else {
                sum
            };
        }
    }

    /// Subtracts another state in-place: (a - b) mod (2^16 - 15)
    /// Operates strictly on u16 without upcasting.
    #[inline(always)]
    pub fn sub(&mut self, other: &Self) {
        for (a, &b) in self.lanes.iter_mut().zip(other.lanes.iter()) {
            *a = if *a >= b {
                *a - b
            } else {
                a.wrapping_add(MODULUS_16) - b
            };
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
            *lane = acc % MODULUS_16;
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
    #[test]
    fn test_strict_bounds_and_no_dual_zero() {
        let mut state = crate::LtHashState::new();
        for &lane in state.lanes.iter() {
            assert!(lane < crate::MODULUS_16);
        }

        let elem = crate::LtHashState::hash_element(b"test_bounds");
        state.add(&elem);
        for &lane in state.lanes.iter() {
            assert!(lane < crate::MODULUS_16);
        }
    }

    #[test]
    fn test_unreduced_input_boundaries() {
        let mut a = crate::LtHashState::new();
        let mut b = crate::LtHashState::new();

        a.lanes[0] = crate::MODULUS_16 - 1; // 65,520
        b.lanes[0] = crate::MODULUS_16 - 1; // 65,520

        a.add(&b);
        assert_eq!(a.lanes[0], crate::MODULUS_16 - 2);
        assert!(a.lanes[0] < crate::MODULUS_16);
    }

    #[test]
    fn test_subtraction_underflow_wrap() {
        let mut a = crate::LtHashState::new();
        let mut b = crate::LtHashState::new();

        a.lanes[0] = 5;
        b.lanes[0] = 10;

        a.sub(&b);
        assert_eq!(a.lanes[0], crate::MODULUS_16 - 5);
        assert!(a.lanes[0] < crate::MODULUS_16);
    }

    #[test]
    fn test_homomorphic_addition_order_independence() {
        let h_a = crate::LtHashState::hash_element(b"item_A");
        let h_b = crate::LtHashState::hash_element(b"item_B");

        let mut sum1 = crate::LtHashState::new();
        sum1.add(&h_a);
        sum1.add(&h_b);

        let mut sum2 = crate::LtHashState::new();
        sum2.add(&h_b);
        sum2.add(&h_a);

        assert_eq!(sum1, sum2);
    }

    #[test]
    fn test_full_vector_net_zero() {
        let h = crate::LtHashState::hash_element(b"net_zero_test");
        let mut state = crate::LtHashState::new();

        state.add(&h);
        state.sub(&h);

        assert_eq!(state, crate::LtHashState::new());
    }
}
