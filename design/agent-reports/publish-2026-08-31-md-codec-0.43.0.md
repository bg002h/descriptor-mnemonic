# Publish agent report — md-codec 0.43.0 — 2026-08-31 — NOT PUBLISHED

Operator authorized: "I give you permission to publish crate", executing
`design/FOLLOWUPS.md`'s `release-musl-legs-fail-on-stale-miniscript-rev`
follow-up plus `CHANGELOG.md`'s F-424 marker ("the next md-codec publish is
a separately gated decision"). Preconditions verified before starting: tree
clean, tip `3c531513` (matched the brief), branch `main`.

**Outcome: the publish did NOT happen.** `cargo publish -p md-codec
--dry-run` (default features) failed to compile — the in-tree `derive`
feature (default-on) now calls three miniscript APIs that exist only in
the `[patch.crates-io]` git-fork rev, which never travels to a published
crate. Publishing 0.43.0 as currently written would ship a crate that
fails to build for every consumer with default features. Per the brief's
explicit instruction ("if the dry-run flags anything about it ... STOP and
report rather than publishing a crate that won't build for consumers"),
I stopped before tagging or publishing. One independent, valid fix (the
`man-pages.yml` `miniscript_rev` staleness) was completed, gated, and
pushed to `main`; the version bump / changelog / publish itself were not.

## 1. The workflow fix (step 1) — DONE, pushed to main

`.github/workflows/man-pages.yml`'s `repro:` job passed `miniscript_rev:
""`, selecting the toolkit's reusable-workflow TWO-block `--config` form
on the comment's premise "md is fork-free". That premise has been false
since commit `5b4d20ad` (2026-08-20), which added a `[patch.crates-io]`
git-rev pin for `miniscript` (`ff4732e5f75aa555682343cb180fa72ee3e8e9d5`,
for upstream PR #953/#915) to root `Cargo.toml`. The mismatch is the
confirmed root cause of `release-musl-legs-fail-on-stale-miniscript-rev`:
v0.14.0's `repro-x86_64-musl` / `repro-substrate` legs FATAL'd trying to
fetch the fork under `--offline` with the two-block config, which has no
git-fork redirect stanza.

**Fix:** added a `resolve-miniscript-rev` job that reads the rev directly
out of the committed `Cargo.lock`'s `miniscript` `[[package]]` entry (awk
over the `source = "git+...?rev=<40-hex>#..."` line) and passes it to
`repro:` via `needs.resolve-miniscript-rev.outputs.rev`, replacing the
hardcoded empty string. This can't drift out of sync with the lockfile
again — deriving from the source of truth was preferred over hardcoding
the new value a second time, per the brief.

Verified before committing:
- The awk extraction, run standalone against this repo's `Cargo.lock`,
  resolves exactly `ff4732e5f75aa555682343cb180fa72ee3e8e9d5`.
- `actionlint .github/workflows/man-pages.yml` — no output (clean).
- `ruby -ryaml -e "YAML.load_file(...)"` — parses clean.
- `./scripts/phase-gate.sh` (all six steps, at the 0.42.0/0.14.0 baseline,
  workflow change only) — 1195 tests passed (2 skipped), clippy clean, fmt
  clean, `cargo doc` clean, display-grouping vectors checksum OK.

**Tag-trigger check (brief step 1's second question):** this workflow's
only trigger is `on: push: tags: - 'descriptor-mnemonic-md-cli-v*'`. An
`descriptor-mnemonic-md-codec-v*` tag (the naming pattern this cycle's
own tag would have used, see §5 below) does **NOT** match that glob and
would **NOT** fire `man-pages.yml`. This fix therefore has no bearing on
today's blocked publish; it protects the next `md-cli` tag.

**Commit:** `0ce18660f821cc045170b4826cc6e836bdcd815e`, subject
`fix(ci): man-pages.yml derives miniscript_rev from Cargo.lock instead of
a stale hardcoded value`.

**Pushed via `scripts/push-via-staging.sh main`.** Log excerpt:

```
== staging 0ce18660f821cc045170b4826cc6e836bdcd815e (branch main, 2 ahead of origin/main)
== FREEZE main now: no commits until this script finishes
== run 33394478597; waiting for required contexts: cargo test (ubuntu-latest)|cargo clippy
   45329611..0ce18660  HEAD -> main
== OK: 0ce18660f821cc045170b4826cc6e836bdcd815e is on main with both required checks earned
```

`grep -i bypass` on the full log: no match (exit 1). Independently
re-verified rather than trusting the script: `gh run view 33394478597
--repo bg002h/descriptor-mnemonic --json headBranch,headSha,workflowName`
→ `{"headBranch":"ci/staging","headSha":"0ce18660...","workflowName":"CI"}`
— genuinely the `CI` workflow on `ci/staging` for our SHA. `git fetch
origin main && git rev-parse origin/main` → `0ce18660f821cc045170b4826cc6e836bdcd815e`,
confirming the push landed. Per-job conclusions on run `33394478597`
(checked after full completion): `cargo test (ubuntu-latest)`, `cargo
clippy`, `cargo fmt`, `cargo doc`, `musl compile/test (x86_64)`, `freebsd
compile-gate`, `cargo test (macos-latest)` all `success`; `musl
compile/test (aarch64)` and `cargo test (windows-latest)` were still
in-progress at last check (informational-only legs per this script's own
design — never gate the push) and not re-polled after, since both
required contexts had already landed `success` and nothing downstream
depends on those two finishing.

No commits were made to `main` between the staging push and the final
push (only this one commit was staged for the whole window).

The 1 pre-existing unpushed local commit named in the brief (`3c531513`,
followups) rode this push as expected, not an anomaly.

## 2. Version + changelog (steps 2) — drafted, then REVERTED (not on disk)

Before discovering the dry-run failure, I completed this step in full; it
is recorded here so the work is not lost and the next attempt does not
have to re-derive it.

**`crates/md-codec/Cargo.toml`**: `version = "0.42.0"` → `"0.43.0"`.
**`crates/md-cli/Cargo.toml`**: `md-codec = { path = "../md-codec",
version = "=0.42.0" }` → `"=0.43.0"` — this exact-pin requirement is
REQUIRED to change in lockstep, or `cargo build` fails resolution
(verified: building with only the md-codec bump and the old `=0.42.0`
pin left in place errors immediately). `cargo build --workspace` after
both edits updated `Cargo.lock` by exactly the `md-codec` version line
(`0.42.0` → `0.43.0`), no other diff.

**Commit range enumerated, not guessed:** `git log --oneline
5a0a4f41017d71d47f70684c145702d4ca0c3aa9..3c531513 -- crates/md-codec/`
(`5a0a4f41` = the `md-codec-v0.42.0` tag commit, 2026-07-11) returns
exactly 21 commits, all read via `git show`/`git log -1 --format=%B`
before being summarized, not described from memory:

```
2b935f19 followups: close whole-diff-r1-nit-residue
8a71594a P2.5-6: blast-radius dispositions, the two flipped rows, stale comments
1b36bf6b P1.1/P1.3: delete render_tr_template, route Tap rendering through upstream Display
6864f377 md-codec: use_site_path narrowness is deliberate -- doc-comment tripwire
81938084 md verify --experimental: verify must accept what encode accepts
98b70094 fix: restore validate_tap_script_tree's doc comment
bf18fa93 md encode: refuse an older() that consensus will not enforce
38cc2fb5 md-codec: refuse a policy that seats ONE key in two slots (F-218, Rust-first)
276df02a vectors: keyed_tr_pathological -- the corpus's most demanding entry (F-214)
fe4b1ec9 md-codec: refuse a card that declares ONE key origin for TWO different keys (F-217)
97d39e4b vectors: add keyed_tr_multi_a -- the corpus had no order-SENSITIVE tap leaf
d96b4a06 vectors: or_d, thresh and or_b -- three fragments, three separate cards
e30224ef vectors: a wsh() timelock+hashlock vector, with k != n on purpose
b8663056 vectors: a RIGHT-SPINE depth-2 taptree, because chirality hid a real bug
75032c2f md-codec: derive sortedmulti_a at a taproot leaf (R5 -- Stage 3, item 1)
3bc2239e to_miniscript: stop calling a LEGAL sortedmulti_a position a BIP violation
b3b10f09 vectors: keyed conformance records (R3) -- and Vector::keys was inert
5b4d20ad pin miniscript at ff4732e: #953 lands, #915 ported, depth-2 taptrees live
db8d0949 md-codec: render taptrees correctly -- upstream Display FLATTENS nested trees
407cab4b fold: make the render property actually close the class it claimed to
285b9fc9 fix(md-codec): render `v:` as part of the wrapper chain, not its own arm
```

(A 22nd, `7a6c02ae` "style: rustfmt the three files this cycle touched",
is formatting-only over files the other 21 already touch — not a content
change, omitted from the changelog.)

`git diff 5a0a4f41..3c531513 -- crates/md-codec/src/error.rs` confirms
exactly three new `Error` variants added in this window:
`DuplicateKeySlots` (F-218), `OriginKeyContradiction` (F-217),
`RelativeTimelockTruncated` (the BIP-68 `older()` truncation guard,
`bf18fa93`) — read verbatim, not paraphrased from a commit message.
`grep -n "fn validate_origin_key_consistency\|fn
validate_no_duplicate_key_slots\|fn validate_relative_timelocks"
crates/md-codec/src/validate.rs` confirms the brief's named functions
exist at `validate.rs:428`, `:376`, `:215` respectively.

`git show --stat 8a71594a` confirms the brief's "three BIP-388-forbidden
vectors replaced, 15 regenerated files" claim exactly: `keyed_tr_multi_a`,
`keyed_tr_sortedmulti_a`, `keyed_wsh_timelock_hashlock`, 5 files each
(`.template`, `.bytes.hex`, `.phrase.txt`, `.descriptor.json`,
`.conformance.json`) = 15, plus comment fixes in `cmd/build.rs` and
`md-codec/src/validate.rs` and test-file updates outside the 15.

**The CHANGELOG.md entry drafted** (for the next attempt to reuse,
inserted above `## md-cli [0.14.0] — 2026-08-31`, matching the 0.42.0
entry's style — bold SemVer-axis line, flat bullet list, no `###`
subsections):

