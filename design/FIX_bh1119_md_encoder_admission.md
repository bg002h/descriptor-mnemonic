# FIX_bh1119_md_encoder_admission: the minting encoder decodes its own output and refuses what the decoder refuses (bh-md-I5, sweep row 10 md-encode-depth-unrestorable; Important)

Full track (bughunt-1119 FIX table row #8, architect ruling T9, coordinator
`agent-reports/architect-bughunt-1119-questions.md` L45 and L120). Repo descriptor-mnemonic, branch
`fix/encoder-admission` from main `c10628f1`, merged with main `562b1d08`. Crates: `md-codec` (the gate) and `md-cli`
(CHANGELOG, one CLI row). Sources: `agent-reports/bughunt-1119/grade-md.md` row bh-md-I5, `triage-md-t2.md` (I5 rows),
`sweep-1122/grade-md.md` row md-encode-depth-unrestorable (engrave F-137). Author: Claude Opus 5.5
(claude-opus-5-5), 2026-11-22.

**Decode mode: PARTIAL** (coordinator ruling 2026-11-22 on the author's STOP Q1): the gate calls
`crate::decode::decode_payload_with_opts(&bytes, total_bits, crate::decode::DecodeOpts::partial())`, the decode
`md decode` and `md inspect` run, because strict `decode_payload` refuses origin-less template cards that
bip-mnemonic-descriptor.mediawiki:219-236 admits. No holder-visible text is new: every refusal below prints a
decoder error that already exists on main. Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0;
`--locked`; nextest `--build-jobs 6 -j 6`; never `--release`. Signet only; no keys are used by any row.

## 1. Decode mode (decided: partial)

The ruling says "`encode_payload` decodes its own output before returning Ok and returns the decoder's error if it
refuses". md has two decoders of one payload: `decode_payload` (strict) and `decode_payload_with_opts(..,
DecodeOpts::partial())`, which differs from strict only by accepting an `@N` with no resolvable origin
(`MissingExplicitOrigin`, a "dead card"). The gate uses the partial one. Evidence, measured with a scratch gate at
`c10628f1`: `cargo nextest run --workspace --all-targets --all-features` 1760 run, 1760 pass once the six tests
of section 6 move to a raw writer. Nothing md mints today stops minting except the rows of section 4.
`md encode 'wsh(pk(@0/<0;1>/*))'` (no `--path`) keeps printing `md1yqpqqxpzsn9favzyjwfcgx` with the pathless
advisory at exit 0. The BIP ("Shapes with no canonical entry (partial decode)",
bip-mnemonic-descriptor.mediawiki:219-236) makes a dead card a renderable card class (exit 4). The existing test
`md-cli::cli_pathless_encode_advisory pathless_shape_without_path_emits_advisory` is control row K5 (section 5): it
fails if a builder uses strict `decode_payload`.

## 2. Ruling restated, and what the gate is

`encode_payload` (and so every admitting entry point built on it: `encode_md1_string`, `chunk::split`,
`Descriptor::canonical_payload_bytes`) decodes the payload it has just written, with the same bytes and the same
exact bit count it returns, with `DecodeOpts::partial()`, and returns the decoder's `Error` unchanged instead of
`Ok` if the decoder refuses. The bytes of every payload it accepts are unchanged.

Decided here (code level, from the ruling's scope and the repo hazard "Admission versus existing cards"):

- The gate runs only under `Admission::Enforce`. `encode_payload_unadmitted` and `encode_payload_for_identity`
  (re-emit, compare, hash) are unchanged and never decode their output, so no card that exists becomes unprintable
  or unhashable, and `chunk::reassemble`'s encoding-id check is untouched.
- Order: canonicalisation, the structural validators, the mint-time admission rules (F-217, F-218, kind-1 shape,
  minimum version), the writers, then the gate. A descriptor that fails an admission rule and would also fail the
  decoder reports the admission rule, as today.
- The gate checks only that the decoder accepts; it does not compare the decoded descriptor with the input. The
  ruling asks for refusal only, and C2 (the case it backstops) was a refusal.
- The `#[cfg(test)]` hook `encode_payload_at_forced_version` runs under `Enforce` and is gated too (no test changes:
  the minimum-version rule refuses first).
- The gate is structural: md's decoder does not type-check miniscript, so a hand-built `Descriptor` whose tree is
  not valid miniscript (e.g. `and_v` over two `pk`) still encodes, decodes at `md decode` exit 0, and is refused at
  restore (`md address`: `typecheck: ... cannot accept children of types B and B`). Not closed here; FOLLOWUPS
  `md-encode-typecheck-gate`.
