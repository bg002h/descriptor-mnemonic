# Release report — md-cli 0.14.0 — 2026-08-31

Release agent execution log for the md-cli 0.14.0 release (wallet-form
converter + post-converter mini-cycle). Operator authorized: "permission to
push tag release." Preconditions verified before starting: `main` tree clean,
tip `39470c0d5775f7dbb09b7d7875a4bd8bdc110c18` (matched the freeze SHA given
in the brief).

## 1. Version bump

`crates/md-cli/Cargo.toml`: `version = "0.13.0"` → `"0.14.0"`.

`cargo build -p md-cli` updated `Cargo.lock` — diff was exactly the two lines
for `md-cli`'s own `[[package]]` stanza:

```
 name = "md-cli"
-version = "0.13.0"
+version = "0.14.0"
```

Grepped for other in-repo pins of the md-cli version string (`0.13.0`, plus
scripts/, gen-man config, README). The only hits were:

- `CHANGELOG.md:432` — the historical `## md-cli [0.13.0]` release heading
  (left as historical record, not a "pin").
- `design/agent-reports/R0-converter-spec-r1.md:10` — a persisted historical
  report, not touched.
- `crates/md-cli/tests/fixtures/decompose/generate.sh` (3 hits) — these
  reference **`mk 0.13.0`** (mnemonic-key's `mk-cli`, a different crate in a
  sibling repo), not md-cli. Confirmed by reading the surrounding comment
  block (lines 40-71): it is a documented flag-probe workaround for a stale
  `mk` binary on PATH, unrelated to md-cli's own version.

No man-page-generation version config or README version string exists in
this repo. **`md-codec` version was NOT touched** (stays at `0.42.0`) — its
next publish is F-424, a separately gated decision, per the brief.

## 2. CHANGELOG roll

Before this release, `CHANGELOG.md` had **seven** stacked
`## md-cli [Unreleased] — <topic>` H2 headers between the file's intro and
the `## md-cli [0.13.0] — 2026-07-11` heading (lines 7, 137, 143, 149, 194,
243, 292 in the pre-edit file) — one per cycle/fix landed since v0.13.0,
never consolidated. The brief called this whole span "the top release
section" and noted it already covers both the wallet-form-converter cycle
and the post-converter mini-cycle.

Transform applied:

- The first header (`## md-cli [Unreleased] — the wallet-form converter`)
  became `## md-cli [0.14.0] — 2026-08-31`, matching the exact heading style
  of every prior release (`## md-cli [X.Y.Z] — DATE`, no subtitle in the H2
  itself — confirmed by grepping all 13 prior `## md-cli [...]` headings).
  Added a `**SemVer-minor — ...**` bold summary line plus an expanded intro
  paragraph naming both cycles explicitly (the original paragraph named only
  the converter).
- The other six interior `## md-cli [Unreleased] — <topic>` headers were
  demoted to `### <topic>` — this repo's changelog already uses
  topic-titled `###` headers for sub-sections within one release (e.g.
  `### Refused, deliberately — BIP-388 key reuse`, `### Unchanged,
  deliberately`), so this is consistent with existing convention rather than
  an invented one. One bare `## md-cli [Unreleased]` (no subtitle, at old
  line 253) was removed outright since the `### Changed — ...` line
  immediately below it already served as the section's real header.
- Confirmed no stray `[Unreleased]` H2 remains between the changelog's intro
  and `## md-codec [0.42.0]` / `## md-cli [0.13.0]`; H2 ordering is now
  strictly descending (`0.14.0` → `md-codec 0.42.0` → `0.13.0` → ...).
- Two unrelated `[Unreleased]` occurrences elsewhere in the file are
  historical, pre-crate-split content from the v0.5/v0.6 era (line ~2225,
  "(empty — v0.6.0 just shipped...)") and were left untouched.
- Checked whether the file's convention keeps an empty `[Unreleased]` stub
  after a release: the only place that pattern appears is that same old
  v0.5/v0.6-era content; **no recent release** (0.13.0, 0.12.0, ... back
  through 0.7.1) left an empty stub — instead, the next cycle's fold just
  adds a fresh `## md-cli [Unreleased] — <topic>` header with real content
  when it lands (confirmed via `git show 5a0a4f41 -- CHANGELOG.md`, the
  0.13.0 release commit, which inserted its version section directly with
  nothing above it). So no stub was added, matching current convention. A
  changelog line (`CHANGELOG.md:2068`, historical prose) independently
  confirms this is the established practice: *"CHANGELOG / MIGRATION
  discipline: `[Unreleased]` entries consolidated at release time."*