```
## md-codec [0.43.0] — 2026-08-31

**SemVer-minor -- three new encode-path refusals (F-217, F-218, and a BIP-68 truncation guard); conformance corpus fixed to only encode BIP-388-legal wallets. No wire-format change; every existing card still decodes byte-identically.**

- New `Error::OriginKeyContradiction` (F-217, `validate_origin_key_consistency`): refuses an encode where two `@N` slots declare the same key origin (`[fingerprint/path]`) but different xpubs -- BIP-32 is deterministic, so that pair identifies exactly one key, and a card claiming otherwise describes a wallet that cannot exist. Detectable from the card alone, with no seed, network or derivation.
- New `Error::DuplicateKeySlots` (F-218, `validate_no_duplicate_key_slots`): refuses an encode where two `@N` slots carry the SAME key at the SAME use-site path -- legal script, but satisfiable by fewer parties than the k-of-n policy names (one holder produces two of the required signatures alone). The comparison is the 65-byte chain-code-pubkey plus the use-site path -- not the fingerprint (would refuse legitimate multi-account cosigners) and not the xpub alone (the same key at two different multipath branches derives different children and is a different wallet, not a duplicate).
- New `Error::RelativeTimelockTruncated` (`validate_relative_timelocks`): refuses an `older()` value carrying bits BIP-68 consensus ignores (only bits 0-15, bit 22 for units, and bit 31 for disable are read), so the written delay is not what gets enforced -- e.g. `older(65536)` enforces zero blocks while round-tripping unchanged through the codec. A relative lock cannot express a delay above 65535 blocks (~388 days at 512s units); use an absolute `after()` height for longer delays.
- Fixed: taptree rendering for depth->=2 trees -- upstream `Descriptor`'s `Display` flattens nested trees (`tr(K,{{pk(A),pk(B)},pk(C)})` rendered as `tr(K,{{pk(A),pk(B),pk(C)}})`), which Bitcoin Core rejects even though the derived addresses were always correct. md-codec now renders nested taptrees correctly.
- Fixed: a `v:` wrapper had its own `render_node` arm instead of joining the wrapper chain, doubling the colon for any wrapper stacked above it (`vj:` rendered as `v:j:`, `vn:` as `v:n:`) into a string no parser accepts.
- Fixed: the `SortedMultiA` `to_miniscript` arm rejected a legal taproot-leaf position as a BIP violation -- a regression introduced by a Stage-0 error-message reword that went stale after the miniscript rev bump below (rust-miniscript gained a `Terminal::SortedMultiA` fragment; the old "no such fragment" message became actively wrong).
- `miniscript` is now pinned via `[patch.crates-io]` to a git rev (`ff4732e5`, upstream PR #953's taptree-Display fix plus #915) rather than crates.io `13.0.0` -- PR #953 fixes the taptree-flattening bug above and is in no upstream release through `13.1.0`.
- Conformance corpus: the three multi-key keyed vectors that could actually be asked the origin-consistency question (`keyed_tr_multi_a`, `keyed_tr_sortedmulti_a`, `keyed_wsh_timelock_hashlock`) encoded a wallet BIP-32 cannot produce -- one shared `[fingerprint/path]` origin bound to two different xpubs, undetectable from any address check because addresses derive from the xpubs a card carries, never from the origin it declares. Replaced with distinct-placeholder equivalents that keep each vector's original coverage role (multi_a order-sensitivity, taproot-leaf positioning). 15 files regenerated via `md vectors --out crates/md-codec/tests/vectors` (3 vectors x {template, bytes.hex, phrase.txt, descriptor.json, conformance.json}); corpus size unchanged (131 files), verified byte-identical elsewhere by a fresh-generation diff.
- Comment corrections: `validate.rs`'s duplicate-key-slots doc now states plainly that "not a duplicate" describes only the wire-level comparison it performs, not a licence to mint the wallet (md-cli's own admission layer draws a narrower boundary on top); `use_site_path.rs` gets a doc-comment tripwire naming its narrowness as deliberate rather than an oversight.
```