- No public signature, type or error variant changes.

## 3. Spec delta (exact texts)

There is no errata file in md (`grep -ril errata design bip docs`: none). The normative spec is
`bip/bip-mnemonic-descriptor.mediawiki`; the frozen per-version specs (`design/SPEC_v0_30_wire_format.md` and
older) are not edited.

**BIP, §"Decoder canonical-form validators".** After that section's validators table (its closing `|}`, the line
before `===Decoder reporting===`) insert one new paragraph:

> Encoders MUST NOT emit a payload that a conforming decoder rejects, with the one relaxation that a decoder's
> partial-allowing entry point makes (an `@N` whose origin cannot be resolved; see §"Shapes with no canonical
> entry (partial decode)"). The reference encoder enforces this by decoding every payload it mints, before
> returning it, and returning the decoder's error instead of the payload if the decoder refuses; so, for example,
> a tree whose recursion depth reaches the decoder's cap of 128 (§"Error taxonomy",
> <code>DecodeRecursionDepthExceeded</code>) is refused at mint time, not discovered at recovery. Re-emitting,
> hashing or comparing a card that already exists does not run this check.

**`crates/md-codec/src/encode.rs`, rustdoc of `encode_payload`.** Append one paragraph:

> Before returning, the payload is decoded with [`crate::decode::DecodeOpts::partial`] (the decode `md decode`
> runs) and the decoder's error is returned instead if it refuses (FIX_bh1119_md_encoder_admission): a card this
> function mints always decodes under `DecodeOpts::partial()`; strict `decode_payload` and
> `Descriptor::from_canonical_payload_bytes` still refuse an `@N` with no resolvable origin. The bytes of an
> accepted payload are unchanged. The check is the decoder's: it does not type-check the miniscript.
> [`encode_payload_unadmitted`] does not run this check.

**`crates/md-codec/src/decode.rs:209-211`** (test doc comment). Replace "The encoder has no root-tag gate (only
placeholder/multipath/taptree validators run), so `encode_payload` of an AndV-rooted descriptor succeeds and
round-trips through `decode_payload` exposes the gap." with "The encoder refuses this wire (it decodes its own
output, FIX_bh1119_md_encoder_admission), so the test writes it with `raw_wire`, which applies no check."

**`crates/md-codec/tests/per_key_use_site_override.rs:514-515`.** Replace "Hand-crafted via encode→decode (our
encoders never emit an `@0` override, so this is adversarial wire)." with "Hand-crafted with `raw_wire` (the
encoder refuses an `@0` override, so this is adversarial wire)."

**CHANGELOG.md, `## md-codec [Unreleased]`** (empty on main at 562b1d08; add a `### Fixed` heading under it):

> - **`encode_payload` no longer returns a payload the decoder refuses.** It now decodes every payload it mints
>   (the decode `md decode` runs) and returns the decoder's error instead. Before, a descriptor built through
>   public fields (a `path_decl.n` that differs from `n`, a non-wrapper root tag, invalid xpub bytes, an empty
>   origin override, an `@0` or redundant use-site override) or a `wsh` over 127 nested single-child wrappers encoded at `Ok` and the card could not be decoded. Bytes of every
>   accepted payload are unchanged; `encode_payload_unadmitted` and hashing do not run the check. Costs one decode
>   per mint.

**CHANGELOG.md, `## md-cli [Unreleased]`** (empty on main; add a `### Fixed` heading under it):

> - **`md encode` refuses a template it cannot decode back.** A tree nested deeper than the decoder's limit, such
>   as `wsh(` followed by 127 `n` wrappers and `:pk(@0/<0;1>/*))`, minted three chunks at exit 0 that `md decode`
>   refused; it now prints `md: codec error: decode recursion depth 128 exceeded maximum 128` and exits 1 with
>   nothing on stdout. 126 wrappers still mint and decode.

No toolkit manual change: no flag and no printed string is new (the manual has no depth text:
`git grep "recursion depth" origin/master -- docs/manual` in mnemonic-toolkit is empty). No GUI schema change.

## 4. Callers and what each sees when the gate refuses

Measured (run at `c10628f1` with the scratch gate, partial mode) or source-read where marked. With the partial gate the only
refusals a parsed template can reach are the depth cap (any template with more than 126 nested single-child
wrappers inside `wsh`) and any future decoder rule the parser lets through; the four I5 rows need a hand-built
`Descriptor`.

