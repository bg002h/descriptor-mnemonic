# Exit-code cutover in md-cli: independent adversarial review (2026-11-26)

**Scope.** `git diff d269ede^..HEAD` on `fx/descriptor-mnemonic`, five commits: d269ede (closed stdout), ada284d (seat_network flake), 1a7063c (the cutover), e2ae76f (docs), 09e9f29 (E17 gate row). Read against `SPEC_constellation_exit_codes.md` rev 5, §2-§5 and §7a. Also read: the implementer's report (`/home/user/fx/impl-md.md`) and the survey (`exit-codes-2026-11-26/md.md`).

**What I ran.**
- I rebuilt `md` at HEAD.
- I probed by hand: repair over-count and duplicate chunks, non-UTF-8 stdin versus a non-UTF-8 file, R2 ordering, `>&-`, `/dev/full`, the legend text against the spec's quoted block (an exact substring match), the network and xprv keys, the compose values, and `--verify-against` with `--chain 7`.
- I ran `cargo test -p md-cli --test exit_codes --test output_errors`: 14 and 13 passed.
- I did not run the whole suite.

**Verdict: 0 Critical, 2 Important, 7 Minor, 7 Nit.**
- Most of the cutover is right: the classification, R1 sums, R8 add-32, the legend, decision 7 for a true incomplete set, decision 6's 22, and R2 `--chain` before the verdict.
- The closed-stdout probe is correct.
- The two Important findings:
  - I-1: `repair` now reports a set with too many or duplicated chunks as an "incomplete" set and exits 0 or 16, where it used to exit 2.
  - I-2: stdin that is not UTF-8 exits 32, while the same bytes in a file exit 2.

---

## Important

### I-1. `repair` reports an over-full or duplicated chunk set as "incomplete" and exits 0 or 16 (it used to exit 2)

**Where:** `crates/md-cli/src/cmd/repair.rs:150-170`, the `Err(md_codec::Error::ChunkSetIncomplete { got, expected })` arm.

**The cause.** `md_codec::chunk::reassemble_with_opts` (`crates/md-codec/src/chunk.rs:425-430`) returns `ChunkSetIncomplete` whenever `parsed.len() != count`. That includes `got > expected`, and it runs before the index-gap and duplicate check (`:432-441`). The new arm treats every such error as decision 7's "incomplete set", so it prints the chunks and exits 0 or 16.

**Measured** with the `md repair --help` example, whose chunk 2 is damaged:

```
$ md repair $A $B $C $B
...
md: repair: the chunk set is incomplete (4 of 3 chunks): each correction passed its chunk's checksum, and is verified when the rest of the set is reassembled
exit 16

$ md repair $A $A          # one distinct chunk, given twice
md: repair: nothing to correct; the chunk set is incomplete (2 of 3 chunks).
exit 0
```

Both messages are false (R5: a message must agree with its code):
- "4 of 3": every chunk is present, so nothing remains to be reassembled later.
- "2 of 3": only one distinct chunk was given.

The code is false too:
- A set that has every chunk but cannot be reassembled is decision 6's "complete set that fails reassembly", which is 2 with nothing printed.
- At d269ede^ both inputs exited 2.

In the first case, a script that reads 16 as "re-engrave, the rest of the set verifies it later" is told something that is not true.

**Fix.** Take the incomplete branch only when `got < expected`, the chunk indices are pairwise distinct, and every index is below `count`. Otherwise exit 2 with nothing printed, as for any other reassembly failure (stderr could say "duplicate chunk N" or "more chunks than the set holds"). Add these rows to `repair_codes_follow_decisions_6_and_7`: the 4-of-3 set, `[A, A]`, and a lone chunk whose index is at or above its count.

### I-2. Non-UTF-8 stdin exits 32 (IO), but the same bytes in a file exit 2 (INVALID)

**Where:**
- `crates/md-cli/src/cmd/mod.rs:308-309`: `read_md1_strings` maps a stdin read error to `CliError::Io`.
- `crates/md-cli/src/decompose/mod.rs:173-174`: `read_descriptor_stdin` does the same.

