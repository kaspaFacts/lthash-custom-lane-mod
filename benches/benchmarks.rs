//! # LtHash Custom Modulo Benchmarking Suite (M = 2^16 - 15)
//!
//! This suite microbenchmarks high-performance, branchless modular arithmetic for
//! **LtHash** across 1,024-lane state vectors under the prime modulus $M = 65,521$.
//!
//! ## Benchmarked Scenarios
//!
//! 1. **U32 Reduction Methods:**
//!    Compares standard hardware modular division (`x % 65,521`) against fast, branchless
//!    Solinas folding (`compress_u32_to_u16`). Solinas reduction avoids hardware division unit
//!    stalls and enables SIMD auto-vectorization across 32-bit registers.
//!
//! 2. **Addition Comparison:**
//!    Evaluates native CPU `u16::wrapping_add` against `custom_mod_add_u16_single` over 1,024 lanes.
//!
//! 3. **Subtraction Comparison:**
//!    Evaluates native CPU `u16::wrapping_sub` against `custom_mod_sub_u16_single` over 1,024 lanes.
//!
//! 4. **End-to-End U32 Ingestion (`add_u32`):**
//!    Measures full vector ingestion taking raw `[u32; 1024]` inputs, reducing each lane via
//!    Solinas folding, and accumulating the result into the `LtHashState` vector in a single pass.
//!
//! ## Running Benchmarks
//!
//! Standard execution:
//! ```bash
//! cargo bench
//! ```
//!
//! Enable native CPU SIMD register auto-vectorization (AVX2 / AVX-512):
//! ```bash
//! RUSTFLAGS="-C target-cpu=native" cargo bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[path = "../src/main.rs"]
mod main_mod;

use main_mod::{compress_u32_to_u16, LtHashState, LANE_COUNT, MODULUS_16};

/// Baseline: Unoptimized hardware u32 division modulo over 1,024 lanes.
#[inline(never)]
fn baseline_modulo_div_u32(input: &[u32; LANE_COUNT], out: &mut [u16; LANE_COUNT]) {
    for (src, dst) in input.iter().zip(out.iter_mut()) {
        *dst = (src % (MODULUS_16 as u32)) as u16;
    }
}

/// Baseline: Hardware u16 wrapping addition without modular reduction.
#[inline(never)]
fn baseline_wrapping_add(a: &mut [u16; LANE_COUNT], b: &[u16; LANE_COUNT]) {
    for (l, r) in a.iter_mut().zip(b.iter()) {
        *l = l.wrapping_add(*r);
    }
}

/// Baseline: Hardware u16 wrapping subtraction without modular reduction.
#[inline(never)]
fn baseline_wrapping_sub(a: &mut [u16; LANE_COUNT], b: &[u16; LANE_COUNT]) {
    for (l, r) in a.iter_mut().zip(b.iter()) {
        *l = l.wrapping_sub(*r);
    }
}

/// Helper function to generate pseudo-random 32-bit state vectors across the full [0, 2^32 - 1] space
/// using an LCG generator to avoid fixed-pattern compiler optimizations.
fn generate_random_u32_lanes(seed: u32) -> [u32; LANE_COUNT] {
    let mut lanes = [0u32; LANE_COUNT];
    let mut state = seed;
    for lane in lanes.iter_mut() {
        // Linear Congruential Generator (Numerical Recipes parameters)
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *lane = state;
    }
    lanes
}

/// Benchmarks u32 reduction methods (Hardware Modulo % vs. Solinas Compression).
fn bench_u32_reduction_comparison(c: &mut Criterion) {
    let raw_u32_inputs = generate_random_u32_lanes(0xdeadbeef);

    let mut group = c.benchmark_group("U32_Reduction_Methods");

    group.bench_function("hardware_modulo_div_u32", |b| {
        b.iter(|| {
            let mut out = [0u16; LANE_COUNT];
            baseline_modulo_div_u32(black_box(&raw_u32_inputs), &mut out);
            black_box(out)
        })
    });

    group.bench_function("branchless_solinas_compress_u32", |b| {
        b.iter(|| {
            let mut out = [0u16; LANE_COUNT];
            for (src, dst) in black_box(&raw_u32_inputs).iter().zip(out.iter_mut()) {
                *dst = compress_u32_to_u16(*src);
            }
            black_box(out)
        })
    });

    group.finish();
}

/// Benchmarks u16 addition comparison (Hardware Wrapping vs. Branchless Z_M Addition).
fn bench_addition_comparison(c: &mut Criterion) {
    let mut elem_a = LtHashState::hash_element(b"element_alpha");
    for lane in elem_a.lanes.iter_mut() {
        *lane %= MODULUS_16;
    }

    let elem_b = LtHashState::hash_element(b"element_beta");

    let mut group = c.benchmark_group("Addition_Comparison");

    group.bench_function("hardware_wrapping_add_u16", |b| {
        b.iter(|| {
            let mut lanes = black_box(elem_a.lanes);
            baseline_wrapping_add(&mut lanes, black_box(&elem_b.lanes));
            black_box(lanes)
        })
    });

    group.bench_function("custom_branchless_mod_add_u16", |b| {
        b.iter(|| {
            let mut state = black_box(elem_a);
            state.add(black_box(&elem_b));
            black_box(state)
        })
    });

    group.finish();
}

/// Benchmarks u16 subtraction comparison (Hardware Wrapping vs. Branchless Z_M Subtraction).
fn bench_subtraction_comparison(c: &mut Criterion) {
    let mut elem_a = LtHashState::hash_element(b"element_alpha");
    for lane in elem_a.lanes.iter_mut() {
        *lane %= MODULUS_16;
    }

    let elem_b = LtHashState::hash_element(b"element_beta");

    let mut group = c.benchmark_group("Subtraction_Comparison");

    group.bench_function("hardware_wrapping_sub_u16", |b| {
        b.iter(|| {
            let mut lanes = black_box(elem_a.lanes);
            baseline_wrapping_sub(&mut lanes, black_box(&elem_b.lanes));
            black_box(lanes)
        })
    });

    group.bench_function("custom_branchless_mod_sub_u16", |b| {
        b.iter(|| {
            let mut state = black_box(elem_a);
            state.sub(black_box(&elem_b));
            black_box(state)
        })
    });

    group.finish();
}

/// Benchmarks full end-to-end 32-bit state ingestion (`add_u32`).
fn bench_u32_pipeline_ingestion(c: &mut Criterion) {
    let mut state = LtHashState::new();
    let raw_u32_inputs = generate_random_u32_lanes(0xcafe_babe);

    let mut group = c.benchmark_group("U32_Pipeline_Ingestion");

    group.bench_function("add_u32_vector_1024_lanes", |b| {
        b.iter(|| {
            let mut current_state = black_box(state);
            current_state.add_u32(black_box(&raw_u32_inputs));
            black_box(current_state)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_u32_reduction_comparison,
    bench_addition_comparison,
    bench_subtraction_comparison,
    bench_u32_pipeline_ingestion
);
criterion_main!(benches);