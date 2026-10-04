# LtHash Custom Modulo Benchmarking Suite (M = 2^16 - 15)

This repository isolates, tests, and benchmarks high-performance, branchless modular arithmetic for **LtHash** (Lattice-based Homomorphic Hash). It evaluates **1,024-lane 16-bit vector operations** under a prime modulus (M = 65,521, where M = 2^16 - 15) against native CPU hardware-wrapping arithmetic (`u16::wrapping_add` / `u16::wrapping_sub`).

The implementation operates strictly on native `u16` types without upcasting to 32-bit/64-bit integers. It includes per-lane input preconditioning and branchless modular adjustments to safely accept raw, unreduced `u16` inputs in [0, 65,535] without requiring division or conditional branching.

---

## Why

A recent paper, ["Two-Bit Lifting for Ternary SIS: Polynomial-Time Collision Attacks on LtHash"](https://eprint.iacr.org/2026/2083), demonstrates a polynomial-time collision attack against LtHash when using standard power-of-two moduli ($2^{16}$). The authors recommend replacing power-of-two moduli with prime fields to restore the intended Short Integer Solution (SIS) lattice hardness assumptions.

This repository provides a reference implementation and benchmark demonstrating that adopting the largest 16-bit Solinas prime ($p = 2^{16} - 15 = 65,521$)—the paper's recommended fix—can be implemented efficiently in Rust, operating at near-native speed compared to standard wrapping primitives.

## Key Features & Architecture

* **Branchless Execution:** Replaces `if` statements with bitwise selection masks (`mask_b`, `mask_sum`, `mask_underflow`) to eliminate branch misprediction penalties and enable AVX2/AVX-512 auto-vectorization.
* **Preconditioning for Raw Inputs:** Supports raw, unreduced `u16` elements (b in [0, 65,535]) directly within `add` and `sub` operations without requiring external reduction calls.
* **Granular Single-Lane API:** Decouples core logic into `#[inline(always)] pub fn custom_mod_add_u16_single` and `custom_mod_sub_u16_single` functions, allowing zero-overhead SIMD vector loop compilation alongside direct scalar testing.
* **Exhaustive Truth-Table Validation:** Includes a full 4.29-billion iteration sweep testing all 65,521 x 65,536 combinations of (a, b) against a 32-bit reference modulo model.

---

## Performance Overview

Microbenchmarks measured on 1,024-lane `[u16; 1024]` state vectors via **Criterion.rs**:

| Operation | Total Vector Time (1,024 lanes) | Per-Lane Time | Overhead vs. Hardware Baseline | Description |
| :--- | :--- | :--- | :--- | :--- |
| **hardware_wrapping_add_u16** | ~1.43 µs | ~1.39 ns | — (baseline) | Raw 16-bit vector addition (2^16 wrapping) |
| **custom_branchless_mod_add_u16** | ~1.64 µs | ~1.60 ns | +0.21 ns/lane (~14%) | Modular addition (M = 65,521) with preconditioning |
| **hardware_wrapping_sub_u16** | ~1.56 µs | ~1.52 ns | — (baseline) | Raw 16-bit vector subtraction (2^16 wrapping) |
| **custom_branchless_mod_sub_u16** | ~1.49 µs | ~1.45 ns | ~0% (margin of error) | Modular subtraction (M = 65,521) with preconditioning |

> **Key Takeaway:** Prime field reduction (mod 65,521) across 1,024 `u16` lanes—including preconditioning for raw inputs—adds only **~210 nanoseconds of total overhead** across the entire vector compared to raw hardware wrapping operations.

---

## Repository Layout

```text
lthash-custom-mod/
├── Cargo.toml
├── README.md
├── benches/
│   └── benchmarks.rs    # Criterion microbenchmarks comparing vector throughput
└── src/
    └── main.rs          # LtHashState ([u16; 1024]), single-lane helpers, and test suite
```

---

## Testing & Validation

The test suite covers state bounds, net-zero property, homomorphic independence, edge boundaries, and exhaustive truth-table coverage:

* **Strict Bounds:** Ensures accumulator output lanes satisfy 0 <= lane < 65,521.
* **Unreduced Preconditioning:** Confirms raw input elements (b >= 65,521) yield results identical to pre-reduced inputs.
* **Homomorphic Order Independence:** Verifies commutativity (A + B == B + A).
* **Net-Zero Inverse Property:** Verifies adding and then subtracting the same element returns the accumulator to zero.
* **Exhaustive Truth Table Sweep:** Validates every possible valid accumulator state (a in [0, 65,520]) against every possible unreduced input (b in [0, 65,535])—totalling 4,294,238,208 test pairs—against a 32-bit reference model.

### Running Tests

Standard test execution (fast unit tests):

```powershell
cargo test
```

Execute the complete **4.29-billion iteration exhaustive truth-table test** (marked with `#[ignore]` to keep daily test runs instant):

```powershell
cargo test --release -- --ignored
```

---

## Running Benchmarks

Execute Criterion microbenchmarks:

```powershell
cargo bench
```

To enable full SIMD auto-vectorization for your local CPU architecture (e.g., AVX2 / AVX-512 register packing):

**PowerShell (Windows):**
```powershell
$env:RUSTFLAGS="-C target-cpu=native"; cargo bench
```

**Bash / Zsh (Linux / macOS):**
```bash
RUSTFLAGS="-C target-cpu=native" cargo bench
```

---

## Results Inspection

After running `cargo bench`, view the interactive HTML performance report generated by Criterion:

**PowerShell (Windows):**
```powershell
Start-Process target/criterion/report/index.html
```

**macOS:**
```bash
open target/criterion/report/index.html
```

**Linux:**
```bash
xdg-open target/criterion/report/index.html
```
