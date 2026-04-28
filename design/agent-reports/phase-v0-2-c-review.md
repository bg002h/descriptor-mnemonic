# Phase C review — Opus 4.7

**Status:** APPROVE_WITH_FOLLOWUPS
**Subject:** commit `3aabcf6` (`p1-bch-4-error-correction`)
**Reviewer model:** Opus 4.7 via general-purpose subagent
**Stage:** combined spec compliance + algorithmic correctness + code quality
**Role:** reviewer

## Findings

### Spec deviations

(none) — every Phase C scope item verified.

### Algorithmic correctness

**Sound.** Cross-checked each layer:

- **Field rep**: `GF(32) = GF(2)[α]/(α⁵+α³+1)` matches BIP 93 / bech32. `GF(1024) = GF(32)[ζ]/(ζ²-ζ-1)` correctly implements `(a+bζ)(c+dζ) = (ac+bd) + (ad+bc+bd)ζ`. Test `zeta_is_primitive_cube_root_of_unity` directly verifies `ζ² = ζ+1` and `ζ³ = 1`.
- **Primitive elements + orders**: `BETA = G·ζ` (order 93), `GAMMA = E + X·ζ` (order 1023). `gamma_has_order_1023_long` verifies via prime-factor test at the divisors of 1023 (`{3, 11, 31}`).
- **Generator-polynomial roots**: `generator_polynomial_evaluates_to_zero_at_specified_roots` reconstructs `g(x)` from `GEN_*[0]` and verifies it vanishes at every BIP-93-listed root. The `{β^77..β^84}` and `{γ^1019..γ^1026}` 8-consecutive windows are subsets, so `j_start = 77 / 1019` is correct.
- **Berlekamp-Massey**: textbook formulation; discrepancy correctly sums `S_k + Σ Λ_i·S_{k-i}`; underflow guard `s_idx < n` after `wrapping_sub` correctly handles the boundary; length-doubling condition `2*l ≤ k` is right.
- **Chien search**: bounded to `0..data_with_checksum_len` so phantom roots in HRP-prefix region cannot be reported. Mismatched-root-count rejection sound.
- **Shifted Forney**: `shift = j_start - 1`; `x_k_inv.pow(shift) = X_k^{1 - j_start}`. Algebra correct for both windows. GF(32)-magnitude post-condition (`mag.hi != 0` ⇒ reject) is right.
- **Defensive verify**: re-runs `bch_verify_*` after applying corrections — guards the >4-error edge case where BM produces a degree-≤4 Λ with valid roots.
- **Performance**: BM `O(t·n)`, Chien `O(n·L)`, Forney `O(t²)` — microsecond-class. No accidental cubic.

**No bugs found.**

### Quality blockers

(none) — no `unsafe`, no panics in hot paths.

### Quality important

(none)

### Quality nits (4)

All in `crates/wdm-codec/src/encoding/bch_decode.rs`, all stylistic/micro-opt:

- **N-1** (`:365`): `lam.last().unwrap()` → `lam.last().is_some_and(|x| x.is_zero())` reads cleaner. Pure style.
- **N-2** (`:329`): `k.wrapping_sub(i)` + `s_idx < n` guard is correct but subtle; `if i > k { continue }` early-out is more obvious.
- **N-3** (`:692-707`): test module re-implements `polymod_run` locally. A `pub(super) use super::polymod_run` would be cheaper.
- **N-4** (`:292`): `compute_syndromes` allocates a `Vec<u8>` of length 13/15 each call. Could be `[u8; 15]` stack-allocated.

## Disposition

| Finding | Action |
|---|---|
| Spec deviations | none — clean scope |
| Algorithmic correctness | sound; explicitly approved |
| All 4 nits (N-1..N-4) | Filed as cluster `phase-c-bch-decode-style-cleanups` (v0.2-nice-to-have) — bundle as a single sweep before v0.2.0 release if any future touch of `bch_decode.rs` happens; otherwise carry to v0.3. |

## Verdict

APPROVE_WITH_FOLLOWUPS — Phase C clear; "an unusually clean port."
