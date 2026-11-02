# Bug report: bg002h/descriptor-mnemonic @ 5dfb1fd

## Summary
- **Scope:** all 86 Rust source files, about 39,900 lines, in `md-codec`, `md-cli` and `xtask`.
- **Agents:** 32 slices, each read by a reader and a second reviewer. All 64 finished.
- **Result:** 16 suspicions were rated 3 or higher. **15 were confirmed**: 8 demonstrated with a failing test and 7 by reading. 1 was rejected.
- **How they were tested:** the pinned toolchain and the rust-miniscript git dependency are unavailable offline, so the crates couldn't be built. Each test is a standalone `rustc` copy of the function in question. Tests are under `/tmp/huntmd/prover-N/` in the sandbox.

The core codec checked out. Reviewers hand-checked GF(32)/GF(1024), Berlekamp–Massey, Chien, Forney, chunk splitting (brute-forced over every payload length up to 2,560 bytes), the header layout and the bitstream, and found nothing wrong. The bugs are in the validation gates, the CLI branches and how strictly input is parsed.

## Confirmed bugs (worst first)

### Medium

**1. `md encode --json` skips the BIP-68 relative-timelock check** (confirmed by reading)
- Where: `md-cli/src/cmd/encode.rs` `run`, lines 121-169. The `--json` branch returns before `validate_relative_timelocks` at line 188.
- Input: `md encode --json 'wsh(and_v(v:pk(@0/<0;1>/*),older(65536)))'`.
- Should: refuse, as the text path does. Consensus truncates this value, so the card would record a timelock that is not actually enforced.
- Does: prints a phrase and exits 0.
- Fix: call `validate_relative_timelocks(&descriptor.tree)?` before the `--json` branch.

**2. The timelock check never looks inside `thresh()`** (demonstrated)
- Where: `md-codec/src/validate.rs` `walk_older`, line 235. It recurses into `Body::Children` and `Body::Tr`, but not `Body::Variable`, which is the body `thresh` uses.
- Input: `wsh(thresh(2,pk(@0),s:pk(@1),sln:older(210000)))`.
- Should: `RelativeTimelockTruncated`.
- Does: returns Ok. The card would say 210,000 blocks while consensus enforces 13,392.
- Test: `prover-2/standalone.rs`.
- Fix: add a `Body::Variable { children, .. }` arm that recurses into the children.

**3. `md encode --json` fails on any policy too large for one string** (confirmed by reading)
- Where: `encode.rs`, line 133. It calls `encode_md1_string(...)?` directly, while the text path uses `mint_md1_cards`, which falls back to chunking.
- Input: any keyed policy too big for one string, e.g. a 2-of-2 multisig with xpubs, with `--json`.
- Should: produce chunks, as the same command without `--json` does.
- Does: fails with `PayloadTooLongForSingleString`.
- Fix: use `mint_md1_cards` in the json branch too.

**4. Multipath alternatives in the hardened range are accepted** (demonstrated)
- Where: `md-cli/src/parse/template.rs` `lex_placeholders`, line 304. Only a trailing `'` or `h` is rejected; values of 2^31 and above get through.
- Input: `wsh(multi(2,@0/<2147483648;1>/*,@1/<0;1>/*))`.
- Should: refuse, because the card can't be derived from an xpub. The code's own comment says hardened multipath must never be let through.
- Does: lexes it as `[2147483648, 1]`.
- Test: `prover-13/t.rs`.
- Fix: reject `v >= 0x8000_0000`.

**5. `split_branches` doesn't split OR alternatives nested under and_v, and_b or wrappers** (confirmed by reading)
- Where: `md-codec/src/policy_shape.rs`, line 425.
- Input: `wsh(and_v(v:pk(@0),or_d(pk(@1),older(10))))`.
- Should: two spend paths, {@0,@1} and {@0, older(10)}.
- Does: one branch {slots [0,1], locks [older 10]}, which claims every key and the lock are all needed. The per-path partition (`fp_partition`) then uses this wrong branch.
- Fix: for conjunctions, combine each child's branches pairwise (a cross product), with a size cap.

**6. Comparing cards goes through the mint-time checks** (confirmed by reading)
- Where: `md-cli/src/seat/compose.rs` `comparison_form`, line 169. It uses `canonical_payload_bytes`, which applies the mint-time admission checks.
- Input: running `md descriptor --verify-against`, or A3 seating, on an existing card that today's mint policy refuses (e.g. DuplicateKeySlots).
- Should: give a verdict. The codec's own F-639 doc says checking a card must never depend on mint policy.
- Does: aborts with a mint-policy error.
- Fix: use `encode_payload_unadmitted`.

### Low

