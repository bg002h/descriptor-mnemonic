# v0.10 Phase 5 Review — `PolicyId::fingerprint()` API

**Reviewer:** opus reviewer agent
**Phase commit:** `d26f891 feat(v0.10-p5): add PolicyId::fingerprint() short-identifier API`
**Phase 4 baseline:** `165c2b8`
**Date:** 2026-04-29

---

## 1. Verdict

**CLEAN** — ship as-is.

This is exactly the small, purely additive API the plan called for. Method body is one obvious line, doctest example is well-chosen, two unit tests cover the spec invariant (first-4-byte prefix) and a deterministic-from-policy round-trip, and the deferral of CLI integration is well-justified with a properly-tiered FOLLOWUPS entry. Nothing to fix.

## 2. Scope reviewed

- `crates/md-codec/src/policy_id.rs` lines 94–120 (method + rustdoc + doctest)
- `crates/md-codec/src/policy_id.rs` lines 560–591 (2 new unit tests)
- `design/FOLLOWUPS.md` `cli-policy-id-fingerprint-flag` entry (lines 762–770)
- Spec source-of-truth: `design/SPEC_v0_10_per_at_N_paths.md` Q13 (line 36) and "What v0.10 ships" item 6 (line 58)
- Build + clippy + fmt + targeted test runs

## 3. Findings

1. **Method body is correct.** `fp.copy_from_slice(&self.0[0..4])` returning a fresh `[u8; 4]` is the canonical way; the spec language is "top 32 bits as a short identifier" (item 6, line 58) and "first 4 bytes" — which `self.0[0..4]` gives directly. `PolicyId` is `pub struct PolicyId([u8; 16])` (line 49) with `as_bytes()` returning `&self.0`, so `fingerprint()` is exactly the documented strict prefix `&id.as_bytes()[0..4]`. No endianness ambiguity arises: the underlying bytes are SHA-256 output bytes (line 186, `SHA-256(canonical_bytecode)[0..16]`), and the rest of the code (`Display`, `truncate`, `LowerHex`) all treat byte 0 as the high-order MSB. `fingerprint()` matches this convention.

2. **Rustdoc is accurate.** Calls out (a) BIP 32 parallel, (b) `0x{:08x}` natural rendering, (c) strict-prefix relationship to `as_bytes()`, (d) collision-resistance caveat (~32-bit, NOT a substitute for the 128-bit `PolicyIdWords`). The caveat is the right size — the v0.10 spec is explicit that the 12-word phrase is the canonical Tier-3 anchor and the fingerprint is a tools/CLI shortcut.

3. **Doctest passes** and is well-chosen: a 16-byte input with non-trivial high nibbles (`0xa1, 0xb2, 0xc3, 0xd4, ...`) so the assertion is meaningful (couldn't pass by accident if the wrong slice was returned).

4. **Unit test 1 (`policy_id_fingerprint_is_first_4_bytes`) does what its name says.** Tests both the literal prefix and the strict-prefix-of-`as_bytes()` invariant.

5. **Unit test 2 (`policy_id_fingerprint_deterministic_from_policy`) does what its name says.** Same `WalletPolicy` parsed twice yields the same fingerprint, and a structurally distinct policy yields a different fingerprint (with a sensible 1-in-2^32 collision-fluke comment).

6. **Canonical-valid template confirmed.** Test uses `wsh(pk(@0/**))` and `wsh(multi(2,@0/**,@1/**))` — both start at `@0` per BIP 388 placeholder rules. The implementer's note about switching from `@1` to `@0` is correct: placeholders MUST start at `@0`. Good catch.

7. **Build / clippy / fmt clean.** Verified locally:
   - `cargo build -p md-codec` → clean
   - `cargo fmt --check` → clean
   - `cargo clippy -p md-codec --all-targets -- -D warnings` → clean
   - Targeted: `cargo test -p md-codec --lib policy_id_fingerprint` → 2/2 pass
   - Doctest: `cargo test -p md-codec --doc PolicyId::fingerprint` → 1/1 pass
   - Workspace test count: 700 unit/integration + 8 doctests = 708 (plus the `--all-targets` integration suites — implementer reported 714/0; aggregate "+3 versus baseline" is consistent with 2 unit tests + 1 doctest).

8. **No spec drift.** Spec calls "top 32 bits"; impl returns first 4 bytes; docs render as `0x{:08x}` — all three views align. No spec touched in Phase 5; the v0.10 SPEC item 6 already binds the API signature.

9. **Pure additive.** No wire format change, no public-API removal, no behavior change for any existing call site. Risk surface is the size of the diff.

## 4. CLI deferral assessment

**Sufficient.** The FOLLOWUPS entry `cli-policy-id-fingerprint-flag` (lines 762–770) covers:

- **Surfaced** with date and phase pointer.
- **Where** with file path and current behavior reference (`cmd_encode`'s unconditional `Policy ID: {12 words}` print, line ~381).
- **What** — concrete proposal (CLI flag rendering as `0x{:08x}`).
- **Why deferred** — the substantive reason: `--fingerprint` is already the master-key fingerprint embedding flag in `md encode` (per BIP §"Fingerprints block"), and resolving the conflict requires either a CLI break (rename existing flag) or a name divergence from the API (`--policy-id-fingerprint`, `--short-id`) that warrants a small design pass with the user. Not Phase-5 scope.
- **Tier** — `v0.11 (or wont-fix if no end-user-facing CLI demand surfaces; library API is the load-bearing surface)`. Honest about the fact that the library API may be sufficient and the CLI may never need it.

The deferral preserves the v0.10 ship surface (library API ships, downstream tools can call it immediately) without forcing a CLI flag-rename design discussion into a small additive phase. Correct call.

## 5. Recommended action

**Mark Phase 5 reviewer-gate complete and proceed to Phase 6** (BIP draft + README + MIGRATION + CHANGELOG).

No inline fixes, no follow-up tickets, no additional FOLLOWUPS entries beyond the one the implementer already filed.
