use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[path = "../src/main.rs"]
mod main_mod;

use main_mod::{LtHashState, LANE_COUNT, MODULUS_16};

/// Baseline: Hardware u16 wrapping addition without modular reduction
fn baseline_wrapping_add(a: &mut [u16; LANE_COUNT], b: &[u16; LANE_COUNT]) {
    for (l, r) in a.iter_mut().zip(b.iter()) {
        *l = l.wrapping_add(*r);
    }
}

/// Baseline: Hardware u16 wrapping subtraction without modular reduction
fn baseline_wrapping_sub(a: &mut [u16; LANE_COUNT], b: &[u16; LANE_COUNT]) {
    for (l, r) in a.iter_mut().zip(b.iter()) {
        *l = l.wrapping_sub(*r);
    }
}

fn bench_addition_comparison(c: &mut Criterion) {
    // `elem_a` represents `self`: pre-reduced into [0, MODULUS_16 - 1]
    let mut elem_a = LtHashState::hash_element(b"element_alpha");
    for lane in elem_a.lanes.iter_mut() {
        *lane %= MODULUS_16;
    }

    // `elem_b` represents unreduced input `other`: raw u16 in [0, 65535]
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

fn bench_subtraction_comparison(c: &mut Criterion) {
    // `elem_a` represents `self`: pre-reduced into [0, MODULUS_16 - 1]
    let mut elem_a = LtHashState::hash_element(b"element_alpha");
    for lane in elem_a.lanes.iter_mut() {
        *lane %= MODULUS_16;
    }

    // `elem_b` represents unreduced input `other`: raw u16 in [0, 65535]
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

criterion_group!(
    benches,
    bench_addition_comparison,
    bench_subtraction_comparison
);
criterion_main!(benches);