# PLAN — md-codec rejects mixed-case md1 per BIP-173 (Cycle 6 / case quirk)

**Date:** 2026-06-12 · **Crate:** `md-codec` · **SemVer:** PATCH `0.35.2 → 0.35.3` (decode-input hardening — rejects malformed input previously accepted leniently; no valid card changes; mirrors mk-codec + ms-codec)
**Source SHA:** descriptor-mnemonic `origin/main` = `a3abdc8`. Companion toolkit `origin/master` = `1971ffa`. From the 2026-06-12 input-text-encoding audit.

## 1. Problem (grounded)

md-codec's md1 decode is the ONE constellation codec that accepts MIXED-CASE input, violating BIP-173 (bech32/codex32 strings must be all-lower or all-upper, never mixed). The siblings reject it: mk-codec `case_check` → `Error::MixedCase` (`bch.rs:126-138,:649`, test `decode_rejects_mixed_case:1250`); ms-codec rejects via the codex32 crate. md-codec just lowercases per-char with no check:
- `codex32.rs:57` `char_to_symbol`: `c.to_ascii_lowercase()` per char.
- `codex32.rs:94` `unwrap_string`: `s.to_ascii_lowercase().starts_with(HRP)` for the prefix + the per-char loop (`:104`) decodes via `char_to_symbol` (lowercasing each).
- `chunk.rs:430` `let lower = chunk.to_ascii_lowercase();` (the correction path).
No mixed-case detection anywhere → `Md1...XYZ` (mixed) decodes happily.

All-UPPER input IS correctly accepted+decoded today (the QR/BIP-173 uppercase form) — only MIXED is the bug. The fix must KEEP all-upper working (canonicalize) and add the mixed reject.

## 2. The fix

