**⚠ Proposed Important grades below (#1, #2, #5, #7, #8, #9, #11, #12, #16); an Opus pass decides.**

# Bug-hunt triage, md (worktree bughunt/triage, tests commit f26ab4b5 on 1f6c5a5f)

## 1. Gate and sets
Gate (once, tip f26ab4b5, `/scratch/cache/bughunt/md/gate.log`):
`cargo nextest run --locked --workspace -E 'test(/bughunt_md_/)' --no-fail-fast` -> 17 tests run: 0 passed, 17 failed, 1544 skipped.
Failing set: 01 02 03 04 05 06 07 08 08b 09 10 11 12 13 14 16 16b.
CONFIRMED set: #1 #2 #3 #4 #5 #6 #7 #8 #9 #10 #11 #12 #13 #14 #16 (rows 08/08b and 16/16b are one bug each). Failing set == CONFIRMED rows exactly. #15 has no row (UNRESOLVED, STOP below).
Verdicts: 15 CONFIRMED, 0 FALSE POSITIVE, 1 UNRESOLVED (#15). Every CONFIRMED row also passed with its scratch fix (`/scratch/cache/bughunt/md/fix-NN.patch`, reverted; fix-08/16 cover both rows). fmt and clippy -D warnings pass. Reverted tree showed only test files plus the one-line hooks (rustfmt splits each hook to 3 lines).
Note: #4 and #6/#13 are CONFIRMED on the diagnosis/private-item as the report names them (see table).

## 2. Table
Test names are `bughunt_md_NN_<slug>` (files: md-codec/tests/bughunt.rs = 02 05 07 08 08b 09 16 16b; md-cli/tests/bughunt.rs = 01 03 04 10 11 12 14; md-cli/src/seat/compose_bughunt.rs = 06; seat/input_bughunt.rs = 13).

| # | verdict | test | unmodified-run evidence | fix evidence | CLI reach | grade | why |
|---|---|---|---|---|---|---|---|
| 1 | CONFIRMED | 01_json_encode_refuses_truncated_timelock | "--json exited 0 for older(65536); stdout={ ... "phrase": "md1yqpqqxpye55uqqqsqqq25gc2qn7xqk5r"" | fix-01: call validate_relative_timelocks in the --json branch; PASS | `md encode 'wsh(and_v(v:pk(@0/<0;1>/*),older(65536)))' --json` -> prints card, exit 0. Same without --json: exit 1 "older(65536) is not what consensus enforces ... locks for 0 blocks" | I | check silently not run on --json; card records an unenforced timelock (BIP-68: only bit 31, bit 22, bits 0-15 matter). |
| 2 | CONFIRMED | 02_thresh_older_truncation_checked | "older(210000) inside thresh must be RelativeTimelockTruncated, got Ok(())" | fix-02 (walk_older descends Body::Variable); PASS | `md encode 'wsh(thresh(2,pk(@0/<0;1>/*),s:pk(@1/<0;1>/*),snl:older(210000)))' --path bip48` -> card, exit 0 | I | same class as #1 via thresh; wrong card. |
| 3 | CONFIRMED | 03_json_encode_chunks_large_policy | "--json failed where the text path chunks: md: codec error: payload is 246 data symbols; ... caps single strings at 80" | fix-03 (use mint_md1_cards in --json); PASS | 3-key policy with --json: exit non-zero as above (test's keyed_args) | M | fails closed, names problem; conditions: payload >80 symbols, --json, text path works. |
| 4 | CONFIRMED (diagnosis only) | 04_multipath_hardened_range_refused_as_hardened | "refused, but not as hardened: md: codec error: varint value 2147483648 exceeds single-extension range (max 2^29 - 1)" | fix-04 (v >= 2^31 -> hardened msg); PASS | `md encode 'wsh(multi(2,@0/<2147483648;1>/*,@1/<0;1>/*))'` exit 1, varint message | N | fails closed; wrong wording only. BIP-32: "hardened child keys use indices 2^31 through 2^32-1". |
| 5 | CONFIRMED | 05_split_branches_nested_or_not_merged | "a branch needs @1 AND older(10), but they are or_d alternatives: Branch { ... slots: [0, 1], ... locks: [Lock { kind: OlderBlocks, value: 10 }]" | fix-05; PASS | policy_shape feeds `md shape-key` (not demonstrated on CLI: needs real xpub fixtures) | I | wrong result: merged branch misstates spend paths. Open Q below. |
| 6 | CONFIRMED | 06_comparison_form_independent_of_mint_policy | "comparison_form aborted on a card mint refuses: Some(Codec(DuplicateKeySlots { a: 0, b: 1, n: 2 }))" | fix-06 (encode_payload_unadmitted); PASS | private item (seat::compose); seating flow only, not run | M | fails closed; conditions: same xpub in two slots, seat/verify path. Verb may migrate to toolkit: Minor deferred. |
| 7 | CONFIRMED | 07_canonicalize_out_of_range_tlv_index_no_panic | "panicked at crates/md-codec/src/canonicalize.rs:137:16: index out of bounds: the len is 2 but the index is 5" | fix-07; PASS | library API only: `md_codec::canonicalize` entry (canonicalize_placeholder_indices) | I | panic on card input. |
| 8 | CONFIRMED | 08_unknown_tlv_tag_above_31..., 08b_unknown_tlv_tag_duplicating_known_tag... | "wrote it, then could not read it back: BitStreamTruncated { requested: 8, available: 7 }" | fix-08; both PASS | library API only: Descriptor encode of unknown TLV | I | writes an unreadable card. Open Q (refuse vs error). |
| 9 | CONFIRMED | 09_varint_noncanonical_extension_refused | "non-canonical 22-bit encoding of 5 must be refused, decoded as Ok(5)" | fix-09; PASS | library API only (decode) | I | canonicality check missing: malleable card accepted. Open Q (refuse vs warn). |
| 10 | CONFIRMED | 10_policy_id_fingerprint_not_on_stdout | "non-artifact line on stdout: "policy-id-fingerprint: 0x3ace1082"" | fix-10 (eprintln); PASS. Breaks existing `encode_with_policy_id_fingerprint_prints_two_lines` (not run, per brief) | `md encode 'wpkh(@0/<0;1>/*)' --policy-id-fingerprint` -> line on stdout, exit 0 | M | fails nothing; opt-in flag, pinned by an existing test. Open Q. |
| 11 | CONFIRMED | 11_inspect_json_keyed_card_not_called_keyless | "keyed card called keyless: note: stdout is a keyless descriptor template (no keys)" | fix-11 (WatchOnly when is_wallet_policy; both inspect sites); PASS | `md inspect --json <keyed chunks>` -> exit 0 with keyless note (run earlier) | I | wrong advisory for a keyed card. inspect slated to move to toolkit: still fixed as I. |
| 12 | CONFIRMED | 12_fingerprint_with_plus_signs_refused | "`+f+f+f+f` accepted; stdout=md1fjfncpspq2tvyyy4qqxppsg2z7zq7rc0pl24menw3tsf9m5rru59s2su80aw2q4wgdpapglaju0gxfs4ycg" | fix-12 (ascii-hex check); PASS | `md encode 'wpkh(@0/<0;1>/*)' --fingerprint @0=+f+f+f+f --key @0=<xpub>` -> cards, exit 0 | I | malformed fingerprint silently accepted (parsed as 0x0f...): wrong data in card. |
| 13 | CONFIRMED | 13_dedupe_strings_strips_display_separators | "same card pasted two ways was not deduped: ["mk1qqqq...", "mk1qqqq-qqqq-..."] left: 2 right: 1" | fix-13 (strip_display_separators); PASS | private item (seat::input); mk1 seat input, not run | M | downstream outcome not checked here; conditions: same card pasted twice, once with hyphens. Seat flow may migrate. |
| 14 | CONFIRMED | 14_decode_single_key_canonical_card_no_origins_note | "note printed: note: key origins carried by this card (not shown in the template): @0: m/84'/0'/0'" | fix-14 (canonical_origin match returns early); PASS | `md decode md1yq802gggqpsqwgtua24e7ssf3` -> note printed, exit 0 | N | stderr noise contradicting the fn's own doc comment. Open Q (what is canonical). |
| 15 | UNRESOLVED | none | n/a | n/a | developer tool only (gen_ap2_grind) | n/a | grind loop is inline in main (bin/gen_ap2_grind.rs ~205-240), no testable fn, real run ~2^32 SHA-256; a test needs a refactor. STOP for this row only. |
| 16 | CONFIRMED | 16_single_string_non_ascii_alias_refused, 16b_chunk_non_ascii_alias_refused | "non-ASCII alias "md1Źq9dsssqrqq20vuhf9agdphs" decoded as if it were "md1yq9dsssqrqq20vuhf9agdphs"" | fix-16; both PASS | `md decode 'md1Źq9dsssqrqq20vuhf9agdphs'` -> template + note, exit 0 | I | check silently not run: non-ASCII accepted as a card. BIP-93 (grep): "a codex32 string MUST be entirely uppercase or entirely lowercase"; alphabet `qpzry9x8gf2tvdw0s3jn54khce6mua7l` (bip-0093.mediawiki:82,178). |

External text (saved in /scratch/cache/bughunt/md/bips/): BIP-68 "If bit (1 << 31) of the sequence number is set, then no consensus meaning is applied"; "Bit (1 << 22) determines if the relative lock-time is time-based or block based" (bip-0068:30,36). BIP-32 "The hardened child keys use indices 2^31 through 2^32-1" (bip-0032:66).

## 3. Open product questions
- #5: exact expected branch shape for nested or_d under and_v (cross product vs other).
- #8: unknown TLV tag >31 or duplicating a known tag: refuse at encode, or error?
- #9: non-canonical varint: refuse or accept with warning?
- #10: should --policy-id-fingerprint stay on stdout (existing test pins it)?
- #14: what counts as the "canonical origin" for the note suppression (BIP-84 path for wpkh assumed).

## 4. Noticed, not tested
- Text-path inspect fix for #11 (second OutputClass::Template site) is not covered by a row.
- #6 and #13 downstream behaviour (what the seat flow does after the bug) not exercised on the CLI.
- #5 and #7-#9 CLI reach not demonstrated.

Counts: 15 CONFIRMED, 0 FALSE POSITIVE, 1 UNRESOLVED (#15); proposed I: 9, M: 4 (#3 #6 #10 #13), N: 2 (#4 #14); gate 17 failed == CONFIRMED rows.