**The cause.** `read_to_string` on stdin fails with `ErrorKind::InvalidData` when the bytes are not UTF-8. The `--in` paths go through `CliError::read_failure`, which classes `InvalidData` as 2. The stdin paths class every error as 32.

**Measured:**
```
printf '\xff\xfe\n' | md decode -        -> md: stdin read: stream did not contain valid UTF-8   exit 32
printf '\xff\xfe\n' > f; md decode --in f -> exit 2
printf '\xff\xfe\n' | md decompose -     -> exit 32
```

The implementer's own report calls a file that is not UTF-8 an input error, 2. By §3 the bytes are the input, and the input is wrong (test 6). Nothing in the system failed (test 2). Binary piped into `md` by a failed upstream step is the pipeline case that decision 3 exists for.

**Fix.** Use `CliError::read_failure(format_args!("stdin read"), &e)` (or the same `InvalidData` split) at both sites. Add a row to `empty_input_is_2_and_a_missing_file_is_32` that writes `b"\xff\xfe"` to stdin.

---

## Minor

### M-1. R13: `Codec(_)` sends every library error to 2, including API-misuse variants that can only be md bugs

**Where:** `crates/md-cli/src/error.rs` `exit_code()`, the `CliError::Codec(_) => INVALID` arm.

- R13 names this pattern ("`From<mr_codec::Error> for Failure` sends every library error to 1") as where unclassified failures hide.
- Most `md_codec::Error` variants are input errors, so 2 is right for them. But some are reachable only through a programming error:
  - `NetworkRequiredForUnspendable`: md-cli always calls the `_with_network` entry points.
  - `Codex32EncodeError`
  - `ChunkSetEmpty`: inputs are checked as non-empty first.
  - possibly `AddressDerivationFailed`
- Those should be 101.
- The R13 table test exercises one codec variant (`ChunkSetEmpty`, the internal kind, asserting 2).

**Fix.** Match on the inner `md_codec::Error` in `exit_code()`, sending the API-misuse variants to `INTERNAL` and the rest to `INVALID`. Extend the table test to list every `md_codec::Error` variant with no wildcard, so a new codec variant fails to compile until it is classified.

### M-2. R2 (usage first) is broken where an input file is read before an argv-only check

**Measured:**
- `md encode --in /missing --force-long-code` exits 32. The usage check is in `cmd/encode.rs:95`, which runs after `main.rs` reads `--in` (`read_template_file`).
- `md encode --in t.txt --force-long-code` exits 64 only because the file exists.
- `md descriptor --from-mk1 mk1… --from-mk1-file /missing`, with no policy card, exits 32:
  - `collect_mk1` (`main.rs:1225`, `:1271`) reads the file before `check_from_mk1_arity`'s Usage refusal ("no md1 policy phrase … and no --template").

R2 says a usage error exits 64 "before any input is read".

**Fix.** Hoist the `--force-long-code` check into `dispatch`, before the `--in` read. Run `check_from_mk1_arity`'s argv-only third clause before `collect_mk1`.

### M-3. R11: `md repair --help` lists 0/16/20/22/2 but not the 32 and 64 it can return

**Where:** `crates/md-cli/src/main.rs:874-883`.

R11 says per-verb help states the bits that verb can set. `repair` sets 32 for an `--in` read failure and adds 32 on a stdout error. The README table does list 32 and 64.

**Fix.** Add "32 -- the --in file or stdout failed (added to the code above)" and "64 -- a bad command line".

### M-4. Spec question 5: an asymmetry worth a spec ruling

- A non-chunked single string that is corrected and then fails decode exits 22 and is printed.
- The same card minted `--force-chunked` (a complete set of one) exits 2, silently.

In both cases no second oracle confirmed the correction: the content-id check runs after the decode, and the decode failed. So the operator's situation is the same.

Decision 6 says "the check is per string" and also that "a complete set that fails reassembly is 2 with nothing printed". A set of one is both, so the implementer's reading is defensible and matches `cli_repair_dead_card_strict`. I'd still put it to the spec owner, because "a person repairing by hand needs to see them" applies equally to both forms. No code change unless the spec owner rules otherwise.

### M-5. Gate scripts and their expected files still pin the old codes and text

