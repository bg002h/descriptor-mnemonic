# FIX_bh1119_md_out_names_input: an md output that is one of its inputs is refused ([md-out-names-input], Important)

Small-fix track (bughunt-1119 batch 2026-11-24). Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-24.
Repo descriptor-mnemonic, branch `fix/md-out-names-input`, cut from `origin/fix/md-out-writes` 5aab2405 (md #9's FIX
head, which differs from main 21cf1444 only by `design/FIX_bh1119_md_out_writes.md`). This is a separate unit, BUILT
AFTER #9: it assumes #9's io-lib 0.1.3 pin (#9 R1/R6), #9's `--out` failure line (R2) and #9's `md encode --json --out`
"write both" (R4). Crate `md-cli` only; no md-codec change, no wire change, no new flag, type or command.

Rulings used, all decided: Brian 2026-11-23 Q2 (an output that names an input is refused, naming both, "Write to a new
name."); coordinator `agent-reports/architect-mt-output-writes-1124.md` section 4 (stdout is an output, so the same-file
refusal covers it; the md row touches every md `--in` verb; shape: refuse before reading, name both, md's one-line
`md: ...` form) and F2 (:82-86); `agent-reports/review-fix-bh-md-out-writes-r1.md` I2 (:79-89: widened to every md input
file, including `--from-mk1-file`); T5 (`same_file` lives in mnemonic-io-lib). Shape copied from the same cases already
decided in mk #10 (`mnemonic-key` `fix/mk-out-writes` 174fbb8: R2, R4, X1, X2; its review r1 I1: R6 and X3) and ms #11
(`mnemonic-secret` `fix/ms-out-writes` 0aaeb1d: R3/E2, `--out` and stdout one file). Exit code: md's usual refusal code
**2** (`CliError::BadArg`, printed `md: <message>` by `main.rs:925-928`). Every new text is PROPOSED (architect confirms).

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (`rust-toolchain.toml`); `--locked`; every cargo call with
`CARGO_BUILD_JOBS=6`; nextest `--build-jobs 6 -j 6`; never `--release`. Test data: the keyless template `TPL` =
`wpkh(@0/<0;1>/*)`, its card `md1yqpqqxqq8xtwhw4xwn4qh`, the public key-card fixture
`crates/md-cli/tests/fixtures/seating/v-b1-wallet.txt` (its md1 line `M`, its five mk1 lines), and the `md decompose`
help example descriptor. No seed and no secret is used.

## 1. The defect (RED evidence, run by the author on the base)

