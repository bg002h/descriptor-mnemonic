# FIX_bh1119_md_printed_examples: two `--help` examples that fail in a shell, and an md-codec README that does not compile (bh-md-I10, bh-md-I11 Important)

Small-fix track (bughunt-1119 FIX table row #41; both keys graded I; I11's grade rests on ruling P8/B8: README and
remedy code count as printed, so "no printed command fails as printed" covers them). Not class B: no card, key,
address or exit code changes. Rule decided by the grader (coordinator
`agent-reports/bughunt-1119/grade-md.md:52`): quote the two help examples; replace the README block with code that
compiles and make it a doctest so it cannot rot; it must start from a card that has an origin (triage-md-t3 row
I11). Repo descriptor-mnemonic, branch `fix/md-printed-examples` from main
`1097371be794a18fbc471975245b846a644e7a8c` (md-codec 0.48.7, md-cli 0.21.1 at that head). No wire change, no new
flag, type, error variant or command; no exit code changes.
Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22.

**No STOP is open for this row.** Every new printed text below (the two help lines, the manual line, the README
blocks and sentences, the CHANGELOG entries) is PROPOSED (architect confirms). Section 2.3 folds in two README
defects the doctest exposes (the BCH block and its stale limitation paragraph, and the stale version pin), because
the doctest cannot pass, or the quickstart cannot run as printed, without them. Section 8 lists related defects
outside this row.

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (`rust-toolchain.toml`); `--locked`; nextest
`--build-jobs 6 -j 6` (2 on a loaded host); never `--release`. `X` below is the public BIP 32 test xpub
`xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf`
(the one the help text already prints).

## 1. The defect (RED evidence, run by the author on the base)

I10. `md encode --help` and `md decompose --help` print an `EXAMPLES:` line that a shell cannot parse: the
template's `(`, `<`, `;` and `*` are unquoted (`crates/md-cli/src/main.rs:109` and `:826`).

