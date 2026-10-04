# LtHash Custom Modulo Benchmarking Suite (M = 2^16 - 15)

> **Disclaimer:** This project and its performance optimizations were developed with assistance from AI (LLM collaboration) for code generation, benchmark structuring, and algebraic verification.

This repository isolates, tests, and benchmarks custom modular arithmetic for **LtHash** (Lattice-based Homomorphic Hash). It evaluates **1,024-lane 16-bit vector operations** under a custom prime modulus (M = 65,521, where M = 2^16 - 15) against native CPU hardware-wrapping arithmetic (`u16::wrapping_add` / `u16::wrapping_sub`).

The implementation operates strictly on native `u16` types without upcasting to 32-bit/64-bit integers, providing a drop-in reference for homomorphic set hashing in high-throughput systems (such as **Rusty Kaspa** node consensus, UTXO set commitments, and mempool synchronization).

---

## Performance Overview

Benchmarks measured on 1,024-lane `[u16; 1024]` vectors via **Criterion.rs**:

| Operation | Mean Time | Overhead vs. Hardware | Status |
| :--- | :--- | :--- | :--- |
| **wrapping_add_u16** | ~834.96 ns | Baseline | Standard 16-bit vector addition (2^16 wrap) |
| **custom_mod_add_u16** | ~905.88 ns | +70.92 ns (~8.5%) | Modular overflow check (mod 65,521) without division |
| **wrapping_sub_u16** | ~833.99 ns | Baseline | Standard 16-bit vector subtraction (2^16 wrap) |
| **custom_mod_sub_u16** | ~894.42 ns | +60.43 ns (~7.2%) | **~93% theoretical hardware speed** (mod 65,521) |
| **hash_element_40bytes** | ~39.11 µs | — | 40-byte input to 1,024-lane `u16` expansion harness |

> **Key Takeaway:** Performing custom prime field reduction (mod 65,521) across 1,024 `u16` lanes adds only **~60–70 nanoseconds total** over raw hardware addition—costing **less than 0.07 ns per lane**.

---

## Repository Layout
```text
lthash-custom-mod/
├── Cargo.toml
├── README.md
├── benches/
│   └── benchmarks.rs   # Criterion benchmark definitions (1,024-lane u16 comparisons)
└── src/
    └── main.rs         # Core LtHashState ([u16; 1024]), custom mod arithmetic, and test suite
```
---

## 1. Unit Testing & Algebraic Integrity

The test suite verifies core mathematical properties required for homomorphic hashing:
- **Strict Bounds:** Ensures all output lanes satisfy 0 <= lane < 65,521.
- **Boundary Overflow/Underflow:** Correctly handles 16-bit hardware integer wrap-around before modular reduction.
- **Homomorphic Order Independence:** Verifies commutativity (A + B == B + A).
- **Net-Zero Inverse Property:** Verifies (A + B) - B == A and multi-item set removals return to zero.

To run the unit test suite:

```powershell
cargo test
```
---

## 2. Running Microbenchmarks

To execute the Criterion microbenchmarks using standard target settings:

```powershell
cargo bench
```
To run benchmarks with maximum native SIMD vectorization (e.g., AVX2 / AVX-512 register packing):

# Windows (PowerShell):
```powershell
$env:RUSTFLAGS="-C target-cpu=native"; cargo bench
```

# Linux / macOS (Bash/Zsh):
```bash
RUSTFLAGS="-C target-cpu=native" cargo bench
```

### What the Benchmarks Measure:

1. **Native vs. Custom Addition:** Compares raw 1,024-lane `u16` `wrapping_add` against `custom_mod_add`.
2. **Native vs. Custom Subtraction:** Compares raw 1,024-lane `u16` `wrapping_sub` against `custom_mod_sub`.
3. **Element Expansion Overhead:** Measures 40-byte input payload expansion into a full 1,024-lane `u16` state vector (`hash_element_40bytes`).

---

## 3. Results Inspection

After running `cargo bench`, open the interactive Criterion HTML performance report in your browser:

# Windows (PowerShell):
```powershell
Start-Process target/criterion/report/index.html
```

# macOS:
```bash
open target/criterion/report/index.html
```

# Linux:
```bash
xdg-open target/criterion/report/index.html
```
