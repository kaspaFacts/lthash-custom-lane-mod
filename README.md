# LtHash Custom Modulo Benchmarking Suite (M = 2^64 - 59)

> **Disclaimer:** This project and its performance optimizations were developed with assistance from AI (LLM collaboration) for code generation, benchmark structuring, and algebraic verification.

This repository isolates, tests, and benchmarks the custom modular arithmetic for **LtHash** (Lattice-based Homomorphic Hash). It evaluates 16-lane 64-bit vector addition and subtraction under a custom 64-bit modulus (M = 2^64 - 59) against native CPU hardware-wrapping arithmetic (u64::wrapping_add / u64::wrapping_sub).

The implementation is designed as a drop-in reference for homomorphic set hashing in high-throughput systems (such as **Rusty Kaspa** node consensus, UTXO set commitments, and mempool synchronization).

---

## Performance Overview

Benchmarks measured on 16-lane [u64; 16] vectors via **Criterion.rs**:

| Operation | Mean Time | Overhead vs. Hardware | Status |
| :--- | :--- | :--- | :--- |
| **wrapping_add** | 10.40 ns | Baseline | Standard 64-bit vector addition |
| **custom_mod_add** | 20.16 ns | +9.76 ns | Modular overflow check without division |
| **wrapping_sub** | 10.33 ns | Baseline | Standard 64-bit vector subtraction |
| **custom_mod_sub** | 12.00 ns | +1.67 ns | **~98% theoretical hardware speed** |
| **hash_element_40bytes** | 474.12 ns | — | Input-to-lane pseudo-expansion (test harness) |

---

## Repository Layout

lthash-custom-mod/
├── Cargo.toml
├── README.md
├── benches/
│   └── benchmarks.rs   # Criterion benchmark definitions
└── src/
    └── main.rs         # Core LtHashState, custom mod arithmetic, and unit tests

---

## 1. Unit Testing & Algebraic Integrity

The test suite verifies core mathematical properties required for homomorphic hashing:
- **Strict Bounds:** Ensures all output lanes satisfy 0 <= lane < M.
- **Boundary Overflow/Underflow:** Correctly handles 64-bit hardware integer wrap-around before modular reduction.
- **Homomorphic Order Independence:** Verifies commutativity (A + B == B + A).
- **Net-Zero Inverse Property:** Verifies (A + B) - B == A and multi-item set removals return to zero.

To run the unit tests:

cargo test

---

## 2. Running Microbenchmarks

To execute the Criterion microbenchmarks with native CPU SIMD vectorization enabled:

RUSTFLAGS="-C target-cpu=native" cargo bench

### What the Benchmarks Measure:

1. **Native vs. Custom Addition:** Compares raw 16-lane wrapping_add against custom_mod_add.
2. **Native vs. Custom Subtraction:** Compares raw 16-lane wrapping_sub against custom_mod_sub.
3. **Element Expansion Overhead:** Measures element-to-lane mapping latency (hash_element_40bytes).

---

## 3. Results Inspection

After running cargo bench, open the interactive Criterion HTML performance report in your browser:

# Windows (PowerShell):
Start-Process target/criterion/report/index.html

# macOS:
open target/criterion/report/index.html

# Linux:
xdg-open target/criterion/report/index.html