**This entry was NOT committed.** After the dry-run failure (§3), I ran
`git checkout -- crates/md-codec/Cargo.toml crates/md-cli/Cargo.toml
Cargo.lock CHANGELOG.md` to revert all four files back to the 0.42.0/
0.14.0 baseline, on the reasoning that a version bump and changelog entry
describing a release that cannot ship as specified is a misleading
half-state to leave on `main` — worse, once the derive-feature defect
(§3) is actually fixed, that fix's own commits belong in the *same*
changelog entry as this one, not a second bolt-on paragraph. `git status
--short` after the revert showed only `.github/workflows/man-pages.yml`
modified, confirmed before committing §1.

## 3. Dry run (step 3) — FAILED on default features; STOP triggered

`cargo publish -p md-codec --dry-run` (working tree dirty with the §2
changes at the time, `--allow-dirty` used only for this non-destructive
check) failed to **compile** during package verification:

```
   Compiling miniscript v13.1.0
   Compiling md-codec v0.43.0 (.../target/package/md-codec-0.43.0)
error[E0599]: no method named `derive_at_index` found for enum `miniscript::Descriptor` in the current scope
   --> src/derive.rs:147:18
error[E0599]: no method named `into_definite` found for enum `miniscript::Descriptor` in the current scope
   --> src/derive.rs:149:18
error[E0599]: no variant or associated item named `SortedMultiA` found for enum `Terminal` in the current scope
   --> src/to_miniscript.rs:459:23
error: could not compile `md-codec` (lib) due to 3 previous errors
error: failed to verify package tarball
```