Add a mixed-case check (mirroring mk-codec's `case_check`, `bch.rs:132-151`): a string (excluding `-`/whitespace separators, which are case-neutral) that contains BOTH an ASCII-uppercase AND an ASCII-lowercase letter → reject. All-upper or all-lower → proceed (canonicalize to lower as today).

**TWO injection sites (R0-I3 — `decode_with_correction` does NOT transit `unwrap_string`):**
1. **`unwrap_string`** (`codex32.rs:92`) — the FIRST statement (R0-M1), over the full original `s`. Covers `decode_md1_string` (`decode.rs:79`) + `reassemble` (per chunk, `chunk.rs:321`). Message: `"string mixes upper and lower case (BIP-173 forbids mixed case)"`.
2. **`parse_chunk_symbols`** (`chunk.rs:429`, the `decode_with_correction` parse) — same check, with the existing `"chunk {i}: "` prefix (the toolkit `repair.rs:1258 parse_md_chunk_index` parses it). Reject mixed on the correction path TOO (R0-Q3: a case-flip is lowercased BEFORE symbol mapping → it's a ZERO-symbol-error event, never in the BCH channel; mk-codec's correcting decode rejects Mixed before correcting; "noise to correct" is fiction; and a single-site check makes the pass-through self-inconsistent — 0-error mixed rejects but 1-error mixed accepts).

**Error (R0-Q2):** REUSE `Error::Codex32DecodeError(String)` — `md_codec::Error` is NOT `#[non_exhaustive]` AND the toolkit `friendly_md_codec` (`friendly.rs:203`) is an EXHAUSTIVE match, so adding `Error::MixedCase` would break the toolkit compile (a coordinated MINOR). Reuse flows through the existing friendly + repair arms with ZERO toolkit code change. **Keep the substring `"mixes upper and lower case"` IDENTICAL at both sites** (R0-M4) so the toolkit test-flip asserts one predicate.

**Helper:** a `fn case_of(s) -> {Lower|Upper|Mixed|Neither}` (digits/separators = Neither), reject on Mixed.

## 3. Tests (md-codec) — RED→GREEN (every "BUG" row is Ok today, Err post-fix)
- `decode_rejects_mixed_case`: valid md1, one data char case-flipped → `Err(Codex32DecodeError)` containing "mixes". Also a mixed-HRP (`Md1…`) variant.
- `decode_accepts_uppercase_round_trip`: `encode_md1_string(d).to_uppercase()` → `decode_md1_string` == `d` (QR form preserved).
- **`reassemble` (R0-I1 — per-chunk):** (a) ACCEPT — one chunk wholly UPPERCASED among lowercase siblings → Ok (cross-chunk heterogeneity is BIP-173-legal; the QR workflow needs it); (b) REJECT — one chunk INTERNALLY mixed → Err "mixes".
- **`decode_with_correction` (R0-M2):** (a) mixed + residue==0 → Err; (b) mixed + 1 corrupted symbol → Err; (c) ALL-UPPER + residue==0 → Ok (pin the pass-through preserves uppercase).

## 4. Release ritual (md-codec 0.35.3)
1. `crates/md-codec/Cargo.toml` `0.35.2 → 0.35.3`; `crates/md-cli/Cargo.toml` exact pin `=0.35.2 → =0.35.3` (lockstep, md-cli version unchanged — the `=0.4.x` precedent).
2. Root `CHANGELOG.md` `## md-codec [0.35.3]` PATCH entry (Fixed: mixed-case md1 reject per BIP-173; aligns with mk-codec/ms-codec; all-upper QR form unaffected).
3. `Cargo.lock` refresh.
4. Commit (explicit paths) + tag `md-codec-v0.35.3`.
5. **STOP — `cargo publish -p md-codec` is user-authorized + irreversible.** Surface for authorization.
6. **Toolkit tail (after publish):** `cargo update -p md-codec` → 0.35.3 (lockfile-only). **R0-I2: the toolkit suite will NOT be all-green as-is** — `inspect_mixed_case_md1_accepted_characterization` (`tests/cli_hrp_case_insensitive.rs:577-592`) asserts mixed-case md1 `inspect` exits 0; INVERT it (assert failure + stderr contains "mixes upper and lower case"; its own doc-comment anticipates this flip). Then full suite green. **RESOLVE the FOLLOWUP `md-codec-accepts-mixed-case-bip173-leniency`** (canonical descriptor-mnemonic `FOLLOWUPS.md:14`) in both repos. The toolkit md1 REPAIR path inherits the reject automatically (`repair.rs:1209` delegates to `decode_with_correction` → `UnparseableInput`); a richer structured toolkit pre-gate error + the ms1/mk1 pre-gate symmetry audit = a SEPARATE FOLLOWUP (R0-Q6), not this cycle.

## 5. Lockstep / scope
- md-cli `--help` unchanged → no manual mirror, no GUI schema_mirror. The toolkit's `mnemonic`/`md inspect`/`ms`/`mk` CLIs: `md inspect`/`mnemonic` decode md1 via md-codec → they inherit the reject (a user-visible behavior change: a mixed-case md1 now errors). Note in the CHANGELOG.
- The toolkit `repair.rs` md1 path may need a parallel fix → FOLLOWUP (out of this md-codec cycle).

## 6. R0 questions — RESOLVED (R0 round 1)
1. **Per-chunk** (BIP-173 per-string; `split()` = independent codex32 strings). `decode_md1_string` + `reassemble` both transit `unwrap_string` (one site); `decode_with_correction` is separate (2nd site, I3). Cross-chunk heterogeneity is LEGAL (I1).
2. **Reuse `Codex32DecodeError`** — enum NOT `#[non_exhaustive]` + toolkit exhaustive friendly-match would break on a new variant.
3. **REJECT on the correction path too** — case is lowercased pre-symbol-mapping (not in the BCH channel); mk-codec's correcting decode rejects Mixed first; single-site = inconsistent pass-through. (4 grounds, R0-Q3.)
4. **PATCH 0.35.2→0.35.3** confirmed — no valid card changes; only the I2 characterization test (designed to flip) feeds mixed-case; precedent ms-codec 0.4.3.