| Caller | Path | Refusal seen | Exit |
|---|---|---|---|
| md-codec `encode_payload`, `encode_md1_string`, `chunk::split`, `Descriptor::canonical_payload_bytes` | library | the decoder's `Error` unchanged, e.g. `DecodeRecursionDepthExceeded { depth: 128, max: 128 }`, Display `decode recursion depth 128 exceeded maximum 128` (measured) | n/a |
| md-cli `md encode` (`cmd/encode.rs:313` `mint_md1_cards`) | `encode_md1_string` then `split` | stderr `md: codec error: decode recursion depth 128 exceeded maximum 128`, stdout empty, no `chunk-set-id:` line (measured) | 1 |
| md-cli `md descriptor --emit md1` | same `mint_md1_cards` | same line (source-read: same function, same `CliError::Codec`, `main.rs:931-933`) | 1 |
| md-cli `md bytecode` (`cmd/bytecode.rs:52`) | `encode_payload` on a card it has just decoded strictly | none: a decoded card re-encodes to bytes that decode (canonical fixpoint, the proptest_roundtrip suite) | unchanged |
| md-cli `md vectors` (`cmd/vectors.rs:61-83`) | fixed corpus | none (suite green) | unchanged |
| md-cli seating `seat::compose::comparison_form` (`compose.rs:169`) | `canonical_payload_bytes` on a seated descriptor built from decoded cards | none reachable (inputs decoded); if ever, `md: codec error: <decoder error>` | 1 |
| md-cli `md inspect`, `md verify` | `encode_payload_for_identity` / `encode_payload_unadmitted` | not gated | unchanged |
| mnemonic-toolkit (pin `cf35d61a`, md-codec 0.47.0) `template.rs:563`, `synthesize.rs:405,441,512,743,1102`, `word_card_adapter.rs:87,104,176` | `split`, `canonical_payload_bytes` | after its pin bump: `ToolkitError::MdCodec`, text from `friendly_md_codec`: `md1 decode recursion depth 128 exceeds maximum 128` (source-read, friendly.rs:426) | 2 (`md_codec_exit_code`, error.rs:595) |
| mnemonic-engrave (pin `c8bb97a5`, md-codec 0.48.5) `me descriptor --as md1` (`descriptor/md1.rs:393` via `as_flag.rs:116`) | `split` | after its pin bump: refusal row `UseSiteOutOfSet`, text `this wallet could not be encoded as md1 text cards: decode recursion depth 128 exceeded maximum 128.` (source-read) | me's refusal exit (unchanged mapping) |
| mnemonic-refugium (pin `6cf6599a`) `mr-codec card.rs` `mint_check` (L1234), `mint_refusal` (L1279) | `encode_payload` | after its pin bump: `Error::MintRefused`, `md-codec's mint policy refuses this card's payload: decode recursion depth 128 exceeded maximum 128` (source-read, mr-codec error.rs:4674) | mr's MintRefused exit (unchanged mapping) |
| mnemonic-key `mk-cli` (md-codec 0.42.0, crates.io) | no admitting call outside tests | unaffected | n/a |
| mnemonic-gui, refugium-wallet | no direct md-codec call (refugium-wallet links mr-codec) | through mr-codec's pin only | n/a |

Downstream rows are reached only when each repo bumps its md-codec pin; each bump PR pins its own printed line.
The md fuzz targets (`md1_decode_string`, `md1_reassemble`) call `encode_md1_string`/`split` under `if let Ok`
and are unaffected.

## 5. TRACE rows

New files `crates/md-codec/tests/bh1119_md_encoder_admission.rs` and
`crates/md-cli/tests/bh1119_md_encoder_admission.rs`; each test is named exactly as its row. The I5 rows start from
the card `md1yqpqqxqq8xtwhw4xwn4qh` (decoded with `decode_md1_string`) and mutate one public field. The depth rows
build `wsh` over N `Tag::ZeroNotEqual` nodes over `Tag::PkK` key 0, shared origin `m/84'`, standard `<0;1>/*`
use-site: the tree `md encode 'wsh(<N x n>:pk(@0/<0;1>/*))' --path bip84` builds (no `Tag::Check` node; BIP walker
normalisation).

RED rows R1-R6, run at `c10628f1`, and R7-R8, run at the merged head `4368c3f9` (`cargo nextest run --locked --build-jobs 6 -j 6 -p md-codec -p md-cli -E
'binary(bh1119_md_encoder_admission)' --no-fail-fast`): 12 run, 4 passed, 8 failed, exactly the eight below.

