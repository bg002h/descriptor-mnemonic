# R0 ARCHITECT REVIEW — `SPEC_md_codec_test_hardening.md`

Opus feature-dev:code-reviewer. Mandatory pre-impl gate. Reviewed against live source @ `ca4591b`. Persisted (folds applied immediately; fold log at end).

## Headline verification (all CONFIRMED)
- **§2.1 canonicalization finding CORRECT** — `encode_payload` canonicalizes internally (`encode.rs:65-68`); decoder rejects non-canonical via `PlaceholderFirstOccurrenceOutOfOrder` (`validate.rs:28-35` ← `decode.rs:56`); `canonicalize_placeholder_indices` (`canonicalize.rs:168`) is the sole rewrite. P1 fixpoint + `encode(d)==encode(canonicalize(d))` byte-equality sound (idempotent fast-path `canonicalize.rs:199`).
- **§4.1 T2c `!= Ok(original)` CORRECT** — `decode_regular_errors` returns a codeword whenever `Some` (no syndrome re-check beyond `deg≤4`, `bch_decode.rs:416`); the guard (`chunk.rs:559-570`) re-verifies "a codeword" not "the original"; a 5–8-error pattern miscorrects to a different codeword (d=9 non-perfect). Faithfully transcribed.
- **API signatures CORRECT** — `encode_payload(&Descriptor)->Result<(Vec<u8>,usize)>`, `decode_payload(&[u8],usize)`, `encode/decode_md1_string(&str)`, `reassemble(&[&str])`, `decode_with_correction(&[&str])`, `split(&Descriptor)->Result<Vec<String>>`, `canonicalize_placeholder_indices(&mut Descriptor)`, `ChunkHeader` (pub fields + pub write/read). All re-exported `lib.rs:41-46`.
- **Theme 3 CORRECT** — `reassemble` hard-verifies (`codex32.rs:115`, no self-correct); toolkit `Md1IndelOracle` calls it (`repair.rs:1043`). `is_err()` safe.
- **Cross-chunk T2e/f/g CONSTRUCTIBLE** — `ChunkHeader` + `codex32::{wrap_payload,unwrap_string}` + `bitstream::{BitReader,BitWriter}` public; the restamp helper is buildable; `ChunkSetInconsistent`/`ChunkIndexGap`/`ChunkSetIdMismatch` reachable.

## CRITICAL — None.

## IMPORTANT
**I1 — P3 panic-freedom FALSE as worded.** `decode_payload(bytes,total_bits)` → `BitReader::with_bit_limit` has `debug_assert!(bit_limit <= bytes.len()*8)` (`bitstream.rs:114`; `decode.rs:16`). Integration tests run debug → fuzzing `total_bits` freely panics. Fix: pin `total_bits = bytes.len()*8` (or `.min(...)`) for the `decode_payload` arm; the `&str` arms stay free-form.
**I2 — taptree strategy can under-cover placeholders → `PlaceholderNotReferenced` (`canonicalize.rs:184`), spuriously failing P1.** Timelock/hash leaves (`older`/`after`/`sha256`) reference no placeholder; `tr(@0,<taptree>)` with `n>1` whose taptree doesn't reference every `@1..n-1` errors in canonicalize. Fix: derive `n` from the distinct placeholder set actually emitted; ≥1 key-bearing leaf per `@i`.
**I3 — `n ∈ 1..=32`, not ≤15.** `KeyCountOutOfRange` gate `origin_path.rs:111`; the SPEC conflated `n` with `MAX_PATH_COMPONENTS=15` (`origin_path.rs:43`, the per-path-component depth). Also the `{1..9}` kiw bias never exercises kiw=5. Fix: `n∈1..=32` (cap multi count/Divergent at 32); `components.len()≤15` separately; add `n∈{15,16,17,31,32}`.
**I4 — `tests/common/mod.rs` dead-code under `-D warnings`.** Compiled fresh into each test binary; unused helpers warn. Fix: `#![allow(dead_code)]` (+ `unused_imports`). (mk hit this.)

## MINOR
**M1** — `round_trip_canonicalize_encode_decode_canonicalize` fn at `canonicalize.rs:955` (attr `:954`). Cite `:955`.
**M2** — T3c: `bch_verify_regular` (`codex32.rs:115`) runs BEFORE the `<13` too-short check (`:122`); a sub-13 delete errors "BCH … failed" not "too short". Assert broader `Codex32DecodeError(_)`, not the `"too short"` substring.
**M3** — T2c: position-0 corruption can flip the chunked-flag → re-route `reassemble` vs `decode_md1_string` (`chunk.rs:590-608`); doesn't affect `!= Ok(original)` but note position 0 is live.
**M4** — P1: `decode_payload` is 2-arity; spell out the `(bytes,total_bits)` destructure.

## VERDICT: RED (0C / 4I / 4M)
Two headline calls (canonical-fixpoint + `!= Ok(original)`) correct + faithfully transcribed; all APIs/oracle/cross-chunk verified. RED solely on the 4 Importants (each a spurious-failure/CI-red hazard). Fold + sweep minors, re-dispatch R1.

---
## FOLD LOG (post-R0)
- I1 → §3.2 P3: pin `total_bits = bytes.len()*8` for the `decode_payload` arm; `&str` arms free-form.
- I2 → §3.1: `n` derived from the emitted placeholder set; ≥1 key-bearing leaf per `@i` in the taptree arm.
- I3 → §3.1: `n∈1..=32` + kiw bias `{…,15,16,17,31,32}`; `components.len()≤15` separated; multi/Divergent capped at 32.
- I4 → §3.3: `#![allow(dead_code)]` + `unused_imports` on `tests/common/mod.rs`.
- M1 → `:955`. M2 → T3c broader `Codex32DecodeError(_)`. M3 → §4.1 position-0 note. M4 → §3.2 P1 destructure spelled out.
