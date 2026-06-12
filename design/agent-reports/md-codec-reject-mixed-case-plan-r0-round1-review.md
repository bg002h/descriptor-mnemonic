# R0 Review — md-codec reject mixed-case md1 (PLAN) — Round 1
Reviewer: Fable 5, 2026-06-12. Verified against descriptor-mnemonic origin/main a3abdc8; companion toolkit 1971ffa.

## Verdict: RED (0C/3I)

Problem statement, injection-point analysis, error-variant lean, SemVer call all grounded. RED on 3 Importants (all plan-text folds, no re-architecture).

## Critical: none.

## Important
- **I1 — §3 multi-chunk test pins WRONG semantics.** Under per-chunk BIP-173 (confirmed), {chunk0 lower, chunk1 ALL-UPPER, chunk2 lower} is VALID (each chunk case-uniform) and reassembles Ok today — the QR workflow needs it. Fold: TWO cells — (a) ACCEPT one wholly-uppercased chunk among lowercase siblings; (b) REJECT one INTERNALLY-mixed chunk (`Md1…`/one flipped char).
- **I2 — §4.6 "toolkit suite expected green" is FALSE.** `inspect_mixed_case_md1_accepted_characterization` (`mnemonic-toolkit/tests/cli_hrp_case_insensitive.rs:577-592`) feeds `Md…` chunks to `mnemonic inspect` and asserts `.code(0)` — goes RED on the 0.35.3 pin bump. Its doc-comment anticipates the flip (cites `md-codec-accepts-mixed-case-bip173-leniency`). Fold: the toolkit tail INVERTS it (assert failure + stderr "mixes upper and lower case") + resolves the FOLLOWUP (canonical descriptor-mnemonic `FOLLOWUPS.md:14`) in both repos. No other mixed-case md1 fixture exists (all-upper cells at `:236-237,:527` stay valid).
- **I3 — `decode_with_correction` needs its OWN injection; one site is inconsistent.** It does NOT transit `unwrap_string` at parse (uses `parse_chunk_symbols` `chunk.rs:429`, lowercases `:430`), but its residue==0 pass-through forwards the ORIGINAL string (`chunk.rs:517`) into `decode_md1_string`/`reassemble` → so a single unwrap_string check rejects 0-error mixed but ACCEPTS 1-error mixed (correction re-encodes lowercase). Indefensible. Fold: add the case check to `parse_chunk_symbols` with the `"chunk {i}:"` message prefix (the toolkit `repair.rs:1258 parse_md_chunk_index` parses it). REJECT on the correction path too (Q3). Pin: ALL-UPPER through correction pass-through stays Ok.

## Minor
- M1: run the check as the FIRST statement of `unwrap_string` (over original `s`) so a mixed-HRP-only string rejects as MixedCase.
- M2: correction-path cells — mixed+residue0 → Err, mixed+1error → Err (both Ok today; clean RED→GREEN), + ALL-UPPER+residue0 → Ok pin.
- M3: mk cite — `case_check` body `:132-151` (`:126` is doc), reject `:648-649`, test `:1250`.
- M4: keep the substring "mixes upper and lower case" IDENTICAL at both sites so the toolkit test inversion asserts one predicate.

## Design decisions
- **Q1 (per-chunk):** CONFIRMED. `decode_md1_string`→`unwrap_string` (codex32.rs:92); `reassemble`→`unwrap_string` per chunk (chunk.rs:321). Those 2 are the only non-correction entries. `split()` wraps each chunk as an independent codex32 string (own HRP+BCH) → case-uniformity per chunk. `decode_with_correction` separate (I3).
- **Q2 (error variant):** REUSE `Codex32DecodeError(String)`. `md_codec::Error` is NOT `#[non_exhaustive]`, AND the toolkit `friendly_md_codec` (`friendly.rs:203-206`) is an EXHAUSTIVE match with a comment requiring it — adding `Error::MixedCase` would break the toolkit compile (MINOR + coordinated change). Reuse flows through the existing friendly arm + repair mapping with ZERO toolkit code change beyond I2.
- **Q3 (correction path):** REJECT mixed everywhere. (1) mk-codec's ONLY decode is its BCH-correcting `decode_string` and it rejects Mixed BEFORE correcting (`bch.rs:648`); ms-codec rejects via codex32 — no strict-only split to mirror. (2) Case is lowercased BEFORE symbol mapping → a case-flip is a ZERO-symbol-error event, never in the BCH channel; "noise to correct" is fiction. (3) Consistency (I3). (4) `mnemonic repair --md1` delegates to `decode_with_correction` → post-fix yields `UnparseableInput` with the codec message, matching ms1/mk1 repair today.
- **Q4 (PATCH):** CONFIRMED 0.35.2→0.35.3. No md-codec/md-cli/toolkit test/golden feeds mixed-case md1 except the I2 characterization (designed to flip). All-upper preserved. Precedent ms-codec 0.4.3 PATCH. Pin chain: md-cli `=0.35.2`→`=0.35.3`; toolkit `"0.35"` → lockfile-only.
- **Q6 (repair.rs deferral):** correct — the toolkit's own bech32/indel machinery is a separate surface (doesn't transit unwrap_string). PRECISION: md1 end-to-end repair INHERITS the reject via the decode_with_correction delegation; the FOLLOWUP residual is only (a) a structured toolkit pre-gate error + (b) the ms1/mk1 pre-gate symmetry audit.

## Probe results (temp test, deleted; repo clean)
decode_md1_string: lower Ok, ALL-UPPER Ok (QR works), mixed-HRP Ok←BUG, mixed-data-char Ok←BUG. reassemble: all-lower Ok, chunk-wholly-upper-others-lower Ok (per-chunk, KEEP), chunk-internally-mixed Ok←BUG. decode_with_correction: mixed+residue0 Ok (pass-through), mixed+1err Ok (1 correction), ALL-UPPER+residue0 Ok. Post-fix targets: all the ←BUG rows → Err "mixes"; the KEEP rows stay Ok.

Re-dispatch for Round 2 after folding I1/I2/I3.
