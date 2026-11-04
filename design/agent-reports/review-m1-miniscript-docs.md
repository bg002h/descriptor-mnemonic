# Review: md PR 45 (M1, miniscript 13.2.0 tracker text + CI clone steps), class D, 2026-11-03

Reviewer: Sonnet 5.5 (independent of the author, Sonnet 5.5 default effort). First review round, so no
previous fold commit exists and nothing in these counts comes from one.
Subject: descriptor-mnemonic PR 45, head `09f6c7dc` (base `c8bb97a5` = origin/main), worktree
`/scratch/code/shibboleth/md-m1-review` (detached at the head; nothing committed).
Scope: the PR diff and the facts it asserts. No fresh audit of md.

## Verdict

0 Critical, 0 Important. The text is accurate on every load-bearing fact: 13.2.0 lacks #910/#911/#915 and
carries the Display fix as #1039, the keep-the-pin wording matches the architect's binding text, and the
four deleted clone steps have no consumer. The defects are inaccurate or stale comment text, 5 Minor and
7 Nit, none of which fails open. The most useful Minors are M1 (the #929 sentence is wrong for md), M2
(the rewritten musl comment is still partly false) and M3 (three other places still say the pin is "for #953").

Machine-checked, with output:

- CI on the head is green. `gh pr view 45`: cargo test (ubuntu, macos, windows), clippy, fmt, doc,
  freebsd compile-gate, musl compile/test (both legs), bitcoind v27.0 differential, vendor-freshness and
  release builds all SUCCESS; the only non-success is "assemble + release" SKIPPED. That covers the two jobs
  the author could not run locally.
- `actionlint` clean on `ci.yml` and `bitcoind-differential.yml` at the head and at base (0 lines each).
- `bash ci/docs-only.test.sh` 73 passed 0 failed; `sha256sum --check --strict ci/shared-ci.sha256` OK;
  `bash ci/code-docs-check.test.sh` 41 passed 0 failed; `bash ci/code-docs-check.sh` ok.

## Q1. Are the added factual claims true?

| Claim in the diff | Check | Result |
| --- | --- | --- |
| 13.2.0 is the highest crates.io release, published 2026-10-17 | crates.io API: `13.2.0 2026-10-17T17:38:50Z yanked=False`; 90 versions; the newest by date is 12.3.8 (2026-10-27) but 13.2.0 is the highest by semver. Its own CHANGELOG heads it "October 15, 2026" (release-notes date, not publish date). | true |
| 13.2.0 has no `Terminal::SortedMultiA` (#910) | `grep -rnE '^\s*SortedMultiA\(Threshold<' miniscript-13.2.0/src` 0 hits; `ff4732e/src/miniscript/decode.rs:161` has it | true |
| 13.2.0 has no `derive_at_index`, no `into_definite` (#911) | `grep -rnE 'pub fn (derive_at_index\|into_definite)\b'` on 13.2.0 0 hits; pin `descriptor/mod.rs:678,706` | true |
| 13.2.0 has no `Terminal::SortedMulti` (#915) | 0 hits for `^\s*SortedMulti\(Threshold<`; pin `decode.rs:157` | true |
| "none of #915's `WshInner` shapes" | 13.2.0 HAS `WshInner` (`segwitv0.rs:234`, with a `SortedMulti` variant) and `ShInner::SortedMulti` (`sh.rs:46`); the pin has neither. #915 removed them. | misleading, see M1 |
| "#929 has no clean source marker; a drop attempt's build catches it" | `pub fn into_plan\b` is absent from 13.2.0 and present at the pin (`descriptor/mod.rs:572`), and 13.2.0 has non-generic `pub struct Plan` (`plan.rs:213`). md-codec and md-cli never call it (`grep` of `crates/` and `fuzz/` for `into_plan`, `plan_mall`, `Plan`, `.plan(` finds only the word "Plan" in comments). | false for md, see M1 |
| 13.2.0 carries the taptree Display fix (#953, as #1039) | Probe crate, `miniscript = "=13.2.0"`, parse then Display of `tr(K,{{pk(A),pk(B)},pk(C)})`: output identical to input, reparse ok=true. Control with `=13.1.0`: output `tr(K,{{pk(A),pk(B),pk(C)}})`, reparse ok=false. CHANGELOG 13.2.0: "fix unbalanced Taproot tree formatting ... #1039". `gh api pulls/1039`: "13.x: descriptor: fix taproot tree formatting", base `release-13.x`, merged 2026-09-03. | true |
| PR numbers and subjects (#910, #911, #915, #929, #953) | `gh api pulls/N`: #910 "Add support for sortedmulti_a" (merged 2026-04-03); #911 "Return error from at_derivation_index when descriptor has no wildcard" (adds `derive_at_index` / `into_definite`, confirmed in its patch); #915 "refactor: remove SortedMultiVec and use Terminal::SortedMulti"; #929 "refactor: make `Descriptor` plan API available for generic `Pk`" (adds `into_plan`); #953 merged 2026-05-25 | true |
| "Dropping the patch does not compile" | Scratch worktree at the head with `[patch.crates-io]` removed: `cargo check --offline -p md-codec --all-targets --keep-going --message-format=short` exits 101, `could not compile md-codec (lib) due to 10 previous errors` (the lib-test target repeats the same 10). The 10 by cause: #910, 3 x `Terminal::SortedMultiA` (`descriptor_route.rs:434,474`, `to_miniscript.rs:697`); #911, `derive_at_index` (`derive.rs:148`) and `into_definite` (`derive.rs:150`); #915, 3 x `Terminal::SortedMulti` (`descriptor_route.rs:432,440,461`), `no field node on &WshInner` (`:440`) and `mismatched types: expected &Miniscript, found &WshInner` (`:443`). Zero errors from #929. | true; but only #910/#911/#915 |
| `to_miniscript.rs:689-699` converts a tap-leaf-root `SortedMultiA` | block is `:689-701` (the `return` is `:700`) | true, range short, see N2 |
| Pin adopted 2026-08-20, commit `5b4d20ad` | `git show -s`: 2026-08-20 "pin miniscript at ff4732e..." | true |
| `t_tr_tree` generates depth <= 2; the cell is inverted and panics "UPSTREAM REGRESSED" | `tests/common/mod.rs:1528-1560` (depth-2 arm restored); `tests/proptest_to_miniscript.rs:632` | true |
| The toolkit's depth gate "is already lifted" | `restore.rs@02052a17:3286,3438-3475`: "depth >=2 is NOT refused -- that gate was lifted 2026-08-20"; `ensure_taptree_depth_le_one` no longer exists; `ensure_taptree_wellformed` replaced it | true (see M5 for the cross-reference) |
| The toolkit's `subtree_contains_sortedmulti_a` gate in `restore.rs` | `restore.rs@02052a17:3362,3422` | true |
| Cargo.toml / FOLLOWUPS decision text | architect Q1 "Changes": "Kept by decision 2026-11-03 (architect miniscript-13-2): no 13.x port; drop when one release carries #910, #915, #911 and #929; the tripwire watches for it." | matches verbatim |
| "vestigial since the 2026-08-20 pin" (`ci.yml` comment) | The `[patch."...apoelstra/rust-miniscript"] miniscript = { path = "../rust-miniscript-fork" }` redirect was removed in `5350f8a7` (2026-04-30, md-codec v0.12.0). The 2026-08-20 pin added a git rev; it did not make the clones unused. | false date, see M2 |

## Q2. Do the deleted CI clone steps have a remaining consumer? Is the musl comment right?

Consumers: none. `git grep -nI 'rust-miniscript-fork\|md-codec-local-stack\|b308afd' -- . ':!vendor'` at the head
returns only history: `CHANGELOG.md` (2 lines), `MIGRATION.md`, old `design/` plans, decision docs and agent
reports, and the new comment in `ci.yml:219`. No `*.toml`, `*.yml`, `*.sh`, `*.py`, `Cross.toml`, `ci/`,
`scripts/` or `.cargo/config.toml` names the path; `fuzz/Cargo.toml:38-39` patches the same git rev as the
root, not the path; `Cargo.lock` has miniscript as `git+...?rev=ff4732e5...`. `git grep 'git clone'` over
`.github/workflows/` returns nothing. The path redirect that read `../rust-miniscript-fork` was deleted in
`5350f8a7`. `Swatinem/rust-cache` has no `workspaces:` entry naming the fork. The head's full CI passing
(including windows and macOS, which the clone step used to run on) confirms it. The deletion is correct.

The PR description declares the `.github/` edit, as RULES requires.

The musl-check comment is the right shape but not right: two statements in the same job comment are still
false (M2). The `FORK-FREE SCOPE` paragraph claims the clones were vestigial since the 2026-08-20 pin
(they were since 2026-04-30), and the aarch64 bullet two paragraphs down still says "same
registry-miniscript resolution as the native leg", which contradicts the paragraph the PR just wrote.

## Q3. Cross-check against the toolkit branch `fix/miniscript-tripwire-13-2`

State of the companion: the branch is LOCAL ONLY. `git ls-remote origin 'refs/heads/fix/miniscript-tripwire*'`
returns nothing; the worktree `/scratch/code/shibboleth/tk-t1-tripwire` is at `02052a17` (= origin/master)
with uncommitted edits to `.github/workflows/miniscript-fork-tripwire.yml`, root and crate `Cargo.toml`,
`fuzz/Cargo.toml`, `design/FOLLOWUPS.md` and untracked `ci/miniscript-tripwire.py`, `.test.sh`, `ci/fixtures/`.
I compared md against that working tree. It is in progress and may change.

Agreement: the keep-the-pin sentence is identical in both; both list #910, #915, #911, #929 as the drop
condition; both say 13.2.0 (2026-10-17) carries only #953, as #1039; both give BASELINE 13.2.0 and name the
workflow `.github/workflows/miniscript-fork-tripwire.yml`.

Divergences, md side (counted above):

- #929: the toolkit names the API ("PR #929 (generic `Descriptor::plan`) - present; absent from 13.2.0");
  md says "no clean source marker". They do not say the same thing, and the toolkit is right (M1).
- The toolkit tripwire's four markers are #910, #915 and #911 (two markers). #929 is not scanned; it is
  caught by the NEXT STEPS drop attempt in mr, which calls it (`mr-codec cost.rs:145`, recon table). md's
  sentence "a drop attempt's build catches it" is true only of that mr build, not of md's (M1).
- md cites the branch by name in three permanent places; see N1.

Divergences, toolkit side (NOT counted against this PR; for the toolkit T1 reviewer and the controller):

- T-a. Toolkit `design/FOLLOWUPS.md` (uncommitted) rewrites `upstream-miniscript-taptree-depth2-display-asymmetry`
  Status to "`open` (the depth>=2 restore refusal is live and slug-citing; ... what remains is the restore-gate
  lift itself)" and keeps "LIFT the restore depth gate (`restore.rs::ensure_taptree_depth_le_one`)". That is
  false: the gate was lifted 2026-08-20 and the function is gone (`restore.rs@02052a17:3286,3438-3475`), and the
  toolkit's own root `Cargo.toml` comment in the same working tree says "DEPTH->=2 TAPROOT RESTORE IS LIVE".
  md's FOLLOWUPS says the gate "is already lifted, see the companion", so the two currently contradict and md is
  the one that is right (M5).
- T-b. Toolkit `fuzz/Cargo.toml` (uncommitted) has a stray non-comment line, `[patch.crates-io]`.` at line 29,
  directly above the real `[patch.crates-io]`. `python3 -I -c 'import tomllib; tomllib.load(open("fuzz/Cargo.toml","rb"))'`
  in that worktree: `Expected newline or end of document after a statement (at line 29, column 18)`. Root and crate
  `Cargo.toml` parse. Almost certainly an edit slip; the fuzz build would fail.

## Findings

### Minor

**M1. `Cargo.toml` (new comment, about lines 33-36), PR body and the umbrella FOLLOWUPS text: the #929 sentence is wrong for md, and "WshInner shapes" is cryptic.**
The comment says "#929 has no clean source marker; a drop attempt's build catches it." Both halves fail:
a marker exists (`pub fn into_plan\b`, generic `Plan<Pk>`), and md's own drop attempt does not catch #929,
because md-codec and md-cli never call the plan API (I ran it: 10 errors, all #910/#911/#915, none from #929).
For md the practical effect is only a longer wait than md needs, so it fails safe. The same paragraph lists
"none of #915's `Terminal::SortedMulti` / `WshInner` shapes", but 13.2.0 HAS `WshInner` (the pre-#915 enum);
what md fails on is `Wsh::as_inner()` returning that enum (`error[E0609]: no field node on type &WshInner`).
A reader who greps 13.2.0 for `WshInner` will conclude the text is wrong.
Fix text (replace from "# the 13.2.0 source:" through "# the patch does not compile."; run: `cargo metadata
--offline --locked` OK, lock unchanged):

```
# the 13.2.0 source: no `Terminal::SortedMultiA` (#910; to_miniscript.rs,
# descriptor_route.rs), no `Descriptor::derive_at_index` and no `into_definite`
# (#911; derive.rs), and no `Terminal::SortedMulti`, with `Wsh::as_inner()` still
# returning the pre-#915 `WshInner` enum (#915; descriptor_route.rs). Dropping the
# patch does not compile: `cargo check -p md-codec --all-targets` against 13.2.0
# fails with errors from #910, #911 and #915 only. #929 (generic `Plan<Pk>`,
# `Descriptor::into_plan`; absent from 13.2.0) is not called by md-codec, so md's
# own drop attempt cannot catch it; mr-codec calls it (`cost.rs`), and it is a
# condition of the constellation-wide drop decision, not of this build.
```

The PR body has the same "#929 has no clean source marker" and "#915 (`SortedMulti` / `WshInner` shapes)"
sentences; they are not in the tree and need no edit after merge.

**M2. `.github/workflows/ci.yml:218-224` (musl-check comment): wrong date, and a residual contradiction two paragraphs down.**
(a) "were vestigial since the 2026-08-20 pin": the clones were unused since 2026-04-30, when `5350f8a7`
removed the `[patch]` path redirect to `../rust-miniscript-fork` (verified with `git show 5350f8a7 -- Cargo.toml`).
The 2026-08-20 pin is a different patch (a git rev). (b) The aarch64 bullet (`ci.yml` about line 231) still
says "same registry-miniscript resolution as the native leg, which already passes fork-free", while the
paragraph above now says miniscript resolves through the `[patch.crates-io]` git rev. Comment-only; the job
runs unchanged. Fix text, run with `actionlint -shellcheck= .github/workflows/ci.yml` (clean) and a YAML load (ok):

```
    # clones a miniscript fork any more (the `md-codec-local-stack` clone steps
    # were vestigial since 2026-04-30, when 5350f8a7 removed the `[patch]` path
    # redirect to `../rust-miniscript-fork`, and were removed 2026-11-03):
```

and

```
    #     binary, not merely compiled. Stays `-p md-cli` (the FORK-FREE SCOPE
    #     above) — same miniscript resolution (the root `[patch.crates-io]` git
    #     rev) as the native leg; the arm64 runner only changes the RUN target,
    #     not the dependency set. No `--skip` needed: md-cli has NO `mlock`
```

(only the middle two lines of the second block change; the other two are context.) The step names
"(native, fork-free)" and "(native arm64; output goldens, fork-free)" are covered by the PR's "label is historic".
Also change the PR body's and author report's "vestigial" attribution if they are reused.

**M3. Three other places still give "#953" as the reason for the pin, and one still lists a satisfied release as the way out.**
The PR rewrote the Cargo.toml rationale to say the Display fix is "no longer a reason", but did not touch:
(a) `.github/workflows/release.yml:11-16` ("this workspace pins rust-miniscript to a git rev for PR #953
(Display flattens ...)") - this is the stated reason md ships binaries not crates; (b) `fuzz/Cargo.toml:27-29`
("a crates.io miniscript 13.x that has neither PR #953's taptree Display nor PR #910's ..."), which is false for
13.2.0; (c) `design/FOLLOWUPS.md` `md-codec-unpublishable-while-patched-to-miniscript-master`, Options item 1,
"Wait for an upstream release carrying #953. Cleanest; not in our control." A release carrying #953 has existed
since 2026-10-17, so a reader following the advice literally finds it already met, bumps to 13.2.0, and gets 10
compile errors (fails closed; no card, no wrong output). The "Update 2026-11-03" paragraph above the list
mitigates (c) but does not edit it. Fix texts, each applied in a scratch worktree and checked (`actionlint` and
YAML load for `release.yml`; `cargo metadata --offline` on `fuzz/Cargo.toml`; `bash ci/code-docs-check.sh` ok):

`release.yml` lines 11-16, replace with:
```
# BINARIES RATHER THAN crates.io, deliberately: this workspace pins
# rust-miniscript to a git rev (root Cargo.toml `[patch.crates-io]`) because no
# crates.io release carries the APIs md-codec calls (PR #910, #911, #915; 13.2.0
# has none of them, and its Display fix, #1039, is no longer the reason), a
# [patch.crates-io] NEVER carries into a published crate, and a git dependency
# fails `cargo publish` outright. A binary built HERE has the patch applied. See
# design/FOLLOWUPS.md, `md-codec-unpublishable-while-patched-to-miniscript-master`.
```

`fuzz/Cargo.toml` lines 27-29, replace with:
```
# md-codec is built here against a crates.io miniscript 13.x that lacks PR #910's
# `Terminal::SortedMultiA`, PR #911's `derive_at_index` / `into_definite` and
# PR #915's `Terminal::SortedMulti` (13.2.0 has PR #953's Display fix, as #1039,
# and none of those three), and the fuzz targets fail to compile
```
(the next lines of that comment, "with `no variant ... SortedMultiA`, ...", are unchanged and still true).

FOLLOWUPS Options item 1, replace with:
```
  1. **Wait for an upstream release** carrying #910, #915, #911 and #929 (a release carrying #953 exists since 2026-10-17, 13.2.0, and is not enough). Cleanest; not in our control.
```

**M4. `crates/md-codec/src/to_miniscript.rs:869-894` still says the tap-leaf `sortedmulti_a` conversion is "unimplemented (R5)" (author's unfiled item 1). Graded Minor.**
The arm is reached only for a `sortedmulti_a` NESTED in a fragment (a tap-leaf root is handled earlier at
`:689-701`, `return Ok(...)` at `:700`). Its comment (`:877-887`: "no SortedMultiA conversion has been written yet
(R5)") and its user-visible message (`:889-891`: "...legal only as a tap-leaf root child (BIP-386/387), and that
conversion is unimplemented (R5); nested inside a miniscript fragment it is forbidden outright") both contradict
`:689-701`. A holder who hits it is told the legal position is unimplemented when the right remedy is to make
`sortedmulti_a` a whole tap leaf. Grade: Minor under the likelihood test. It fails closed (the call returns
`Err`; no descriptor, address or card is produced), no printed command fails, and reaching it needs an md1
card with a nested `sortedmulti_a` (`md encode` refuses to write one, so it takes a card from another tool,
for example the toolkit `bundle` intake gap, architect V10) plus a derive/convert call: two unusual conditions.
Reachability by an actual card was read from the code and the arm's own comment; I did not construct a card.
No test, doc or snapshot pins the string (`grep -rn 'not yet converted\|unimplemented (R5)' crates docs design/*.md`
finds only the source). Fixing it changes a user-visible string, so the author was right to file it rather than
fold it here. Owning phase: the next md patch release that touches `to_miniscript.rs`, and no later than
toolkit cycle T3, which adds the toolkit-side nested-refusal wording (the architect's tie-break is one wording
across md and the toolkit). File this at the tail of `design/FOLLOWUPS.md` (applied in a scratch copy;
`bash ci/code-docs-check.sh` ok):

```
### `md-codec-sortedmultia-refusal-text-says-unimplemented` — `to_miniscript.rs` still tells the holder the tap-leaf `sortedmulti_a` conversion is "unimplemented (R5)"

- **Surfaced:** 2026-11-03, md PR 45 (M1) author report; graded Minor in `design/agent-reports/review-m1-miniscript-docs.md`.
- **Where:** `crates/md-codec/src/to_miniscript.rs:869-894` (the `(Tag::SortedMultiA, Body::MultiKeys { .. })` arm of `node_to_miniscript`; comment `:870-887`, message `:889-891`). It contradicts the handler at `:689-701`, which has converted a tap-leaf-root `Tag::SortedMultiA` since the ff4732e pin (2026-08-20, `5b4d20ad`).
- **What:** the arm is reached only when `sortedmulti_a` is nested inside a fragment (the BIP-386/387 refusal). Its message ends "...and that conversion is unimplemented (R5); nested inside a miniscript fragment it is forbidden outright" and its comment says "no SortedMultiA conversion has been written yet (R5)". Both are false. A holder who hits it is told the legal position is unimplemented, when the remedy is to make `sortedmulti_a` a whole tap leaf. It fails closed (no card or address is produced), and no test pins the string.
- **Fix:** replace the message with md-cli's existing wording (`crates/md-cli/src/parse/template.rs:1538`): "sortedmulti_a() is valid only as a taproot leaf (BIP-386/387); it cannot be nested inside a miniscript fragment" (one wording across md and the toolkit, architect miniscript-13-2 Q3), and rewrite the comment to say a tap-leaf root never reaches this arm. Message-only: PATCH, no wire change. Re-grep `crates`, `docs` and snapshots for the old string before editing.
- **Status:** `open`
- **Owning phase:** the next md patch release that touches `to_miniscript.rs`, and no later than toolkit cycle T3 (which adds the toolkit-side nested-refusal wording, so both read alike).
- **Tier:** `next-cycle`
```

**M5. `design/FOLLOWUPS.md`, `upstream-miniscript-taptree-depth2-display-asymmetry` Status line: "its depth gate is already lifted, see the companion" points at a companion that currently says the opposite.**
The fact is right (Q1 table) but the cross-reference is to the toolkit's FOLLOWUPS entry, whose in-progress
rewrite (T-a above) says the refusal is live. Until the toolkit text is corrected, a reader who follows "see the
companion" finds a contradiction and cannot tell which is true. Make the md text self-contained (applied in a
scratch copy; `bash ci/code-docs-check.sh` ok):

```
(3) is the toolkit's, and is done: its depth gate is lifted (`restore.rs::ensure_taptree_wellformed` at toolkit `02052a17`, lifted 2026-08-20; `ensure_taptree_depth_le_one` no longer exists).
```
in place of "(3) is the toolkit's (its depth gate is already lifted, see the companion)." The toolkit T1 text
needs its own fix (T-a); that is the controller's to route.

### Nit

- **N1.** The branch name `fix/miniscript-tripwire-13-2` is written into permanent text in three places
  (`Cargo.toml` pin comment; the Companion lines of `md-codec-sortedmulti-a-to-miniscript-rendering-gap` and
  `upstream-miniscript-taptree-depth2-display-asymmetry`, "also being corrected, toolkit branch ..."). The
  branch is unpushed and will be deleted after merge, and the present-tense "watches for it" is not true until
  T1 merges (toolkit master's tripwire still has BASELINE 13.1.0). The author brief directed the citation, so
  this is not the author's fault. Controller: merge toolkit T1 before or with M1, then use the Cargo.toml
  replacement already shown in M1's scratch edit: `# carries #910, #915, #911 and #929; mnemonic-toolkit's` /
  `# .github/workflows/miniscript-fork-tripwire.yml watches for it.` (applied; `cargo metadata --offline --locked` ok);
  and "(corrected in toolkit T1, 2026-11-03)" for the two Companion lines.
- **N2.** Line ranges cited are short: `to_miniscript.rs:689-699` should be `689-701`; `:869-889` should be `869-894`
  (message `:888-893`). Both in the first FOLLOWUPS UPDATE.
- **N3.** `musig-bip390-encode-support` (author's unfiled item 2, `design/FOLLOWUPS.md` about line 2142):
  "latest is 13.1.0" and "(resolved 13.0.0/13.1.0)" are stale. The substance holds: 13.2.0's only `musig` text is
  commented-out BIP-388 vectors (`src/descriptor/wallet_policy/mod.rs:321,322,376`; 0 hits elsewhere in `src/`), same as
  the pin. Upstream-gated entry, fails closed, so Nit. Owning phase: this PR (one phrase; fold it in). Exact edit
  (applied in a scratch copy; docs check ok), replacing the text between "md-codec pins `miniscript = "13"` (" and
  "no 14.x;":
  ```
  resolves to git `ff4732e5` through `[patch.crates-io]`; crates.io 13.0.0 to 13.2.0 are the same on this point), which has **NO** BIP-390 `musig()` support — rust-miniscript has not implemented it (latest crates.io is **13.2.0**, 2026-10-17, re-checked 2026-11-03: its only `musig` text is commented-out BIP-388 vectors in `src/descriptor/wallet_policy/mod.rs`;
  ```
- **N4.** Two code comments still say #953 is unreleased: `crates/md-cli/src/compile.rs:153` ("merged but in no
  release through 13.1.0") and `crates/md-codec/tests/bitcoind_differential.rs:766-771` ("STILL UNRELEASED ...
  ff4732e is not an ancestor of miniscript-13.1.0"). Literally true of 13.1.0, misleading as of 13.2.0 (#1039).
  Comment-only edits, so not for this docs PR unless the author wants them; file with the M4 patch release.
- **N5.** FOLLOWUPS convention (header lines 44 and 2036): `Status: resolved <COMMIT>` and move the entry to
  "Resolved items". The two newly resolved entries are marked `**resolved** <date> ... (md pin commit ...)`
  in place (lines about 2074 and 2094, in the tail after "Convention notes") and keep `Tier: next-cycle` /
  `upstream`. The file already has 4 other `**resolved**` Status lines and 166 mixed forms, so the convention is
  not enforced; do not hold the PR for it.
- **N6.** The first UPDATE of `upstream-miniscript-taptree-depth2-display-asymmetry` enumerates the stale phrases
  ("13.0.0", "NOT in any release", ...) but omits the Companion line's "The same miniscript bump > 13.1.0
  dissolves both entries, plus ... the toolkit's own `[patch.crates-io]`", which is false for a bump to 13.2.0
  (the pin cannot drop). The UPDATE's headline ("The pin is NOT removable ... 13.2.0") limits the harm. Fix
  (applied in a scratch copy; docs check ok): in the UPDATE, replace `"generator ... depth <= 1" and "reparse
  asserted Err" are the 2026-06 filing` with `"generator ... depth <= 1", "reparse asserted Err" and the Companion
  line's "The same miniscript bump > 13.1.0 dissolves both entries, plus ... the toolkit's own
  `[patch.crates-io]`" are the 2026-06 filing`.
- **N7.** The new text cites the recon and architect report by bare slug (`cycle-prep-recon-miniscript-13-2`,
  "architect miniscript-13-2"). Locations: recon at mnemonic-toolkit branch `recon/miniscript-13-2` @`4988f115`
  (pushed; `design/agent-reports/cycle-prep-recon-miniscript-13-2.md`); architect at the coordinator repo,
  `coordinator/agent-reports/architect-2026-11-03-miniscript-13-2.md`. Add the location once, in the Cargo.toml
  comment or the first UPDATE.

## Q4. The two unfiled items

| Item | Grade | Why | Exact text | Owning phase |
| --- | --- | --- | --- | --- |
| `to_miniscript.rs:869-889` stale "unimplemented (R5)" | Minor (M4) | False clause in a user-visible refusal; fails closed; needs a nested-`sortedmulti_a` card plus a derive call; the string is unpinned by any test | the FOLLOWUPS entry in M4 | next md patch release touching `to_miniscript.rs`, no later than toolkit T3 |
| `musig-bip390-encode-support` "latest is 13.1.0" | Nit (N3) | One stale version number on an upstream-gated entry; the claim it supports is still true at 13.2.0 and at the pin | the replacement in N3 | this PR (fold in) |

## Reproduction notes

- Probe crate: `miniscript = "=13.2.0"` and `"=13.1.0"`, `cargo run --offline`, tree
  `tr(K,{{pk(A),pk(B)},pk(C)})` with fixed test keys. 13.2.0 round-trips and reparses; 13.1.0 prints
  `{{pk(A),pk(B),pk(C)}}` and fails to reparse. Probe and its target dir were removed afterwards.
- Drop attempt: scratch worktree `/scratch/code/shibboleth/md-m1-review-drop` (detached at `09f6c7dc`),
  `[patch.crates-io]` removed, `CARGO_TARGET_DIR` on `/scratch/cache` (since removed). That worktree now holds
  every fix text above as an uncommitted diff (`git -C /scratch/code/shibboleth/md-m1-review-drop diff`:
  `Cargo.toml`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `fuzz/Cargo.toml`,
  `design/FOLLOWUPS.md`), validated as stated in each finding, for the controller to take or discard. The PR
  branch was not touched.
- Toolkit state read at `/scratch/code/shibboleth/tk-t1-tripwire` (read-only; I did not run anything there that
  writes).

0 C / 0 I / 5 M / 7 N
