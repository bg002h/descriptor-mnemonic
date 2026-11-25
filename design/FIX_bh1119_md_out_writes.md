# FIX_bh1119_md_out_writes: `--out` writes that fail closed (bh-md-C4, bh-md-I1, bh-md-I7; all graded Important)

Small-fix track (bughunt-1119 FIX table row #9). Rulings used, all decided: coordinator
`agent-reports/architect-bughunt-1119-questions.md` T5 (safe write lives in mnemonic-io-lib `write_private`; md takes it by
a pin bump), T6 (failed-write contract: `md: cannot write the output: <error>`, exit 1, for stdout AND `--out`), T7
(`--json` with `--out`: write both), P5/P6 grades; Fable review `agent-reports/bughunt-1119/fable-critical-review.md`
(bh-md-C4 UPHELD I). Batch overrides `briefs/bh1119-batch-1124-overrides.md` (io-lib pin rule).
Repo descriptor-mnemonic, branch `fix/md-out-writes` from main `d02bc778`. Crate `md-cli` only; no md-codec change, no
wire change, no new flag, type or command. Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-24.

Final grades: **bh-md-C4 Important** (hunter Critical; grader and Fable: I, the P5 class), **bh-md-I1 Important**,
**bh-md-I7 Important**.

**Q1 (the io-lib pin form) is settled: Brian 2026-11-24, registry `mnemonic-io-lib = "0.1.2"`, published from mnemonic-engrave e09d538d (section 9). Everything is decided.** Every new
holder-visible text is PROPOSED (architect confirms) unless it is quoted from T6.

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (`rust-toolchain.toml`); `--locked`; every cargo call
with `CARGO_BUILD_JOBS=6`; nextest `--build-jobs 6 -j 6`; never `--release`. No keys are needed: every row uses a
keyless template (`T` = `wpkh(@0/<0;1>/*)`, `P` = `pkh(@0/<0;1>/*)`, `W` = `wsh(pk(@0/<0;1>/*))`, which has no origin
path, so its policy id cannot be computed).

## 1. The defects (RED evidence, run by the author on the base d02bc778)

Base binary `target/debug/md` built with `cargo build --locked -p md-cli --bin md`. "FSIZE8" = run under
`bash -c "trap '' XFSZ; exec prlimit --fsize=8 md ..."` (a file-size limit of 8 bytes, the signal ignored so the write
returns EFBIG; this stands in for a full disk or quota).

- **bh-md-C4, a failed overwrite destroys the previous `--out` file.** `md encode T --out F` (F = 25 bytes,
  `md1yqpqqxqq8xtwhw4xwn4qh` + newline), then FSIZE8 `md encode P --out F`: exit 2, stderr
  `md: --out F: File too large (os error 27)`, and F is now the 8-byte fragment `md1yqpqq`. The same through
  `md descriptor <v-b1-wallet md1> --from-mk1-file <its mk1 lines> --emit md1 --out D`: D goes from 353 bytes to 8.
  Cause: `write_private` in mnemonic-io-lib 0.1.0 opens, truncates, then writes in place (registry crate; md-cli
  `Cargo.toml:54`; the `vendor/` copy is not what the build compiles).
- **bh-md-I1, `md encode --json --out F` ignores `--out`.** Exit 0, JSON on stdout, F never created.
  `md encode T --json --out nodir/x.md1` (no such directory) also exits 0. The JSON branch
  (`crates/md-cli/src/cmd/encode.rs:152-201`) never reads `args.out_file`.
- **bh-md-I7, a failing `md encode` has already written its artifact.** `printf 'PRIOR\n' > H; md encode W
  --policy-id-fingerprint --out H`: exit 1, stderr
  `md: codec error: non-canonical wrapper requires explicit origin for @0, but none provided`, and H now holds
  `md1yqpqqxpzsn9favzyjwfcgx`. Without `--out` the card is on stdout at exit 1. With an absent `--out N`, N is created.
  With `--force-chunked`, stdout carries the chunk and stderr `chunk-set-id: 0xc992e` before the error. Cause: the
  text branch computes the policy id (`encode.rs:247-253`) after writing (`encode.rs:242-246`). The JSON branch
  already computes it before printing (`encode.rs:168-175`), so `--json` is not affected.
- Also seen (the T6 gap, same lines): every `--out` write error exits **2** with `md: --out <FILE>: <error>`
  (`write_artifact` maps to `CliError::BadArg`, `cmd/mod.rs:109-112`; `main.rs:927-930` gives BadArg exit 2), where T6
  rules `md: cannot write the output: <error>`, exit 1.

## 2. Spec delta (behaviour)

- **R1 (C4, T5).** md-cli takes mnemonic-io-lib **0.1.2** from crates.io: `crates/md-cli/Cargo.toml:54` becomes `mnemonic-io-lib = "0.1.2"` (Brian 2026-11-24; no git pin, no `.github/` edit). The builder first runs `cargo search mnemonic-io-lib --limit 1`; if 0.1.2 is not listed it does every step that needs no build and STOPs (overrides). `write_artifact` keeps calling
  `mnemonic_io_lib::write::write_private` and adds no write logic of its own. What the holder sees, for both
  `md encode --out FILE` and `md descriptor --emit md1 --out FILE`, all from 0.1.2's documented behaviour
  (`crates/mnemonic-io-lib/src/write.rs:77-98` at mnemonic-engrave e09d538d):
  - an existing regular FILE (or an absent one) is written to a temporary `<name>.<pid>.<n>.tmp` beside it, created
    0600, synced, and renamed over FILE; any error leaves an existing FILE byte-identical and removes the temporary;
  - a run killed mid-write can leave that temporary behind (0600);
  - FILE's directory must be writable: an existing writable FILE in a read-only directory, written in place before,
    is now refused (R2 text) and left unchanged;
  - a symlink FILE is followed and its target replaced; the link stays a link. A hard-linked FILE is replaced at this
    name only;
  - a FIFO or device is written directly; a standard-stream alias (`/dev/stdout`, `/dev/fd/N`) is written into the open stream at its own offset and never truncated (io-lib 0.1.3).
- **R2 (T6).** Any error from writing the `--out` file (including a path that cannot be created) exits **1** (was
  `md: --out <FILE>: <error>`, exit 2). `<error>` is the `std::io::Error` Display, unmodified (for example `File too
  large (os error 27)`). Before the write md takes `std::fs::metadata(FILE)` (symlinks followed): for a regular file it
  keeps (dev, ino, len, mtime); NotFound (including a dangling symlink) is "absent"; anything else is "other". A FILE that names a standard stream is always "other": its typed path, or its canonical parent joined with its name, starts with `/dev/` or `/proc/`, or a hop of its symlink chain (`std::fs::read_link`, repeated) does. md then prints only `md: cannot write the output: <FILE>: <error>`. (io-lib 0.1.3 writes such a name into the open stream, so a second look cannot say what is on disk.) On a
  write error md takes the same metadata again and prints exactly one stderr line, then exits 1: regular before and the
  same (dev, ino, len, mtime) after: `md: cannot write the output: <FILE>: <error>. <FILE> is unchanged.`; absent
  before and still NotFound after: `md: cannot write the output: <FILE>: <error>. Nothing was written.`; anything else:
  `md: cannot write the output: <FILE>: <error>`. <FILE> is --out as typed (Path::display()). "Unchanged" is about the
  bytes: io-lib's preflight may already have set an existing file to 0600. Implementation note: write_artifact returns
  a CliError variant that is not BadArg, whose Display is the whole line after `md: `, so main's generic arm
  (`main.rs:931-934`) prints it and exits 1. A failed stdout write keeps `md: cannot write the output: <error>`.
  Nothing is printed on stdout by a run that fails here. A verb already failing keeps its own code (no change: the
  write is the last fallible step, R3).
- **R3 (I7).** `md encode` finishes every fallible step before it writes the `--out` file, prints the artifact on
  stdout, or prints the `chunk-set-id:` line: the policy id (`--policy-id-fingerprint`) is computed right after
  `mint_md1_cards` (`encode.rs:226`) and before the chunk-set-id line (`encode.rs:227-236`). On failure the error line
  and exit code are unchanged (`md: codec error: ...`, exit 1), stdout is empty, an existing `--out` file is
  byte-identical, an absent one stays absent, and no `chunk-set-id:` line is printed. The policy-id stderr line on
  success is unchanged in text and position (after the write).
- **R4 (I1, T7).** `md encode --json --out FILE` writes both: FILE receives exactly the bytes `md encode` writes to
  FILE without `--json` (the md1 string(s), one per line, grouped under `--group-size`, so `--group-size 5` groups the
  file while the JSON `phrase` stays unbroken), through `write_artifact` (R1, R2); stdout carries the same JSON object
  as without `--out` (it still holds `phrase` or `chunk_set_id`+`chunks`); stderr is unchanged. Order: FILE is written
  after every fallible step (including the policy id) and before the JSON is printed. If the FILE write fails: R2
  text, exit 1, stdout empty. If FILE is written and stdout then fails (`> /dev/full`): FILE is kept, and the existing
  stdout contract applies (`md: cannot write the output: No space left on device (os error 28)`, exit 1).
- **R5 (help).** `md encode --out` help text changes (section 3, H1). `md descriptor --out` help is unchanged (it
  says it mirrors `md encode --out`). No flag is added, removed or renamed, so the mnemonic-gui schema mirror is not
  touched (`md gui-schema` carries no help text; checked: no `OVERWRITES` in its output).
- **R6 (pin consequences).** `cargo update -p mnemonic-io-lib --precise 0.1.2` (the Cargo.lock entry becomes registry 0.1.2 with its checksum), re-vendor `vendor/mnemonic-io-lib` with `cargo vendor` (today 0.1.0) and commit it with `Cargo.lock`. The `crates/md-cli/Cargo.toml:39-53` comment stays (it already records the registry rationale). `crates/md-cli/tests/crate_pin.rs:4` "is pinned by exact git rev in `Cargo.toml`" (stale today) becomes "is pinned to the registry version in `Cargo.toml`". No `vendor-git-sources.txt` row and no `.github/` change.

## 3. Texts (exact)

- **E (T6, quoted from the ruling and the existing stdout sink, not new):** `md: cannot write the output: <error>`.
- **H1, `md encode --out` help (`crates/md-cli/src/main.rs:119-124`), PROPOSED:**
  `Write the md1 artifact to FILE instead of stdout, CREATED 0600 (SPEC §6b, F-244) — which a shell redirect cannot do. OVERWRITES an existing file, and tightens its mode; a failed write leaves the existing file unchanged. The file holds exactly what stdout would have without `--json` (grouped only under `--group-size`); with `--json` the JSON still goes to stdout. The chunk-set-id and the advisories still go to stderr`
- **M1, toolkit manual `docs/manual/src/40-cli-reference/42-md.md:30` (`md encode` `--out` row), PROPOSED, whole row:**
  ``| `--out <FILE>` | write the md1 string(s) to FILE instead of stdout, created owner-only (`0600`) — which a shell redirect cannot do. Overwrites an existing file and tightens its mode; the file holds exactly what stdout would without `--json` (grouped under `--group-size`); with `--json` the file still gets the md1 string(s) and the JSON goes to stdout. The new contents are written to a temporary `FILE.<pid>.<n>.tmp` beside FILE that then replaces it, so a failed write leaves an existing FILE unchanged, FILE's directory must be writable, and a run killed mid-write can leave that owner-only temporary behind. A symlink is followed and its target replaced. The chunk-set id and advisories stay on stderr |``
- **M2, manual `42-md.md:806` ("When stdout fails"), PROPOSED, sentence appended to the paragraph:**
  `` A write error on the `--out` file of `md encode` or `md descriptor --emit md1` (a full disk, a directory that does not exist or cannot be written) prints `md: cannot write the output: <FILE>: <error>` followed by `. <FILE> is unchanged.` or `. Nothing was written.` when md can see that is so, and exits 1, and an existing file is left unchanged.``
- **C1, CHANGELOG `## md-cli [Unreleased]` / `### Fixed`, PROPOSED (three bullets):**
  - ``**A failed `--out` write no longer destroys the existing file.** `md encode --out` and `md descriptor --emit md1 --out` now write a temporary file beside FILE and rename it over FILE (mnemonic-io-lib 0.1.2), so a full disk or quota leaves the old file byte-identical. FILE's directory must now be writable (bh-md-C4).``
  - ``**`md encode --json --out FILE` writes FILE.** The md1 string(s) go to FILE and the JSON to stdout; before, FILE was silently not written at exit 0 (bh-md-I1).``
  - ``**A failing `md encode` writes nothing.** `--policy-id-fingerprint` on a template with no origin path used to write the card to `--out` (or stdout) and then fail; the policy id is now computed first. A `--out` write error now prints `md: cannot write the output: <FILE>: <error>` and says whether FILE is unchanged and exits 1 (was `md: --out FILE: <error>`, exit 2) (bh-md-I7).``

## 4. TRACE rows (RED on the base; GREEN after the build)

New file `crates/md-cli/tests/bh1119_md_out_writes.rs`, one test function per row, named exactly as the row. Data
files in a `tempfile::tempdir()`. FSIZE8 rows use `std::process::Command` with `pre_exec` (`libc::setrlimit
RLIMIT_FSIZE 8/8`, `libc::signal(SIGXFSZ, SIG_IGN)`), as triage test `bh_md_c4_failed_out_write_keeps_previous_backup`
did (`agent-reports/bughunt-1119/triage-md-t1.md`). `CARD` = `md1yqpqqxqq8xtwhw4xwn4qh` + newline (T's card).
`ERR27(X)` = `md: cannot write the output: X: File too large (os error 27). X is unchanged.`; X is the --out path as typed; `CODEC` =
`md: codec error: non-canonical wrapper requires explicit origin for @0, but none provided`; `NOTE` =
`note: stdout is a keyless descriptor template (no keys)`. Every base result below was seen by the author on d02bc778;
every fixed result was seen on a scratch prototype of R1-R4 (reverted; section 5).

| row | setup; command (after `md`) | base (seen) | fixed (seen on prototype) |
|---|---|---|---|
| o1_failed_overwrite_keeps_previous_file | F = output of `encode T --out F`; FSIZE8 `encode P --out F` | rc 2, `md: --out F: File too large (os error 27)`, F = `md1yqpqq` (8 bytes) | rc 1, stderr exactly ERR27(F), stdout empty, F == CARD, no `*.tmp` in the dir |
| o2_failed_overwrite_keeps_previous_file_descriptor_emit_md1 | D = output of `descriptor <v-b1-wallet md1> --from-mk1-file <its mk1 lines> --emit md1 --out D` (353 bytes); the same under FSIZE8 | rc 2, stderr `chunk-set-id: 0x7a2e1` + `md: --out D: File too large (os error 27)`, D 8 bytes | rc 1, stderr `chunk-set-id: 0x7a2e1` then ERR27(D), D byte-identical |
| o3_out_write_error_text_and_exit | `encode T --out <dir>/nodir/x.md1` | rc 2, `md: --out <dir>/nodir/x.md1: No such file or directory (os error 2)` | rc 1, stderr exactly `md: cannot write the output: <dir>/nodir/x.md1: No such file or directory (os error 2). Nothing was written.`, stdout empty |
| o4_read_only_dir_refuses_and_keeps_file | `encode T --out ro/F`, chmod 0500 ro; `encode P --out ro/F` (skip when euid is 0) | rc 0, F replaced by `md1yqpqqxzq2qwfv8urt848e` | rc 1, stderr exactly `md: cannot write the output: <dir>/ro/F: Permission denied (os error 13). <dir>/ro/F is unchanged.`, F == CARD |
| o5_symlink_out_replaces_target_keeps_link | `T.md1` = `OLD\n` 0644, `L.md1 -> T.md1`; `encode T --out L.md1` | rc 0, L a symlink, T.md1 == CARD, mode 600 | same (control, GREEN on base) |
| o6_json_with_out_writes_both | `encode T --json --out F` (F absent) | rc 0, F absent | rc 0, F == CARD mode 600, stdout == stdout of `encode T --json`, stderr == NOTE + newline |
| o7_json_with_out_grouped_file_unbroken_json | `encode T --json --group-size 5 --out F` | rc 0, F absent | rc 0, F == `md1yq pqqxq q8xtw hw4xw n4qh` + newline, JSON `phrase` == `md1yqpqqxqq8xtwhw4xwn4qh` |
| o8_json_with_unwritable_out_fails | `encode T --json --out <dir>/nodir/x.md1` | rc 0, JSON on stdout | rc 1, stdout empty, stderr exactly `md: cannot write the output: <dir>/nodir/x.md1: No such file or directory (os error 2). Nothing was written.` |
| o9_json_with_out_and_full_stdout_keeps_file | `encode T --json --out F` with stdout on `/dev/full` (skip off Linux) | rc 1, stderr NOTE + `md: cannot write the output: No space left on device (os error 28)`, F absent | rc 1, same stderr, F == CARD |
| o10_failed_policy_id_keeps_prior_file | F = `PRIOR\n`; `encode W --policy-id-fingerprint --out F` | rc 1, CODEC, F == `md1yqpqqxpzsn9favzyjwfcgx\n` | rc 1, stderr exactly CODEC, stdout empty, F == `PRIOR\n` |
| o11_failed_policy_id_prints_nothing_on_stdout | `encode W --policy-id-fingerprint` | rc 1, stdout `md1yqpqqxpzsn9favzyjwfcgx\n` | rc 1, stdout empty, stderr exactly CODEC |
| o12_failed_policy_id_absent_out_stays_absent | `encode W --policy-id-fingerprint --out N` (N absent) | rc 1, N created | rc 1, N absent |
| o13_failed_policy_id_force_chunked_prints_only_the_error | `encode W --policy-id-fingerprint --force-chunked` | rc 1, stdout 1 chunk line, stderr `chunk-set-id: 0xc992e` then CODEC | rc 1, stdout empty, stderr exactly CODEC (no chunk-set-id line) |
| o14_failed_policy_id_json_out_keeps_prior_file | F = `PRIOR\n`; `encode W --policy-id-fingerprint --json --out F` | rc 1, CODEC, F == `PRIOR\n` | same (control, GREEN on base; pins R4's order) |
| o15_stdio_alias_onto_a_file_names_no_state | s.txt = `OLD BACKUP\n`; with RLIMIT_FSIZE 0 and SIGXFSZ ignored, stdout appended to s.txt: `encode T --out /dev/stdout` (skip off Linux) | rc 2, `md: --out /dev/stdout: File too large (os error 27)`, s.txt 0 bytes | rc 1, stderr exactly `md: cannot write the output: /dev/stdout: File too large (os error 27)` (no state sentence), s.txt still `OLD BACKUP\n` (11 bytes) |

o13's fixed column is R3's rule; the prototype computed the id after the chunk-set-id line, so it still printed it.
Every other fixed cell was seen. RED expectation for the builder: o1-o4, o6-o13 and o15 fail on the base; o5 and o14 pass
on the base and after.

Coverage map (rule -> rows): R1 old file kept -> o1, o2; R1 read-only directory -> o4; R1 symlink -> o5; R1 hard link,
FIFO and killed-run temporary -> mnemonic-io-lib's own rows (not repeated here); R2 text and exit -> o1, o2, o3, o4,
o8; second look: o15; R3 -> o10, o11, o12, o13, o14; R4 write both -> o6, o7; R4 order -> o8, o9, o14; R5 -> `cli_channels.rs`
`the_help_surface_carries_the_channels` (unchanged) and the manual diff; R6 -> gate step 2h (vendor-freshness) and
`crate_pin.rs`.

## 5. Tests that change

Measured: the scratch prototype (git pin e09d538d at the time; reviewer r1 re-ran it at the registry form through a [patch.crates-io] path override: `1112 tests run: 1112 passed, 3 skipped`; the author's run: the R2 text through `CliError::Compose`, the policy id moved before
the write, the R4 write in the JSON branch) run as `cargo nextest run --locked -p md-cli --all-features --no-fail-fast
--build-jobs 6 -j 6` on d02bc778: `1112 tests run: 1112 passed, 3 skipped`. No existing test asserts the old
`md: --out ...` text or its exit 2. So no existing test changes; only `crates/md-cli/tests/crate_pin.rs:4` (doc
comment) follows the registry pin (R6). The triage tests `bh_md_c4_*`, `bh_md_i1_*` (t1) and `i7_*` (t3) are superseded by
o1, o6/o8 and o10/o11 and are not copied in.

## 6. FOLLOWUPS

Closes nothing in `design/FOLLOWUPS.md` (bh-md-C4, I1 and I7 are not filed there; the coordinator's bughunt records
close them). No new line is filed.

## 7. Gate line

`bash scripts/fix-bh1119-md-out-writes-gate.sh` (builder commits it, with its coordinator catalog row; shape of
`scripts/fix-bh1119-md-slot-bindings-gate.sh`; every cargo call with `CARGO_BUILD_JOBS=6`): step 1 builds md-cli and
runs o1, o3, o4, o6, o8, o10, o11, o13 and o15 as printed commands against the built `md` (exit code, stdout, the exact
stderr and the file bytes compared to the fixed column; FSIZE8 by `prlimit`), then the repo CI set:
`cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast --build-jobs 6 -j 6`,
`cargo test --locked --workspace --doc --all-features`, `cargo clippy --locked --workspace --all-targets
--all-features -- -D warnings`, `cargo fmt --all --check`, `cargo build --locked -p md-cli --no-default-features`,
`RUSTDOCFLAGS=-D warnings cargo doc --locked --workspace --no-deps --document-private-items --all-features`,
`bash ci/code-docs-check.sh`, and (2h) `bash ci/repro/vendor-freshness.sh` (needs network for its check 4).
Shown RED once on the base. Not covered: the toolkit manual edit (M1, M2; separate toolkit change), the man-pages and
release workflows (CI only), macOS/Windows/
FreeBSD/musl legs, the WORDING_REVIEW rows for H1, M1, M2.

## 8. Overlaps

- `cmd/encode.rs` is also touched by row #29 (`FIX_bh1119_md_slot_bindings`, `descriptor_for_mint`) and row #30
  (`FIX_bh1119_md_change_branch`); different lines; whichever lands second rebases.
- `md vectors --out DIR` and `md gen-man --out DIR` write directories of generated files, not through
  `write_artifact`; out of scope (not a holder backup and not in the hunt's C4).
- `md descriptor --emit md1 --out FILE` with FILE the same file as `--from-mk1-file` (or a link to it) replaces the mk1
  key cards at exit 0 (reviewer r1 I2, measured). Brian 2026-11-23 Q2 rules refuse; it is NOT built here and goes to
  the same separate md row as architect 1124 F2 (`md encode --in F --out F`), whose scope is widened to every md input
  file, including `--from-mk1-file`.
- The other io-lib consumers (mk, ms, mt, me, tk) take 0.1.2 in their own rows; their manual chapters will need the
  same M2-style sentence, which this FIX does not write.

## 9. Q1 (settled)

Brian 2026-11-24 (overrides, io-lib bullet): mnemonic-io-lib 0.1.2 is published to crates.io from mnemonic-engrave e09d538d and md pins the registry `"0.1.2"`; no git pin, no `.github/` edit. This is the author's option (a). Until 0.1.2 is listed by `cargo search`, the builder STOPs at the first build step.
