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

## Known Issues & Methodology Caveats (to be addressed)

### 1. `hash_element` inadvertently preconditions all inputs — masking a correctness gap

`hash_element` reduces every lane with `% MODULUS_16` (src/main.rs:51), so every test and benchmark input is guaranteed to be in `[0, 65520]`. This differs from a real LtHash expansion, which emits raw `u16` lanes over the full `[0, 65535]` range (mod 2^16 wrap only).

**Consequence:** `LtHashState::add`/`sub` are only correct when both operands are fully reduced; an unreduced lane ≥ 65,521 produces a silently wrong result (`add` performs at most one conditional subtraction; `sub` assumes `b < M`). Because `hash_element` is the sole input generator and `lanes` is `pub`, the test suite can never exercise — or fail on — the unreduced-input path. Roughly 15/65,536 of real expansion outputs would hit this case.

### 2. Current benchmarks are apples-to-oranges at the scheme level

The benchmark compares `wrapping_add/sub` (mod 2^16, no reduction step) against `custom_mod_add/sub` (mod 65,521, extra per-lane reduction) — but both arms are fed the *same preconditioned* inputs produced for the custom scheme. The delta correctly prices the extra reduction step, but the comparison does not represent "real LtHash pipeline vs. custom-mod pipeline":  

- The wrapping arms model a scheme whose expansion would be **division-free** (no `% MODULUS_16`), while `hash_element_40bytes` measures an expansion that pays a per-lane division — overstating expansion cost for the custom scheme's harness and omitting expansion entirely from the add/sub arms.
- Absolute times include a symmetric 2 KB `black_box` copy per iteration, so only the *relative delta* is meaningful.  

### 3. Test-suite gaps
  
- `test_unreduced_input_boundaries` is mislabeled — it writes `MODULUS_16 - 1` (a reduced value) and never exercises unreduced input.  
- Missing: exact-boundary cases (`sum == MODULUS_16`, `(M-1)+1 == 0`), `a == b` subtraction, additive identity, associativity, and an exhaustive `u16 × u16` sweep against a `u32` reference (`(a + b) % 65521`), which is feasible and would fully verify the reduced domain.

### Suggested fix path

1. Add `hash_element_wrapping(data) -> [u16; LANES]` emitting raw mod-2^16 lanes (no `%`), modeling a real LtHash expansion. Returning a bare array instead of `LtHashState` uses the type system to prevent feeding raw lanes into `add`.
2. Keep `hash_element` (preconditioned) for the custom-mod scheme.
3. Add pipeline-level benchmark arms — `expand + add` inside `b.iter` — for *both* schemes (`Pipeline_Expand_Add` / `Pipeline_Expand_Sub`), so the comparison reflects end-to-end scheme cost rather than isolated arithmetic.
4. Add a boundary test feeding unreduced lanes into `add`/`sub` — either documenting the reduced-input precondition as a contract, or driving a fix (e.g., per-lane normalization of `b`).
5. Re-benchmark under both `cargo bench` and `RUSTFLAGS="-C target-cpu=native" cargo bench`; record CPU model and rustc version. Replace fixed nanosecond figures with relative overhead.  
6. Move `criterion` from `[dependencies]` to `[dev-dependencies]`.

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
