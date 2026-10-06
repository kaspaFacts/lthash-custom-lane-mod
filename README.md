# LtHash Custom Modulo Benchmarking Suite (M = 2^16 - 15)

This repository isolates, tests, and benchmarks high-performance, branchless modular arithmetic and Solinas prime reduction for **LtHash** (Lattice-based Homomorphic Hash). It evaluates **1,024-lane vector operations** under a prime modulus ($M = 65,521$, where $M = 2^{16} - 15$) against native CPU hardware-wrapping arithmetic (`u16::wrapping_add` / `u16::wrapping_sub`) and hardware division modulo (`% 65,521`).

The implementation provides zero-overhead $u32 \to u16$ Solinas field reduction alongside 16-bit lane addition and subtraction, operating without conditional branching or hardware division instructions.

---

## Why

A recent paper, ["Two-Bit Lifting for Ternary SIS: Polynomial-Time Collision Attacks on LtHash"](https://eprint.iacr.org/2026/2083), demonstrates a polynomial-time collision attack against LtHash when using standard power-of-two moduli ($2^{16}$). The authors recommend replacing power-of-two moduli with prime fields to restore the intended Short Integer Solution (SIS) lattice hardness assumptions.

### Restoring Randomness & Ingesting 32-Bit Inputs

When ingesting higher-width data streams (such as 32-bit hash outputs), computing `x % 65,521` using standard hardware division introduces severe performance bottlenecks because division instructions take dozens of clock cycles and stall CPU SIMD pipelines.

This suite extends the prime field implementation with **branchless Solinas reduction** ($u32 \to u16$). By exploiting the prime structure identity $2^{16} \equiv 15 \pmod{65,521}$, raw 32-bit input elements are compressed into valid 16-bit field elements ($[0, 65,520]$) in just a few clock cycles. This eliminates sub-field algebraic biases, restores uniform randomness across state lanes, and enables full auto-vectorization across 1024-lane vectors.

---

## Key Features & Architecture

- **Two-Pass Solinas Folding ($u32 \to u16$):** Decomposes 32-bit values into $16$-bit halves via $x \equiv (x_{\text{high}} \times 15) + x_{\text{low}} \pmod M$, compressing any $u32$ element into $[0, 65,520]$ in two fast bitwise passes without hardware division.
- **Branchless Execution:** Replaces `if` statements with bitwise selection masks (`mask_b`, `mask_sum`, `mask_underflow`, `mask`) to eliminate branch misprediction penalties and allow AVX2 / AVX-512 vectorization.
- **Preconditioning & Decoupled Pipeline:**
    - `compress_u32_to_u16`: Reduces raw 32-bit inputs into $Z_M$.
    - `custom_mod_add_u16_single` / `custom_mod_sub_u16_single`: Core modular arithmetic operations accepting both pre-reduced elements and raw unreduced 16-bit inputs.
    - `add_u32` / `sub_u32`: Vector methods combining fast compression and state vector updates.
- **Exhaustive Truth-Table Validation:** Includes both $u16$ truth-table coverage ($4.29$ billion combinations) and an exhaustive sweep across all $4,294,967,296$ possible 32-bit integers ($0 \dots 2^{32}-1$).

---

## Performance Overview

Microbenchmarks measured on 1,024-lane state vectors via **Criterion.rs**:

| Operation | Total Vector Time (1,024 lanes) | Per-Lane Time | Description |
| :--- | :--- | :--- | :--- |
| **hardware_modulo_div_u32** | ~1.25 µs | ~1.22 ns | Standard hardware modulo division (`x % 65,521`) |
| **branchless_solinas_compress_u32** | ~1.47 µs | ~1.44 ns | Solinas compression mapping $u32 \to Z_{65,521}$ |
| **hardware_wrapping_add_u16** | ~1.45 µs | ~1.42 ns | Raw 16-bit vector addition ($2^{16}$ wrapping baseline) |
| **custom_branchless_mod_add_u16** | ~1.58 µs | ~1.54 ns | Modular addition ($M = 65,521$) with preconditioning |
| **hardware_wrapping_sub_u16** | ~1.44 µs | ~1.41 ns | Raw 16-bit vector subtraction ($2^{16}$ wrapping baseline) |
| **custom_branchless_mod_sub_u16** | ~1.51 µs | ~1.47 ns | Modular subtraction ($M = 65,521$) with preconditioning |
| **add_u32_vector_1024_lanes** | ~2.49 µs | ~2.43 ns | Full pipeline: 32-bit Solinas reduction + field addition |

> **Key Takeaway:** Custom branchless modular addition/subtraction in $Z_{65,521}$ operates within **~0.12 ns per lane** of raw $u16$ hardware wrapping arithmetic. End-to-end 32-bit vector ingestion and accumulation across 1,024 state lanes completes in under **2.5 µs** (~2.43 ns per lane).

---

## Repository Layout

```text
lthash-custom-mod/
├── Cargo.toml
├── README.md
├── benches/
│   └── benchmarks.rs    # Criterion benchmarks (u32 division vs. Solinas, u16 wrapping vs. mod add/sub)
└── src/
    └── main.rs          # LtHashState ([u16; 1024]), Solinas compression, single-lane arithmetic, and tests
```

---

## Testing & Validation

The test suite covers state bounds, net-zero property, homomorphic independence, edge boundaries, $u32$ edge cases, and exhaustive correctness sweeps:

- **Strict Bounds:** Ensures accumulator output lanes satisfy $0 \le \text{lane} < 65,521$.
- **Unreduced Preconditioning:** Confirms raw input elements ($b \ge 65,521$) yield results identical to pre-reduced inputs.
- **Homomorphic Order Independence:** Verifies commutativity ($A + B == B + A$).
- **Net-Zero Inverse Property:** Verifies adding and then subtracting the same 16-bit or 32-bit element returns the accumulator to zero.
- **U32 Edge Cases:** Validates zero, field boundary conditions ($65,520$, $65,521$, $65,522$), fold capacity peaks, and boundary inputs (`u32::MAX`).
- **Exhaustive Sweeps:** Validates all $4.29$ billion combinations of single-lane additions/subtractions and all $2^{32}$ possible $u32$ integers against standard modulo arithmetic.

### Running Tests

Standard test execution (fast unit tests):

```powershell
cargo test
```

Execute the full **exhaustive sweeps** (marked with `#[ignore]` to keep standard development test runs instant):

```powershell
cargo test --release -- --ignored
```

> **Note:** Always run ignored tests in `--release` mode. LLVM SIMD auto-vectorization completes the 4.29 billion $u32$ iteration sweep in **under 1 second**.

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