`git diff --stat -- CHANGELOG.md`: 1 file changed, 16 insertions(+), 8
deletions(-).

## 3. Gate + commit

`./scripts/phase-gate.sh` — all six steps passed:

```
cargo nextest run --locked --all-features: 1195 tests run: 1195 passed, 2 skipped
cargo test --workspace --doc --all-features: 0 passed; 0 failed
cargo clippy --locked --all-targets --all-features -- -D warnings: clean
cargo fmt --check: clean
cargo doc --workspace --no-deps --document-private-items --all-features: clean (RUSTDOCFLAGS="-D warnings")
design/display-grouping-vectors.tsv.sha256: OK
phase-gate: all six steps passed
```

Staged exactly the three expected files (`CHANGELOG.md`, `Cargo.lock`,
`crates/md-cli/Cargo.toml`) and committed:

- Commit: `a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996`
- Subject: `release: md-cli 0.14.0 — wallet-form converter + key-reuse
  admission taxonomy + S->K mint`
- Body carries the gate summary above.

## 4. Push via staging

Ran `scripts/push-via-staging.sh main` — this repo's **first post-fix run**
of `df0e893e` (the fix that filters `gh run list` by
`--workflow CI --branch ci/staging` rather than an unfiltered `.[0]`).

Log (`push-staging-output.log`):

```
== staging a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996 (branch main, 12 ahead of origin/main)
== FREEZE main now: no commits until this script finishes
 * [new branch]        HEAD -> ci/staging
== run 33368575410; waiting for required contexts: cargo test (ubuntu-latest)|cargo clippy
   bdb031a4..a82bc3ee  HEAD -> main
 - [deleted]           ci/staging
== post-push straggler report ...
== OK: a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996 is on main with both required checks earned
EXIT:0
```

No "Bypassed rule violations" string anywhere in the log (`grep -i bypass`
exit 1 = no match). Independently re-verified run selection rather than
trusting the script alone, per the brief's instruction:

```
$ gh run view 33368575410 --repo bg002h/descriptor-mnemonic \
    --json headBranch,headSha,workflowName,status,conclusion
{"headBranch":"ci/staging","headSha":"a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996","workflowName":"CI", ...}
```

headBranch, headSha and workflowName all match exactly what the ritual
requires — this was genuinely the `CI` workflow on `ci/staging` for our SHA,
not an order-dependent misselection. Final per-job conclusions on that run
(confirmed after full completion, all nine `ci.yml` jobs): `cargo clippy`,
`cargo test (macos-latest)`, `cargo fmt`, `cargo doc`, `musl compile/test
(x86_64)`, `cargo test (ubuntu-latest)`, `musl compile/test (aarch64)`,
`freebsd compile-gate`, `cargo test (windows-latest)` — all `success`.

`origin/main` confirmed at `a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996` via
`git fetch origin main && git rev-parse origin/main`. No commits were made
to `main` between the staging push and the final push.

## 5. Tag

Created an **annotated** tag at the exact release commit:

```
git tag -a descriptor-mnemonic-md-cli-v0.14.0 \
  -m "release: md-cli 0.14.0 — wallet-form converter + key-reuse admission taxonomy + S->K mint" \
  a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996
git push origin descriptor-mnemonic-md-cli-v0.14.0
```

Verified on origin:

```
$ git ls-remote --tags origin descriptor-mnemonic-md-cli-v0.14.0 'descriptor-mnemonic-md-cli-v0.14.0^{}'
209378dd68a82e5513af8d8664c1df52182d9ba7	refs/tags/descriptor-mnemonic-md-cli-v0.14.0
a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996	refs/tags/descriptor-mnemonic-md-cli-v0.14.0^{}
```

The tag object (`209378dd...`) dereferences (`^{}`) to commit
`a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996` exactly.

### Naming-drift note