| Row | Test | Mutation / input | RED on main (seen) | GREEN |
|---|---|---|---|---|
| R1 | md-codec `bh_md_i5_path_decl_count_mismatch_refused` | `path_decl.n = 2` | panics `encoded n mismatch: ([32, 66, 0, 24, 0], 36)` | `Err(BitStreamTruncated { requested: 1, available: 0 })` |
| R2 | md-codec `bh_md_i5_non_root_tag_refused` | `tree.tag = Tag::PkK` | panics `encoded PkK root: ([32, 2, 0, 24, 160], 36)` | `Err(OperatorContextViolation { tag: PkK, context: TopLevel })` |
| R3 | md-codec `bh_md_i5_invalid_pubkey_bytes_refused` | `tlv.pubkeys = Some(vec![(0, [0u8; 65])])` | panics `encoded invalid xpub bytes: (..., 575)` | `Err(InvalidXpubBytes { idx: 0 })` |
| R4 | md-codec `bh_md_i5_empty_origin_override_refused` | `tlv.origin_path_overrides = Some(vec![(0, OriginPath { components: vec![] })])` | panics `encoded empty origin override: ([32, 2, 0, 24, 1, 156, 0], 52)` | `Err(EmptyOriginOverride { idx: 0 })` |
| R5 | md-codec `bh_md_row10_depth_127_encode_refused` | N = 127 | panics `encoded depth 127: ([32, 6, 245, 32, 1, 130, ...], 822)` | `Err(DecodeRecursionDepthExceeded { depth: 128, max: 128 })` |
| R6 | md-cli `bh_md_row10_cli_depth_127_encode_refused` | `md encode 'wsh(<127 x n>:pk(@0/<0;1>/*))' --path bip84` | `left: (0, "md1fqypapqpqpm6jzzqqvzfyjf...\n" (3 lines), "chunk-set-id: 0x0103d\nnote: stdout is a keyless descriptor template (no keys)\n")` | `(1, "", "md: codec error: decode recursion depth 128 exceeded maximum 128\n")` |
| R7 | md-codec `bh_md_i5_at0_use_site_override_refused` | `divergent_wsh_multi(UseSitePath::standard_multipath(), None)` (helper copied verbatim from `tests/per_key_use_site_override.rs:120`) plus `tlv.use_site_path_overrides = Some(vec![(0, <2;3>)])` | `encode_payload` returns `Ok` (seen: 1211 bits) | `Err(BaselineUseSiteOverride { idx: 0 })` |
| R8 | md-codec `bh_md_i5_redundant_use_site_override_refused` | same fixture plus `use_site_path_overrides = Some(vec![(1, UseSitePath::standard_multipath())])` | `encode_payload` returns `Ok` (seen: 1208 bits) | `Err(RedundantUseSiteOverride { idx: 1 })` |

Each GREEN row asserts the exact `Error` value with `matches!` (R1-R5, R7, R8) or the exact `(code, stdout, stderr)` triple
(R6).

Control rows (pass on main and after the fix; they stop the gate over-reaching):

| Row | Test | Asserts |
|---|---|---|
| K1 | md-codec `bh_md_i5_control_unmodified_roundtrips` | the unmutated card encodes and `decode_payload` accepts it |
| K2 | md-codec `bh_md_row10_depth_126_encodes_and_decodes` | N = 126 encodes and decodes |
| K3 | md-codec `bh_md_row10_depth_127_unadmitted_still_serialises` | `encode_payload_unadmitted` of N = 127 is `Ok` (re-emit path not gated) |
| K4 | md-cli `bh_md_row10_cli_depth_126_encodes_and_decodes` | `md encode` of N = 126 exits 0 and `md decode` of its lines exits 0 |
| K5 | md-cli `cli_pathless_encode_advisory pathless_shape_without_path_emits_advisory` (existing, unchanged; passes on main, seen) | `md encode 'wsh(pk(@0/<0;1>/*))'` without `--path` keeps exit 0 and the advisory; fails if the gate uses strict `decode_payload` |

## 6. Tests that change

These six build a wire the decoder must refuse by calling `encode_payload`; under the gate that call returns `Err`
(measured: exactly these six fail with the partial gate). Each moves to a private helper `raw_wire(&Descriptor) -> (Vec<u8>,
usize)`, one copy in each of the three files, which writes the descriptor's fields as given with the public
writers and no canonicalisation or validation: a `BitWriter`; `Header { version: d.wire_version(),
divergent_paths: <path_decl is Divergent> }.write`; `path_decl.write`; `use_site_path.write`;
`tree::write_node(.., d.key_index_width(), version)`; `tlv.write(.., d.key_index_width())`; returns the bytes and
`bit_len`. Every assertion stays as it is.

