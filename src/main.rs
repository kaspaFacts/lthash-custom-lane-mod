use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::env;

pub const CUSTOM_MODULUS_U16: u16 = 65521; // Prime M = 2^16 - 15
pub const LANES: usize = 1024;
pub const PRECOMPUTED_TABLE_SIZE: usize = 10_000;

// ============================================================================
// 1. Core Arithmetic Functions (Single Lane & Full Vector)
// ============================================================================

/// Branchless Custom Modulo Addition on a single u16 lane (M = 65,521)
#[inline(always)]
pub fn add_lane_custom_mod(a: u16, b: u16) -> u16 {
    debug_assert!(a < CUSTOM_MODULUS_U16 && b < CUSTOM_MODULUS_U16);
    let threshold = CUSTOM_MODULUS_U16 - b;
    let diff = a.wrapping_sub(threshold);
    let mask = ((diff as i16) >> 15) as u16;
    diff.wrapping_add(threshold & mask)
}

/// Branchless Custom Modulo Subtraction on a single u16 lane (M = 65,521)
#[inline(always)]
pub fn sub_lane_custom_mod(a: u16, b: u16) -> u16 {
    debug_assert!(a < CUSTOM_MODULUS_U16 && b < CUSTOM_MODULUS_U16);
    let diff = a.wrapping_sub(b);
    let mask = ((diff as i16) >> 15) as u16;
    diff.wrapping_add(CUSTOM_MODULUS_U16 & mask)
}

/// Native 16-bit Wrapping Addition on a single u16 lane (M = 65,536 / 2^16)
#[inline(always)]
pub fn add_lane_native_wrap(a: u16, b: u16) -> u16 {
    a.wrapping_add(b)
}

/// Native 16-bit Wrapping Subtraction on a single u16 lane (M = 65,536 / 2^16)
#[inline(always)]
pub fn sub_lane_native_wrap(a: u16, b: u16) -> u16 {
    a.wrapping_sub(b)
}

// ============================================================================
// 2. Accumulator Struct (Full 1024-Lane Vector)
// ============================================================================

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LtHash16 {
    pub lanes: [u16; LANES],
}

impl LtHash16 {
    pub fn new() -> Self {
        Self { lanes: [0u16; LANES] }
    }

    pub fn is_identity(&self) -> bool {
        self.lanes.iter().all(|&l| l == 0)
    }

    #[inline(always)]
    pub fn add_vector_custom_mod(&mut self, other: &[u16; LANES]) {
        for i in 0..LANES {
            self.lanes[i] = add_lane_custom_mod(self.lanes[i], other[i]);
        }
    }

    #[inline(always)]
    pub fn sub_vector_custom_mod(&mut self, other: &[u16; LANES]) {
        for i in 0..LANES {
            self.lanes[i] = sub_lane_custom_mod(self.lanes[i], other[i]);
        }
    }

    #[inline(always)]
    pub fn add_vector_native_wrap(&mut self, other: &[u16; LANES]) {
        for i in 0..LANES {
            self.lanes[i] = add_lane_native_wrap(self.lanes[i], other[i]);
        }
    }

    #[inline(always)]
    pub fn sub_vector_native_wrap(&mut self, other: &[u16; LANES]) {
        for i in 0..LANES {
            self.lanes[i] = sub_lane_native_wrap(self.lanes[i], other[i]);
        }
    }
}

// ============================================================================
// 3. Precomputed Data Table Generator
// ============================================================================

/// Generates a deterministic precomputed table of 16-bit numbers in [0, 65520].
/// Bypasses all PRNG / Hashing overhead during actual benchmark timing.
pub fn generate_precomputed_table(count: usize) -> Vec<[u16; LANES]> {
    let mut table = Vec::with_capacity(count);
    let mut seed = 0x87654321u32; // Deterministic XorShift seed

    for _ in 0..count {
        let mut element = [0u16; LANES];
        for lane in element.iter_mut() {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *lane = (seed % (CUSTOM_MODULUS_U16 as u32)) as u16;
        }
        table.push(element);
    }
    table
}

// ============================================================================
// 4. Criterion Benchmarking Suite
// ============================================================================