The prior release used the **short-form** tag `md-cli-v0.13.0`, against the
**long-form** series convention every other release in the repo's tag list
uses (`descriptor-mnemonic-md-cli-v0.12.0`, `-v0.11.3`, ... back through
`-v0.5.0`). This tag resumes the long-form convention. Confirmed via
`git tag -l "*md-cli*" --sort=-creatordate`.

### Correction to the brief's premise — this repo DOES have a tag-triggered
### workflow, and pushing this tag has real, non-trivial side effects

The brief stated: *"This repo has NO tag-triggered workflows (verified:
ci/fuzz/bitcoind/man-pages/vendor-freshness only) — the tag is the
release."* This is **factually wrong** about `man-pages.yml`, which triggers
directly on `push: tags: - 'descriptor-mnemonic-md-cli-v*'` — exactly the
pattern just pushed. This is not a new discovery about a hazard the repo's
maintainers were unaware of: it is the same **established, long-running
release-asset pipeline** every prior long-form-tagged release used (verified
via `gh release view descriptor-mnemonic-md-cli-v0.12.0`: authored by
`github-actions[bot]`, assets `md-man.tar.gz`, `md-0.12.0-{aarch64,x86_64}-
linux-musl.tar.gz`, `PROVENANCE.*.txt`, `SHA256SUMS.*`). It builds `md`,
generates man pages, creates a GitHub Release named after the tag, and (via
a matrix job gated on a toolkit-homed reproducibility check) attaches
statically-linked musl binaries for both architectures.

Because v0.13.0 was mistakenly tagged short-form, this pipeline **never
ran** for that release — `gh release view md-cli-v0.13.0` returns "release
not found." Resuming the long-form convention here was therefore not inert;
it re-enabled the full release-publishing pipeline that v0.13.0 silently
skipped. I proceeded (rather than stopping to ask) because: this is the
established mechanism used successfully for every prior long-form release,
not a novel action; the operator had already explicitly authorized the tag
push; and the brief's own naming-drift instruction implies awareness that
resuming the convention matters. The corrected fact changes what "the tag is
the release" means in practice, so it is recorded here rather than silently
absorbed.

### The pipeline ran and PARTIALLY FAILED — release is missing musl binaries

Run `33368957642` (`man-pages` workflow, triggered on the tag push,
`headSha` = `a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996`), final per-job
result:

```
man-pages: success
repro / build-container (resolve BUILT-DIGEST): success
repro / repro-aarch64-musl: skipped
repro / repro-x86_64-musl: FAILURE
repro / repro-substrate: FAILURE
musl-binary (matrix): skipped   (needs: repro, which failed)
```

The `man-pages` job itself succeeded: `gh release view
descriptor-mnemonic-md-cli-v0.14.0` shows the release was created
(`author: github-actions[bot]`, `published: 2026-08-31T07:36:01Z`) with
`md-man.tar.gz` attached. But `repro-x86_64-musl` and `repro-substrate` both
failed with the **identical** root cause:

```
error: failed to load source for dependency `miniscript`
Caused by:
  Unable to update https://github.com/rust-bitcoin/rust-miniscript?rev=ff4732e5f75aa555682343cb180fa72ee3e8e9d5#ff4732e5
Caused by:
  can't checkout from 'https://github.com/rust-bitcoin/rust-miniscript': you are in the offline mode (--offline)
##[error]Process completed with exit code 101.
```

