# FIX_bh1119_md_slot_bindings: one `--key` and one `--fingerprint` per slot (bh-md-C3, graded Important)

Small-fix track (bughunt-1119 FIX table row #29). Ruling: coordinator
`agent-reports/architect-bughunt-1119-questions.md` T1 (bh-md-C3 graded I: self-contradictory input, the OQ1 class;
the fix proceeds unchanged). Fix shape from `agent-reports/bughunt-1119/grade-md.md` line 48: "refuse unequal
duplicate `--key` and `--fingerprint` bindings per slot (compare typed values, not Debug strings), both orders, all
subcommands that take `--key`; fix the stale comment build.rs:262-266".
Repo descriptor-mnemonic, branch `fix/md-slot-bindings` from main `6aa7e5e32638e963014d43fcce251720090820b2`.
Crate `md-cli` only; no md-codec change, no wire change, no new flag, type or command; clap help unchanged.
Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22.

**Q1 is ruled (section 9): an EQUAL repeat refuses, option B, exit 2, on all four commands** (coordinator, 2026-11-22;
it follows Brian's B2 for the sibling mk question, "refuse all repeats, one rule is smaller"). The grader's fix shape
refused only UNEQUAL repeats; R6 extends it to equal ones (rows e1-e6). Nothing is open. Every new holder-visible text
is PROPOSED (architect confirms).

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (`rust-toolchain.toml`); `--locked`;
nextest `--build-jobs 6 -j 6`; never `--release`. Signet keys only.

## 1. The defects (RED evidence, run by the author on the base, signet)

Keys: BIP-39 test vector "abandon x11 about" (fingerprint `73c5da0a`), signet, derived with
`mnemonic convert --from phrase=- --to xpub --network signet --path <P>`:

| name | path | depth | tpub |
|---|---|---|---|
| KA | m/84'/1'/0' | 3 | `tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M` |
| KB | m/84'/1'/1' | 3 | `tpubDC8msFGeGuwnP2xwTZBBZSie1BLgRAkJhgzpFYTdpGgZNzguXQhNDVWp7mJbHJUjQQvV2myLU9dkx67a7VAUnzY7yT7nvhHj7FgS4oNivvq` |

`T` = `wpkh(@0/<0;1>/*)`, `T2` = `wsh(sortedmulti(1,@0/<0;1>/*,@1/<0;1>/*))`, `CARD` = the two strings
`md encode T --key @0=KA --network signet` prints. Every command below adds `--network signet`.

- **Key arm, first wins** (`cmd/build.rs:305` forwards every `--key`; `md-codec canonicalize.rs:383-385`
  `sparse_lookup` takes the first match): `md address --template T --key @0=KA --key @0=KB` exit 0, stdout
  `tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl` (KA's wallet); the reverse order exits 0 with
  `tb1qp7shgcwx3mpzgxjvff0d77vuhchcldzfy60x6s` (KB's). `md descriptor` the same: `...#34l6j3pf` vs `...#jg5cuypq`.
- **Fingerprint arm, last wins** (`cmd/build.rs:276-279` `fp_map.insert` per `--fingerprint`):
  `md descriptor --template T --key @0=KA --fingerprint @0=11223344 --fingerprint @0=aabbccdd` exit 0, stdout
  `wpkh([aabbccdd]tpubD6Nz.../<0;1>/*)#8ljlmjwr`; reversed exits 0 with `[11223344]...#3xm0shwm`. `md address` with
  either order exits 0.
- **Bracket-path arm, first wins** (`cmd/build.rs:258-269` `bracket_sourced.entry(..).or_insert_with`):
  `md descriptor --template T --key "@0=[73c5da0a/84'/1'/0']KA" --key "@0=[73c5da0a/84'/1'/1']KA"` exit 0 with
  origin `[73c5da0a/84'/1'/0']` (`#t0h7m450`); reversed exits 0 with `[73c5da0a/84'/1'/1']` (`#h38540ey`).
- **Bare vs bracketed, bracket wins silently:** `--key @0=KA --key "@0=[73c5da0a/84'/1'/0']KA"`, both orders, exit 0
  `#t0h7m450`.
- **Bracket-fingerprint arm, wrong source named:** `--key "@0=[73c5da0a/84'/1'/0']KA" --key
  "@0=[aabbccdd/84'/1'/0']KA"` exits 1 with `md: MISMATCH: @0: --fingerprint 73c5da0a disagrees with the
  origin-notated --key fingerprint aabbccdd for this slot (...)`: no `--fingerprint` flag was given
  (`fp_map` was seeded from the first bracket, build.rs:276-290).
- **encode / verify refuse, but say nothing useful:** `md encode T --key @0=KA --key @0=KB`, `md encode T --key @0=KA
  --fingerprint @0=11223344 --fingerprint @0=aabbccdd`, `md verify CARD --template T --key @0=KA --key @0=KB` and
  `md verify CARD --template T --key @0=KA --fingerprint @0=11223344 --fingerprint @0=aabbccdd` each exit 1 with
  `md: codec error: override ordering violation: @0 followed by @0; require ascending` (the TLV writer meeting two
  entries for `@0`). The same text and exit for the EQUAL repeats `--key @0=KA --key @0=KA` and
  `--fingerprint @0=11223344 --fingerprint @0=11223344`.
- **Equal repeats on descriptor/address:** `--key @0=KA --key @0=KA` and `--fingerprint @0=11223344 --fingerprint
  @0=11223344` exit 0 with the single-binding output.
- **Stale comment:** `cmd/build.rs:262-266` says "the CLI has no syntax for binding two different --key values to
  the same @i in the first place, so this is not a reachable disagreement in practice"; the bracket-path row above
  reaches it.
- **Control (unchanged by this FIX):** `md descriptor --template T2 --key @0=KA --key @1=KB` exit 0
  `...#5zanlgag`.

## 2. Spec delta (behaviour)

**R1. One check, four commands, one place in each.** `md encode`, `md verify`, `md descriptor --template` and
`md address --template` (the four subcommands with `--key`/`--fingerprint`; `main.rs:153-157, 225-228, 397-453,
645-701`) run the slot-binding check after every `--key` and `--fingerprint` value has parsed and before anything
else reads them: in `cmd/encode.rs` `descriptor_for_mint` (so `md decompose --emit commands`' route proofs, which call
it, run it too), in `cmd/verify.rs` `run` before `parse_template_ext`, and in `cmd/build.rs`
`resolve_keys_fingerprints_and_precedence` before the inline-path scan (so V-PATHAGREE, N3's bracket source and
V-FPAGREE each see at most one `--key` per slot). One helper serves all four (builder's choice of name and module).
A parse error in any value still wins (it is reported first, unchanged).

**R2. What "the same value" means (typed, never Debug text).** Two `--key` values for one slot are equal iff
their key payloads (`ParsedKey::payload`, the 65 bytes md reads) are equal AND their bracket fingerprints are equal
(both absent, or both the same 4 bytes) AND their bracket paths are equal (`DerivationPath` equality; no bracket =
empty path). On `md encode`/`md verify` (`parse_key`, no bracket syntax) only the payload exists. Consequences, all
decided by this rule: a bare `@0=KA` and `@0=[73c5da0a/84'/1'/0']KA` are DIFFERENT values; two spellings of one key
that differ only in depth, parent fingerprint or child number are the SAME value (the composed wallet never
depends on them; `ParsedKey::depth` is not compared, though each value's own depth check still runs at parse). Two `--fingerprint` values are equal iff their 4 bytes are equal
(`@0=AABBCCDD` and `@0=0xaabbccdd` are the same value).

**R3. Unequal repeat of `--key` refuses.** Walk the `--key` arguments in command-line order, remembering the first
value bound to each slot. At the first argument whose slot already holds a DIFFERENT value, refuse: exit 1, stdout
empty, stderr exactly one line (text K, section 3). `{a}` is the 1-based position, among the `--key` arguments only,
of the slot's first binding; `{b}` that of the refusing argument. Both orders refuse (the rule is symmetric; only
`{a}`/`{b}` and nothing else in the text depend on order).

**R4. Unequal repeat of `--fingerprint` refuses.** The same walk over the `--fingerprint` arguments; at the first
argument whose slot already holds a DIFFERENT fingerprint, refuse: exit 1, stdout empty, stderr text F. `{x}` is the
slot's first fingerprint, `{y}` the refusing one, each as 8 lower-case hex digits (`fp_hex`, build.rs:378).

**R5. Order between the two walks.** The `--key` walk runs first, then the `--fingerprint` walk; the first refusal
found is the only line printed. (clap collects the two flags into separate lists, so their mutual interleaving on the
command line is not observable and plays no part.)

**R6. Equal repeats refuse** (Q1 ruled B by the coordinator, 2026-11-22). At the first argument whose slot already holds an EQUAL value (R2), refuse: exit 2, stdout empty, stderr exactly one line, text E-K for --key ({a}, {b} as in R3) or E-F for --fingerprint ({x} the value as 8 lower-case hex digits, fp_hex). The R3/R4 walk handles equal and unequal repeats in one pass, in argument order; the first repeat found decides the text and the exit.

**R7. Stale comment.** Replace build.rs:262-266's "the CLI has no syntax for binding two different --key values to
the same @i ... not a reachable disagreement in practice" with a statement that the R1 check has already refused two
different `--key` values for one slot, so at most one bracket reaches this arm per slot (and `or_insert_with` is
then never a choice). No behaviour hangs on the comment.

**R8. Nothing else changes.** Two slots with one key each (control row), V-FPAGREE between one `--fingerprint` and
one bracketed `--key` (its text and exit), key reuse across slots (`refuse_key_reuse_across_slots`), and the
`--from-mk1` / phrase routes (no `--key` there) are untouched.

## 3. Texts (exact; all PROPOSED, architect confirms)

Rendered through `CliError::Mismatch` (Display `MISMATCH: {m}`, main's generic arm prefixes `md: `, exit 1), the
variant V-FPAGREE already uses for "two values for one slot datum disagree" (build.rs:284-291).

- **K** (R3): `md: MISMATCH: @{i}: --key arguments {a} and {b} both bind this slot, with different values; a slot takes exactly one key, and md never picks one for you. Keep the --key @{i}=... you mean and remove the other.`
- **F** (R4): `md: MISMATCH: @{i}: --fingerprint {x} and --fingerprint {y} both bind this slot; a slot takes exactly one master fingerprint, and md never picks one for you. Keep the one you mean and remove the other.`
- Rendered through `CliError::BadArg` (main prints `md: {m}`, exit 2: a flag repeated
  with the same value is a spelling fault, the class `reject_unreferenced_bindings` already sends to exit 2):
  - **E-K**: `md: @{i}: --key arguments {a} and {b} bind this slot to the same value; give each slot's --key once.`
  - **E-F**: `md: @{i}: --fingerprint {x} is given more than once for this slot; give each slot's --fingerprint once.`

Help text: unchanged (`Repeatable.` stays true: one per slot). Toolkit manual lockstep (mnemonic-toolkit master
`b0b99a16`, `docs/manual/src/40-cli-reference/42-md.md`), exact replacements, PROPOSED:
- L35 (`md encode`) and L621 (`md verify`): `| \`--key <@i=XPUB>\` | concrete xpub for placeholder \`@i\`; repeatable, one per slot (a second \`--key\` for one \`@i\` is refused, even with the same value) |`
- L36 (`md encode`): `| \`--fingerprint <@i=HEX>\` | master-key fingerprint for placeholder \`@i\`; repeatable, one per slot (a second \`--fingerprint\` for one \`@i\` is refused, even with the same value) |`
- L622 (`md verify`): `| \`--fingerprint <@i=HEX>\` | master-key fingerprint for placeholder \`@i\`; one per slot (a second \`--fingerprint\` for one \`@i\` is refused, even with the same value) |`
- L170: in the cell, `repeatable; requires \`--template\`.` becomes `repeatable, one per slot (a second \`--key\` for one \`@i\` is refused, even with the same value, and the same xpub with and without an origin counts as two values); requires \`--template\`.`
- L171: after `requires \`--template\`;` insert ` one per slot (a second \`--fingerprint\` for one \`@i\` is refused, even with the same value);`.

CHANGELOG (CHANGELOG.md, `## md-cli [Unreleased]`, `### Fixed`; a new bullet), PROPOSED: `- **\`md encode\`, \`verify\`, \`descriptor\` and \`address\` refuse a second \`--key\`, or a second \`--fingerprint\`, for one \`@i\`.** Two different values exit 1 (\`descriptor\` and \`address\` used to compose the first key or the last fingerprint; \`encode\` and \`verify\` failed with an ordering error, exit 1). The same value given twice exits 2 (\`descriptor\` and \`address\` used to accept it; \`encode\` and \`verify\` failed with the ordering error, exit 1) (bh-md-C3).`

## 4. TRACE rows (RED on the base; GREEN after the build)

New file `crates/md-cli/tests/bh1119_md_slot_bindings.rs`, one test function per row, named exactly as the row.
Every row passes `--network signet`. "K(i,a,b)" / "F(i,x,y)" = exactly that line on stderr, stdout empty, exit 1.
All base outcomes below were run by the author on `6aa7e5e3` (section 1).

| row | command (after `md`) | base (seen) | fixed |
|---|---|---|---|
| s1_address_two_keys_one_slot | `address --template T --key @0=KA --key @0=KB` | rc 0, `tb1q6rz28m...` | K(0,1,2) |
| s2_address_two_keys_one_slot_reversed | `address --template T --key @0=KB --key @0=KA` | rc 0, `tb1qp7shgc...` | K(0,1,2) |
| s3_descriptor_two_keys_one_slot | `descriptor --template T --key @0=KA --key @0=KB` | rc 0, `#34l6j3pf` | K(0,1,2) |
| s4_descriptor_two_keys_one_slot_reversed | `descriptor --template T --key @0=KB --key @0=KA` | rc 0, `#jg5cuypq` | K(0,1,2) |
| s5_descriptor_two_fingerprints_one_slot | `descriptor --template T --key @0=KA --fingerprint @0=11223344 --fingerprint @0=aabbccdd` | rc 0, `[aabbccdd]` | F(0,11223344,aabbccdd) |
| s6_descriptor_two_fingerprints_one_slot_reversed | as s5, fingerprints swapped | rc 0, `[11223344]` | F(0,aabbccdd,11223344) |
| s7_address_two_fingerprints_one_slot | `address --template T --key @0=KA --fingerprint @0=11223344 --fingerprint @0=aabbccdd` | rc 0, `tb1q6rz28m...` | F(0,11223344,aabbccdd) |
| s8_descriptor_two_bracket_paths_one_slot | `descriptor --template T --key "@0=[73c5da0a/84'/1'/0']KA" --key "@0=[73c5da0a/84'/1'/1']KA"` | rc 0, `#t0h7m450` | K(0,1,2) |
| s9_descriptor_two_bracket_fingerprints_one_slot | `descriptor --template T --key "@0=[73c5da0a/84'/1'/0']KA" --key "@0=[aabbccdd/84'/1'/0']KA"` | rc 1, MISMATCH naming `--fingerprint 73c5da0a` | K(0,1,2) |
| s10_descriptor_bare_and_bracketed_one_slot | `descriptor --template T --key @0=KA --key "@0=[73c5da0a/84'/1'/0']KA"` | rc 0, `#t0h7m450` | K(0,1,2) |
| s11_encode_two_keys_one_slot | `encode T --key @0=KA --key @0=KB` | rc 1, `override ordering violation` | K(0,1,2) |
| s12_encode_two_fingerprints_one_slot | `encode T --key @0=KA --fingerprint @0=11223344 --fingerprint @0=aabbccdd` | rc 1, `override ordering violation` | F(0,11223344,aabbccdd) |
| s13_verify_two_keys_one_slot | `verify CARD --template T --key @0=KA --key @0=KB` | rc 1, `override ordering violation` | K(0,1,2) |
| s14_verify_two_fingerprints_one_slot | `verify CARD --template T --key @0=KA --fingerprint @0=11223344 --fingerprint @0=aabbccdd` | rc 1, `override ordering violation` | F(0,11223344,aabbccdd) |
| s15_first_conflict_in_argument_order | `descriptor --template T2 --key @0=KA --key @1=KB --key @1=KA --key @0=KB` | rc 0, `#5zanlgag` | K(1,2,3) |
| s16_keys_before_fingerprints | `descriptor --template T --key @0=KA --key @0=KB --fingerprint @0=11223344 --fingerprint @0=aabbccdd` | rc 0, `[aabbccdd]` `#8ljlmjwr` | K(0,1,2) only |
| s17_one_key_per_slot_still_composes | `descriptor --template T2 --key @0=KA --key @1=KB` | rc 0, `#5zanlgag` | unchanged (GREEN on base) |
| e1_descriptor_same_key_twice | `descriptor --template T --key @0=KA --key @0=KA` | rc 0, `#34l6j3pf` | E-K(0,1,2), exit 2 |
| e2_encode_same_key_twice | `encode T --key @0=KA --key @0=KA` | rc 1, `override ordering violation` | E-K(0,1,2), exit 2 |
| e3_descriptor_same_fingerprint_twice | `descriptor --template T --key @0=KA --fingerprint @0=11223344 --fingerprint @0=11223344` | rc 0, `#3xm0shwm` | E-F(0,11223344), exit 2 |
| e4_encode_same_fingerprint_twice | `encode T --key @0=KA --fingerprint @0=11223344 --fingerprint @0=11223344` | rc 1, `override ordering violation` | E-F(0,11223344), exit 2 |
| e5_descriptor_same_fingerprint_two_spellings | `descriptor --template T --key @0=KA --fingerprint @0=AABBCCDD --fingerprint @0=0xaabbccdd` | rc 0, `[aabbccdd]` `#8ljlmjwr` | E-F(0,aabbccdd), exit 2 |
| e6_descriptor_same_bracket_two_spellings | `descriptor --template T --key "@0=[73c5da0a/84'/1'/0']KA" --key "@0=[73C5DA0A/84h/1h/0h]KA"` | rc 0, `#t0h7m450` | E-K(0,1,2), exit 2 |

RED expectation for the builder: s1-s16 and e1-e6 fail on the base (s11-s14 fail on the text, not the exit; e2 and e4 fail on both);
s17 passes on the base and after. The triage test
`bh_md_c3_conflicting_slot_bindings_refused` (`/scratch/cache/bh1119-md-t1/`) is superseded by s1-s5 and is not
copied in.

Coverage map (rule -> rows): R1 four commands -> s1/s3/s11/s13 (keys), s5/s7/s12/s14 (fingerprints); R1 runs before
V-FPAGREE / bracket-path -> s8, s9; R2 typed equality -> s8, s9, s10, e1-e6 (e5, e6 fail a string compare); R3 both orders -> s1/s2, s3/s4; R4 both
orders -> s5/s6; R3 `{a}`/`{b}` and argument order -> s15; R5 -> s16; R6 -> e1-e6; R7 -> (comment, no row); R8 ->
s17 and the unchanged existing suite.

## 5. Tests that change

Measured: a scratch patch refusing every repeated slot (equal or not) on the four commands, run as
`cargo nextest run --locked -p md-cli --no-fail-fast` on `6aa7e5e3`: `1014 tests run: 1013 passed, 1 failed,
3 skipped`. The one failure:
- `crates/md-cli/tests/duplicate_key_slots.rs:372` `t_row_the_same_slot_supplied_twice_keeps_first_wins` (and its
  doc comment L366-370) pins FIRST-WINS for `--key @0=K0 --key @0=K1 --key @1=K1` on `md descriptor` (measured
  2026-08-30). It becomes `t_row_the_same_slot_supplied_twice_is_refused`: the same command exits 1, stdout empty,
  stderr K(0,1,2); the doc comment says one placeholder cannot be two cosigners, so this is not shape (1) and is
  refused by the slot-binding check, not by the key-reuse check (stderr must not contain `BIP 388`).
No existing test supplies an equal repeat, so Q1-B changes no further existing test.

## 6. FOLLOWUPS

Closes nothing in `design/FOLLOWUPS.md` (bh-md-C3 is not filed there; the coordinator's bughunt records close it).
No new line is filed.

## 7. Gate line

`bash scripts/fix-bh1119-md-slot-bindings-gate.sh` (builder commits it, with its coordinator catalog row): runs the
rows of section 4 as printed commands against the built `md` (each row's exit code, stdout and stderr line compared
to the fixed column; e1-e6 included), requires all PASS, then the repo CI set:
`cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast --build-jobs 6 -j 6`,
`cargo test --locked --workspace --doc --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D
warnings`, `cargo fmt --all --check`, `cargo build --locked -p md-cli --no-default-features`, `RUSTDOCFLAGS=-D
warnings cargo doc --workspace --no-deps --document-private-items --all-features`, `bash ci/code-docs-check.sh`.
Shown RED once on the base. Not covered: the toolkit manual edit (separate toolkit change at the pin bump), the
mnemonic-gui schema mirror (no flag or help change, so none owed), macOS/Windows/FreeBSD/musl legs, the
WORDING_REVIEW row for texts K, F (and E-K, E-F).

## 8. Overlaps

- `cmd/encode.rs` `descriptor_for_mint` is shared with `md decompose --emit commands` route proofs (row #4,
  merged). Decompose emits one `--key` and at most one `--fingerprint` per slot, so its proofs cannot reach R3/R4;
  an equal repeat cannot arise either.
- Row #9 (`FIX_bh1119_md_out_writes`, `cmd/encode.rs` write path) and row #30 (`FIX_bh1119_md_change_branch`) may
  touch the same files at other lines (not checked: neither branch existed when this was written); whichever
  lands second rebases.

## 9. Q1 (RULED B, coordinator 2026-11-22): an EQUAL repeat (`--key @0=KA --key @0=KA`, `--fingerprint @0=X --fingerprint @0=X`)

An equal repeat refuses on all four commands with text E-K / E-F, exit 2 (R6), the same one rule as for unequal repeats and
the one P2 / Brian B2 chose for mk's key records ("BIP-388 forbids repeated keys and one rule is smaller"). Effect on the
base: `md descriptor` and `md address` newly refuse an input they accepted with the correct single-binding output (e1, e3);
`md encode` and `md verify` keep refusing but with E-K / E-F at exit 2 instead of the ordering error at exit 1 (e2, e4).
Option A (an equal repeat is a no-op, as V-FPAGREE treats agreement, build.rs:295) was considered and not taken.
Nothing in this FIX is left for the builder to decide.
