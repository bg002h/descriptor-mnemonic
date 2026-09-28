Verdict: 0 Critical, 3 Important, 5 Minor, 4 Nit

# R0 r1 — `IMPLEMENTATION_PLAN_split_unadmitted.md` (md-codec 0.48.4, `chunk::split_unadmitted`)

Reviewer: independent adversarial R0 subagent, 2026-09-28. Baseline `18b4a76` (HEAD, clean except the untracked plan).

Method: read every cited file/line; prototyped the plan's change (`split_with(d, admitted)` + `split_unadmitted` + root re-export) in a `git archive` copy in the scratchpad (repo untouched); ran a probe test over the whole vendored corpus and both mint-refused shapes; ran the full `md-codec` suite against the prototype; bumped the version in the copy and ran `cargo metadata --locked`. (`cargo-nextest` is not installed in this container, so `cargo test` was used; results are unaffected.)

## Machine-verified facts: check

| Plan claim | Result |
|---|---|
| `chunk.rs:241 split`, calls `encode_payload(d)` at l.246, `compute_md1_encoding_id` at l.249 | Exact. |
| `compute_md1_encoding_id` uses non-admitting `encode_payload_for_identity` (`identity.rs:40`) | Correct: fn at l.40, the call at l.45. |
| `encode.rs:198 encode_payload_unadmitted` is public; same bytes; both via `encode_payload_inner` | Correct (l.137 `encode_payload` = `Enforce`, l.198 = `SkipPolicy`). The admission block is **l.242-253**, not 235-246 (that range is mostly the comment). See N1. |
| `lib.rs:53-56` re-exports `split` | Correct (the `pub use chunk::{...}` spans 53-56). |
| `contradictory_card()` is an F-217 shape that `encode_payload` refuses and decode reads | Correct (`tests/mint_policy_does_not_reach_decode.rs:35`; its `the_unadmitted_serialiser_reads_every_mint_refused_shape` already asserts `decode_payload(unadmitted bytes) == card`). |
| md-cli pin `=0.48.3` at `crates/md-cli/Cargo.toml:28` | Correct. |

## Additivity / bytes of `split` unchanged: VERIFIED

- `split_with(d, true)` is the old body verbatim. Admission is a pure refusal that runs before any byte is written (`encode.rs:242-253`), so both branches produce identical bytes whenever the admitted one succeeds. The header version (`d.wire_version()`), the chunk-set id (already non-admitting) and the chunk sizing are shared code.
- Prototype: full `cargo test --locked -p md-codec` passed, all 47 test binaries green. That includes `wire_golden.rs::wsh_sortedmulti_2chunk_chunk_strings_frozen`, the frozen split-strings golden.
- Probe over all 68 `tests/vectors/*.phrase.txt`: 68 decode, `split` accepts all 68 (54 are multi-chunk), and `split_unadmitted` returns identical strings on every one.
- The chunk-count > 64 path behaves the same, because it is shared code after the encode. One difference: a descriptor that is both admission-refused and too large gets `ChunkCountExceedsMax` from `split_unadmitted` but the admission error from `split`. This is expected (N2).
- `split`'s error on the F-217 shape really is the admission error: `OriginKeyContradiction { a: 0, b: 1, fingerprint: "5436d724", path: "48'/0'" }` (measured). `split_unadmitted` on it gives **4 chunks**, and `reassemble` gives back `== card`. The header chunk-set id equals `derive_chunk_set_id(compute_md1_encoding_id(card))`.

## Important

**I1. `Cargo.lock` is missing from the change list, so the plan's own `--locked` gate fails.**
Bumping `crates/md-codec/Cargo.toml` and md-cli's pin to 0.48.4 without updating `Cargo.lock` (l.513, `version = "0.48.3"`) makes every `--locked` command fail before compiling. Measured in the copy: `the lock file ... needs to be updated but --locked was passed`. The repo convention, stated in 53718d1's commit message after this exact item was missed once: "bump, changelog, lock and code land in ONE commit titled `md-codec X.Y.Z: ...`". Add `Cargo.lock` (regenerated with `cargo update -p md-codec --offline` or an unlocked build) to Change §2, and name the single-commit convention.

**I2. Test row 1 cannot be implemented as written.**
- `md_codec::test_vectors::MANIFEST` holds **template strings** (`Vector.template`). The template parser lives in md-cli (`parse::template`). `Descriptor` derives no `Deserialize` in md-codec, where serde is a dev-dependency only. So an md-codec integration test cannot turn "every manifest vector" into a `Descriptor`.
- "(forced chunked and not)" means nothing for `split`, which always chunks. `force_chunked` is an md-cli `encode` dispatch between `split` and `encode_md1_string`.
- Feasible replacement, already used in-repo: `include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/common/vendored.rs"))`, then `all_vendored_vector_names()` → `load_vendored_phrase` → `decode_vendored` → compare `split` with `split_unadmitted`.
- Measured: 68 vectors, 68 accepted, 54 multi-chunk, 0 refused. Assert a floor on the accepted count (for example `>= 68`, or `== names.len()`), so the row cannot pass vacuously if the loader returns nothing.
- Note that this row, like row 4, only characterises. Before the change it is RED solely as a compile error, which is acceptable under the repo's TDD practice but should be said. Row 2 is the only one with a behavioural RED.