Only `fix-bh1119-md-placeholder-order-gate-erows.sh` E17 was updated, and none of the gates were run.

| file | what it still pins |
|---|---|
| `scripts/fix-bh1119-md-slot-bindings-gate.sh:23-24` and its 22 `row` lines | rc 1 and the `md: MISMATCH: @…` text; now rc 2 with no `MISMATCH:` |
| `scripts/fix-bh1119-md-out-writes-gate.sh:49,55,63,77,96` | `code = 1` for an `--out` write failure; now 32 |
| `scripts/fix-bh1119-md-out-writes-gate.sh:82,86,90` | 1 for a codec error; now 2 |
| `design/FIX_bh1119_md_out_names_input/expected.txt`, used by `scripts/fix-bh1119-md-out-names-input-gate.sh` | 29 rows of `rc=2` (output names its input, now 8) and one `rc=5` (repair) |

These now fail loudly, not falsely. But re-running a historical gate is how this repo re-checks a fix.

**Fix.** Update them in the docs commit, or mark them superseded at their headers.

### M-6. Incomplete-set chunks are not checked as chunks of their set

Each chunk's header passes `ChunkHeader::read`, but the incomplete path never checks that `index < count` or that the indices are distinct. A lone chunk whose header claims index 40 of a 3-chunk set is printed as a member of an incomplete set (0 or 16).

Decision 6 asks that each string decode "as a string or chunk of its format". This is the same fix as I-1.

### M-7. No FOLLOWUP records the cross-repo lockstep items

- The report lists the toolkit manual lines (`42-md.md`) and the GUI manual lines (`50-md/*`) that now state old codes.
- Spec §7 and CLAUDE.md require these to move in lockstep or carry a companion FOLLOWUP.
- `design/FOLLOWUPS.md` gains none: only the demote entry is resolved.
- `rehearse.sh` in the fx worktree already expects 20.

**Fix.** File a `cross-repo` FOLLOWUP that lists the manual and GUI lines, unless the coordinator tracks them centrally.

---

## Nit

- **N-1.** `tests/output_errors.rs`, the doc comment on `closed_stdout_exits_32_with_the_line`, still says "A verb that prints exits 1 with the EBADF line".
- **N-2.** `src/error.rs`, the `Compose` doc comment is garbled: "structural (§4e), lock band (§4c), DSL, or an / DSL."
- **N-3.** `every_variant_maps_to_its_class` copies the implementation's match. `assert_eq!(all.len(), 20)` does not prove that every variant is in `all`: a new variant forces a new `want` arm, but not a new array entry.
- **N-4.** `crates/md-cli/README.md`, the `md repair` row now says "20 for a single string's correction", but it keeps the stale "**Chunked-form only** at md-cli v0.6.0; non-chunked single-string md1 input is rejected".
- **N-5.** A `--from-mk1-file` that cannot be read exits 32 (IO), yet its message starts "seating refused:" (`seat/mod.rs:251-256`). The text predates this change, but the word now disagrees with the class.
- **N-6.** `exit.rs`'s `the_legend_is_the_specs_text` checks only the start and end. The integration test `root_help_ends_with_the_exit_status_legend` checks the whole text, and I confirmed it matches the spec's quoted block exactly, so this is cosmetic.
- **N-7. Spec defect, not the implementer's.** §7a lists `--allow-invalid-miniscript` among md's override refusals, but md-cli has no such flag. Report it to the spec owner.

---

## Spec questions: is the chosen reading the one closest to §3?

1. **Repeated `--key`/`--fingerprint` for one slot, and a `--key` bracket that disagrees with the template, `--path` or `--fingerprint`: implemented as 2. I agree, with a caveat.**
   - Test 1's "conflict" means a flag conflict, and decision 2 puts keys and fingerprints on the input side. So 64 is out.
   - Test 4 lists "a template" among the things an input may fail to match, and the earlier design deliberately used `MISMATCH` (1) for a repeat with a different value. But test 4 and the 1 row describe a comparison the verb *reports as its answer* ("read the verdict"). Here md refuses to build because its inputs contradict each other, and the operator's action is "fix the input".
   - So 2 is the closer reading. Worth one line in the spec's ambiguity table, since the prefix `MISMATCH:` was also dropped from the printed text (a printed-text change; see M-5 for the gate that pins it).
