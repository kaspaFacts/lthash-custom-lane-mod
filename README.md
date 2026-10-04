# LtHash Custom Modulo Benchmarking Suite (M = 2^16 - 15)

INCOMPLETE: Publishing as public for sharing ONLY, testing is NOT complete or exhaustive.

This repository isolates, tests, and benchmarks custom modular arithmetic for **LtHash** (Lattice-based Homomorphic Hash). It evaluates **1,024-lane 16-bit vector operations** under a custom prime modulus (M = 65,521, where M = 2^16 - 15) against native CPU hardware-wrapping arithmetic (`u16::wrapping_add` / `u16::wrapping_sub`).

The implementation operates strictly on native `u16` types without upcasting to 32-bit/64-bit integers, providing drop-in modular alternatives to u16::wrapping_add and u16::wrapping_sub, allowing callers to replace 2¹⁶ wrapping arithmetic with arithmetic modulo a custom 16-bit modulus.

---

## Performance Overview

Benchmarks measured on 1,024-lane `[u16; 1024]` vectors via **Criterion.rs**:

| Operation | Overhead vs. Hardware Baseline | Description |  
| :--- | :--- | :--- |  
| **wrapping_add_u16** | — (baseline) | Standard 16-bit vector addition (2^16 wrap) |  
| **custom_mod_add_u16** | Single-digit % overhead | Modular reduction (mod 65,521) without division |  
| **wrapping_sub_u16** | — (baseline) | Standard 16-bit vector subtraction (2^16 wrap) |  
| **custom_mod_sub_u16** | Single-digit % overhead | Modular subtraction (mod 65,521) without division |  
| **hash_element_40bytes** | — | 40-byte input to 1,024-lane `u16` expansion harness |  
  
> **Key Takeaway:** On a typical x86-64 build (`cargo bench`, default codegen), custom prime  
> field reduction (mod 65,521) across 1,024 `u16` lanes costs only ~7–9% more than raw  
> wrapping arithmetic — a few tens of nanoseconds total on the benchmark machine.  
> Exact figures depend on CPU, compiler version, and `RUSTFLAGS`; run `cargo bench`  
> to measure on your hardware.

---

## Repository Layout
```text
lthash-custom-mod/
├── Cargo.toml
├── README.md
├── benches/
│   └── benchmarks.rs    # Criterion benchmark definitions (1,024-lane u16 comparisons)
└── src/
    └── main.rs          # Core LtHashState ([u16; 1024]), custom mod arithmetic, and test suite
```
---

### Known Limitations & Test Gaps

**Precondition: inputs must be fully reduced.**
`LtHashState::add` and `LtHashState::sub` are only correct when every input lane is already in `[0, MODULUS_16 - 1]`. This invariant is currently guaranteed only because `hash_element` reduces each lane with `% MODULUS_16`.
  
- `add` performs at most one conditional subtraction of `MODULUS_16`. If an input lane is ≥ 65,521 (unreduced), the result can remain out of bounds (e.g. `a + b` near `u16::MAX` wraps and a single reduction is insufficient).
- `sub` similarly assumes `b < MODULUS_16`; an unreduced `b` yields an incorrect residue.  
- Callers constructing `LtHashState` manually (i.e., writing `lanes` directly) must reduce inputs themselves — the API does not enforce this.  
  
**Current test suite gaps:**  
  
- `test_unreduced_input_boundaries` is mislabeled — it uses `MODULUS_16 - 1`,  
  which is a fully *reduced* value, so the unreduced-input failure above is  
  never exercised.  
- Missing edge cases worth adding:  
  - `(M - 1) + 1 == 0` — result landing exactly on zero after reduction  
  - `sum == MODULUS_16` exactly — the `>= MODULUS_16` boundary in `add`  
  - `a == b` in `sub` → `0`  
  - `state.sub(&state)` → all-zero state  
  - Additive identity: `state.add(&LtHashState::new())` is a no-op  
  - Associativity: `(a + b) + c == a + (b + c)`  
- No exhaustive or property-based testing. Since lanes are `u16`, an exhaustive  
  sweep of all `u16 × u16` input pairs against a `u32`-upcasted reference  
  (`(a as u32 + b as u32) % MODULUS_16 as u32`) is feasible and would fully  
  verify `add`/`sub` over the reduced domain.

The current test suite verifies:

- **Strict Bounds:** Ensures all output lanes satisfy `0 <= lane < 65,521`.
- **Boundary Overflow/Underflow:** Tests 16-bit arithmetic wrap-around and modular subtraction underflow.
- **Homomorphic Order Independence:** Verifies commutativity (`A + B == B + A`).
- **Net-Zero Inverse Property:** Verifies adding and then removing the same element returns the accumulator to zero.

To run the unit test suite:

```powershell
cargo test

```
---

## Running Microbenchmarks

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

## Results Inspection

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