**I3. The plan changes a documented invariant without saying so.**
- `encode_payload_unadmitted`'s doc (`encode.rs:181-197`) says it is "for COMPARISON and HASHING of cards that already exist, **never for minting**" and "A caller that is about to write a card must use `encode_payload` (or `encode_md1_string`)". The `Admission::SkipPolicy` doc (l.148-150) says "to hash it".
- `split_unadmitted` is an **emitting** API: its output gets printed or engraved. Nothing in its `&Descriptor` signature can check that the card "already exists". Any caller can mint a fresh F-217/F-218/§6-refused card through it, and through `wrap_payload(encode_payload_unadmitted(..))` today.
- The plan should state this reasoning and amend both doc comments:
  - The rule becomes: re-emitting a card that exists elsewhere (here, the md1 half of an existing `mr1` card) is not minting.
  - The mint gate remains the caller's duty (`mr` must mint new cards via `split`/`encode_md1_string`).
  - Say whether the `mr` tool's *mint* path, as opposed to its reprint path, is required to use `split`.
- Also consider linking the FOLLOWUPS entry `encode-time-policy-reaches-decode-via-the-encoding-id` (FOLLOWUPS.md:3043), which sets the "mint vs existing card" policy this change extends.

## Minor

**M1. Only one shape is tested (row 2/3), but the gate file pins the whole class.**
`mint_policy_does_not_reach_decode.rs` exists to pin "the CLASS, not the instance". Its M2 note records that an F-217-only test stayed GREEN while §6 regressed. Add `kind_1_card_with_a_sortedmulti_a_leaf()` (copied the same way) to rows 2 and 3. Measured: `split` → `UnspendableWithSortedMultiA`; `split_unadmitted` → 1 chunk, header version 8, and `reassemble` gives `== card`. Add an F-218 duplicate-key-slot shape too if one can be built cheaply.

**M2. Row 4 is ambiguous and partly redundant.**
For an admissible descriptor it is implied by row 1: identical strings mean identical headers. For the F-217 shape, `split` errors, so "the one `split` would compute" does not exist. Rewrite it as: every header's `chunk_set_id` on the refused shape equals `derive_chunk_set_id(&compute_md1_encoding_id(&d)?)`. Measured equal. Parse the header with `codex32::unwrap_string` + `ChunkHeader::read`. Note that row 3's `reassemble` already enforces this through `ChunkSetIdMismatch`, so row 4 is a readability pin, not new coverage.

**M3. The md-cli release handling is unspecified.**
"CHANGELOG entries for both crates (md-cli: pin bump only)" names no md-cli version, and a Keep-a-Changelog entry needs a `## md-cli [x.y.z] — date` header. Precedent is split two ways:
- md-codec 0.44.1 / 0.44.2 → md-cli 0.16.1 / 0.16.2, with "Picks up md-codec 0.44.x (above). No CLI surface change."
- md-codec 0.45.1 (53718d1) → pin moved, no md-cli bump and no md-cli entry.

Pick one explicitly. The 0.45.1 form is recommended: no md-cli release, pin moved, and the md-codec entry under `### Added` with header `## md-codec [0.48.4] — 2026-09-28` placed above `## md-cli [0.20.3]`. No manual change is needed; this is confirmed, as no md-cli flag changes.

**M4. The build gate has two gaps.**
(a) `cargo nextest` does not run doc-tests. CI keeps a separate doc-test step (`.github/workflows/ci.yml:58`), and the new public item carries a doc comment with intra-doc links, so add `cargo test --locked --doc -p md-codec`.
(b) The workspace sets `missing_docs = "warn"` (`Cargo.toml:12`), and clippy runs with `-D warnings`. A new `tests/split_unadmitted.rs` without a `//!` crate doc fails the gate; the probe file emitted exactly this warning. State it in the plan so the implementer does not hit it cold.

**M5. The FOLLOWUPS entries should follow the template and the cross-repo convention.**
- Template (FOLLOWUPS.md l.24-31): Surfaced / Where / What / Why deferred / Status / Tier.
- Per CLAUDE.md, a cross-repo item has its primary entry in the originating repo (mnemonic-refugium) and a mirrored companion here, each with a `Companion:` line citing the other, and `Status: resolved <COMMIT>` when shipped. The plan says only "citing the mr plan".
- For `refugium-liana-key-per-member`, cite the md-codec source of the `<0;1>` claim: `crates/md-codec/src/to_miniscript.rs:521-528` ("ALWAYS the `<0;1>` derivation").

## Nit

**N1.** The citation `encode.rs:235-246` should be `encode.rs:242-253` (the `if admission == Admission::Enforce` block).

**N2.** State in the `split_unadmitted` doc that structural errors, including `ChunkCountExceedsMax`, still surface. Also note the error-precedence difference for a descriptor that is both refused and too large.

**N3.** Release-surface items checked, with no change needed:
- `crates/md-cli/tests/crate_pin.rs` pins **mnemonic-io-lib**, not md-codec.
- No vector file, conformance JSON or md-cli source embeds the md-codec version string. There is no GENERATOR-version field; the `vectors` subcommand prints none.
- `vendor/` holds only registry/git crates, and md-codec is a path dependency, so `ci/repro/vendor-freshness.sh` is unaffected.
- `fuzz/Cargo.lock` pins md-codec **0.47.0**. It is already stale, is a separate lockfile, and is outside the main `--locked` gate. No action; optionally note it.
- No cargo-semver-checks or public-API snapshot exists in `.github/workflows`.

**N4.** The plan's clippy gate omits `--workspace`, which CI uses (`ci.yml:75`: `cargo clippy --workspace --all-targets --all-features -- -D warnings`). Mirror CI exactly. The row-scoped `-E 'test(split_unadmitted_)'` = 4 is sound, since no existing test name contains that substring. It becomes 4 + any M1 additions if those are split into separate rows.

## What would make this GREEN

Fix I1-I3 in the plan text: add the lock and the one-commit convention, re-source row 1 from `vendored.rs` with a count floor, and state the doc-contract change. Folding M1-M5 in is cheap. The code change itself is correct and byte-neutral for `split`; the prototype proves it.