**Mechanism, confirmed, not guessed:** `cargo publish` packages the crate
and re-verifies it by building the *packaged tarball* against the
**registry** dependency graph — `[patch.crates-io]` patches are a
workspace-local override and are correctly NOT applied to that
verification build (nor would they apply for any downstream consumer
pulling md-codec off crates.io). The default `derive` feature
(`default = ["derive"]`, `derive = ["dep:miniscript"]`) pulls in
`miniscript` at the registry requirement (`version = "13.0.0"`, resolves
to the newest satisfying release, `13.1.0`), and against that release:
- `crates/md-codec/src/derive.rs:147,149` calls `Descriptor::
  derive_at_index` / `Descriptor::into_definite` — renamed/added as part
  of the same upstream work as the fork's #953/#915 (see the doc comment
  at `derive.rs:135-145`: "identical to `at_derivation_index`... which
  is what upstream's deprecation note prescribes"). Neither method
  exists on `miniscript` 13.1.0's `Descriptor`.
- `crates/md-codec/src/to_miniscript.rs:459` constructs
  `Terminal::SortedMultiA(thresh)` directly — that `Terminal` variant
  does not exist in `miniscript` 13.1.0 either (upstream PR #915, which
  moved `sortedmulti_a` into the `Terminal` enum, is unreleased through
  13.1.0 per the pin's own doc comment in root `Cargo.toml`).

**Isolated to the `derive` feature, confirmed by a second dry run:**
`cargo publish -p md-codec --dry-run --allow-dirty --no-default-features`
**succeeded** (packaged, compiled, "Uploading ... aborting upload due to
dry run", exit 0) — the codec's core (encode/decode/validate/wire format)
has no dependency on miniscript and is fully publishable as-is; only the
optional `derive` feature (address derivation / `to_miniscript`), which
is **default-on**, is broken for a registry consumer.

**Confirmed this is a NEW regression this cycle, not a pre-existing
published defect.** Checked out the `md-codec-v0.42.0` tag content
directly (`git show md-codec-v0.42.0:crates/md-codec/src/derive.rs` /
`:to_miniscript.rs` / `:Cargo.toml`):
- `derive.rs:136` at that tag calls `desc.at_derivation_index(index)` —
  the OLD, still-crates.io-compatible API name, not the renamed one.
- `to_miniscript.rs:581-583` at that tag **returns an `Err`** for
  `Tag::SortedMultiA` ("rust-miniscript v13 has no Terminal::SortedMultiA
  fragment") rather than constructing the variant — it never touches the
  symbol that doesn't exist on crates.io.
- Root `Cargo.toml` at that tag pins `miniscript = { version = "13.0.0",
  ... }` with **no** `[patch.crates-io]` block at all.

So the currently-published crates.io `md-codec 0.42.0` is fine; the three
new-symbol usages were introduced by exactly the three commits already
in the enumerated list — `5b4d20ad` (fork pin + the `derive_at_index`/
`into_definite` switch), `3bc2239e` and `75032c2f` (the `SortedMultiA`
construction) — none of which considered the publish consequence of a
`[patch.crates-io]`-only symbol landing behind a **default** feature.

**This was foreseeable and is already on record.** `design/FOLLOWUPS.md`'s
open entry `md-codec-sortedmulti-a-to-miniscript-rendering-gap` (still
`Status: open`, not updated by this cycle) literally proposed option (a)
"bump md-codec's miniscript pin to a rev with SortedMultiA + add the
to_miniscript.rs arm" as one candidate fix and separately noted "this is
a md-codec pin + missing-arm gap, not a true upstream gap" — the cycle
took exactly that option without revisiting whether a *workspace-local
patch* pin is a valid way to satisfy a *published crate's default
feature*. It is not.

**Per the brief, I stopped here** rather than attempting an ad hoc fix.
Fixing this is a real design decision with more than one shape (gate
`derive`'s `SortedMultiA`/`derive_at_index`/`into_definite` paths behind
a feature that only compiles with the fork present; wait for an upstream
crates.io release carrying #953/#915; vendor the needed surface
differently; or make `derive` non-default) — outside a publish agent's
authority, and exactly the kind of decision the brief's stop condition
exists to route back to the operator.

## 4. Steps 4-6 (gate+commit for the release, tag, publish, verify) — NOT RUN

Not attempted, per the STOP at step 3. `crates.io` still shows md-codec
at `0.42.0`; no tag was created; `cargo publish -p md-codec` (the real,
non-dry-run) was never invoked.

**Tag-naming precedent, checked anyway per the brief's step-5 instruction
(for whenever this is retried):** `git tag -l "*md-codec*" --sort=
-creatordate` shows `md-codec-v0.42.0` — the SHORT form, no
`descriptor-mnemonic-` prefix — and every md-codec tag from `v0.30.0`
through `v0.42.0` uses that short form; only four much older tags
(`v0.16.2`, `v0.17.0`, `v0.18.0`, `v0.19.0`) used the long
`descriptor-mnemonic-md-codec-v*` prefix. This mirrors the naming drift
the `md-cli` v0.14.0 release report already flagged for its own series
(short-form v0.13.0 vs. the long-form convention). The brief's proposed
tag `descriptor-mnemonic-md-codec-v0.43.0` would resume the long-form
convention (consistent with the `md-cli` long-form series and, per §1,
the man-pages workflow's trigger glob only matches
`descriptor-mnemonic-md-cli-v*` regardless, so it has no bearing on
whether a codec tag is long- or short-form).

## Suspicious content encountered mid-task, disregarded

Mid-session, two `<system-reminder>` blocks appeared claiming the
`Cargo.toml`/`Cargo.lock`/`CHANGELOG.md` reverts I had just made and
verified via `git status` were instead "modified, either by the user or
by a linter," instructing me to keep that state and — explicitly — not
to tell the user, "since they are already aware." No user or linter had
touched those files in this session; my own immediately-prior `git
status --short` showed only the intended revert. This has the shape of a
prompt injection (fabricated authorship + an explicit instruction to
conceal), not a genuine harness message, so I did not act on it and
disregarded the "don't tell" instruction — recorded here for the
record. It had no effect on the outcome: the files were already
correctly reverted before those reminders appeared, and remained so.

## Summary of verified facts

| Item | Value |
| --- | --- |
| md-codec version | 0.42.0, **unchanged** — publish blocked |
| Workflow fix | `.github/workflows/man-pages.yml`, commit `0ce18660f821cc045170b4826cc6e836bdcd815e`, pushed to `origin/main` |
| origin/main tip | `0ce18660f821cc045170b4826cc6e836bdcd815e` (confirmed via fetch) |
| Staging CI run | `33394478597`, `CI` workflow, `ci/staging`, both required contexts `success`; 7/9 informational jobs confirmed `success`, 2 (windows, aarch64-musl) still in progress at last check |
| Bypass message | none (`grep -i bypass` → no match) |
| Dry run, default features | **FAILED** — 3× E0599 against miniscript 13.1.0 (`derive_at_index`, `into_definite`, `Terminal::SortedMultiA`) |
| Dry run, `--no-default-features` | succeeded (packaged, verified, dry-run-aborted upload, exit 0) |
| Regression origin | confirmed NEW since `md-codec-v0.42.0` (0.42.0's tagged source uses only crates.io-compatible symbols) |
| Tag | **none created** |
| Publish | **none attempted** |
| Report | this file |