fn benchmark_suite(c: &mut Criterion) {
    // 1. Precompute table in memory BEFORE timing starts
    let table = generate_precomputed_table(PRECOMPUTED_TABLE_SIZE);

    // Extract single-lane inputs for single-lane tests
    let single_lane_inputs: Vec<(u16, u16)> = table
        .iter()
        .take(PRECOMPUTED_TABLE_SIZE)
        .map(|arr| (arr[0], arr[1]))
        .collect();

    let mut group = c.benchmark_group("LtHash Arithmetic Benchmarks");

    // --- Benchmark A: Single Lane Add/Sub Operations ---
    group.bench_function(BenchmarkId::new("1. Single Lane - Native Wrap (2^16)", PRECOMPUTED_TABLE_SIZE), |b| {
        b.iter(|| {
            let mut acc = 0u16;
            for &(x, _) in black_box(&single_lane_inputs) {
                acc = add_lane_native_wrap(acc, x);
            }
            for &(x, _) in black_box(single_lane_inputs.iter().rev()) {
                acc = sub_lane_native_wrap(acc, x);
            }
            assert_eq!(acc, 0, "FAILED: Single lane failed to land at net-zero!");
            acc
        });
    });

    group.bench_function(BenchmarkId::new("2. Single Lane - Custom Mod (65,521)", PRECOMPUTED_TABLE_SIZE), |b| {
        b.iter(|| {
            let mut acc = 0u16;
            for &(x, _) in black_box(&single_lane_inputs) {
                acc = add_lane_custom_mod(acc, x);
            }
            for &(x, _) in black_box(single_lane_inputs.iter().rev()) {
                acc = sub_lane_custom_mod(acc, x);
            }
            assert_eq!(acc, 0, "FAILED: Single lane failed to land at net-zero!");
            acc
        });
    });

    // --- Benchmark B: Full 1,024-Lane Vector Operations ---
    group.bench_function(BenchmarkId::new("3. 1024-Lane Vector - Native Wrap (2^16)", PRECOMPUTED_TABLE_SIZE), |b| {
        b.iter(|| {
            let mut acc = LtHash16::new();
            for element in black_box(&table) {
                acc.add_vector_native_wrap(element);
            }
            for element in black_box(table.iter().rev()) {
                acc.sub_vector_native_wrap(element);
            }
            assert!(acc.is_identity(), "FAILED: Emptied set but failed to land at 0!");
            acc
        });
    });

    group.bench_function(BenchmarkId::new("4. 1024-Lane Vector - Custom Mod (65,521)", PRECOMPUTED_TABLE_SIZE), |b| {
        b.iter(|| {
            let mut acc = LtHash16::new();
            for element in black_box(&table) {
                acc.add_vector_custom_mod(element);
            }
            for element in black_box(table.iter().rev()) {
                acc.sub_vector_custom_mod(element);
            }
            assert!(acc.is_identity(), "FAILED: Emptied set but failed to land at 0!");
            acc
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_suite);

// ============================================================================
// 5. Main Binary Entry Point (CLI Test & Verification Harness)
// ============================================================================

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if invoked via `cargo bench`
    if args.iter().any(|arg| arg == "--bench") {
        criterion_main();
        return;
    }

    println!("===============================================================");
    println!(" LtHash Custom Modulo (M = 65,521) Verification & Test Suite");
    println!("===============================================================\n");

    println!("[1/3] Generating precomputed table of 10,000 x 1024-lane elements...");
    let table = generate_precomputed_table(10_000);
    println!("      Done. Precomputed table ready in RAM.\n");

    println!("[2/3] Running Single-Lane Integrity Test...");
    let mut single_acc = 0u16;
    for element in &table {
        single_acc = add_lane_custom_mod(single_acc, element[0]);
    }
    for element in table.iter().rev() {
        single_acc = sub_lane_custom_mod(single_acc, element[0]);
    }
    assert_eq!(single_acc, 0, "Single lane net-zero verification failed!");
    println!("      SUCCESS: Single lane returned to exact 0!\n");

    println!("[3/3] Running Full 1,024-Lane Vector Integrity Test...");
    let mut vec_acc = LtHash16::new();
    for element in &table {
        vec_acc.add_vector_custom_mod(element);
    }
    for element in table.iter().rev() {
        vec_acc.sub_vector_custom_mod(element);
    }
    assert!(vec_acc.is_identity(), "Vector net-zero verification failed!");
    println!("      SUCCESS: All 1,024 vector lanes returned to exact 0!\n");

    println!("===============================================================");
    println!(" VERIFICATION COMPLETE: ALL INTEGRITY TESTS PASSED (Net Zero = 0)");
    println!(" To run microbenchmarks, execute: cargo bench");
    println!("===============================================================");
}

// ============================================================================
// 6. Unit Tests (cargo test)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_lane_add_sub_boundary() {
        let a = 65520u16; // M - 1
        let b = 100u16;

        let sum = add_lane_custom_mod(a, b);
        assert_eq!(sum, 99); // Wrap over M

        let diff = sub_lane_custom_mod(sum, b);
        assert_eq!(diff, a); // Return to original
    }

    #[test]
    fn test_full_vector_lifecycle_net_zero() {
        let table = generate_precomputed_table(1_000);
        let mut acc = LtHash16::new();

        for elem in &table {
            acc.add_vector_custom_mod(elem);
        }
        assert!(!acc.is_identity());

        for elem in table.iter().rev() {
            acc.sub_vector_custom_mod(elem);
        }
        assert!(acc.is_identity(), "Vector failed to return to net zero!");
    }
}
