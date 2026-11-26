# descriptor-mnemonic r7: review of the Fable fold (9912ffde..65a0162a)

Commits reviewed: bc0db771, 006d061b, 4e7e2065, 65a0162a on `claude/project-thread-utm1aa`. Read-only: no edits,
commits or pushes. Logs: `scratchpad/r7-mdcli-allfeat.log`, `r7-nd-check.log`, `r7-nd-test.log`, `r7-d-test.log`,
`r7-nd-tests-head.log`, `r7-errlocs.txt`; live BIP-388 copy at `scratchpad/r7bip/bip388.mw`.

**Counts: 0 Critical, 2 Important, 2 Minor, 2 Nit.**

**Verdict:** I-2 is fixed correctly and CI's new steps pass. I-1's code change is correct (and minted bytes match the
hand-typed tailed template), but the fold missed one more `cli-compiler` test, so CI's `--all-features` run is still
red, and the PR body's "0 failed" for that run comes from a run that stopped early. Both are a one-file fix plus one
full `--no-fail-fast` run.

## Important

### I-A. `tests/compile.rs::compiler_golden_table` still expects bare placeholders; CI's `--all-features` run stays red

**Where:** `crates/md-cli/tests/compile.rs:13-14` (unchanged by the fold):
```
("pk(@0)", "segwitv0", "wsh(pk(@0))"),
("pk(@0)", "tap", "tr(@0)"),
```
**Measured:** `cargo test --locked -j 2 -p md-cli --features cli-compiler --no-fail-fast` →
1237 passed, **1 failed**, 3 ignored (93 test-result lines):
```
thread 'compiler_golden_table' panicked at crates/md-cli/tests/compile.rs:28:9:
compile(pk(@0), segwitv0) → wsh(pk(@0/<0;1>/*)), expected prefix wsh(pk(@0))
```
The tap row fails too (`tr(@0/<0;1>/*)` does not start with `tr(@0)`). This file is `#![cfg(feature = "cli-compiler")]`,
so default-feature runs never see it. CI runs `cargo nextest run --workspace --all-targets --all-features`, so the PR
is still red for the same class of reason as Fable I-1.

**Why it was missed:** the implementer's run (`scratchpad/fable-fold-mdcli-allfeat.log`) was fail-fast: it stopped
at `cmd_compile` (822 passed, 4 failed, 47 binaries). The fix in 65a0162a was checked by re-running `cmd_compile`
only; the `compile` test binary, which runs after `cmd_compile`, never ran.

**Fix:** change the two expected prefixes to `wsh(pk(@0/<0;1>/*))` and `tr(@0/<0;1>/*)`. Then run the whole suite once
with `--all-features --no-fail-fast`. All other tests passed in my run, including the 11 Fable listed and the new and
changed compile tests (listed below).

### I-B. The PR body's `--all-features` count reports a fail-fast partial run as "0 failed"

**Where:** `scratchpad/prbodies/descriptor-mnemonic.md:207`: "After the Fable fold, `cargo test --locked -j 2 -p md-cli
--all-features`: 826 passed, 0 failed, 3 ignored (47 binaries; ... `cmd_compile` then ran 11 of 11)."

**Evidence:** 826 = 822 passed + 4 failed from `fable-fold-mdcli-allfeat.log`, which stopped after 47 binaries at
the first failing one. The complete run has about 1241 tests in 93 result lines and one failure (I-A). So this line
says the CI-matching set passed when it was never run in full. Rewrite the line after the full run. FOLLOWUPS
`bh1125-fable-m3-pr-tests-not-ci-features` says "The PR body's test section now quotes the --all-features run".
It stays open, which is right, but that sentence is only true once the full run is quoted.

## Minor

### m-A. The toolkit manual's `md compile` examples are now stale, and nothing tracks the update

`mnemonic-toolkit/docs/manual/src/40-cli-reference/42-md.md:409,419` (and the other examples at 413-435) show
`# tr(@0)` and `multi_a(2,@0,@1,@2)`. `md compile` now prints the tails. The PR body (line 199) mentions this, but no
FOLLOWUPS entry or toolkit companion records it. CI's `manual-md` lint checks flags, not printed text, so nothing
will catch it. File an item, or include the manual edit with the toolkit PR this branch already needs.

### m-B. README and CHANGELOG say `skeleton` "always" returns `NotMiniscript` without `derive`; gate 1 comes first

`crates/md-codec/README.md` (added lines) says "`skeleton` always returns `SkeletonError::NotMiniscript`", and the
CHANGELOG says much the same. `skeleton.rs:225` runs `expand_per_at_n` before the type gate (`:230`), so a card whose
keys do not expand returns `KeysDoNotExpand` in a no-derive build. It still fails closed and no key is produced, so
only the wording is wrong. Suggest: "never returns a key (`NotMiniscript`, or an earlier key-expansion error)".

## Nit

### n-A. `canonical_tails` rewrites inside malformed tokens and gives a confusing error

The regex `@([0-9]+)(/)?` (`compile.rs:104`) matches anywhere in the text. `pk(@0abc)` becomes `@0/<0;1>/*abc`,
which is refused as "derivation steps after the multipath group". `pk(foo@0)` becomes `foo@0/<0;1>/*` (refused,
"base58 encoding error"). `pk( @0 )` keeps its spaces and is refused the same way. Every case fails closed, and none
mints. Only the message refers to text the user did not type.

### n-B. The four new md-codec integration tests follow a pre-existing no-derive build break

