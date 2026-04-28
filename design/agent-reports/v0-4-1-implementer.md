# v0.4.1 Implementer Report

**Date:** 2026-04-27
**Agent:** Claude Sonnet 4.6
**Branch:** release/v0.4.1
**Base commit:** f4b7bbf (v0.4.0 release)

---

## Status: COMPLETE — all gates green, pushed to origin

---

## Commit SHAs

| Commit | Description |
|--------|-------------|
| `270bf57` | release(v0.4.1): BIP doc cleanup + BCH known-vector repin |
| `6c98c37` | followup: update FOLLOWUPS.md with release commit SHA 270bf57 |

Release commit: `270bf57b483e07afc7670b1b69aa963c38935db0`

---

## gen_vectors --verify results

```
gen_vectors: PASS — committed file matches regenerated schema-1 vectors (10 positive, 30 negative)
gen_vectors: PASS — committed file matches regenerated schema-2 vectors (22 positive, 43 negative)
```

Both vector file SHAs UNCHANGED from v0.4.0:
- v0.1.json: `bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26` UNCHANGED
- v0.2.json: `caddad36ecc3893e3aae87a6bb57ff1928ed9d8b8710d05a78a6501dbd1e5770` UNCHANGED

Family-stable promise validated — first v0.4.x patch with byte-stable SHAs.

---

## Pinned checksum byte arrays (audit trail)

Computed by `/tmp/compute_bch_md_pins.py` — independent Python BIP 93 `ms32_polymod` reference.

**HRP expansion:** `hrp_expand("md") = [3, 3, 0, 13, 4]` (matches spec + Rust test)

**Regular code** (HRP `"md"`, data `[0, 1, 2, 3, 4, 5, 6, 7]`):
```
[25, 14, 21, 4, 26, 20, 18, 15, 5, 15, 23, 30, 15]
```
Length: 13. Self-verify: True.

**Long code** (HRP `"md"`, data `[0, 1, 2, ..., 15]`):
```
[23, 8, 11, 10, 1, 2, 13, 8, 29, 0, 17, 11, 14, 25, 11]
```
Length: 15. Self-verify: True.

Both pinned as `assert_eq!(actual, [Xu8, ...])` in `bch_known_vector_regular` and `bch_known_vector_long`. Round-trip `assert!` preserved as defense in depth.

---

## Test count

**609 passing, 0 ignored, 0 failed** — identical to v0.4.0 baseline. No new tests; just stronger assertions in 2 existing tests.

---

## Gate outputs

```
cargo build --workspace --all-targets: Finished `dev` profile — OK
cargo test -p md-codec: 609 passed; 0 failed; 0 ignored
cargo clippy --workspace --all-targets -- -D warnings: Finished (no warnings)
cargo fmt --check: clean (no diff)
```

Note: One minor type-annotation fix needed during implementation — `assert_eq!(actual, &[...])` fails because `actual` is `[u8; N]` (fixed array) not a slice. Fixed to `assert_eq!(actual, [Nu8, ...])` with explicit `u8` type suffix. Also one rustfmt line-length wrap applied to the long-code assert.

---

## FOLLOWUPS items closed

| ID | Status |
|----|--------|
| `p10-bip-header-status-string` | resolved 270bf57 |
| `bip-preliminary-hrp-disclaimer-tension` | resolved 270bf57 |
| `bch-known-vector-repin-with-md-hrp` | resolved 270bf57 |

All three moved from Open items to Resolved items in `design/FOLLOWUPS.md`.

---

## BIP changes summary

1. `Status:` preamble: `Pre-Draft, AI only, not yet human reviewed` → `Pre-Draft, AI + reference implementation, awaiting human review`
2. HRP disclaimer: `preliminary and subject to change before this BIP is finalized` → `subject to formal SLIP-0173 registration (see FOLLOWUPS entry slip-0173-register-md-hrp)`. Added cross-reference to §"Why a new HRP?" collision-vet claim so both statements are now consistent.

---

## Concerns

None. The patch is entirely non-breaking: doc fixes + stronger test assertions + version bump. Wire format, vector SHAs, and test count are all stable.
