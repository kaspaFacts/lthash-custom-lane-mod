use std::fmt;

pub const LANE_COUNT: usize = 16;
pub const MODULUS: u64 = 0xFFFF_FFFF_FFFF_0001; // 64-bit prime/modulus

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LtHashState {
    pub lanes: [u64; LANE_COUNT],
}

impl LtHashState {
    pub fn new() -> Self {
        Self {
            lanes: [0; LANE_COUNT],
        }
    }

    /// Fast modular addition without 128-bit division
    pub fn add(&mut self, rhs: &Self) {
        for (l, r) in self.lanes.iter_mut().zip(rhs.lanes.iter()) {
            let (sum, overflow) = l.overflowing_add(*r);
            if overflow || sum >= MODULUS {
                *l = sum.wrapping_sub(MODULUS);
            } else {
                *l = sum;
            }
        }
    }

    /// Fast modular subtraction without 128-bit division
    pub fn sub(&mut self, rhs: &Self) {
        for (l, r) in self.lanes.iter_mut().zip(rhs.lanes.iter()) {
            if *l >= *r {
                *l -= *r;
            } else {
                *l = (*l + MODULUS) - *r;
            }
        }
    }

    pub fn hash_element(data: &[u8]) -> Self {
        let mut lanes = [0u64; LANE_COUNT];
        let mut state = 0x85ebca6b_u64;
        for (i, lane) in lanes.iter_mut().enumerate() {
            let mut val = state;
            for &byte in data {
                val = val.wrapping_mul(31).wrapping_add(byte as u64);
            }
            val = val.wrapping_add(i as u64);
            *lane = val % MODULUS;
            state = val;
        }
        Self { lanes }
    }
}

impl Default for LtHashState {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LtHashState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LtHash(")?;
        for (i, lane) in self.lanes.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:#x}", lane)?;
        }
        write!(f, ")")
    }
}

#[allow(dead_code)]
pub fn main() {
    println!("LtHash Custom Lane Mod");

    let elem_a = b"apple";
    let elem_b = b"banana";

    let hash_a = LtHashState::hash_element(elem_a);
    let hash_b = LtHashState::hash_element(elem_b);

    let mut set_hash = LtHashState::new();
    set_hash.add(&hash_a);
    set_hash.add(&hash_b);

    println!("Combined Set Hash: {}", set_hash);

    set_hash.sub(&hash_a);
    println!("After removing 'apple': {}", set_hash);
    println!("Matches 'banana' alone? {}", set_hash == hash_b);
}

// --- UNIT TESTS ---

#[cfg(test)]
mod tests {
    use super::LtHashState;
    use super::MODULUS;

    #[test]
    fn test_strict_bounds_and_no_dual_zero() {
        let mut state = LtHashState::new();
        for lane in state.lanes.iter() {
            assert!(*lane < MODULUS);
        }

        let elem = LtHashState::hash_element(b"test_bounds");
        state.add(&elem);
        for lane in state.lanes.iter() {
            assert!(*lane < MODULUS);
        }
    }

    #[test]
    fn test_unreduced_input_boundaries() {
        let h1 = LtHashState::hash_element(b"input_1");
        let h2 = LtHashState::hash_element(b"input_2");

        let mut sum = h1;
        sum.add(&h2);

        for lane in sum.lanes.iter() {
            assert!(*lane < MODULUS);
        }
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
}
