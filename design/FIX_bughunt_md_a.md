# FIX-A: md-cli intake and output (md bug hunt, small-fix track)

Findings: #1 #3 #4 (CLI half) #10 #11 #12 #14, graded and ruled in
`design/agent-reports/grade-bughunt-md.md` (rulings R-1, R-3, R-4, R-10,
R-11, R-12, R-14 bind). Repo: descriptor-mnemonic, crate `md-cli` only.
Manual lockstep: mnemonic-toolkit `docs/manual/src/40-cli-reference/42-md.md`
lines 40 and 43. Review class: A (printed text, manual) and B (address and
descriptor derivation).

Read the STOP sections (§7) first: S-1 and S-4 must be answered before the
build can close green.

## 1. Build order and base

1. FIX-B (md-codec) builds and merges first.
2. FIX-A's base is FIX-B's merged head on `main`. Rebase `fix/bughunt-a`
   (this doc plus the RED rows in `crates/md-cli/tests/bughunt.rs`) onto it
   before writing any production code.
3. FIX-A rows whose expected output depends on a FIX-B codec change: **none**.
   - 04b and 04c are the only rows a FIX-B change reaches. On B's head before
     FIX-A they still fail, but with the codec text
     (FIX-B's `VarintOverflow` Display for 2147483648, exit 1) instead of today's exit 0. After
     FIX-A the lexer refuses first, so their expected text is the hardened
     message whatever the codec does.
   - Row 01 uses `older(65536)` at top level, not inside `thresh`, so FIX-B's
     R-2 walk change does not alter it.
4. On B's head, re-run the §3 rows once to a file before GREEN and confirm
   all 10 fail for the reasons in §3's table (04b/04c may show the varint
   text instead of exit 0; that is the only allowed difference).

## 2. Spec delta (exact texts)

All refusals use exit code 1 and existing error variants. No new flag, type,
command, error variant or text.

### 2.1 R-1 (#1): `md encode --json` checks relative timelocks

- `crates/md-cli/src/cmd/encode.rs`, json branch (`if args.json {`, line 122):
  its first statement becomes
  `md_codec::validate::validate_relative_timelocks(&descriptor.tree)?;`
  (the same call the text path makes at line 188), before the JSON object is
  built.
- Expected for `md encode 'wsh(and_v(v:pk(@0/<0;1>/*),older(65536)))' --json`:
  stdout empty; stderr byte-identical to the text path:
  `md: codec error: older(65536) is not what consensus enforces: BIP-68 reads only the low 16 bits (and bit 22 for units), so this locks for 0 blocks, not 65536. A relative timelock cannot exceed 65535 blocks -- use an absolute after() height for longer delays`
  plus newline; exit 1.

### 2.2 R-3 (#3): `--json` chunks exactly when the text path does

- In the json branch, replace the `if args.force_chunked { split … } else
  { encode_md1_string … }` block with a call to
  `mint_md1_cards(&descriptor, args.force_chunked)?` (encode.rs:288):
  - `(cards, Some(csid))`: insert `chunk_set_id` = `format!("0x{csid:05x}")`
    and `chunks` = `cards` (a JSON array of strings);
  - `(cards, None)`: insert `phrase` = `cards[0]`.
- All other keys (`schema`, `network`, `policy_id_fingerprint`) and the
  stderr advisories after the `println!` are unchanged. Key order is
  serde_json's (alphabetical), as today.
- Expected for row 03's command
  `md encode --json 'wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))' --key @0=XPUB_A --key @1=XPUB_B --fingerprint @0=73c5da0a --fingerprint @1=aabbccdd --path m/48h/0h/0h/2h --group-size 0`
  (constants as in the row): exit 0, stdout parses to
  `{"chunk_set_id":"0x95075","chunks":[<the 4 cards below>],"network":"mainnet","schema":"md-cli/2"}`:
  - `md1fj5r4pspq2tvyyy4qqxppsg2z7z883w6pt24menw3tsf9m5rru59s2su80aw2q4wgdpapguu6w2y2jygprc`
  - `md1fj5r4psf875x67p5s3wem7sgluxl3d2a3syx3m7halwd7s7d5e8l2xm3y3xzfmadfj6e2wnj3gvx34m0wnt`
  - `md1fj5r4pshsdlkvt6f6cthyl98xtqcj3lluycagp8vv3nmlgam2ug04zw29zsq0u7st858yuyz646z0r98kyg`
  - `md1fj5r4pslq8kxupyjz229sgx620d93cwcs6he5skltczfzylx0ndtm9fdvtp6hyhaccqrfrshqj9459eg`
  These are byte-identical to the text path's four cards and to today's
  `--json --force-chunked` output.

