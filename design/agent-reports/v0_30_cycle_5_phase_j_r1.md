# Phase J — code review r1 (2026-05-10)

**Working tree:** dirty at HEAD `d04ec30`; not yet committed.

**Scope:** md-codec v0.30 Cycle 5 Phase J — final release commit. Version bump `0.19.0` → `0.30.0`; crate-level doc rewrite (lib.rs:8–13); Descriptor module-doc rewrite (encode.rs:11) + `key_index_width` doc v0.18-brand drop (encode.rs:35); tag.rs operator count fix (35 → 36); CHANGELOG.md new entry citing all 8 phase commits; 2 FOLLOWUPs resolved. Plus dep-spec sync in md-cli (md-codec dep `0.19.0` → `0.30.0`; necessary for workspace resolve; md-cli's own version unchanged at 0.4.3).

**Files reviewed:** `CHANGELOG.md`, `Cargo.lock`, `crates/md-cli/Cargo.toml`, `crates/md-codec/Cargo.toml`, `crates/md-codec/src/{encode, lib, tag}.rs`, `design/FOLLOWUPS.md`.

---

## Critical (block ship)

None.

## Important (must fix before ship)

None.

## Low (fixed inline post-r1)

### L-1 — Pre-existing `spec v0.13` references at encode.rs:44 + encode.rs:60 (FIXED INLINE)

- **Where:** `crates/md-codec/src/encode.rs:44` (`is_wallet_policy` doc-comment) and `crates/md-codec/src/encode.rs:60` (`encode_payload` doc-comment).
- **What:** Both cited "spec v0.13 §X.Y" in active prose. Pre-existing (predates Phase J), outside Phase J's named scope. But this is the FINAL phase of the v0.30 release — there is no "next doc sweep" — so leaving them in active prose would ship stale brands.
- **Fix (applied):** rewrote to cite "SPEC §3.3" and "SPEC §6.1" without the `v0.13` brand (the SPEC document path is now `design/SPEC_v0_30_wire_format.md`; the section numbers are stable across versions).
- **Disposition:** consistent with architect r1 I-1's intent (sweep stale version brands in encode.rs); the architect named `v0.18` explicitly, but the same rule applies to any pre-v0.30 brand citation in active prose.

## Nit

None.

---

## Correctness checks (all passed)

1. **lib.rs crate-level doc.** Lines 8–13 match the architect-r1-revised target verbatim — "5-bit single-payload header (4-bit version=4 + `divergent_paths` flag)" and "decoder auto-dispatch between single and chunked payloads via the first 5-bit symbol's bit 0" are present as distinct clauses. ✓
2. **encode.rs lines 11 + 35.** Line 11: "v0.30 wire payload". Line 35: no "v0.18 reserved" phrase; new prose describes v0.30 behavior directly. ✓
3. **tag.rs operator count.** Line 3: "36 operators in primary 6-bit space (0x00..=0x23)". 36 verified by enum-variant count. ✓
4. **Cargo.toml version bump.** md-codec at "0.30.0"; md-cli unchanged at "0.4.3". ✓
5. **md-cli dep-spec sync.** `md-codec = { path = "../md-codec", version = "0.30.0" }`; md-cli version unchanged. ✓
6. **Cargo.lock.** Only md-codec version diff visible; no unrelated drift. ✓
7. **CHANGELOG.md entry.** Date 2026-05-10; all 8 commit hashes (Phase A/B/C/E/F/G/H/I) cited and verified via `git log --oneline`; Phase D absent (correct — SW2 reverted per IMPLEMENTATION_PLAN_v0_30.md §3); Removed section names Tag::TrUnspendable + 4 error variants; Documentation section cites Phase I (d04ec30); Migration section present; Keep-a-Changelog format consistent. ✓
8. **FOLLOWUPs resolved.** `v0.30-phase-b-r1-nit-1` and `v0.30-phase-i-tag-rs-operator-count-off-by-one` both Status: resolved with Phase J references. ✓
9. **Workspace test/clippy/build.** `cargo test --workspace --all-features`: 451/0/0. `cargo clippy --workspace --all-features -- -D warnings`: clean. `cargo build --workspace --release`: clean. ✓
10. **v0.x purge.** `grep -E 'v0\.(11|13|18)' crates/md-codec/src/encode.rs crates/md-codec/src/lib.rs` → 0 occurrences post-L-1 fix. ✓
11. **Commit/tag/push sequence.** Working tree dirty; implementer did NOT commit, tag, or push. ✓

---

## Verdict

**Ship.** 0C/0I/1L (fixed inline)/0N. Phase J is ready for atomic commit + annotated tag `md-codec-v0.30.0` + push.

Post-Phase-J cycle exit state: 2 open FOLLOWUPs remain, both v1+ tier:
- `v0.30-phase-g-operator-context-violation-unwired` (stub-only; no natural decoder fire site)
- `repo-hygiene-stale-file-location-doc-artifacts` (pre-existing artifacts at derive.rs:1 + address_derivation.rs:1)