**Root cause, confirmed not to be caused by this release's commit:**
`.github/workflows/man-pages.yml`'s `repro:` job call passes
`miniscript_rev: ""` — the comment says *"md is fork-free ⇒ EMPTY rev
selects the TWO-block --config form"* (crates-io + vendored-sources only,
no git-fork redirect stanza). That was true once, but root `Cargo.toml` has
pinned `miniscript` to a **git rev** since commit `5b4d20ad` ("pin
miniscript at ff4732e: #953 lands, #915 ported, depth-2 taptrees live"),
dated **2026-08-20** — over a month after the v0.13.0 release
(2026-07-11) and comfortably inside this release's own content window.
`git show a82bc3ee -- Cargo.lock | grep miniscript` returns nothing: this
release's own commit never touched the miniscript entries. Under
`--offline` with the two-block config, cargo cannot resolve the git-sourced
`miniscript` dependency, so both musl build legs fail identically.

Because v0.13.0's short-form tag silently skipped this whole pipeline, this
misconfiguration has been latent and undetected since 2026-08-20 — this
release is the first time it has actually run since the git-pin landed.

**Consequence:** `descriptor-mnemonic-md-cli-v0.14.0` on GitHub currently
carries only `md-man.tar.gz`, unlike every prior long-form release (which
also carry `md-<ver>-{aarch64,x86_64}-linux-musl.tar.gz`, `SHA256SUMS.*`,
`PROVENANCE.*.txt`). This is a genuine release-completeness gap.

**Scope decision:** I did not fix `man-pages.yml` or attempt to re-trigger
the pipeline. This is release mechanics (version/changelog/tag) work, not a
CI-infrastructure fix, and re-running or editing a release workflow that
performs a repro-container build + QEMU cross build + GitHub Release asset
upload is its own decision with its own blast radius, outside what
"permission to push tag release" authorized. **Recommended follow-up for the
operator:** fix `man-pages.yml`'s `repro:` job call to pass the correct
`miniscript_rev` (matching the pinned SHA in root `Cargo.toml`,
`ff4732e5f75aa555682343cb180fa72ee3e8e9d5`) so the reusable workflow selects
the three-block `--config` form, then decide whether to re-run/backfill
musl binaries for `v0.14.0` (and consider whether `v0.13.0` deserves the
same, since it never got a release at all).

## 5b. Install smoke test

```
cargo install --git file:///scratch/code/shibboleth/descriptor-mnemonic \
  --tag descriptor-mnemonic-md-cli-v0.14.0 md-cli --features cli-compiler \
  --root /tmp/claude-1000/relcheck --locked
```

Succeeded with `--locked` on the first try (no retry needed):

```
    Compiling md-codec v0.42.0 (.../md-codec)
    Compiling md-cli v0.14.0 (.../md-cli)
    Finished `release` profile [optimized] target(s) in 22.14s
  Installing /tmp/claude-1000/relcheck/bin/md
   Installed package `md-cli v0.14.0 (file:///scratch/code/shibboleth/descriptor-mnemonic?tag=descriptor-mnemonic-md-cli-v0.14.0#a82bc3ee)` (executable `md`)
EXIT:0
```

`/tmp/claude-1000/relcheck/bin/md --version` → `md 0.14.0`.

This smoke test is **independent** of the GitHub Actions pipeline above (it
is a local `git`-based install straight from the pushed tag) — it proves the
tag is installable exactly the way toolkit CI installs it, and that result
is unaffected by the `man-pages.yml` repro-gate failure. `/tmp/claude-1000/
relcheck` was removed after the check (`rm -rf`, confirmed via `ls` failing
afterward).

## Summary of verified facts

| Item | Value |
| --- | --- |
| md-cli version | 0.13.0 → 0.14.0 |
| md-codec version | 0.42.0, unchanged (F-424 gates the next publish) |
| Release commit | `a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996` |
| origin/main tip | `a82bc3ee7ffb6a6ed8869c99d5b303204d7ac996` (confirmed via fetch) |
| Staging CI run | `33368575410`, `CI` workflow, `ci/staging`, both required jobs + all 9 ci.yml jobs `success` |
| Bypass message | none (`grep -i bypass` → no match) |
| Tag | `descriptor-mnemonic-md-cli-v0.14.0` (annotated), object `209378dd68a82e5513af8d8664c1df52182d9ba7`, dereferences to `a82bc3ee...` |
| Tag naming | resumes long-form convention; v0.13.0 was short-form and got no release at all |
| man-pages.yml pipeline | `man-pages` job: success (release + man bundle published); `repro-x86_64-musl` + `repro-substrate`: FAILURE (pre-existing `miniscript_rev` misconfig, unrelated to this commit, dated 2026-08-20); `musl-binary` matrix: skipped |
| Install smoke test | PASS — `cargo install --git ... --tag ... --locked` succeeded, `md --version` → `md 0.14.0` |
| phase-gate.sh | all 6 steps passed — 1195/1195 tests, 0 clippy/fmt/doc warnings, checksum OK |
