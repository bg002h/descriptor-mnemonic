# End-of-Cycle R0 Architect Review — md-codec test-hardening (themes 1/2/3)

Opus feature-dev:code-reviewer. Mandatory end-of-cycle gate before ff-merge to `main` (no version bump). Reviewed against live source in `crates/md-codec/`. Branch `md-codec-test-hardening`. Read in full: `tests/common/mod.rs`, `tests/proptest_roundtrip.rs`, `tests/bch_adversarial.rs`, `tests/indel_reject_contract.rs`, plus production `chunk.rs` / `codex32.rs` / `bitstream.rs` / `canonicalize.rs` / `validate.rs` and the chore-touched lines in `md-cli/src/format/json.rs` + `tests/parity_smoke.rs`.

**Tooling note:** review agent had no Write/Edit tool; returned verbatim and persisted here by the controller.

## Headline confirmations

**Theme 1.** `descriptor_strategy` (common/mod.rs:198-280) — all 7 arms reachable via `prop_oneof!`; `descriptor_from_tree` (:166-196) derives `n` from `referenced_indices` and renumbers via `renumber_tree` to a contiguous `0..n`, so `canonicalize_placeholder_indices` never trips `PlaceholderIndexOutOfRange`/`PlaceholderNotReferenced` (canonicalize.rs:174,182-189). Explicit-origin arms get `divergent_path(n,3)` → satisfies `validate_explicit_origin_required` (validate.rs:182); non-explicit arms carry a populated Shared path (value 84), accepted independent of canonical origin (validate.rs:588). P1 (proptest_roundtrip.rs:11-19) is a real bijection assert: `decode_payload(encode_payload(canon(d))) == canon(d)` + the `encode(d)==encode(canon(d))` byte/bit tuple-equality — catches decode regressions + encoder/explicit canonicalization desync. P2 (:23-30) distinct (F4-class permutation-lockstep). P3 (:34-42): `decode_payload` arm pins `total_bits=bytes.len()*8`, avoiding the `with_bit_limit` debug_assert (bitstream.rs:114); `&str` arms derive bit count internally. P4/P5 distinct surfaces. No vacuous/self-comparison asserts.

**Theme 2.** Miscorrection theory verified: `decode_with_correction`'s re-verify (chunk.rs:559-569) confirms "*a* codeword," never "*the original*," so T2c/T2i `!= Ok(original)` (NOT `is_err()`) is the correct robust invariant. T2c (bch_adversarial.rs:118-149) produces 5-8 DISTINCT symbol errors (BTreeSet positions; xor mask forced nonzero `((x as u8)|1)&0x1F`). `restamp_chunk_header` (:211-226) uses the exact public API and recomputes BCH via `wrap_payload`→`bch_create_checksum_regular`, so each restamped chunk passes per-chunk hard-verify and the mutated header reaches reassemble's cross-chunk checks. `restamp_identity_round_trips` (:230-245) is a genuine non-vacuous oracle (compares to original `c`, not itself; runtime-asserts ≥3 chunks). T2e→`ChunkSetInconsistent` (chunk.rs:343-349, unit variant, matched without `{..}`), T2f→`ChunkIndexGap` (post-sort gap, chunk.rs:360-366), T2g→`ChunkSetIdMismatch` (deepest branch chunk.rs:381-386; foreign csid `0x0AAAA`<`1<<20` passes the write guard chunk.rs:46; collision ~2⁻²⁰). Each routes for the right reason.

**Theme 3.** `reassemble` hard-verifies with no correction path (codex32.rs:114-119), so T3a/T3b/T3d `is_err()` catches a fail-open regression. T3d's `assert_ne!(r.ok(), Some(d))` is a real self-correct tripwire. T3c broad `Codex32DecodeError(_)` correct per plan-R0 M2: `bch_verify_regular` (codex32.rs:115) runs before the `len<13` "too short" check (codex32.rs:122).

**Chore (stable 1.95.0).** All no-behavior-change: `bit_count.div_ceil(5)` ≡ `(bit_count+4)/5` for all reachable usize (codex32.rs:24); `.rfind(pred)` ≡ `.filter(pred).last()` (parity_smoke.rs:74-77, "last md1 line" preserved); `#[allow(dead_code)]` suppression-only (json.rs:18,33); doc-overindent + 5 fmt reformats are whitespace. FOLLOWUPS entry `stable-rust-1-95-toolchain-fmt-clippy-drift` accurate.

**Cross-cutting.** No new `pub` library API (test files + dev-dep proptest + `#[allow]` only); no CLI/clap change ⇒ no GUI schema-mirror / manual lockstep; test-only + no-behavior chore ⇒ no version bump correct. No `#[ignore]`/`todo!`/`unimplemented!` in the test tree.

## CRITICAL — None.
## IMPORTANT — None.

## MINOR
- **M1** — T2a `details.len() >= count` (bch_adversarial.rs:90) is looser than SPEC §4's `== count`. NOT false-passing (conjoined with `assert_eq!(got, d)` at :89). Observation only.
- **M2** — `UNCORRECTABLE_5ERR=[1,4,7,10,13]` (bch_adversarial.rs:114) is brittle-by-design (a 5-error pattern isn't provably uncorrectable); self-documented + loud-panic fenced. Architect-adjudicated deterministic-verified-once design. Future fixture/fmt shifts that flip it are expected maintenance, not a bug.

## VERDICT: GREEN (0C / 0I / 2M)

GREEN gate met. No test is false-passing/vacuous/false-failing/false-passing-against-broken-production; the two trickiest cells (T2c miscorrection, T2g csid branch) correct against source; `restamp_identity_round_trips` is a sound non-vacuous oracle; the chore is genuinely no-behavior-change; FOLLOWUPS entry accurate. Clear to ff-merge to `main`, no version bump (SPEC §6). The two MINORs are observations only, no action before merge.
