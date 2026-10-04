use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[path = "../src/main.rs"]
mod main_mod;

use main_mod::{LtHashState, LANE_COUNT};

/// Hardware baseline: raw 16-bit wrapping addition without modular reduction
fn baseline_wrapping_add(a: &mut [u16; LANE_COUNT], b: &[u16; LANE_COUNT]) {
    for (l, r) in a.iter_mut().zip(b.iter()) {
        *l = l.wrapping_add(*r);
    }
}

/// Hardware baseline: raw 16-bit wrapping subtraction without modular reduction
fn baseline_wrapping_sub(a: &mut [u16; LANE_COUNT], b: &[u16; LANE_COUNT]) {
    for (l, r) in a.iter_mut().zip(b.iter()) {
        *l = l.wrapping_sub(*r);
    }
}

fn bench_addition_comparison(c: &mut Criterion) {
    let elem_a = LtHashState::hash_element(b"element_alpha");
    let elem_b = LtHashState::hash_element(b"element_beta");

    let mut group = c.benchmark_group("Addition_Comparison");

    group.bench_function("wrapping_add_u16", |b| {
        b.iter(|| {
            let mut lanes = black_box(elem_a.lanes);
            baseline_wrapping_add(&mut lanes, black_box(&elem_b.lanes));
            lanes
        })
    });

    group.bench_function("custom_mod_add_u16", |b| {
        b.iter(|| {
            let mut state = black_box(elem_a);
            state.add(black_box(&elem_b));
            state
        })
    });

    group.finish();
}

fn bench_subtraction_comparison(c: &mut Criterion) {
    let elem_a = LtHashState::hash_element(b"element_alpha");
    let elem_b = LtHashState::hash_element(b"element_beta");

    let mut group = c.benchmark_group("Subtraction_Comparison");

    group.bench_function("wrapping_sub_u16", |b| {
        b.iter(|| {
            let mut lanes = black_box(elem_a.lanes);
            baseline_wrapping_sub(&mut lanes, black_box(&elem_b.lanes));
            lanes
        })
    });

    group.bench_function("custom_mod_sub_u16", |b| {
        b.iter(|| {
            let mut state = black_box(elem_a);
            state.sub(black_box(&elem_b));
            state
        })
    });

    group.finish();
}

fn bench_hash_element(c: &mut Criterion) {
    let data = b"benchmark_payload_data_string_1234567890";

    c.bench_function("hash_element_40bytes", |b| {
        b.iter(|| LtHashState::hash_element(black_box(data)))
    });
}

criterion_group!(
    benches,
    bench_hash_element,
    bench_addition_comparison,
    bench_subtraction_comparison
);
criterion_main!(benches);