Base: `CARGO_BUILD_JOBS=6 cargo build --locked -p md-cli --bin md` at 5aab2405 (md main's code). Every row below was run
as printed by `bash design/FIX_bh1119_md_out_names_input/red.sh target/debug/md /scratch/cache/mdni-red` (each row in a
fresh directory holding `tpl.txt` = TPL, `card.txt` = the card, `desc.txt` = the descriptor, `K.txt` = the five mk1
lines, `V.txt` = the keyed card from `md descriptor M --from-mk1-file K.txt --emit md1`). Base results are the "base"
column of section 4. In short:

- `--out` names an input: `md encode --in tpl.txt --out tpl.txt` exits 0 and tpl.txt now holds the card, the template
  gone; the same through a hard link, a symlink, `./tpl.txt` and `--out /dev/stdout >>tpl.txt`.
  `md descriptor M --from-mk1-file K.txt --emit md1 --out K.txt` exits 0 and K.txt (five mk1 key cards, the xpub backup)
  now holds four md1 chunks (review r1 I2); the same with `--verify-against V.txt --out V.txt`.
- stdout names an input: `md encode --in tpl.txt >>tpl.txt`, `md decode|verify|inspect|bytecode|repair --in card.txt
  >>card.txt`, `md decompose --in desc.txt >>desc.txt`, `md decode - <card.txt >>card.txt`, `md descriptor|address M
  --from-mk1-file K.txt >>K.txt` and `md descriptor ... --verify-against V.txt >>V.txt` all exit 0 and append md's
  output to the input file. With `>` the shell empties the file first and md then reports a misleading empty-input
  error (`md: --in tpl.txt: no BIP-388 template in this file (it is empty or blank)`, exit 2).
- `--out` and stdout one file: `md encode TPL --json --out F >>F` exits 0. On the base the JSON lands in F (main ignores
  `--out` under `--json`, bh-md-I1); after #9 (R4 writes both, R1 renames a temporary over F) the card replaces F and
  the JSON goes to the replaced, unlinked file, so it is lost at exit 0.

## 2. Spec delta (behaviour)

- **S1 (the guard).** A new function in `crates/md-cli/src/cmd/mod.rs` (beside `write_artifact`, builder names it)
  takes the verb's output endpoints and its input endpoints, each input with its label, and asks
  `mnemonic_io_lib::write::same_file` (io-lib 0.1.3, `crates/mnemonic-io-lib/src/write.rs:585-605` at mnemonic-engrave
  0cc8383f; already on #9's pin, no pin change). Identity is io-lib's: dev+ino of the file a path resolves to (symlinks
  followed, hard links equal); an absent path by its canonical directory plus name, so `--in x --out x` with x absent is
  one file; a pipe, terminal or device never matches; a failed lookup never matches; off Unix a hard link and a
  redirected stdin or stdout are not detected (io-lib's documented limit).
- **S2 (where it runs).** The guard is the FIRST step of each verb below, before any other check, before any input is
  read and before anything is printed on stdout or stderr: as the first statement of the verb's arm in `dispatch`
  (`main.rs:960` Encode, `:1046` Decode, `:1051` Verify, `:1070` Inspect, `:1075` Bytecode, `:1130` Descriptor,
  `:1172` Address, `:1208` Decompose) and as the first statement of `cmd::repair::run` (`cmd/repair.rs:88`). On a
  refusal md prints exactly one stderr line (X1, X2 or X3), nothing on stdout, writes no file and exits **2**. `--json`
  changes nothing here (md prints every error on stderr; it has no error envelope). No other verb is checked
  (`compile`, `compose`, `shape-key`, `build`, `partial`, `verdict` and `gui-schema` read no file; `vectors --out DIR`
  and `gen-man --out DIR` write directories and are out of scope, as in #9 section 8).
- **S3 (inputs, in this order; the first match is named).**
  - `encode`: `--in PATH` when given (`Endpoint::Path`). The template on argv is not an input.
  - `decode`, `verify`, `inspect`, `bytecode`, `repair`: `--in PATH` when given; otherwise standard input
    (`Endpoint::Stdin`) when the positional strings include `-` (what `read_md1_strings`, `cmd/mod.rs:114-133`, reads).
  - `decompose`: `--in PATH` when given; otherwise standard input when the positional is exactly one `-`
    (`cmd/decompose.rs:45`).
  - `descriptor`: `--from-mk1-file PATH` when given (always a path, even when it is `-`: `seat/mod.rs:251`), then
    `--verify-against ARG` when `std::path::Path::new(ARG).is_file()` is true (the same test
    `resolve_verify_against`, `cmd/descriptor.rs:312-315`, uses to read it as a file; an md1 string is never an input).
  - `address`: `--from-mk1-file PATH` when given.
- **S4 (outputs and the order of the three checks).**
  - Standard output is an output when the verb will write on it: always for `decode`, `verify`, `inspect`, `bytecode`,
    `repair`, `decompose` and `address`; for `encode` when `--out` is not given or `--json` is given (#9 R4); for
    `descriptor` unless `--emit md1` and `--out` are both given. `--out FILE` is an output for `encode` and for
    `descriptor` (where clap already requires `--emit`).
  - Check 1 (X2): when stdout is an output, `same_file(Endpoint::Stdout, inputs)`.
  - Check 2 (X1): when `--out` is given, `same_file(Endpoint::Path(--out), inputs)`.
  - Check 3 (X3): `encode` only, when `--json` and `--out` are both given, `same_file(Endpoint::Stdout,
    [Endpoint::Path(--out)])`. (`descriptor` cannot reach it: `--emit md1` conflicts with `--json`.)
  - The first check that matches refuses; the later ones are not run.
- **S5 (help).** `md encode --out` help: #9's H1 gets one more sentence, H2. `md descriptor --out` help
  (`main.rs:564-572`): one sentence added, H3. No flag is added, removed or renamed, so the mnemonic-gui md schema
  mirror is not touched (`md gui-schema` carries no help text; #9 R5 checked).
- **S6 (manifest comment).** `crates/md-cli/Cargo.toml:51-53` ("`write_private` is the only item `md` adopts (P3
  boundary table, 1 of 11) ...") becomes exactly:
  `# `write_private` and (since FIX_bh1119_md_out_names_input) `same_file` with its`
  `# `Endpoint` are the items `md` adopts from the P3 boundary table, reached as`
  `# `mnemonic_io_lib::write::...` -- `write` is a `pub mod` with NO root re-export,`
  `# so an unqualified path is an E0425.`
- **Not changed:** every success path (rows c1-c7 keep their base result); #9's failure line and write path; the
  `md: cannot write the output: <error>` stdout sink.

## 3. Texts (exact)

`<OUT>` is `--out` as typed (`Path::display()`). `<INPUT>` is one of `--in <PATH>`, `--from-mk1-file <PATH>`,
`--verify-against <PATH>` (PATH as typed) or `standard input`. Each is printed by `main` as `md: <message>`.

- **X1, `--out` names an input, PROPOSED (architect confirms; mk #10 X1 in md's form):**
  `md: --out <OUT> is the same file as <INPUT>, so writing the card would replace the input md is reading. Write to a new name.`
- **X2, standard output names an input, PROPOSED (mk #10 X2 / mt §8.2i in md's form; "the output" because six of
  the eight verbs print something other than a card):**
  `md: standard output is the same file as <INPUT>, so the output would go into the file md reads its input from. A > redirect empties that file before md starts, so check that it still holds what you expect. Write to a new name.`
- **X3, `--out` and standard output one file, PROPOSED (ms #11 E2 / mk #10 X3 with md):**
  `md: --out <OUT> and standard output are the same file (the same path, a symlink or a hard link), so what md prints on stdout would go into the file it writes. A > redirect empties that file before md starts, so check that it still holds what you expect. Write to a new name.`
- **H2, appended to #9's H1 (`md encode --out`, `main.rs:119-124`), PROPOSED:**
  ` Refused (exit 2) when FILE is the --in file, or, with --json, when stdout is FILE.`
- **H3, `md descriptor --out` help, inserted after "advisories still go to stderr." (`main.rs:568-569`), PROPOSED:**
  ` Refused (exit 2) when FILE is the --from-mk1-file or --verify-against file.`
- **M3, toolkit manual (mnemonic-toolkit master 4bfa645f, `docs/manual/src/40-cli-reference/42-md.md`), PROPOSED:** a
  new paragraph after the "When stdout fails" paragraph (`:806`, after #9's M2 sentence):
  `` **When an output is one of the inputs.** md refuses, before it reads anything, a run whose `--out` FILE or redirected stdout is the same file (the same path, a symlink or a hard link) as one of its inputs (`--in`, `--from-mk1-file`, a `--verify-against` FILE, or stdin read through `-`), and an `md encode --json --out FILE` whose stdout is FILE. It names both and exits 2 (`md: --out tpl.txt is the same file as --in tpl.txt, so writing the card would replace the input md is reading. Write to a new name.`). A `>` redirect has already emptied the file before md starts; `>>` leaves it as it was. Pipes and terminals are never refused. ``
  And the `md encode` `--out` row (`:30`, #9's M1) and the `md descriptor` `--out` row (`:223`) each end with
  `` ; refused (exit 2) when FILE is one of the inputs ``.
- **C1, CHANGELOG `## md-cli [Unreleased]` / `### Fixed`, PROPOSED, one bullet:**
  ``**An output that is one of md's inputs is refused.** `md encode --in F --out F` replaced the template with the card, `md descriptor --from-mk1-file K --emit md1 --out K` replaced the mk1 key cards, and `md decode --in F >>F` (and every reading verb) appended to its input, all at exit 0. md now refuses these before reading anything, names both, and exits 2; so does `md encode --json --out F` with stdout redirected to F ([md-out-names-input]).``

Each of X1, X2, X3, H2, H3, M3 and C1 gets a `design/WORDING_REVIEW.md` row (direct-to-master record, after merge).

## 4. TRACE rows (RED on the base; GREEN after the build)

New file `crates/md-cli/tests/bh1119_md_out_names_input.rs`, one test function per row, named exactly as the "test"
column. Each test runs the built `md` (`env!("CARGO_BIN_EXE_md")`) in a `tempfile::tempdir()` holding the section 1
files; a redirect row opens the file with `OpenOptions` (`append(true)` for `>>`, `write(true).truncate(true)` for `>`)
and passes it as the child's `Stdio`; stdin rows pass `File::open`. "unchanged" = byte-identical to the file before the
run. Abbreviations: `X1(o, i)` = the X1 line with `<OUT>` = o and `<INPUT>` = i; likewise `X2(i)`, `X3(o)`. In the
fixed column stdout is empty and stderr is exactly the one line given.

| row | test | command (in the row dir) | base (measured, 5aab2405) | fixed |
|---|---|---|---|---|
| n1 | `n1_encode_out_same_path_as_in_refuses` | `md encode --in tpl.txt --out tpl.txt` | rc 0, tpl.txt = the card | rc 2, `X1(tpl.txt, --in tpl.txt)`, tpl.txt unchanged |
| n2 | `n2_encode_out_hard_link_to_in_refuses` | `ln tpl.txt hl`; `md encode --in tpl.txt --out hl` | rc 0, tpl.txt = the card | rc 2, `X1(hl, --in tpl.txt)`, tpl.txt and hl unchanged |
| n3 | `n3_encode_out_symlink_to_in_refuses` | `ln -s tpl.txt sl`; `md encode --in tpl.txt --out sl` | rc 0, tpl.txt = the card | rc 2, `X1(sl, --in tpl.txt)`, tpl.txt unchanged |
| n4 | `n4_encode_out_other_spelling_of_in_refuses` | `md encode --in ./tpl.txt --out tpl.txt` | rc 0, tpl.txt = the card | rc 2, `X1(tpl.txt, --in ./tpl.txt)`, tpl.txt unchanged |
| n5 | `n5_encode_json_out_same_as_in_refuses` | `md encode --json --in tpl.txt --out tpl.txt` | rc 0, tpl.txt unchanged, JSON on stdout (main ignores `--out` under `--json`) | rc 2, `X1(tpl.txt, --in tpl.txt)`, tpl.txt unchanged |
| n6 | `n6_descriptor_out_same_as_mk1_file_refuses` | `md descriptor M --from-mk1-file K.txt --emit md1 --out K.txt` | rc 0, K.txt = 4 md1 chunks, no mk1 line | rc 2, `X1(K.txt, --from-mk1-file K.txt)`, K.txt unchanged |
| n7 | `n7_descriptor_out_symlink_to_mk1_file_refuses` | `ln -s K.txt kl`; `md descriptor M --from-mk1-file K.txt --emit md1 --out kl` | rc 0, K.txt = 4 md1 chunks | rc 2, `X1(kl, --from-mk1-file K.txt)`, K.txt unchanged |
| n8 | `n8_descriptor_out_same_as_verify_against_file_refuses` | `md descriptor M --from-mk1-file K.txt --emit md1 --group-size 5 --verify-against V.txt --out V.txt` | rc 0, V.txt = the grouped card | rc 2, `X1(V.txt, --verify-against V.txt)`, V.txt unchanged |
| n9 | `n9_encode_out_dev_stdout_onto_in_refuses` | `md encode --in tpl.txt --out /dev/stdout >>tpl.txt` | rc 0, tpl.txt = the card | rc 2, `X1(/dev/stdout, --in tpl.txt)`, tpl.txt unchanged |
| n10 | `n10_encode_stdout_appended_to_in_refuses` | `md encode --in tpl.txt >>tpl.txt` | rc 0, tpl.txt = 2 lines | rc 2, `X2(--in tpl.txt)`, tpl.txt unchanged |
| n11 | `n11_encode_stdout_truncating_in_refuses` | `md encode --in tpl.txt >tpl.txt` | rc 2, `md: --in tpl.txt: no BIP-388 template in this file (it is empty or blank)`, tpl.txt empty | rc 2, `X2(--in tpl.txt)`, tpl.txt empty (the shell's doing) |
| n12 | `n12_decode_stdout_appended_to_in_refuses` | `md decode --in card.txt >>card.txt` | rc 0, card.txt = 2 lines | rc 2, `X2(--in card.txt)`, card.txt unchanged |
| n13 | `n13_verify_stdout_appended_to_in_refuses` | `md verify --in card.txt --template TPL >>card.txt` | rc 0, card.txt = 2 lines | rc 2, `X2(--in card.txt)`, card.txt unchanged |
| n14 | `n14_inspect_stdout_appended_to_in_refuses` | `md inspect --in card.txt >>card.txt` | rc 0, card.txt = 10 lines | rc 2, `X2(--in card.txt)`, card.txt unchanged |
| n15 | `n15_bytecode_stdout_appended_to_in_refuses` | `md bytecode --in card.txt >>card.txt` | rc 0, card.txt = 4 lines | rc 2, `X2(--in card.txt)`, card.txt unchanged |
| n16 | `n16_repair_stdout_appended_to_in_refuses` | `md repair --in card.txt >>card.txt` | rc 0, card.txt = 2 lines | rc 2, `X2(--in card.txt)`, card.txt unchanged |
| n17 | `n17_decompose_stdout_appended_to_in_refuses` | `md decompose --in desc.txt >>desc.txt` | rc 0, desc.txt = 7 lines | rc 2, `X2(--in desc.txt)`, desc.txt unchanged |
| n18 | `n18_decode_stdin_and_stdout_one_file_refuses` | `md decode - <card.txt >>card.txt` | rc 0, card.txt = 2 lines | rc 2, `X2(standard input)`, card.txt unchanged |
| n19 | `n19_decompose_stdin_and_stdout_one_file_refuses` | `md decompose - <desc.txt >>desc.txt` | rc 0, desc.txt = 7 lines | rc 2, `X2(standard input)`, desc.txt unchanged |
| n20 | `n20_descriptor_stdout_appended_to_mk1_file_refuses` | `md descriptor M --from-mk1-file K.txt >>K.txt` | rc 0, K.txt = 6 lines | rc 2, `X2(--from-mk1-file K.txt)`, K.txt unchanged |
| n21 | `n21_address_stdout_appended_to_mk1_file_refuses` | `md address M --from-mk1-file K.txt >>K.txt` | rc 0, K.txt = 6 lines | rc 2, `X2(--from-mk1-file K.txt)`, K.txt unchanged |
| n22 | `n22_descriptor_stdout_appended_to_verify_against_refuses` | `md descriptor M --from-mk1-file K.txt --verify-against V.txt >>V.txt` | rc 0, V.txt = 5 lines | rc 2, `X2(--verify-against V.txt)`, V.txt unchanged |
| n23 | `n23_encode_json_out_stdout_appended_to_in_refuses` | `md encode --json --in tpl.txt --out G >>tpl.txt` | rc 0, tpl.txt = 6 lines (JSON appended), G absent | rc 2, `X2(--in tpl.txt)`, tpl.txt unchanged, G absent |
| n24 | `n24_encode_json_out_is_stdout_appended_refuses` | `printf 'keep\n' >F`; `md encode TPL --json --out F >>F` | rc 0, F = `keep` + the JSON (6 lines) | rc 2, `X3(F)`, F = `keep\n` |
| n25 | `n25_encode_json_out_is_stdout_truncated_refuses` | `printf 'keep\n' >F`; `md encode TPL --json --out F >F` | rc 0, F = the JSON (5 lines) | rc 2, `X3(F)`, F empty (the shell's doing) |
| n26 | `n26_stdout_check_runs_before_out_check` | `md encode --json --in tpl.txt --out tpl.txt >>tpl.txt` | rc 0, tpl.txt = 6 lines | rc 2, `X2(--in tpl.txt)` (not X1), tpl.txt unchanged |
| n27 | `n27_decode_stdin_and_stdout_truncated_refuses` | `md decode - <card.txt >card.txt` | rc 2, `md: expected at least one md1 string (positional or via stdin with '-')`, card.txt empty | rc 2, `X2(standard input)`, card.txt empty |
| n28 | `n28_encode_absent_in_and_out_one_place_refuses` | `md encode --in x --out x` (x absent) | rc 2, `md: --in x: No such file or directory (os error 2)`, x absent | rc 2, `X1(x, --in x)`, x absent |
| n29 | `n29_first_matching_input_is_named` | `md descriptor M --from-mk1-file K.txt --emit md1 --verify-against K.txt --out K.txt` | rc 1, `md: --verify-against K.txt: a file exists at this path and was read as one ... string does not start with HRP md1`, K.txt unchanged | rc 2, `X1(K.txt, --from-mk1-file K.txt)`, K.txt unchanged |
| c1 | `c1_encode_out_new_name_writes` | `md encode --in tpl.txt --out new.txt` | rc 0, new.txt = the card | same |
| c2 | `c2_decode_stdout_other_file_writes` | `md decode --in card.txt >out.txt` | rc 0, out.txt = TPL | same |
| c3 | `c3_decode_stdout_pipe_never_matches` | `md decode --in card.txt \| cat` | rc 0 | same |
| c4 | `c4_descriptor_out_set_stdout_not_checked` | `md descriptor M --from-mk1-file K.txt --emit md1 --out G >>K.txt` | rc 0, K.txt unchanged, G = the card | same |
| c5 | `c5_verify_against_md1_string_is_not_an_input` | `md descriptor M --from-mk1-file K.txt --emit md1 --verify-against M --out M` (no file named M) | rc 5 (NOT spend-equal), a file named M = the keyed card | same |
| c6 | `c6_encode_out_set_stdout_not_checked` | `md encode TPL --out F >>tpl.txt` | rc 0, tpl.txt unchanged, F = the card | same |
| c7 | `c7_encode_text_out_onto_stdout_not_checked` | `printf 'keep\n' >F`; `md encode TPL --out F >>F` | rc 0, F = the card | same |

The test functions assert stdout bytes, the exact stderr line and the protected file's bytes. Rows n1-n29 fail on the
base (exit code or stderr); c1-c7 pass on both. c5 is the rule that an md1 string given to `--verify-against` is not
compared (two absent paths naming one place would otherwise match). c4, c6 and c7 pin S4: stdout is not checked when
the verb writes nothing on it.

## 5. Tests that change

None. Searched (`grep -l '"--out"' crates/md-cli/tests/*.rs` then the hits for `"--in"`/`--from-mk1-file`/
`--verify-against`): no existing test passes an output that is one of its inputs, and every test runs `md` with piped
stdout, which never matches. The builder confirms with the full suite (gate step 2).

## 6. FOLLOWUPS

`design/FOLLOWUPS.md` on main does not hold `[md-out-names-input]` yet (it is a proposed line in the coordinator
reports). The author's report lists the line to file and its closing line; nothing is appended here.

## 7. Gate line

`bash scripts/fix-bh1119-md-out-names-input-gate.sh` (builder commits it with its coordinator catalog row; shape of #9's
`fix-bh1119-md-out-writes-gate.sh`; every cargo call with `CARGO_BUILD_JOBS=6`): step 1 builds md-cli and runs
`design/FIX_bh1119_md_out_names_input/red.sh` against the built `md`, comparing each row's rc, stdout, the exact stderr
line and the file bytes to the fixed column; step 2 the repo CI set exactly as #9's section 7 lists it (nextest workspace
all-targets all-features, doc tests, clippy `-D warnings`, fmt, `--no-default-features` build, rustdoc `-D warnings`,
`ci/code-docs-check.sh`, `ci/repro/vendor-freshness.sh`). Shown RED once on the base (rows n1-n29 differ). Not covered:
the toolkit manual edit (M3; a separate toolkit change), macOS/Windows/FreeBSD legs (io-lib's same_file limits off
Unix), the WORDING_REVIEW rows.

## 8. Overlaps

- md #9 (`FIX_bh1119_md_out_writes`): this doc is built on top of it. It edits #9's H1 (adds H2), #9's M1 manual row
  and the CHANGELOG section #9 also adds to; `cmd/mod.rs` gains the guard beside #9's `write_artifact`; `main.rs`
  dispatch arms gain one call each. Build #9 first; a rebase over it is textual only.
- md rows #29 and #30 touch `cmd/encode.rs`; this row does not.
- mk and ms: mk #10 refuses the same three cases on `mk encode` only and files its read verbs
  (`bh-mk-read-verbs-stdout-names-input`); ms #11 refuses X3's case and files input aliasing (its section 9). md covers
  every verb here because the architect ruled F2 over all md `--in` verbs.

## 9. Author's choices inside the rulings (flag for review)

1. Exit **2** (md's `BadArg` refusal code; the brief: "md's usual refusal code").
2. The guard runs first in each verb, before every other check (as mk #10 R2/R4), so `--in x --out x` with x absent
   refuses as X1 rather than "No such file" (n28).
3. Check order X2, X1, X3 (mk #10 after its review I1); the first match names the first input in S3's order (n26, n29).
4. `standard input` without a flag name (md reads stdin only through `-`; mt's label; mk names its flag because it has
   two stdin flags).
5. `--verify-against` is an input only when it names an existing file, by the same `is_file` test that decides how it
   is read (c5).
6. X1 says "the card" (both md `--out` sites write an md1 card); X2 says "the output" (most verbs print other text).
