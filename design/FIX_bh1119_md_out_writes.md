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

**One point is OPEN: Q1, the form of the mnemonic-io-lib 0.1.2 pin (section 9). Everything else is decided.** Every new
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

- **R1 (C4, T5).** md-cli takes mnemonic-io-lib **0.1.2** (pin form: Q1). `write_artifact` keeps calling
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
  - a FIFO, device or standard-stream alias (`/dev/stdout`, `/dev/fd/N`) is written directly, as today.
- **R2 (T6).** Any error from writing the `--out` file (including a path that cannot be created) prints exactly one
  line `md: cannot write the output: <error>` on stderr and exits **1** (was `md: --out <FILE>: <error>`, exit 2).
  `<error>` is the `std::io::Error` Display, unmodified (for example `File too large (os error 27)`). Implementation
  note: a `CliError` variant that is not `BadArg`, so `main`'s generic arm (`main.rs:931-934`) prefixes `md: ` and
  exits 1; its Display is `cannot write the output: {e}`. Nothing is printed on stdout by a run that fails here. A
  verb already failing keeps its own code (no change: the write is the last fallible step, R3).
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
- **R6 (pin consequences, either form).** Re-vendor `vendor/mnemonic-io-lib` with `cargo vendor` and commit it with
  `Cargo.lock`; the md-cli `Cargo.toml:39-53` comment and `crates/md-cli/tests/crate_pin.rs:4` ("pinned by exact git
  rev", stale today) are brought in line with the pin form Q1 picks. Git-pin form only: add the row
  `mnemonic-io-lib|https://github.com/bg002h/mnemonic-engrave|crates/mnemonic-io-lib` to
  `ci/repro/vendor-git-sources.txt` and change `.github/workflows/man-pages.yml` (see Q1).

## 3. Texts (exact)

- **E (T6, quoted from the ruling and the existing stdout sink, not new):** `md: cannot write the output: <error>`.
- **H1, `md encode --out` help (`crates/md-cli/src/main.rs:119-124`), PROPOSED:**
  `Write the md1 artifact to FILE instead of stdout, CREATED 0600 (SPEC §6b, F-244) — which a shell redirect cannot do. OVERWRITES an existing file, and tightens its mode; a failed write leaves the existing file unchanged. The file holds exactly what stdout would have without `--json` (grouped only under `--group-size`); with `--json` the JSON still goes to stdout. The chunk-set-id and the advisories still go to stderr`
- **M1, toolkit manual `docs/manual/src/40-cli-reference/42-md.md:30` (`md encode` `--out` row), PROPOSED, whole row:**
  ``| `--out <FILE>` | write the md1 string(s) to FILE instead of stdout, created owner-only (`0600`) — which a shell redirect cannot do. Overwrites an existing file and tightens its mode; the file holds exactly what stdout would without `--json` (grouped under `--group-size`); with `--json` the file still gets the md1 string(s) and the JSON goes to stdout. The new contents are written to a temporary `FILE.<pid>.<n>.tmp` beside FILE that then replaces it, so a failed write leaves an existing FILE unchanged, FILE's directory must be writable, and a run killed mid-write can leave that owner-only temporary behind. A symlink is followed and its target replaced. The chunk-set id and advisories stay on stderr |``
- **M2, manual `42-md.md:806` ("When stdout fails"), PROPOSED, sentence appended to the paragraph:**
  `` A write error on the `--out` file of `md encode` or `md descriptor --emit md1` (a full disk, a directory that does not exist or cannot be written) prints the same `md: cannot write the output: <error>` and exits 1, and an existing file is left unchanged.``
- **C1, CHANGELOG `## md-cli [Unreleased]` / `### Fixed`, PROPOSED (three bullets):**
  - ``**A failed `--out` write no longer destroys the existing file.** `md encode --out` and `md descriptor --emit md1 --out` now write a temporary file beside FILE and rename it over FILE (mnemonic-io-lib 0.1.2), so a full disk or quota leaves the old file byte-identical. FILE's directory must now be writable (bh-md-C4).``
  - ``**`md encode --json --out FILE` writes FILE.** The md1 string(s) go to FILE and the JSON to stdout; before, FILE was silently not written at exit 0 (bh-md-I1).``
  - ``**A failing `md encode` writes nothing.** `--policy-id-fingerprint` on a template with no origin path used to write the card to `--out` (or stdout) and then fail; the policy id is now computed first. A `--out` write error now prints `md: cannot write the output: <error>` and exits 1 (was `md: --out FILE: <error>`, exit 2) (bh-md-I7).``

## 4. TRACE rows (RED on the base; GREEN after the build)

New file `crates/md-cli/tests/bh1119_md_out_writes.rs`, one test function per row, named exactly as the row. Data
files in a `tempfile::tempdir()`. FSIZE8 rows use `std::process::Command` with `pre_exec` (`libc::setrlimit
RLIMIT_FSIZE 8/8`, `libc::signal(SIGXFSZ, SIG_IGN)`), as triage test `bh_md_c4_failed_out_write_keeps_previous_backup`
did (`agent-reports/bughunt-1119/triage-md-t1.md`). `CARD` = `md1yqpqqxqq8xtwhw4xwn4qh` + newline (T's card).
`ERR27` = `md: cannot write the output: File too large (os error 27)`; `CODEC` =
`md: codec error: non-canonical wrapper requires explicit origin for @0, but none provided`; `NOTE` =
`note: stdout is a keyless descriptor template (no keys)`. Every base result below was seen by the author on d02bc778;
every fixed result was seen on a scratch prototype of R1-R4 (reverted; section 5).

| row | setup; command (after `md`) | base (seen) | fixed (seen on prototype) |
|---|---|---|---|
| o1_failed_overwrite_keeps_previous_file | F = output of `encode T --out F`; FSIZE8 `encode P --out F` | rc 2, `md: --out F: File too large (os error 27)`, F = `md1yqpqq` (8 bytes) | rc 1, stderr exactly ERR27, stdout empty, F == CARD, no `*.tmp` in the dir |
| o2_failed_overwrite_keeps_previous_file_descriptor_emit_md1 | D = output of `descriptor <v-b1-wallet md1> --from-mk1-file <its mk1 lines> --emit md1 --out D` (353 bytes); the same under FSIZE8 | rc 2, stderr `chunk-set-id: 0x7a2e1` + `md: --out D: File too large (os error 27)`, D 8 bytes | rc 1, stderr `chunk-set-id: 0x7a2e1` then ERR27, D byte-identical |
| o3_out_write_error_text_and_exit | `encode T --out <dir>/nodir/x.md1` | rc 2, `md: --out <dir>/nodir/x.md1: No such file or directory (os error 2)` | rc 1, stderr exactly `md: cannot write the output: No such file or directory (os error 2)`, stdout empty |
| o4_read_only_dir_refuses_and_keeps_file | `encode T --out ro/F`, chmod 0500 ro; `encode P --out ro/F` (skip when euid is 0) | rc 0, F replaced by `md1yqpqqxzq2qwfv8urt848e` | rc 1, stderr exactly `md: cannot write the output: Permission denied (os error 13)`, F == CARD |
| o5_symlink_out_replaces_target_keeps_link | `T.md1` = `OLD\n` 0644, `L.md1 -> T.md1`; `encode T --out L.md1` | rc 0, L a symlink, T.md1 == CARD, mode 600 | same (control, GREEN on base) |
| o6_json_with_out_writes_both | `encode T --json --out F` (F absent) | rc 0, F absent | rc 0, F == CARD mode 600, stdout == stdout of `encode T --json`, stderr == NOTE + newline |
| o7_json_with_out_grouped_file_unbroken_json | `encode T --json --group-size 5 --out F` | rc 0, F absent | rc 0, F == `md1yq pqqxq q8xtw hw4xw n4qh` + newline, JSON `phrase` == `md1yqpqqxqq8xtwhw4xwn4qh` |
| o8_json_with_unwritable_out_fails | `encode T --json --out <dir>/nodir/x.md1` | rc 0, JSON on stdout | rc 1, stdout empty, stderr exactly `md: cannot write the output: No such file or directory (os error 2)` |
| o9_json_with_out_and_full_stdout_keeps_file | `encode T --json --out F` with stdout on `/dev/full` (skip off Linux) | rc 1, stderr NOTE + `md: cannot write the output: No space left on device (os error 28)`, F absent | rc 1, same stderr, F == CARD |
| o10_failed_policy_id_keeps_prior_file | F = `PRIOR\n`; `encode W --policy-id-fingerprint --out F` | rc 1, CODEC, F == `md1yqpqqxpzsn9favzyjwfcgx\n` | rc 1, stderr exactly CODEC, stdout empty, F == `PRIOR\n` |
| o11_failed_policy_id_prints_nothing_on_stdout | `encode W --policy-id-fingerprint` | rc 1, stdout `md1yqpqqxpzsn9favzyjwfcgx\n` | rc 1, stdout empty, stderr exactly CODEC |
| o12_failed_policy_id_absent_out_stays_absent | `encode W --policy-id-fingerprint --out N` (N absent) | rc 1, N created | rc 1, N absent |
| o13_failed_policy_id_force_chunked_prints_only_the_error | `encode W --policy-id-fingerprint --force-chunked` | rc 1, stdout 1 chunk line, stderr `chunk-set-id: 0xc992e` then CODEC | rc 1, stdout empty, stderr exactly CODEC (no chunk-set-id line) |
| o14_failed_policy_id_json_out_keeps_prior_file | F = `PRIOR\n`; `encode W --policy-id-fingerprint --json --out F` | rc 1, CODEC, F == `PRIOR\n` | same (control, GREEN on base; pins R4's order) |

o13's fixed column is R3's rule; the prototype computed the id after the chunk-set-id line, so it still printed it.
Every other fixed cell was seen. RED expectation for the builder: o1-o4 and o6-o13 fail on the base; o5 and o14 pass
on the base and after.

Coverage map (rule -> rows): R1 old file kept -> o1, o2; R1 read-only directory -> o4; R1 symlink -> o5; R1 hard link,
FIFO and killed-run temporary -> mnemonic-io-lib's own rows (not repeated here); R2 text and exit -> o1, o2, o3, o4,
o8; R3 -> o10, o11, o12, o13, o14; R4 write both -> o6, o7; R4 order -> o8, o9, o14; R5 -> `cli_channels.rs`
`the_help_surface_carries_the_channels` (unchanged) and the manual diff; R6 -> gate step 2h (vendor-freshness) and
`crate_pin.rs`.

## 5. Tests that change

Measured: the scratch prototype (git pin e09d538d, the R2 text through `CliError::Compose`, the policy id moved before
the write, the R4 write in the JSON branch) run as `cargo nextest run --locked -p md-cli --all-features --no-fail-fast
--build-jobs 6 -j 6` on d02bc778: `1112 tests run: 1112 passed, 3 skipped`. No existing test asserts the old
`md: --out ...` text or its exit 2. So no existing test changes; only `crates/md-cli/tests/crate_pin.rs:4` (doc
comment) follows the pin form (R6). The triage tests `bh_md_c4_*`, `bh_md_i1_*` (t1) and `i7_*` (t3) are superseded by
o1, o6/o8 and o10/o11 and are not copied in.

## 6. FOLLOWUPS

Closes nothing in `design/FOLLOWUPS.md` (bh-md-C4, I1 and I7 are not filed there; the coordinator's bughunt records
close them). No new line is filed.

## 7. Gate line

`bash scripts/fix-bh1119-md-out-writes-gate.sh` (builder commits it, with its coordinator catalog row; shape of
`scripts/fix-bh1119-md-slot-bindings-gate.sh`; every cargo call with `CARGO_BUILD_JOBS=6`): step 1 builds md-cli and
runs o1, o3, o4, o6, o8, o10, o11 and o13 as printed commands against the built `md` (exit code, stdout, the exact
stderr and the file bytes compared to the fixed column; FSIZE8 by `prlimit`), then the repo CI set:
`cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast --build-jobs 6 -j 6`,
`cargo test --locked --workspace --doc --all-features`, `cargo clippy --locked --workspace --all-targets
--all-features -- -D warnings`, `cargo fmt --all --check`, `cargo build --locked -p md-cli --no-default-features`,
`RUSTDOCFLAGS=-D warnings cargo doc --locked --workspace --no-deps --document-private-items --all-features`,
`bash ci/code-docs-check.sh`, and (2h) `bash ci/repro/vendor-freshness.sh` (needs network for its check 4).
Shown RED once on the base. Not covered: the toolkit manual edit (M1, M2; separate toolkit change), the man-pages and
release workflows (CI only; under the git-pin form of Q1 their `--offline` build is the real check), macOS/Windows/
FreeBSD/musl legs, the WORDING_REVIEW rows for H1, M1, M2.

## 8. Overlaps

- `cmd/encode.rs` is also touched by row #29 (`FIX_bh1119_md_slot_bindings`, `descriptor_for_mint`) and row #30
  (`FIX_bh1119_md_change_branch`); different lines; whichever lands second rebases.
- `md vectors --out DIR` and `md gen-man --out DIR` write directories of generated files, not through
  `write_artifact`; out of scope (not a holder backup and not in the hunt's C4).
- The other io-lib consumers (mk, ms, mt, me, tk) take 0.1.2 in their own rows; their manual chapters will need the
  same M2-style sentence, which this FIX does not write.

## 9. Q1 (OPEN, STOP): the form of the mnemonic-io-lib 0.1.2 pin

The overrides say: pin `0.1.2` from crates.io if it is there, else pin the git rev the way ms-cli and mt-cli do.
Measured 2026-11-24: `cargo search mnemonic-io-lib` lists only `0.1.0`; 0.1.2 is merged on mnemonic-engrave master
(PR #41, merge `e09d538dcd0b552cdc58fe7ff228e2eca5f7f64e`; its `crates/mnemonic-io-lib` tree equals that of
`3274570342ed0741498fdf021a67fa528185b915` and of master head). ms-cli and mt-cli pin
`{ git = "https://github.com/bg002h/mnemonic-engrave", rev = "d8fec67..." }` (ms `crates/ms-cli/Cargo.toml:31`, mt
`crates/mt-cli/Cargo.toml:29`). So the overrides route md to a git pin. Two facts the overrides do not address:

1. md left the git pin on purpose: commit `a9b6da1b` ("deps: md-cli takes mnemonic-io-lib from crates.io, not by git
   rev", 2026-08-27) cites an architect ruling "applied constellation-wide" that a git source has no published tarball
   to anchor its bytes; `crates/md-cli/Cargo.toml:39-53` records it, and the same commit removed a
   mnemonic-engrave git-source exemption from `ci/repro/vendor-freshness.sh`.
2. md's `.github/workflows/man-pages.yml` fails closed on any git source but miniscript (prep step, lines 272-281) and
   passes no `git_source_url`/`git_source_rev` to the toolkit reusable workflow (lines 153-170; the pinned toolkit
   ref 4120af85 has those inputs). A git pin needs those `.github/` edits (ms precedent: `man-release.yml:92-170` and
   `:293-373`) plus the `vendor-git-sources.txt` row, inside the PR.

Options: **(a)** publish mnemonic-io-lib 0.1.2 to crates.io (T5 already planned "registry" for md, mk, tk), then md
pins `mnemonic-io-lib = "0.1.2"`: one manifest line, `Cargo.lock`, re-vendor, no `.github/` edit. **(b)** git pin at
e09d538d as the overrides say, reversing a9b6da1b, with the `.github/workflows/man-pages.yml` edits and the
vendor-git-sources row (a CI change in a normally reviewed PR, said in its description). The author recommends (a).
Rows o1-o14 and every text above are the same under either.
