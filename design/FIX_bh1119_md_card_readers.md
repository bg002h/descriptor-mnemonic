# FIX_bh1119_md_card_readers: `md repair` emits each corrected card in its input's case (bh-md-I3 Important; bh-md-I4 split out)

Small-fix track (bughunt-1119 FIX table row #40). Ruling: coordinator
`agent-reports/architect-bughunt-1119-questions.md` T10 (decided). Repo descriptor-mnemonic, branch
`fix/md-card-readers` from main `1097371be794a18fbc471975245b846a644e7a8c`. Crate `md-cli` only; no md-codec
change, no wire change, no new flag, type, command or printed text (the only holder-visible change is the
letter case of a corrected card).
Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22.

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (`rust-toolchain.toml`); `--locked`; nextest
`--build-jobs 6 -j 6` (2 on a loaded host); never `--release`. No keys in any row (keyless templates only).

**No STOP is open.** Source files this FIX changes (for build ordering): `crates/md-cli/src/cmd/repair.rs`,
new `crates/md-cli/tests/bh1119_md_card_readers.rs`, `CHANGELOG.md`, new `scripts/fix-bh1119-md-card-readers-gate.sh`.

## 0. bh-md-I4 splits to its own full-track FIX (T10)

T10: `md bytecode` prints the bytes the card carries as the decoder consumed them (payload bit length, padding
dropped), never re-minted; for a chunked set, the reassembled payload. "If md-codec has no public accessor for that,
I4 splits into its own full-track FIX." Checked at base `1097371b`:
- the consumed bit length is not exposed: `decode_payload_with_opts` (`crates/md-codec/src/decode.rs:73`) returns
  only the `Descriptor`; its `BitReader` (`crates/md-codec/src/bitstream.rs`) has no public position getter;
  `decode_md1_string_with_opts` (`decode.rs:187`) passes the symbol-aligned count from `unwrap_string`;
- the reassembled payload is not exposed: `reassemble_with_opts` (`crates/md-codec/src/chunk.rs:375`) builds
  `full_bytes` privately and decodes it as `full_bytes.len() * 8` bits.

So I4 leaves this FIX: it needs a new public md-codec accessor, a codec release and the md-cli pin bump
(full track; proposed name `FIX_bh1119_md_bytecode_wire_bytes`, coordinator assigns). Evidence, run on the base:

| command | exit | output |
|---|---|---|
| `md bytecode md1gqpqqxqql29tezq74c8ha` | 0 | `payload-bits: 36` / `payload-bytes: 5` / `hex: 2002001800` (the card carries `4002001800`) |
| `md bytecode md1gzfdsssjuqqcreyygvauszq6hnnx9up` | 1 | `md: codec error: wire kind 1 (Liana unspendable internal key) with a sortedmulti_a leaf is refused: ...` |
| `md decode md1gzfdsssjuqqcreyygvauszq6hnnx9up` | 0 | `tr(UNSPENDABLE(liana),sortedmulti_a(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))` ... |

The rest of this document is I3 only.

## 1. The defect (RED evidence, run by the author on the base)

`apply_corrections` (`crates/md-cli/src/cmd/repair.rs:235-253`) writes each correction in the case of the DAMAGED
character (`chars[abs_idx].is_ascii_uppercase()`, `:245`). The codec reports `now` in lowercase. A digit is never
uppercase, so when an all-uppercase card (the QR / BIP-173 uppercase form) has a letter misread as a digit, the
corrected card gets one lowercase letter and is mixed case, which `md decode` (and every other md reader) refuses:

```
$ md repair MD12QPQQXQQ8XTWHW4XWN4QH
# Repair report
#   md1 chunk 0: 1 correction at position 0: '2' -> 'y'
MD1yQPQQXQQ8XTWHW4XWN4QH
note: stdout is a keyless descriptor template (no keys)
exit=5
$ md decode MD1yQPQQXQQ8XTWHW4XWN4QH
md: codec error: codex32 decode error: string mixes upper and lower case (BIP-173 forbids mixed case)
exit=1
```

Every caller of `apply_corrections` goes through `build_reports` (`repair.rs:133`): the normal path (`run`, single and
multi-string, text and `--json`) and the unsupported-wire-version path (`corrected_but_unsupported`, `:167`). All three
were run on the base and emit a mixed-case card (rows R1-R6 below).

## 2. Spec delta (behaviour)

S1. Each corrected card `md repair` prints (text form and `--json` `corrected_chunks` / `corrected_chunk`) is in the
letter case of the input string it corrects: if that input string contains an uppercase letter, every corrected
character is written in uppercase; otherwise it is written as the codec reports it (lowercase). Input strings are
single case by the time correction runs (a mixed-case string is refused first, exit 2, unchanged), and every input
string starts with the HRP `md`/`MD`, so its case is always determined.

S2. The case is decided per input string, not per set. A set whose strings differ in case (each single case; the
decoder accepts such a set today) is emitted with each corrected string in its own input's case.

S3. Unchanged: exit codes (0 / 5 / 2), the `# Repair report` lines, the JSON `corrected_positions` `was` / `now`
values (the codec's lowercase characters, as today for uppercase letter damage too), the `original_chunk` field,
stderr lines, the output-class note, and lowercase input behaviour (byte-identical output).

Code shape (builder's choice of form): `apply_corrections` computes one flag from `original` (any ASCII uppercase char)
and uses it in place of `chars[abs_idx].is_ascii_uppercase()`; the M-9 comment at `:241-244` is updated to say the
case comes from the whole input string because the damaged character may be a digit. The toolkit's `mnemonic repair` never emits mixed case: it re-encodes every corrected chunk in lowercase (mnemonic-toolkit 04c56bfa `crates/mnemonic-toolkit/src/repair.rs:792` `encode_chunk`, :870); md keeps M-9's input-case rule, so the two differ in case for an uppercase input, as they already do today for letter damage. No toolkit change; the code comment does not cite the toolkit.

## 3. Help, manual, CHANGELOG

Help: unchanged. Toolkit manual (`docs/manual/src/40-cli-reference/42-md.md` `## md repair`, L639-): says nothing
about output case; no lockstep edit. GUI mirror: no flag change, none.

CHANGELOG (`CHANGELOG.md`, under `## md-cli [Unreleased]`, `### Fixed`), PROPOSED (architect confirms):

> - **`md repair` keeps an uppercase card uppercase.** When a letter on an all-uppercase md1 card (the QR form) was
>   misread as a digit, the corrected card was printed with that one letter in lowercase (for example
>   `MD1yQPQQXQQ8XTWHW4XWN4QH`), a mixed-case string `md decode` refuses. Each corrected card is now printed in the
>   case of the string it corrects (`MD1YQPQQXQQ8XTWHW4XWN4QH`), in text and `--json` output and for chunked sets
>   (bh-md-I3).

## 4. TRACE rows (new file `crates/md-cli/tests/bh1119_md_card_readers.rs`)

Fixtures (keyless templates; the chunked set was minted on the base by
`md encode 'wsh(sortedmulti(2,@0/<0;1>/*,...,@11/<0;1>/*))' --path bip48 --fingerprint @0=00000001 ... --fingerprint @11=0000000c`,
then uppercased; the v12 card is `cli_repair_unsupported_version.rs:31` `V12_CLEAN` uppercased):
- `S_DAMAGED = MD12QPQQXQQ8XTWHW4XWN4QH`, `S_CLEAN = MD1YQPQQXQQ8XTWHW4XWN4QH` (template `wpkh(@0/<0;1>/*)`)
- `C0 = MD1F0C9LQSPZ6TVYYY4QQXPPC2CPYDZK0ZDTPNKQQQQQQQZYQQQQQZYQQQQQPNQQQQQPZQCJG0YYNDM69P4`
- `C1_CLEAN = MD1F0C9LQSGQQQQP2SQQQQQESQQQQQWUQQQQQGSQQQQQYEQQQQQZ4QQQQQPWCQQQQQCQ7H9LLL0W28FY4`
- `C1_DAMAGED` = `C1_CLEAN` with data position 7 `G` replaced by `7`
- `c0_lower` = `C0` lowercased
- `V12_DAMAGED = MD12ZFDSSSJJTVYYW2FDSSJ54QQXPPCGSCU5E7M9JGAWLHG`, `V12_UPPER = MD1UZFDSSSJJTVYYW2FDSSJ54QQXPPCGSCU5E7M9JGAWLHG`

The test file states each fixture's provenance (the command above) and asserts `md decode C0 C1_CLEAN` exits 0
(fixture sanity) inside R3.

| row (test fn) | command | expected after the fix | on the base (seen) |
|---|---|---|---|
| R1 `i3_r1_repair_upper_digit_damage_text_emits_upper_card` | `md repair S_DAMAGED` | exit 5; stdout exactly `# Repair report\n#   md1 chunk 0: 1 correction at position 0: '2' -> 'y'\nS_CLEAN\n`; then `md decode S_CLEAN` exit 0, stdout first line `wpkh(@0/<0;1>/*)` | 3rd line `MD1yQPQQXQQ8XTWHW4XWN4QH`; decode of it exit 1 |
| R2 `i3_r2_repair_upper_digit_damage_json_emits_upper_card` | `md repair --json S_DAMAGED` | exit 5; JSON `corrected_chunks == [S_CLEAN]`, `repairs[0].corrected_chunk == S_CLEAN`, `repairs[0].original_chunk == S_DAMAGED`, `repairs[0].corrected_positions == [{position 0, was "2", now "y"}]` | `"MD1yQPQQXQQ8XTWHW4XWN4QH"` in both fields |
| R3 `i3_r3_repair_upper_chunked_digit_damage_text_emits_upper_set` | `md repair C0 C1_DAMAGED` | exit 5; stdout exactly `# Repair report\n#   md1 chunk 1: 1 correction at position 7: '7' -> 'g'\nC0\nC1_CLEAN\n`; then `md decode C0 C1_CLEAN` exit 0 | 4th line `MD1F0C9LQSgQQQQ...`; decode of the emitted pair exit 1 (mixed case) |
| R4 `i3_r4_repair_upper_chunked_digit_damage_json_emits_upper_set` | `md repair --json C0 C1_DAMAGED` | exit 5; `corrected_chunks == [C0, C1_CLEAN]`; `repairs` has one entry, `chunk_index 1`, `corrected_chunk == C1_CLEAN` | `MD1F0C9LQSgQQQQ...` |
| R5 `i3_r5_repair_set_keeps_each_strings_case` | `md repair c0_lower C1_DAMAGED` | exit 5; stdout lines 3-4 exactly `c0_lower` then `C1_CLEAN`; `md decode c0_lower C1_CLEAN` exit 0 | line 4 `MD1F0C9LQSgQQQQ...` |
| R6 `i3_r6_repair_upper_unsupported_version_emits_upper_card` | `md repair V12_DAMAGED` | exit 5; stdout exactly `# Repair report\n#   md1 chunk 0: 1 correction at position 0: '2' -> 'u'\nV12_UPPER\n`; stderr contains `md: repair: corrected, but this build cannot read wire version 12 (accepted: 4, 8)` | 3rd line `MD1uZFDSSS...` |
| C1 `i3_c1_repair_upper_letter_damage_unchanged` | `md repair MD1ZQPQQXQQ8XTWHW4XWN4QH` | exit 5; stdout line 3 `S_CLEAN` (passes on the base too) | same |
| C2 `i3_c2_repair_lower_digit_damage_unchanged` | `md repair md12qpqqxqq8xtwhw4xwn4qh` | exit 5; stdout line 3 `md1yqpqqxqq8xtwhw4xwn4qh` (passes on the base) | same |
| C3 `i3_c3_repair_mixed_case_input_refused_unchanged` | `md repair Md12qpqqxqq8xtwhw4xwn4qh` | exit 2; stdout empty; stderr `md: repair: codex32 decode error: chunk 0: string mixes upper and lower case (BIP-173 forbids mixed case)` (passes on the base) | same |

RED proof: R1-R6 fail on the base at the assertions named in the last column (the author ran every command on base
`1097371b` built with `cargo build --locked -p md-cli`; outputs as shown). GREEN proof: the author applied the S1 code
shape to a scratch copy, rebuilt, ran every command and saw exactly the "expected" column, and the existing repair
test binaries (`binary(/repair/) | test(/repair/)`, 25 tests) passed; the scratch change was reverted, not committed.

Coverage map: S1 -> R1, R2, R3, R4, R6; S2 -> R5; S3 -> R1-R4 and R6 (report lines, `was`/`now`, stderr), C1, C2, C3.

## 5. Tests that change

None. The one existing uppercase repair test,
`crates/md-cli/tests/cli_repair_unsupported_version.rs:251` `an_uppercase_card_is_corrected_in_uppercase_and_still_reads`
(the M-9 test), damages letters only (data position 9 `V` -> `T`, and the v12 card's position 0 `Q` -> `U`), so it passes on the base and under
S1 unchanged (it was in the 25 passing under the scratch change). The other repair tests use lowercase cards, whose
output is byte-identical under S1. New: the rows of section 4. The triage file
(`agent-reports/bughunt-1119/triage-md-t2.md`, `bh1119_md_t2.rs` I3 tests) is evidence only; R1-R2 and C1 replace it.

## 6. FOLLOWUPS

Closes nothing in `design/FOLLOWUPS.md` (I3 is not filed there). Lines to file after merge are listed in the
author's report (not appended here).

## 7. Gate line

`bash scripts/fix-bh1119-md-card-readers-gate.sh` (the builder commits it with its coordinator catalog row), on the
pattern of `scripts/fix-bh1119-md-change-branch-gate.sh`: step 1 every TRACE row of section 4 by exact nextest name
(match count 1 each) in `md-cli`, then run; step 2 the repo CI set (`cargo nextest run --locked --workspace
--all-targets --all-features --no-fail-fast`, `cargo test --locked --workspace --doc --all-features`, `cargo clippy
--locked --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`, `cargo build --locked -p
md-cli --no-default-features`, `RUSTDOCFLAGS=-D warnings cargo doc --locked --workspace --no-deps
--document-private-items --all-features`, the grouping and shared-ci sha checks, `ci/docs-only.test.sh`,
`ci/code-docs-check.test.sh`, `ci/code-docs-check.sh`); step 3 no golden moved against the merge-base. Shown RED once
on the base (step 1 fails on R1-R6). Not covered: bitcoind-differential, fuzz-smoke, FreeBSD and musl legs, the I4
split FIX, the release.

## 8. Overlaps

`crates/md-cli/src/cmd/repair.rs` is touched by no other open bughunt row known to the author. `CHANGELOG.md`
`## md-cli [Unreleased]` is shared with every md row; the later builder rebases.
