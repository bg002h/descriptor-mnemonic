# v0.2 residual reviewer-nits audit — Opus 4.7

**Status:** DONE
**Top-line:** AUDIT CLEAN — 5 silent slips found; all 5 fixed inline as trivial rustdoc / comment edits. No FOLLOWUPS entry filed.
**Reviewer model:** Opus 4.7 (audit-pass subagent)
**Worktree:** `agent-aeab9861fb17a5104`
**Scope:** post-v0.2.0-release residual-nit audit

## Summary

Re-read all 9 v0.2 reviewer reports + 9 implementer self-reports. Built a finding-by-finding inventory, classified each as APPLIED-INLINE / FILED / SILENT-NO-ACTION / SLIPPED, then verified each SLIPPED candidate against current `main` HEAD code state.

5 nits genuinely slipped (the reviewer flagged them, marked them "acknowledged / cosmetic / no action / folded", but no controller fix-up commit or Phase G polish sweep ever addressed them). All 5 are pure-rustdoc / pure-comment edits that meet the "trivial" bar. Applied as a single commit; no FOLLOWUPS.md entry needed because nothing remains pending.

Wire format byte-stable: both `gen_vectors --verify v0.1.json` and `--verify v0.2.json` PASS unchanged.

## Total findings inventoried

| Report | Quality blockers | Quality important | Quality nits | Implementer self-flagged |
|---|---:|---:|---:|---:|
| Phase A bucket A | 0 | 0 | 5 (N-1..N-5) | 0 |
| Phase A bucket B | 0 | 2 (Q-1, Q-2) | 4 (N-1..N-4) | 0 |
| Phase B bucket A | 0 | 0 | 3 (N-1..N-3) + 1 memory | 0 |
| Phase B bucket B | 0 | 1 (Q-1) | 3 (N-1..N-3) | 0 |
| Phase B bucket C | 0 | 0 | 4 (N-1..N-4) | 0 |
| Phase C | 0 | 0 | 4 (N-1..N-4) | 0 |
| Phase D | 0 | 0 | 3 (N-1..N-3) | 3 (forward-declared FOLLOWUPS) |
| Phase E | 0 | 1 (N-1) | 3 (N-2..N-4) | 2 (forward-declared FOLLOWUPS) |
| Phase F | 0 | 0 | 3 | 0 |
| **Total** | **0** | **4** | **33** | **5** |

## Classification table

### APPLIED-INLINE (controller fixup or Phase G polish sweep)

| Finding | File:line | Verified by |
|---|---|---|
| A-bucket-A N-1 | `chunking.rs:345` | exhaustive `match` confirmed |
| A-bucket-B Q-2 | `policy.rs:243` (decoded_shared_path field) | rustdoc Eq note present |
| B-bucket-A N-1 | `encoding.rs:564-565` | "data part" disambiguation present |
| B-bucket-B N-1 | `policy.rs:1213-1222` | `bytes != baseline` assert present |
| B-bucket-C N-1 | `bin/wdm/json.rs` | `From<&BchCode>` confirmed |
| B-bucket-C N-2 | `bin/wdm/json.rs` | filename = `json.rs` (not `wdm_json.rs`) |
| C N-1..N-4 | `bch_decode.rs` (cluster) | Phase G polish sweep `0ef70f9`+`511e7a9` |
| D N-1 | `decode.rs::tag_to_bip388_name` | Phase G polish sweep `0ef70f9` |
| E N-1 | `policy.rs::policy.rs:410` (`u8::try_from`) | Phase G polish sweep `0ef70f9` |

### FILED (entry exists in FOLLOWUPS.md, open or resolved)

