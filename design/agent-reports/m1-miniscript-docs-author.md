# md M1 author report: miniscript 13.2.0 tracker text and CI clone steps (2026-11-03)

Brief: `coordinator/briefs/m1-md-miniscript-docs.md`. Branch `docs/miniscript-13-2`
from origin/main (c8bb97a5). No dependency bump; `Cargo.lock` untouched.

## What changed

Commit 1, `docs: miniscript 13.2.0 tracker text and pin rationale (md)`:
- `Cargo.toml` comment above `[patch.crates-io]`: states the keep-the-pin decision
  (architect miniscript-13-2, 2026-11-03), names the toolkit tripwire branch
  `fix/miniscript-tripwire-13-2`, and replaces "pinned for the Display fix" with
  the current reason: 13.2.0 carries the Display fix (#1039, was #953) but lacks
  #910 (`Terminal::SortedMultiA`, `to_miniscript.rs`), #915's `SortedMulti` /
  `WshInner` shapes (`descriptor_route.rs`) and #911 (`derive_at_index`,
  `into_definite`, `derive.rs`); #929 has no clean source marker. Dropping the
  patch does not compile. Original #953 text kept as history. Comment-only.
- `design/FOLLOWUPS.md`, three entries, each with a dated UPDATE and a corrected
  Status line (historical filing text kept):
  - `md-codec-sortedmulti-a-to-miniscript-rendering-gap`: resolved for the
    tap-leaf root (`to_miniscript.rs:689-699`); the remaining Err (`:869-889`) is
    the deliberate BIP-386/387 refusal.
  - `upstream-miniscript-taptree-depth2-display-asymmetry`: fixed in the pin since
    2026-08-20, released in 13.2.0 as #1039; the pin is still required by
    #910/#915/#911/#929.
  - `md-codec-unpublishable-while-patched-to-miniscript-master`: the "latest 13.1.0"
    bullet replaced by a 2026-11-03 re-check (13.2.0 highest, still lacks the
    APIs) plus the decision text; the Display half is overtaken, the API gap is
    what forces not publishing.

Commit 2, `ci: drop four vestigial miniscript-fork clone steps` (`.github/` edit):
- Deleted the "Clone patched miniscript fork (md-codec-local-stack)" step from the
  `test`, `clippy` and `doc` jobs of `ci.yml` and from `bitcoind-differential.yml`.
  They cloned into `../rust-miniscript-fork` at b308afd; nothing reads that path
  (the patch is a git rev). No live reference to the path or branch remains outside
  history files (CHANGELOG, MIGRATION, old plans and reports).
- Judgement call, no product point: the musl-check "FORK-FREE SCOPE" comment in
  `ci.yml` would have been false after the deletion, so it was reworded (no job
  clones a fork any more; the label "fork-free" below is historic). Text only.

## Gate (captured once; local, `cargo 1.85.0`)

Run with `--locked`, output at
`/tmp/claude-1000/-scratch-code-shibboleth-refugium-coordinator-coordinator/125b457d-0745-4589-8739-dbea56aad7bf/scratchpad/m1-gate.txt` (all exit 0):
- `ci/docs-only.test.sh` 73 passed 0 failed; `sha256sum --check --strict ci/shared-ci.sha256`;
  `ci/code-docs-check.test.sh` 41 passed 0 failed; `ci/code-docs-check.sh`.
- `design/display-grouping-vectors.tsv.sha256` check; `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `cargo nextest run --workspace --all-targets --all-features --no-fail-fast`:
  1607 run, 1607 passed, 6 skipped. `cargo test --workspace --doc --all-features`.
- `cargo doc --workspace --no-deps --document-private-items --all-features`.
- `cargo check --target x86_64-unknown-freebsd -p md-cli` (freebsd-compile-gate).
- `bash ci/repro/vendor-freshness.sh`.
- fuzz-smoke `build` job: `cargo +nightly-2026-04-27 fuzz build --target
  x86_64-unknown-linux-gnu` exit 0 (the `smoke` job's 60 s fuzz runs not executed).
- YAML of both edited workflows parses.

Not run locally:
- `musl-check` (both legs): `musl-gcc` is not installed; `cargo check --target
  x86_64-unknown-linux-musl -p md-cli` fails in cc-rs ("failed to find tool
  x86_64-linux-musl-gcc"). The aarch64 leg needs an arm64 runner. The edit to that
  job is a comment only, so its steps are unchanged; CI on the PR head covers it.
- `bitcoind-differential`: needs a Bitcoin Core download. Its edit is one deleted
  step; CI covers it. `man-pages.yml`, `release.yml`: untouched.

## Follow-ups (not fixed, behaviour untouched)

- `crates/md-codec/src/to_miniscript.rs:869-889`: the comment and Err message still
  say "unimplemented (R5)" for SortedMultiA, contradicting the handler at
  `:689-699`. Fixing changes a user-visible error string, so it is behaviour; owner
  proposal: a later md patch phase.
- `design/FOLLOWUPS.md` ~L2142 (`musig-bip390-encode-support`) still says "latest is
  13.1.0". Left alone: it is a historical WHAT line of an open, unrelated entry;
  a one-word update is available if the reviewer wants it.
- The #929 caveat: the architect marker table has no source marker for #929, so the
  Cargo.toml text claims no compile evidence for it; it is listed as a condition
  only because the decision text lists it.

## Reviewer ask

Cross-check the Cargo.toml and FOLLOWUPS wording (the #910/#915/#911/#929 mapping
and the 13.2.0 facts) against the companion toolkit branch
`fix/miniscript-tripwire-13-2`; the two must say the same thing.

## Findings

No Critical or Important findings. 0 Critical, 0 Important, 0 Minor, 0 Nit.