### 2.3 R-4 (#4, CLI half): a bare multipath alt >= 2^31 is hardened

- `crates/md-cli/src/parse/template.rs`, `lex_placeholders`, line 294: the
  hardened test `n.ends_with('\'') || n.ends_with('h')` gains a third
  disjunct: `n` parses as `u32` and the value is `>= 0x8000_0000`. The error
  is the existing `CliError::TemplateParse` with the existing format string,
  so the message is:
  ``md: template parse error: @0 multipath alt `2147483648` is hardened; hardened derivation is impossible on a watch-only (xpub) card``
  exit 1, stdout empty.
- Applies to `encode`, `address` and `descriptor` (shared lexer). Any alt
  position (`<1;2147483648>` is refused the same way, naming
  `2147483648`).
- Values in [2^29, 2^31) are not touched by FIX-A (see STOP S-1 for what
  FIX-B does to them). A value that does not parse as `u32`
  (e.g. `4294967296`) keeps today's text
  ``@0 multipath alt `4294967296` is not a bare unsigned integer`` unless
  STOP S-3 rules otherwise.

### 2.4 R-10 (#10): `--policy-id-fingerprint` goes to stderr

- encode.rs:230 `println!(` becomes `eprintln!(`. The text is unchanged:
  `policy-id-fingerprint: 0x%08x`.
- The `--json` branch is unchanged: the fingerprint stays the JSON field
  `policy_id_fingerprint` on stdout.
- Expected for `md encode 'wpkh(@0/<0;1>/*)' --policy-id-fingerprint`:
  stdout exactly `md1yqpqqxqq8xtwhw4xwn4qh\n`; stderr has a line exactly
  `policy-id-fingerprint: 0x3ace1082`; exit 0.

### 2.5 R-11 (#11): inspect calls a keyed card watch-only

- `crates/md-cli/src/cmd/inspect.rs` lines 85 (`--json`) and 150 (text): the
  `OutputClass::Template` argument becomes
  `OutputClass::WatchOnly` when `descriptor.is_wallet_policy()`, else
  `OutputClass::Template` (the same choice encode.rs:162 and repair.rs:122
  make).
- Expected stderr line for a keyed card (row 11/11b input):
  `note: stdout is watch-only — public keys only, cannot spend`; the text
  `keyless` does not appear on stderr. Keyless cards keep
  `note: stdout is a keyless descriptor template (no keys)`.

### 2.6 R-12 (#12): fingerprint characters must be hex digits

- `crates/md-cli/src/parse/keys.rs`, `parse_fingerprint` (line 522): inside
  the per-2-char loop, before `u8::from_str_radix`, a chunk whose bytes are
  not all `is_ascii_hexdigit` returns the existing
  `CliError::BadFingerprint { i, why: format!("invalid hex byte: {s}") }`.
- Expected for `md encode 'wpkh(@0/<0;1>/*)' --fingerprint @0=+f+f+f+f`:
  stderr exactly `md: --fingerprint @0: invalid hex byte: +f\n`, stdout
  empty, exit 1.

### 2.7 R-14 (#14): no origins note for a single canonical key

- `crates/md-cli/src/cmd/decode.rs`, `emit_origins_stderr` (line 153): after
  the `expanded.is_empty()` return, return early when all hold:
  `expanded.len() == 1`, `expanded[0].fingerprint.is_none()`, and
  `md_codec::canonical_origin::canonical_origin(&d.tree).as_ref() == Some(&expanded[0].origin_path)`.
  This suppresses both the note and its `--path` hint. Every other case is
  unchanged.