At HEAD, `cargo check -p md-codec --no-default-features --tests --keep-going` fails in 9 test files. Five were
already at the base with the same ungated lines: `bughunt.rs:129`, `compose_crosscheck.rs:17`,
`coordinator.rs:9,13`, `liana_unspendable.rs:22` and `skeleton_key_conformance.rs:28,32`. Those files are unchanged on
the branch, and the base `lib.rs` gates `to_miniscript` and `descriptor_route` behind `derive` the same way. The
other four files are new on the branch: `coordinator_thresh`, `policy_shape_cases`, `policy_shape_thresh` and
`policy_shape_thresh_props`. So the implementer's statement is correct: the break predates the branch, and the
branch extends it. The new CI step uses `--lib` and passes. A `#![cfg(feature = "derive")]` on the new files would
stop the pile growing. I could not compile the base itself: the root volume was full (0-150 MB free) and a base
build rebuilds every dependency. I established this from the files instead, using `scratchpad/fable-base`, whose
md-codec tree, `Cargo.toml` and `Cargo.lock` I checked blob for blob against `c8cde234`.

## Checked and sound

**I-1 rewrite (`canonical_tails`, `compile.rs:102-113`)**
- Multi-digit placeholders: `pk(@10)` → `wsh(pk(@10/<0;1>/*))`.
- Placeholders that already have a tail are left as written: `@1/**`, `@1/<2;3>/*`, `@0/*` and `@0/84'/0'/0'`.
- The weight syntax `9@pk(@0)` is untouched, because `@` followed by a letter does not match.
- A mixed `@0` and `@0/<2;3>/*` compiles to two keys. A bare `@0` beside `@0/<0;1>/*` gives "Policy contains
  duplicate keys", which fails closed.
- `musig` cannot occur in the policy language, so the rewrite never reaches inside one.

**Bare-`@N` refusal still holds.** `md encode 'wsh(pk(@0))'` and `--from-policy 'pk(@0/84'/0'/0')'` are both refused
with the A2 text. The new unit test `a_hand_typed_bare_placeholder_is_still_refused` passes. The rewrite is called
only from `compile_policy_to_template`, which only `md compile` and `encode --from-policy` reach (`main.rs:1029`,
`cmd/compile.rs:14`). `md compose` already emits tails.

**Minted bytes are identical** (compiled policy versus the hand-typed tailed template, `md encode`):
- `pk(@0)` tap
- `pk(@0)` segwitv0 `--path bip48`
- the inheritance tap policy
- 2-of-3 segwitv0 `--path bip48`
- 2-of-3 tap with NUMS
- `@N/**` against `/<0;1>/*`: the same card `md1yzfdsssj5qqcyxpzrqrederj32kqg2c`

**BIP-388.** Fetched live from `raw.githubusercontent.com/bitcoin/bips/master/bip-0388.mediawiki`, line 164: "The
`/**` in the placeholder template represents commonly used paths for receive/change addresses, and is equivalent to
`/<0;1>/*`." The code comment and the PR body quote it correctly.

**Decode, hashing and re-emit are untouched.** In md-cli the fold changes only the compiler front end. In md-codec it
adds a cfg-split `policy_type_checks`, a cfg-split Display arm and a test module.

**Tests (`--features cli-compiler`), all pass:**
- The 11 Fable listed: 8 in `cmd_encode` and 3 `compile.rs` units.
- `compile_writes_the_canonical_tail_on_a_bare_placeholder` and `a_hand_typed_bare_placeholder_is_still_refused`.
- All `compile::tests`, and the 4 changed `cmd_compile` tests.

**I-2.**
- Without `derive`, `policy_type_checks` is `false`. `policy_shape` ANDs it into `complete` (`policy_shape.rs:314`).
  `skeleton` refuses on it before any key work (`skeleton.rs:230`).
- `coordinator::build` reaches keys only through `skeleton` (`coordinator/build.rs:172`), and `walk_shape` is
  private. No no-derive path reports `complete` or produces a key.
- The default build's code and Display text are unchanged.
- `cargo check -p md-codec --no-default-features` builds.
- `cargo test -p md-codec --no-default-features --lib derive_feature_tests`: 1 passed. I confirmed it ran the
  no-derive branch: the rustc line has no `feature=` cfg, and the binary contains the no-derive message but not the
  derive one. With default features: 1 passed.

**CI YAML.** `yaml.safe_load` parses it. The steps sit in the clippy job after `dtolnay/rust-toolchain@1.85.0`, use
the same `docs_only` guard, and run what their names say. `ci/shared-ci.sha256` still verifies, so `ci.yml` is not in
it. `cargo fmt --all --check` is clean.

**FOLLOWUPS and the saved report.** The 4 new entries are present, accurate and `status=open`, apart from the m3
sentence noted in I-B. The saved report `design/agent-reports/bughunt-2026-11-25-descriptor-mnemonic-review-fable.md`
is byte-identical to `scratchpad/reviews/md-fable.md`.

**PR body.** The I-1 and I-2 descriptions, the n-1 wording and line 208 are accurate. Line 207 is not (I-B).

## Out of scope (not caused by the fold)

`bash ci/code-docs-check.sh` exits 1 locally. It reports two `design/FIX_bh1119_md_out_names_input/` references in
`scripts/fix-bh1119-md-out-names-input-gate.sh:32-33`. That script and `ci/` are unchanged since `c8cde234`, so this
predates the branch. It may depend on CI context; I did not investigate.

## Housekeeping

The volume was full (0 MB free) when I started. I deleted the regenerable
`descriptor-mnemonic/target/debug/incremental`. I also deleted the dependency artifacts my aborted base check had
written into `target/` (files newer than `r7-nd-tests-head.log`, about 222 MB). I built with `CARGO_INCREMENTAL=0`.
I created no worktree.