| Finding | FOLLOWUPS short-id | Status |
|---|---|---|
| A-bucket-A forward-look | `p4-with-chunking-mode-builder` | open (v0.2-nice-to-have) |
| A-bucket-A N-2..N-5 | `p4-chunking-mode-stale-test-names` | resolved `0ef70f9` |
| A-bucket-B Q-2 (MIGRATION) | `wallet-policy-eq-migration-note` | resolved `548dc10` |
| B-bucket-A memory | `decoded-string-data-memory-microopt` | open (v0.3) |
| B-bucket-B Q-1 | `phase-b-encode-signature-and-copy-migration-note` | resolved `548dc10` |
| B-bucket-C N-3 | `cli-json-debug-formatted-enum-strings` | open (v1+) |
| C N-1..N-4 | `phase-c-bch-decode-style-cleanups` | resolved `0ef70f9` |
| D self-flagged | `phase-d-tap-leaf-wrapper-subset-clarification` | open (v0.3) |
| D self-flagged | `phase-d-taproot-corpus-fixtures` | resolved `5348b12` |
| D self-flagged | `phase-d-tap-miniscript-type-check-parity` | open (v0.3) |
| D N-1 | `phase-d-tap-decode-error-naming-parity` | resolved `0ef70f9` |
| E N-1 | `phase-e-encoder-count-cast-hardening` | resolved `0ef70f9` |
| E self-flagged | `phase-e-cli-fingerprint-flag` | open (v0.2-nice-to-have) |
| E self-flagged | `phase-e-fingerprints-behavioral-break-migration-note` | resolved `548dc10` |

### SILENT-NO-ACTION (defensible rationale; not a slip)

| Finding | Rationale (cited from review) |
|---|---|
| A-bucket-B N-3 (long test name) | "Acceptable" — documents dual assertion |
| A-bucket-B N-4 (eager clone) | `DerivationPath` clone is cheap; explicit "leave as-is" |
| B-bucket-A N-2 (q↔p flip prompt) | Belt-and-braces guard makes single-error case correct |
| B-bucket-A N-3 (Phase F note) | Positive forward-look note |
| B-bucket-B N-3 (regression-guard) | Positive note |
| B-bucket-C N-4 (Phase E forward-look) | Positive note |
| D N-2 (TapLeafSubsetViolation vs PolicyScopeViolation) | Reviewer wrote "Cosmetic; doesn't affect rejection correctness" |
| D N-3 (TapTree rejection duplication) | Reviewer wrote "deduplicating would obscure both call sites" |
| E N-2 (FingerprintsCountMismatch listed twice in error.rs) | Accurate — really fires from both decode-stage AND encode-side |
| E N-3 (pre-existing `unwrap` in `key_count`) | Pre-existing, well-guarded by `peek().is_some_and(...)`; not introduced by Phase E |
| F N-2 (29 unwrap/panic sites in vectors.rs) | All in vector-build-time paths; supposed to panic on codec bug |
| F N-3 (`_header` / `_fragment` underscore-named) | Future-debug placeholders |

### SLIPPED (5 items — all trivially fixed inline this audit)

| # | Finding | Original review | Disposition |
|---|---|---|---|
| 1 | `policy.rs:620` — long-line reflow (109 chars vs ~75 surrounding) | A-bucket-A N-5 ("Folded") — never actually addressed by the cluster sweep, which was test-name-focused | trivial-fix-applied |
| 2 | `policy.rs:660, 1140` — Unicode `→` ↔ ASCII `->` consistency in rustdoc | A-bucket-B N-2 ("Cosmetic.") — silent ack, no action | trivial-fix-applied (2 sites) |
| 3 | `bin/wdm/main.rs::cmd_bytecode` — missing comment explaining intentional asymmetry vs `cmd_encode` | B-bucket-B N-2 ("Not filed (too small)") | trivial-fix-applied |
| 4 | `policy.rs:644-651` — `WdmBackup.fingerprints` rustdoc says "recovered from a fingerprints block" but encode side mirrors caller-supplied options | E N-4 ("Cosmetic only") | trivial-fix-applied |
| 5 | `vectors.rs:504` — `build_test_vectors()` doc didn't make alias chain to `_v1` visible | F nit #1 ("Minor.") — explicitly "no FOLLOWUPS needed" by F reviewer but was a real residual | trivial-fix-applied |

## Verification work for SLIPPED items

For each candidate I ran the file:line check against current `main` HEAD:

1. **A-bucket-A N-5**: original "policy.rs:461" cited a rustdoc line that was 120 chars at review time. That same rustdoc anchor (`crate::EncodeOptions::chunking_mode`) now lives at `policy.rs:620` after Phase B/E refactors and is 109 chars — still notably longer than surrounding ~75-char rustdoc. NOT addressed by `0ef70f9` (Phase G polish sweep, which was test-name-focused). Confirmed STILL TRUE.
2. **A-bucket-B N-2**: `grep "→\|->"` in `policy.rs` rustdoc finds 5 matches: 3 use Unicode `→` (lines 182, 314, 1243) and 2 use ASCII `->` (lines 660, 1140). Mixed within the same file. Confirmed STILL TRUE.
3. **B-bucket-B N-2**: read `bin/wdm/main.rs:cmd_bytecode` directly; no comment near the `EncodeOptions::default()` call. Confirmed STILL TRUE.
4. **E N-4**: read both `policy.rs::WdmBackup.fingerprints` doc (line 644-651) and `decode_report.rs::DecodeResult.fingerprints` doc (line 143-152). The WdmBackup doc says "recovered from a fingerprints block" but `encode.rs:127` populates it from `options.fingerprints.clone()`. Asymmetry not documented. Confirmed STILL TRUE.
5. **F nit #1**: read `vectors.rs::build_test_vectors()` (line 504-516). It is a backward-compat alias forwarding to `build_test_vectors_v1` but the doc didn't say so explicitly. Confirmed STILL TRUE.

None of the 5 was RESOLVED-INDIRECTLY by some other commit.

## Inline-fix list

| File:line | Before → After |
|---|---|
| `policy.rs:620` | reflowed `passes [...] = [...] on a small input.` → split before `[crate::ChunkingMode::ForceChunked]` |
| `policy.rs:660` | rustdoc `truncate -> ChunkWalletId` → `truncate → ChunkWalletId` |
| `policy.rs:1140` | rustdoc `to_bytecode -> from_bytecode` → `to_bytecode → from_bytecode` |
| `bin/wdm/main.rs:362-368` | added 5-line comment block on `EncodeOptions::default()` line stating intentional debug-aid asymmetry vs `cmd_encode` |
| `policy.rs:644-651` | `WdmBackup.fingerprints` rustdoc rewritten: now states encode-side mirrors `EncodeOptions::fingerprints`; cross-references `DecodeResult::fingerprints` for the parsed-from-wire counterpart |
| `vectors.rs:504-509` | `build_test_vectors()` doc gains a "Backward-compat alias: forwards to [`build_test_vectors_v1`]" sentence |

All 6 edits are pure-doc / pure-comment; no semantic / functional / signature change.

## Quality gates (post-fix)

| Gate | Result |
|---|---|
| `cargo test -p wdm-codec` | PASS — 561 tests, 0 failed (lib + integration + 5 doctests) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS — clean |
| `cargo fmt --all --check` | PASS — clean |
| `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items` | PASS — clean |
| `cargo run --bin gen_vectors -- --verify v0.1.json` | PASS — 10 positive, 30 negative, byte-identical |
| `cargo run --bin gen_vectors -- --verify v0.2.json` | PASS — 14 positive, 34 negative, byte-identical |

`format_collect` lint check: clippy clean — no 4th instance of the recurring lint surfaced anywhere in the audit; the only `format_collect` reference in the tree is `vectors.rs:1499` (a comment documenting the `fold + write!` workaround idiom).

## New FOLLOWUPS entry text

None. All 5 slipped items were resolved inline in the same commit as this audit. Per the audit instructions, "If the audit finds no real slips (all 'silent' items were actually fine), file no FOLLOWUPS entry; just write the audit report." Equivalent reasoning here: all real slips were trivial-fix-applicable, so nothing remains pending.

## Notes for the controller

- This is the audit-pass commit only; it produces zero behavioral change and zero wire-format change. The committed hashes for `v0.1.json` and `v0.2.json` are unchanged.
- The Phase A bucket B reviewer's N-1 ("Phase B will need to insert `EncodeOptions::shared_path` as tier 0") is verified RESOLVED-INDIRECTLY by Phase B bucket B (`policy.rs:306-319` precedence-chain rustdoc explicitly lists tier 0 as `opts.shared_path`). Not a slip.
- A symlink `.claude/worktrees/rust-miniscript-fork → /scratch/code/shibboleth/rust-miniscript-fork` was created at the start of this audit so the workspace `[patch]` redirect resolves; it is harmless to keep around for future worktree audits and disappears when `apoelstra/rust-miniscript#1` lands.