- Expected for `md decode md1yq802gggqpsqwgtua24e7ssf3` (wpkh at
  m/84'/0'/0'): stdout exactly `wpkh(@0/<0;1>/*)\n`; stderr exactly
  `note: stdout is a keyless descriptor template (no keys)\n`; exit 0.

### 2.8 Manual lockstep (toolkit, `docs/manual/src/40-cli-reference/42-md.md`)

Base: toolkit `origin/master` f6355e22 (lines 40 and 43 checked there). The builder makes a toolkit worktree;
this FIX does not edit the toolkit.

- Line 40 old:
  ``| `--policy-id-fingerprint` | print the freshly-computed PolicyId fingerprint after the phrase |``
- Line 40 new:
  ``| `--policy-id-fingerprint` | print the freshly-computed PolicyId fingerprint on stderr after the card (stdout carries only the md1 artifact) |``
- Line 43 old:
  ``| `--json` | emit JSON output |``
- Line 43 new:
  ``| `--json` | emit JSON output: `phrase` for a single-string card, `chunk_set_id` + `chunks` when the card is chunked (as the text path chooses, or with `--force-chunked`) |``

## 3. RED rows (`crates/md-cli/tests/bughunt.rs`, committed with this doc)

Constants in the file: `XPUB_A` (signet-test xpub), `keyed_cards()` (the four
cards in §2.2), `HARDENED_2_31` (the §2.3 text plus `\n`), `WATCH_ONLY_NOTE`,
`KEYLESS_NOTE` (the §2.5 texts). Rows marked json need the default `json`
feature.

| row | command (via `Command::cargo_bin("md")`) | expected after FIX-A | failure on unmodified code (b2bfdd63, run) |
|---|---|---|---|
| `bughunt_md_01_json_encode_refuses_truncated_timelock` (json) | `encode 'wsh(and_v(v:pk(@0/<0;1>/*),older(65536)))'` with and without `--json` | json exit == text exit == 1; json stderr == text stderr (§2.1); json stdout empty | `--json exited 0` |
| `bughunt_md_03_json_encode_chunks_large_policy` (json) | row 03's command (§2.2) | §2.2 JSON, exit 0 | caps-single-strings refusal (`payload is 246 data symbols …`), exit 1 |
| `bughunt_md_04_multipath_hardened_range_refused_as_hardened` | `encode 'wpkh(@0/<2147483648;1>/*)'` | exit 1, stderr == `HARDENED_2_31` | refused, but not as hardened: varint message |
| `bughunt_md_04b_address_multipath_hardened_range_refused_as_hardened` (new) | `address --template 'wpkh(@0/<2147483648;1>/*)' --key @0=XPUB_A --count 1` | exit 1, stdout empty, stderr == `HARDENED_2_31` | exit 0, stdout `bc1q0qgl7sdpw0vvdn3g0kha052hkjugcm8zujqhea` |
| `bughunt_md_04c_descriptor_multipath_hardened_range_refused_as_hardened` (new) | `descriptor --template 'wpkh(@0/<2147483648;1>/*)' --key @0=XPUB_A` | exit 1, stdout empty, stderr == `HARDENED_2_31` | exit 0, stdout `wpkh(xpub…/<2147483648;1>/*)#26hvecv9` |
| `bughunt_md_10_policy_id_fingerprint_not_on_stdout` | `encode 'wpkh(@0/<0;1>/*)' --policy-id-fingerprint` | §2.4 | non-artifact stdout line `policy-id-fingerprint: 0x3ace1082` |
| `bughunt_md_11_inspect_json_keyed_card_not_called_keyless` (json) | `inspect --json <keyed_cards>` | success; a stderr line == `WATCH_ONLY_NOTE`; no `keyless` | keyed card called keyless |
| `bughunt_md_11b_inspect_text_keyed_card_not_called_keyless` (new) | `inspect <keyed_cards>` | success; a stderr line == `WATCH_ONLY_NOTE`; no `keyless` | keyed card called keyless |
| `bughunt_md_12_fingerprint_with_plus_signs_refused` | `encode 'wpkh(@0/<0;1>/*)' --fingerprint @0=+f+f+f+f` | §2.6 | `+f+f+f+f` accepted, exit 0 |
| `bughunt_md_14_decode_single_key_canonical_card_no_origins_note` | `decode md1yq802gggqpsqwgtua24e7ssf3` | §2.7 | origins note and `--path` hint printed |

Rows 01, 03, 04, 10, 11, 12, 14 are the triage rows (names unchanged, now
asserting the ruled exact texts). 04b, 04c and 11b are new; 11b's name is
R-11's, 04b/04c names are this doc's proposal (STOP S-2).

Evidence:
- RED on unmodified code: `/scratch/cache/bughunt/md/fixa-red-unmodified.log`
  (filter `test(/(^|::)bughunt_md_/)`, md-cli: 12 run, 12 failed: the 10
  above plus private rows 06 and 13, see S-4).
- GREEN check with the hint patches applied as a scratch change (reverted,
  not committed): `/scratch/cache/bughunt/md/fixa-green-scratch.log`,
  10 run, 10 passed. Workspace run with the same scratch:
  `/scratch/cache/bughunt/md/fixa-green-scratch-ws.log`, 1560 run, 1549
  passed, 11 failed = the 8 FIX-B codec rows, private rows 06 and 13, and
  `cmd_encode::encode_with_policy_id_fingerprint_prints_two_lines` (§4).

#13 (Q-B) probe: `md descriptor --from-mk1` fed the same mk1 card twice
(once hyphen-grouped, once unbroken) refuses: exit 1, "not a decodable mk1
string: invalid data-part length: 135". md fails closed, so #13 stays Minor
and no #13 row is added.

## 4. Tests that change

- `crates/md-cli/tests/cmd_encode.rs:151`
  `encode_with_policy_id_fingerprint_prints_two_lines` is renamed
  `encode_with_policy_id_fingerprint_prints_it_on_stderr`, and its
  `.stdout(predicate::str::contains("policy-id-fingerprint: 0x"))` becomes
  `.stderr(predicate::str::contains("policy-id-fingerprint: 0x"))`. (R-10.)
- No other existing test changes. Checked: `cli_output_class.rs` inspect
  tests use a keyless fixture; `decode_origins_note.rs` uses a multi-key
  card; `cmd_decode.rs:164` uses a non-canonical `wsh(and_v(...))` card; no
  other test uses a 2^31 alt or the json single-string refusal. The scratch
  workspace run (§3) confirms: only this test broke.

## 5. Build steps

1. Rebase onto FIX-B's merged head (§1); re-run the §3 rows once to a file,
   confirm RED.
2. Apply §2.1 to §2.7 in md-cli; apply §4.
3. Run the gate (§6) once to a file; grep it.
4. Toolkit worktree: apply §2.8; run the toolkit's `manual` CI job locally.
5. Commit (production + test change in one GREEN commit, quoting the matched
   row count); do not push unless the build brief says so.

## 6. Gate

Run each once to a file and grep it:

- `cargo nextest run --locked --workspace`
- `cargo test --locked --doc --workspace`
- `cargo fmt --all --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- The FIX-A rows by exact name:
  `cargo nextest run --locked -p md-cli -E 'test(/(^|::)bughunt_md_(01|03|04|04b|04c|10|11|11b|12|14)_/)'`
  Expected: **10 run, 10 passed**. A short or empty count is a FAIL. Also
  `cargo nextest run --locked -p md-cli -E 'test(=encode_with_policy_id_fingerprint_prints_it_on_stderr)'`:
  1 run, 1 passed.
- Toolkit: its `manual` CI job (mdBook build plus its flag/doc lint and
  mutation self-test), on the toolkit worktree.

The workspace nextest cannot be green while the private rows 06 and 13 stay
RED (STOP S-4).

## 7. STOP sections (undecided; the builder does not choose)

**S-1. R-4 is internally inconsistent about [2^29, 2^31).** R-4 says
"Values in [2^29, 2^31) are unchanged: address and descriptor accept them",
and in the same ruling makes the codec's `to_miniscript` return
`VarintOverflow` for any value > 2^29-1. `md address` and `md descriptor`
reach `to_miniscript`, so after FIX-B they refuse e.g. `<536870912;1>`
(today: exit 0, `bc1qghsy27425xapuv6gltah6uf2q9w5usrjucr5uy` /
`…#akxwxgma`). Which holds: (a) the codec rule (address/descriptor refuse
[2^29, 2^31) with the codec's VarintOverflow text, and the "unchanged"
sentence is void), or (b) address/descriptor keep accepting, which needs a
different codec change. Its owner is FIX-B; FIX-A adds no [2^29, 2^31) row
until it is ruled. If (a), the exact stderr text and whether FIX-A or FIX-B
owns the address/descriptor rows must be ruled.