1. `md-codec decode::tests::decode_rejects_non_canonical_root_tag` (src/decode.rs:213; drops the
   `use crate::encode::encode_payload;` import).
2. `md-codec::partial_decode empty_origin_override_rejected_for_non_canonical_shape_via_path_decl_fallback`
   (tests/partial_decode.rs:294, call at :323).
3. `md-codec::partial_decode empty_origin_override_rejected_for_canonical_wpkh_shape` (:338, call at :360).
4. `md-codec::partial_decode empty_origin_override_still_rejects_under_partial_mode` (:366, call at :372).
5. `md-codec::per_key_use_site_override decode_rejects_baseline_at0_override` (tests/per_key_use_site_override.rs:517).
6. `md-codec::per_key_use_site_override decode_rejects_redundant_override_equal_to_baseline` (:536).

The ruling and grade-md name four such tests; the measured set is six (rows 5 and 6 are new). No other test
changes.

## 7. Cost

The gate adds one `decode_payload_with_opts` per admitted encode (two on a chunked `md encode`, which tries the
single string first). Measured on the largest committed vector, `keyed_tr_pathological` (937 bytes, 7489 bits),
2000 calls each, opt-level 2 debug profile, box load average 37-47 on 24 cores (so the ranges are wide):
`decode_payload` 33-120 us/call; `encode_payload` 61-149 us gated against 64-106 us ungated; `split` 148-196 us
gated against 81-133 us ungated. The upper bound is one decode per mint, well under a millisecond; no caller
loops over mints at a scale where that matters (`md vectors` and seating run a handful).

## 8. Release

This FIX bumps no version. The branch is rebased onto origin/main at or after 562b1d08 (release md-cli 0.21.0 +
md-codec 0.48.6; done, merge `4368c3f9`) and adds its CHANGELOG lines under the existing `## md-codec [Unreleased]`
and `## md-cli [Unreleased]` headings (adding `### Fixed`). The next `release:` commit (cut-release.sh) picks the
versions; a patch is expected (no public signature, type or variant change). T9's clause "ship in the same md-codec
release as FIX_bh1119_md_placeholder_order" is not met: #5 shipped in md-codec 0.48.6 (1bae7c18 in 562b1d08)
without this FIX. Downstream pins (toolkit, engrave, refugium) bump in their own PRs after that release.

## 9. Step order and gate

1. RED: add the two test files with R1-R8 and K1-K4; run the RED command of section 5; see 8 fail, 4 pass.
2. Move the six tests of section 6 to `raw_wire` (they still pass: no gate yet).
3. GREEN: the gate in `encode_payload_inner` under `Admission::Enforce` (section 2), the rustdoc and comments of
   section 3; all rows pass.
4. BIP paragraph and CHANGELOG lines (no version bump).
5. Gate = every job of `.github/workflows/ci.yml`: `cargo nextest run --locked --workspace --all-targets
   --all-features --no-fail-fast` (scratch result with the partial gate: 1760 run, 1760 pass, 6 skipped), `cargo test --locked
   --workspace --doc --all-features`, `cargo clippy --locked --workspace --all-targets --all-features -- -D
   warnings`, `cargo build --locked -p md-cli --no-default-features`, `cargo fmt --all --check`, the docs-only and
   display-grouping checks of that file; plus `fuzz-smoke.yml` build (the fuzz targets call `encode_md1_string` and
   `split`) and `vendor-freshness.yml` (no vendor change expected). Captured once to a file and grepped.

Coverage map: ruling "decodes its own output ... returns the decoder's error" -> R1-R6, K1; "closes row 10" -> R5,
R6, K2, K4; "the four I5 mutations" -> R1-R4 (and R7, R8 for the two use-site overrides); "decoder tests move to a raw bit writer" -> section 6 (six tests);
"backstop for C2" -> the gate itself (C2's own rows live in FIX_bh1119_md_placeholder_order); hazard "admission
versus existing cards" -> K3 and the not-gated rows of section 4; decode mode (partial) -> the existing `md-cli::cli_pathless_encode_advisory pathless_shape_without_path_emits_advisory`, unchanged and passing (control row K5); "no new wrong-mint for an `@0` or redundant use-site override" -> R7, R8.