**7. `canonicalize` panics on an out-of-range TLV index** (demonstrated)
- Where: `md-codec/src/canonicalize.rs` `remap_tlv_vec`, line 137.
- Input: n=2, the tree `multi(@1,@0)`, and fingerprints `[(5, fp)]`.
- Should: return a typed error.
- Does: panics with an index-out-of-bounds error, although the docstring lists no panics.
- Test: `prover-15/t.rs`.

**8. Unknown TLV tags are written without any checks** (confirmed by reading)
- Where: `md-codec/src/tlv.rs` `TlvSection::write`, line 197.
- Does: a tag above 31 is cut to 5 bits (so 0x21 is read back as fingerprints), and a tag that repeats a known or another unknown tag is still written. Either way the round-trip the code promises breaks.

**9. The varint decoder accepts non-canonical encodings** (demonstrated)
- Where: `md-codec/src/varint.rs` `read_varint`, line 51.
- Input: a 22-bit extended-form encoding of 5.
- Does: decodes to 5, but re-encoding gives 7 bits, so one value has several wire forms.
- Test: `prover-10/varint_noncanon.rs`.

**10. `--policy-id-fingerprint` prints to stdout** (confirmed by reading)
- Where: `encode.rs`, line 230.
- Does: the line is added after the md1 artifact, or is the only thing on stdout with `--out`. The code's own rule (§6a) says stdout carries only the artifact.
- Fix: `eprintln!`.

**11. `md inspect --json` says "keyless template" for wallet policies that include xpubs** (confirmed by reading)
- Where: `md-cli/src/cmd/inspect.rs`, line 85. The output class is hardcoded to Template.
- Does: the note says there are no keys while stdout contains them.
- Fix: choose WatchOnly when `is_wallet_policy()`, as `encode` and `repair` do.

**12. `--fingerprint @0=+f+f+f+f` is accepted** (demonstrated)
- Where: `md-cli/src/parse/keys.rs` `parse_fingerprint`, line 540. `from_str_radix` accepts a `+`.
- Does: accepted as fingerprint `0f0f0f0f`.
- Test: `prover-6/t.rs`.
- Fix: check that every character is a hex digit.

**13. mk1 deduplication removes only whitespace, not `-` or `,`** (demonstrated)
- Where: `md-cli/src/seat/input.rs` `dedupe_strings`, line 157.
- Does: a hyphen-grouped card and the same card pasted unbroken are not recognised as duplicates.
- Test: `prover-7/t.rs`.
- Fix: use `strip_display_separators`.

**14. `md decode` always prints the origins note** (confirmed by reading)
- Where: `md-cli/src/cmd/decode.rs` `emit_origins_stderr`, line 158.
- Does: the note and the `--path` hint appear even for a single-key card with the canonical origin, which the doc says should get no note.

**15. `gen_ap2_grind` is not deterministic** (demonstrated)
- Where: `md-cli/src/bin/gen_ap2_grind.rs`, line 224. The first thread to find a match wins, rather than the smallest matching counter.
- Does: with several matches, the result depends on the thread count, which breaks the fixture-determinism guarantee. This is a developer tool.
- Test: `prover-4/t.rs`.
- Fix: use `fetch_min` and stop each thread only once it passes the best counter so far.

## Rejected
- **`Check(RawPkH)` renders a double check** (`render.rs:611`). Rejected because the repo's own miniscript test shows `c:expr_raw_pkh(..)` parses back to exactly Wsh→Check→RawPkH, so the output is correct. Only a doc comment is misleading.

## Not checked
These 59 suspicions were rated 2 or lower and were not sent to provers. The notable ones:
- **Panics and truncation:** in release builds, `bitstream.rs:114` can panic and a `LianaUnspendable` at the wrong wire version (`tree.rs:216`) silently turns into NumsPoint, because each guard is only a `debug_assert`. Key indices are silently truncated in `tree.rs:126`, and `tlv.rs:117` has no index bounds check. `policy_shape.rs:374` overflows `depth + 1` at nesting depth 256.
- **Hardened index handling:** `to_miniscript.rs:356` and `:262` build an aliased index from values of 2^31 or more instead of returning an error.
- **Wrong values or messages:** plain `sh` gets BIP-48 script type 2' (`compose/mod.rs:88`), and `OlderUnits` keeps the disable bit.
- **Missing warnings:** `--json decode` skips the warning about a spend path that needs no key, and a descriptor where some keys lack origins gets no warning.
- **Lenient parsing:** `+` and leading zeros are accepted in several index parsers, and duplicate multipath alternatives like `<0;0>` are accepted.
- **Smaller items:** stale comments and help text, and lenient parsing in xtask.

## What was read
Every non-test line of all 86 files was read by both a reader and a reviewer. A few reviewers read less:
- `coordinator/table.rs` (generated data): about 70 of 254 lines, plus scripted checks over all 246 entries.
- `template.rs`: the second read covered lines 1-510 closely.