```text
$ PATH=$PWD/target/debug:$PATH bash -c "md encode wpkh(@0/<0;1>/*) --group-size 0"
bash: -c: line 1: syntax error near unexpected token `('
bash: -c: line 1: `md encode wpkh(@0/<0;1>/*) --group-size 0'
exit 2
$ PATH=$PWD/target/debug:$PATH bash -c "md decompose wpkh([73c5da0a/48'/0'/0'/2']X/<0;1>/*) --emit template"
bash: -c: line 1: syntax error near unexpected token `('
exit 2
```

The other subcommands with a `$` example: `md decode --help` runs as printed (control); `md descriptor`, `md address`
and `md repair` print elided placeholders (`md1qq...`), which section 8 F1 files, out of this row.

I11. `crates/md-codec/README.md:56-76` ("Encode a wallet policy and decode it back") uses an API md-codec no longer
has. Run as a doctest (trial harness of section 2.2, reverted):

```text
$ cargo test --locked -p md-codec --doc -- ReadmeDoctests
test crates/md-codec/src/../README.md - readme_doctests::ReadmeDoctests (line 58) ... FAILED
test crates/md-codec/src/../README.md - readme_doctests::ReadmeDoctests (line 90) ... FAILED
error[E0432]: unresolved imports `md_codec::DecodeOptions`, `md_codec::EncodeOptions`, `md_codec::WalletPolicy`
error[E0423]: expected function, found module `encode`
error[E0423]: expected function, found module `decode`
called `Result::unwrap()` on an `Err` value: Codex32DecodeError("chunk 0: character '.' not in codex32 alphabet")
test result: FAILED. 0 passed; 2 failed; ...
```

The second failure is the BCH block (`README.md:90-99`): it compiles, but its inputs are the placeholders
`"md1q..."`, so it fails as printed. And the dependency line the quickstart tells a reader to add
(`README.md:53`, `md-codec = "0.32"`) resolves on crates.io to a 0.32 release whose API is not the one the
example uses (the crate is 0.48.7).

## 2. Spec delta

### 2.1 Help examples (I10), `crates/md-cli/src/main.rs`

- S1 (`main.rs:109`, `md encode`): the example command line becomes, exactly (PROPOSED):
  `  $ md encode 'wpkh(@0/<0;1>/*)' --group-size 0`
  The output line below it (`  md1yqpqqxqq8xtwhw4xwn4qh`) is unchanged.
- S2 (`main.rs:826`, `md decompose`): the example command line becomes, exactly (PROPOSED; double quotes, because
  the descriptor holds the `'` hardened markers; the descriptor text itself is unchanged):
  `  $ md decompose "wpkh([73c5da0a/48'/0'/0'/2']X/<0;1>/*)" --emit template`
  (with `X` written out). In the Rust string literal the quotes are `\"`. The output line
  (`  wpkh(@0/48'/0'/0'/2'/<0;1>/*)`) is unchanged.
- Both forms were run by the author under bash, `zsh -f` and `fish --no-config`: exit 0, stdout equals the printed
  output line. No other help text changes.

### 2.2 README doctest harness (I11), md-codec

- S3: new file `crates/md-codec/src/readme_doctests.rs`, declared in `lib.rs` as
  `#[cfg(any(test, doctest))] mod readme_doctests;` (after the last `pub use`). It holds:
  - `#[cfg(doctest)] #[doc = include_str!("../README.md")] pub struct ReadmeDoctests;` so `cargo test --doc` runs
    every ```` ```rust ```` block of `crates/md-codec/README.md` (the `toml` and `sh` blocks are not Rust and are not
    run). CI already runs `cargo test --workspace --doc --all-features` (`ci.yml:87`), so a README block that stops
    compiling or running fails CI.
  - `#[cfg(test)]` unit test `i11_readme_dependency_pin_matches_crate_version`: the README contains
    `md-codec = "<MAJOR>.<MINOR>"` built from `CARGO_PKG_VERSION_MAJOR`/`_MINOR`; failure message
    `README quickstart must pin md-codec = "0.48"` (with the current pair). A minor bump then fails the test until
    the README pin moves with it.
- S4: `ci/code-docs-check.sh` flags `include_str!("../README.md")` as naming the root `README.md` (a docs file): its
  whole-name match sees `/README.md`. The file actually read, `crates/md-codec/README.md`, is NOT a docs file
  (`ci/docs-only-paths.txt` covers only top-level `*.md`), so a change to it runs every job and the skip the check
  guards against cannot hide this test. Resolution: append to `ci/code-docs-exempt.txt` (repo-local, not in
  `ci/shared-ci.sha256`):

```text

# Reads crates/md-codec/README.md, which is not a docs file (a change to it runs every job);
# the check matches its "../README.md" tail against the root README.md.
^crates/md-codec/src/readme_doctests\.rs$
```

  The harness lives in its own file so the exemption does not cover `lib.rs`. (Alternative rejected: a
  `- ^README\.md$` rule would make root-README edits run full CI on the false claim that code reads it. Rewording
  the line is forbidden by the check's own rule, since the line reads a file.) Trial: `bash ci/code-docs-check.sh`
  -> `code-docs check: ok`; `bash ci/code-docs-check.test.sh` exit 0.

### 2.3 README text (I11), `crates/md-codec/README.md` (all PROPOSED)

- S5 (`README.md:53`): `md-codec = "0.32"` becomes `md-codec = "0.48"`.
- S6 (`README.md:56-76`): replace the sentence "Encode a wallet policy and decode it back:" and its block with:

````text
Decode an engraved md1 card and encode it back:

```rust
use md_codec::{decode_md1_string, descriptor_to_template, encode_md1_string};

// A single-string card: `wpkh(@0/<0;1>/*)` with origin m/84'/0'/0'.
let card = "md1yq802gggqpsqwgtua24e7ssf3";
let descriptor = decode_md1_string(card)?;
println!("template: {}", descriptor_to_template(&descriptor)?);

// Encoding is canonical: the decoded descriptor re-encodes to the same card.
assert_eq!(encode_md1_string(&descriptor)?, card);
# Ok::<(), Box<dyn std::error::Error>>(())
```
````

  The card is `md encode 'wpkh(@0/<0;1>/*)' --path "m/84'/0'/0'"` at the base (it carries an origin, as triage
  requires: a pathless card fails strict encode with `MissingExplicitOrigin`); the block prints
  `template: wpkh(@0/<0;1>/*)`. md-codec has no template-text parser (that lives in md-cli), so the quickstart starts
  from a card, not from a policy string.
- S7 (`README.md:90-99`, the BCH block): replace the code block with:

````text
```rust
use md_codec::decode_with_correction;

// The card above with one character damaged (`gggq` engraved as `ggqq`).
let inputs = ["md1yq802ggqqpsqwgtua24e7ssf3"];
let (descriptor, corrections) = decode_with_correction(&inputs)?;
for c in &corrections {
    println!("chunk {} pos {}: {:?} -> {:?}", c.chunk_index, c.position, c.was, c.now);
}
assert_eq!(corrections.len(), 1);
assert_eq!(md_codec::encode_md1_string(&descriptor)?, "md1yq802gggqpsqwgtua24e7ssf3");
# Ok::<(), md_codec::Error>(())
```
````

  It prints `chunk 0 pos 7: 'q' -> 'g'` (cross-checked: `md repair md1yq802ggqqpsqwgtua24e7ssf3` reports
  `1 correction at position 7: 'q' -> 'g'`).
- S8 (`README.md:114-119`): delete the paragraph "**v0.34.0 limitation — chunked-form only:** ... Tracked at
  `design/FOLLOWUPS.md` `md-codec-decode-with-correction-supports-non-chunked-md1`." It is false since md-codec
  v0.35.0 (`chunk.rs:700-727` routes a single non-chunked string to `decode_md1_string`; that FOLLOWUPS entry says
  "RESOLVED in md-codec-v0.35.0") and S7's single-string example would contradict it. In its place (PROPOSED):
  "A single non-chunked md1 string (the form plain `encode` emits for small payloads) is corrected the same way
  (md-codec v0.35.0+)."

## 3. Help, manual, CHANGELOG (exact texts, PROPOSED (architect confirms))

- Help: S1 and S2 only. Man pages: none in the repo. mnemonic-gui md schema mirror: unchanged (no flag change; the
  gui holds neither example).
- Toolkit manual lockstep (mnemonic-toolkit master `04c56bfac8cded5e8303e34bf4920c97bef90b50`,
  `docs/manual/src/40-cli-reference/42-md.md`): line 341 becomes
  `$ md decompose "wpkh([73c5da0a/48'/0'/0'/2']X/<0;1>/*)" --emit template` (with `X` written out). Line 49
  (`md encode 'wpkh(@0/<0;1>/*)'`) is already quoted; no other manual line changes. Line 341 is not a
  `verify-examples.sh` transcript, so no `.cmd`/`.out` pair moves.
- CHANGELOG.md, under `## md-cli [Unreleased]`, add `### Fixed` if absent and append:
  "- **The `md encode` and `md decompose` help examples run as printed.** Their templates were unquoted, so pasting
  either line into a shell failed with a syntax error. They are now quoted (bh-md-I10)."
- CHANGELOG.md, under `## md-codec [Unreleased]` / `### Fixed`, append:
  "- **The README examples compile and run.** The quickstart used an API md-codec no longer has and pinned 0.32;
  it now decodes a card and re-encodes it with the current API and pins 0.48. The error-correction example uses a
  real damaged card instead of placeholders, and the stale \"chunked-form only\" note is gone. Both README blocks now
  run as doctests, and a test keeps the version pin in step with the crate (bh-md-I11)."

## 4. TRACE rows (RED on the base; GREEN after the build)

All new tests. md-cli rows live in new file `crates/md-cli/tests/bh1119_md_printed_examples.rs`. Each reads
`md <sub> --help`, takes every line whose trimmed form starts with `$ `, runs it with `bash -c` with the built `md`
first on PATH (the pattern of `bh1119_md_decompose_routes.rs:285-296`), and asserts exit 0 and that trimmed stdout
equals the next help line, trimmed; it also asserts exactly one `$` line for that subcommand.

| row | crate | command | RED on base | GREEN |
|---|---|---|---|---|
| i10_encode_help_example_runs_as_printed | md-cli | `cargo nextest run --locked -p md-cli -E 'test(=i10_encode_help_example_runs_as_printed)'` | FAIL: `help example failed as printed: md encode wpkh(@0/<0;1>/*) --group-size 0` / `` bash: -c: line 1: syntax error near unexpected token `(' `` (left Some(2), right Some(0)) | PASS |
| i10_decompose_help_example_runs_as_printed | md-cli | same, `test(=i10_decompose_help_example_runs_as_printed)` | FAIL: same message for the decompose line | PASS |
| c1_decode_help_example_runs_as_printed | md-cli | same, `test(=c1_decode_help_example_runs_as_printed)` | PASS (control: the harness is sound) | PASS |
| i11_readme_dependency_pin_matches_crate_version | md-codec | `cargo nextest run --locked -p md-codec -E 'test(=readme_doctests::tests::i11_readme_dependency_pin_matches_crate_version)'` | FAIL: `README quickstart must pin md-codec = "0.48"` | PASS |
| i11_readme_blocks_run (doctest) | md-codec | `cargo test --locked -p md-codec --doc -- readme_doctests::ReadmeDoctests` | FAIL: 2 failed (line 58: E0432 x1 line naming 3 imports, E0423 x2; line 90: Codex32DecodeError panic) | `test result: ok. 2 passed` |

Order (test first): commit RED = the md-cli test file, `readme_doctests.rs` + its `lib.rs` line + the S4
exemption, with the README and help unchanged; run every row above and see the RED column. Commit GREEN = S1, S2,
S5-S8, CHANGELOG; run the gate. The doctest row has no function name; it is identified by the filter
`readme_doctests::ReadmeDoctests` and its count (exactly 2 tests, 2 passed).

Author trial (reverted; logs in `/scratch/cache/md-printed-examples-trial/`): md-cli rows RED 1 passed 2 failed,
GREEN 3 passed; pin row RED then GREEN; doctest RED 2 failed, GREEN 2 passed; `cargo clippy -p md-codec
--all-targets --all-features -- -D warnings` clean; `cargo fmt --all --check` clean.

## 5. Tests that change

None existing. No test pins either help line or the README: `git grep -F` for both example command texts finds only
`main.rs:109` and `:826`. The README card `md1yq802gggqpsqwgtua24e7ssf3` is already pinned (as the output of encoding the
inline-origin template `wpkh(@0/84'/0'/0'/<0;1>/*)`) by `crates/md-cli/tests/cli_unhardened_origin_note.rs:95`,
so a codec change that moves it fails that test too.

## 6. Coverage map (spec sentence -> row)

| spec | row(s) |
|---|---|
| S1 | i10_encode_help_example_runs_as_printed |
| S2 | i10_decompose_help_example_runs_as_printed |
| harness soundness | c1_decode_help_example_runs_as_printed |
| S3 doctest, S6, S7 | i11_readme_blocks_run |
| S3 pin test, S5 | i11_readme_dependency_pin_matches_crate_version |
| S4 | gate step 2k (`ci/code-docs-check.sh`) |
| S8 | prose only; reviewer reads it against `chunk.rs:700-727` |
| section 3 manual | toolkit manual PR; the manual's own CI |

## 7. Gate

`scripts/fix-bh1119-md-printed-examples-gate.sh` (committed with GREEN), modelled on
`scripts/fix-bh1119-md-change-branch-gate.sh`: no arguments, run from the repo root, every step runs, exit 0 only
if all pass. Steps: 1 each md-cli and md-codec TRACE row by exact nextest name (match count 1 each), then run; 1d
`cargo test --locked -p md-codec --doc -- readme_doctests::ReadmeDoctests` with output grepped for
`test result: ok. 2 passed`; 2 the ci.yml set exactly as in the change-branch gate (2a nextest workspace
all-targets all-features, 2b doctests, 2c clippy, 2d fmt, 2e no-default-features build, 2f doc with
`RUSTDOCFLAGS=-D warnings`, 2g grouping sha, 2h shared-ci sha, 2i docs-only test, 2j code-docs test, 2k code-docs
check); 3 no golden moved against `git merge-base HEAD origin/main`. Does NOT cover: the bitcoind-differential,
fuzz-smoke, freebsd, windows/macos and musl/aarch64 jobs, the toolkit-manual PR, the release (an md-codec and md-cli
patch release carries the CHANGELOG entries; publishing is the coordinator's).

## 8. Related defects outside this row (file, do not fix here)

- F1 (md-cli help): `md descriptor --help`, `md address --help` and `md repair --help` print `$ md descriptor
  md1qq...`, `$ md address md1qq...`, `$ md repair md1qq...` (and two more repair lines). These are elided
  placeholders and fail as printed. Whether an elided placeholder is a "printed command" under B8 is not decided by
  the rulings; options: (a) replace each with a real card and its real output, adding each subcommand to the
  section 4 harness; (b) mark elision explicitly and exempt it. Recommend (a), as a separate small row.
- F2 (md-codec README prose, not code): the scope note names `bytecode::encode::validate_tap_leaf_subset_with_allowlist`
  and a sibling crate `md-signer-compat`; neither exists in the repo at the base (FOLLOWUPS marks that work
  wont-fix/moved). Not runnable code, so not rule 2; a docs Minor.