**S-2. Names of the new #4 rows.** R-4 asks for address and descriptor rows
but gives no names. This doc uses
`bughunt_md_04b_address_multipath_hardened_range_refused_as_hardened` and
`bughunt_md_04c_descriptor_multipath_hardened_range_refused_as_hardened`
(the triage row 04 pattern). Confirm or rename.

**S-3. A bare alt above u32::MAX.** R-4 says "a bare alt whose value is >=
2^31 is refused with the existing hardened message". `<4294967296;1>` is
>= 2^31 but does not parse as `u32`, and today gets
``@0 multipath alt `4294967296` is not a bare unsigned integer``. Does it
keep that text (this doc's §2.3 as written) or get the hardened message?

**S-4. The private RED rows 06 and 13 keep the gate red.**
`seat::compose::bughunt::bughunt_md_06_comparison_form_independent_of_mint_policy`
(#6, deferred to the toolkit migration) and
`seat::input::bughunt::bughunt_md_13_dedupe_strings_strips_display_separators`
(#13, Minor, not fixed) are committed RED in md-cli and are in neither FIX.
The workspace nextest fails while they remain. Rule how they are disposed
(`#[ignore]` with a reason naming the FOLLOWUPS line, deletion, or kept and
excluded from the gate), and by which FIX.