2. **Seating refusals as 2: I agree.**
   - The cards are inputs that are wrong or insufficient for the verb (test 6).
   - Test 4's "a plan it should belong to" is a membership answer the verb is asked for (`mr`'s "not a member"). md is asked to build a wallet, not whether a card belongs.
   - The CLAUDE.md seating hazard (refuse, never guess) is preserved: only the number changed.
   - The budget refusal (`partition.rs` `budget_refusal`) is a limit that no flag raises, so it is not test 5's 8. 2 is right.
3. **A clean single string that does not decode is 2 with no output: I agree.** There is no correction to show, and R1's own example is "emits nothing and exits 2".
4. **No output-class advisory for an incomplete set: I agree.** There is no descriptor to classify, and a guess would be the L4 mislabel.
5. **A chunked count-1 card that fails is 2 silently; a non-chunked one is 22 and printed: defensible.** See M-4; ask the spec owner.
6. **A missing build feature is 32: correct.** §3's worked table says so for `md compile`. `--json` without the json feature follows the same rule.
7. **`--chain`, `--index`, `--count`, `--group-size`, `--separator`, `--network`, `--emit` are settings (64 from clap); a `--chain` the card lacks is 2: correct.**
   - These are the "counts, output form, ranges" settings of decision 2.
   - I checked that md has no clap-typed *input* value that would need the 2 mapping: `--wrapper`, `--preset`, `--key`, `--fingerprint`, `--path` and `--seat` are all free strings judged by the verbs. Measured: `compose --wrapper bogus` and `--preset nonsense` exit 2, and `--fingerprint @0=zz` exits 2.

## Checked and found correct

- **Clap errors.** All exit 64, including no verb, an unknown flag, a missing positional and non-UTF-8 argv. `--help` and `--version` exit 0.
- **R11 legend.** The root `--help` ends with the spec's text exactly (a substring check against the quoted block).
- **R1 and R8.** `out::exit_code` ORs 32 into codes below 64 and keeps 64 and 101. Measured:
  - `/dev/full` gives 32.
  - A partial decode gives 36.
  - repair's 20 gives 52.
  - EPIPE keeps the verb's own code.
- **Closed-stdout probe (d269ede).**
  - A `.init_array` / `__mod_init_func` constructor runs before std's `/dev/null` sanitizing.
  - It records a closed fd 1 and fills fds 0-2 in order, so no `--out` open can take fd 1. The test checks the file's bytes.
  - Measured: `decode`, `gui-schema` and `--help` with `>&-` exit 32 with the EBADF line. A failing verb keeps 2. `encode --out F >&-` exits 0 with the right file.
  - `unsafe(link_section)` needs edition 2024, which the workspace has, with rust-version 1.85.
- **R2.** `--chain 7` with `--verify-against` exits 2 with no verdict on stderr. Without `--chain` the verdict prints and the run exits 1. The test is not a false pass.
- **Network.** A tpub under `--network mainnet` exits 8, through a bare key and through an origin-bracketed key (`parse_key_with_origin` delegates to `parse_key`). An xprv exits 2. decompose's network mismatch exits 8. Mixed and wrong-network mk1 cards exit 8, and the "seating refused:" text is kept.
- **Refused.**
  - A sigless template exits 8 on encode (string-sniffed "require a signature", as decompose already does).
  - compose's "needs --experimental" and `--md-only` exit 8.
  - An output that names its input exits 8.
- **Decision 7** for a genuine incomplete set (lone chunk, 2 of 3): printed, 16 or 0. A complete set exits 16. A single string exits 20 with an UNVERIFIED note. An unsupported version on one string exits 22 and is printed. Several strings at such a version exit 2 (ruling 7 kept).
- **R13.** No bare `Ok(1)`, `Ok(2)` or `Ok(5)`, and no `ExitCode::FAILURE`, remains in `src/`. `main` uses `e.exit_code()`.
- **Admission hazard.** The sigless 2→8 change does not add a new refusal on reading verbs: `verify`, `descriptor` and `address` all take `--experimental`, which waives it (R4).
