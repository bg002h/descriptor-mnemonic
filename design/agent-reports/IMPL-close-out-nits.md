# IMPL-close-out-nits — close-out batch, three FOLLOWUPS entries

**Date**: 2026-08-31
**Agent**: implementer (dispatched by controller)
**Repo**: descriptor-mnemonic (this checkout: `descriptor-mnemonic-mdcli-mini`)
**Branch**: `close-out-nits`, built from `e5dcb24f` (the main checkout's HEAD
at dispatch time) via `git fetch ../descriptor-mnemonic main && git checkout
-b close-out-nits && git merge --ff-only FETCH_HEAD` — a clean fast-forward,
verified before any work began.

## Item 1 — `md-address-help-summary-is-blank`

**Commit `3e3e5903`.**

`md address`'s subcommand summary in `md --help` was blank because it had no
doc comment at all. Investigating the surrounding source (not just adding a
line) turned up a second, unfiled defect: `md descriptor`'s own `--help`
summary opened with a sentence that is accurate for `address`, not
`descriptor` — "Derive bitcoin addresses from a wallet-policy-mode
descriptor." `git blame` traced both that misplaced line and the `Address`
variant's own declaration to the same original commit (`9e122530f`,
2026-05-03): the line was written for `Address` and landed on the *next*
variant's (`Descriptor`'s) doc comment instead.

Fix: moved the line to `Address`'s own doc comment. `Descriptor`'s second
sentence ("Emit the CONCRETE output descriptor…") already read as a
complete, accurate summary once the misplaced line was gone, so no new
prose was needed there.

**Evidence** — built binary's `--help`, before/after:

```
# before
descriptor  Derive bitcoin addresses from a wallet-policy-mode descriptor. Emit the CONCRETE output descriptor -- real xpubs, key origins and the BIP-380 checksum -- for pasting into a coordinator
address

# after
descriptor  Emit the CONCRETE output descriptor -- real xpubs, key origins and the BIP-380 checksum -- for pasting into a coordinator
address     Derive bitcoin addresses from a wallet-policy-mode descriptor
```

Both subcommands' long `--help` (`md descriptor --help`, `md address --help`)
also checked for a clean lead paragraph.

## Item 2 — `whole-diff-r1-nit-residue` (4 of 5)

**Commit `2b935f19`.** Source: nits section of
`design/agent-reports/FOLD-mdcli-mini-whole-diff-r1.md`.

### N1 — cross-file "rule (1)" rendered-string consistency

Fetched the real `bitcoin/bips` `bip-0388.mediawiki` (raw GitHub) before
touching anything, per the constellation's rule on verifying external
protocol facts against source text. Its "Additional rules" section is plain
prose with **no bullet/ordinal numbering at all** — the repo invented "rule
(1)"/"rule (2)" itself. Counting the BIP's own paragraphs: (1) non-empty
key-placeholder vector, (2) pairwise-distinctness, (3) disjointness, (4) no
repeated musig `KI`, (5) placeholder ordering. So the repo's "rule (1)"
(pairwise-distinct) is really the BIP's *second* paragraph, and "rule (2)"
(disjointness) is really its *third* — **both** invented labels were off by
one, not just the one the review flagged. Renumbering only the flagged line
to "(2)" would have collided with the disjointness rule's existing "(2)"
label — a new inconsistency, not a fix.

Dropped the invented ordinal everywhere it appears (20 occurrences across 9
live source/test files — `bip388.rs`, `cmd/build.rs`, `error.rs`,
`parse/reuse.rs`, `decompose/mod.rs`, `md-codec/validate.rs`,
`tests/cmd_decompose.rs`, `tests/duplicate_key_slots.rs`,
`tests/n1_admission_taxonomy.rs`) in favor of naming each rule ("BIP 388's
pairwise-distinctness rule" / "the disjointness rule"), matching a phrasing
pattern already used elsewhere in `reuse.rs`. Historical review-report prose
under `design/` was left untouched (13 more occurrences there — those are
records of what a past review said, not living documentation).

Exactly one occurrence is a RENDERED string reaching an operator
(`parse/reuse.rs`'s `KeyAtDisjointUseSites` message, R-N1d) with a matching
pinned test string (`n1_admission_taxonomy.rs::MSG_N1D`); both were updated
to match byte-for-byte and verified by running the test.

**Evidence**: `cargo nextest run --locked -p md-cli -E 'binary(n1_admission_taxonomy)'`
— 31/31 passed, including `r_n1d_delta_refuses_at_encode` and
`r_n1d_message_meets_its_mandate` (the two tests pinning the changed
string). Plus `cmd_decompose`, `duplicate_key_slots`, and the `parse::reuse`
unit tests — 22 more, all green.

### N3 — CI/script behavior nit (doctests not widened to `--all-features`)

`.github/workflows/ci.yml` and `scripts/phase-gate.sh` both ran `cargo test
--workspace --doc` **without** `--all-features`, while every other gated
step (nextest, clippy, `cargo doc`) already carried it — and
`phase-gate.sh`'s own header comment already (incorrectly) claimed "the four
all-features lines below" were widened; that claim was off by one until this
fix made it true. Both now run `cargo test --workspace --doc --all-features`.

**Evidence**: gate output below (`cargo test --workspace --doc
--all-features: ok, 0 doctests` — unchanged behavior today since the
workspace has none, but the feature axis is no longer a latent hole).

### N4 — `--verify-against` existence-routing error-context plumbing

`resolve_verify_against` decided file-vs-literal-string with
`Path::new(arg).is_file()` before either branch could fail, but a decode
failure afterward gave no indication which branch was taken. Constructed
the exact collision the review reported (a real file literally named after
the pasted string, holding content that also fails to decode) and confirmed
the defect first: the message read "codec error: … does not start with HRP
md1" either way, with no branch context.

Added `CliError::VerifyAgainstUnreadable` (same exit code 1 as the
`CliError::Codec` passthrough it replaces; never a spend-equality verdict,
per SPEC R3's garbage-argument row) and rewrote `resolve_verify_against` to
record `is_file` once and label the eventual decode error with the branch
that actually ran — "a file exists at this path and was read as one…" vs.
"no file exists at this path, so it was read as a literal md1 string…".

Updated the one pre-existing pinned test whose assertion named the replaced
prefix, and added a new test
(`r3_verify_against_argument_colliding_with_a_real_filename_says_a_file_was_read`)
that builds the actual collision — a temp dir, a real file named after the
garbage argument, `Command::current_dir` pointed at it — since no existing
test exercised the FILE branch of this message at all.

**Evidence**: `cargo nextest run --locked -p md-cli -E
'binary(r3_verify_against)'` — 7/7 passed, including both the updated
garbage-argument test and the new collision test.

### N5 — missing test fixtures (`count_occurrences`'s `Body::Tr` arm)

`count_occurrences` (`parse/reuse.rs`) walks a decoded card's tree to find
R-N1a/R-N1d repeats; its `Body::Tr` arm (the tap internal key, a bare index,
plus recursion into the tap tree) had never been exercised by any CARD
fixture — every existing one (`r-n1a-keyed.txt`, `v-r5m1.txt`) puts its
repeat in `Body::MultiKeys`.

This shape is refused by the *current* shipped binary too (same predicate,
either arm — measured: `md encode "tr(@0/<0;1>/*,pk(@0/<0;1>/*))" --key
@0=<KEY 1> --path "48'/0'/0'/2'"` on today's binary exits 1, "unsupported: @0
appears at 2 use sites…"), so a new fixture needed the same
frozen-pre-refusal-binary treatment the existing two used: built `md` in a
scratch git worktree at `b8a64938` (the mdcli-mini plan's baseline commit —
the same one `r-n1a-keyed.txt` was minted from) and ran the same encode
command there, where it still succeeds (chunk-set-id `0x380f5`). Confirmed
on the *current* binary: `md decode` on this card completes at exit 0 with
`Finding::SamePathExpression`'s warning, proving the Tr arm now reaches the
classifier through a real decoded card, not a synthetic in-memory
construction.

New fixture: `crates/md-cli/tests/fixtures/n1/r-n1a-tr-internal-key.txt`,
provenance header in the same style and level of detail as
`r-n1a-keyed.txt` (mint command, baseline commit, key material, why no
generator exists). Added three tests to `n1_admission_taxonomy.rs`
(`r_n1a_tr_internal_key_card_refuses_at_descriptor`,
`_refuses_at_address`, `_decodes_at_exit_0_with_a_warning`), reusing the
existing `MSG_N1A`/`MSG_N1A_WARN` constants rather than new ones —
`Finding::message` never quotes the surrounding template shape, only `@{i}`
and `{sites}`, so the rendered line is byte-identical regardless of which
arm produced the repeat, and reusing the constant is itself a check that one
classifier serves both arms rather than a drifted second copy.

**Evidence**: `cargo nextest run --locked -p md-cli -E
'binary(n1_admission_taxonomy)'` — 34/34 passed (31 pre-existing + 3 new).

### N2 — SKIPPED, unchanged

The session-date-vs-git-date convention question in
`design/BRAINSTORM_mdcli_mini.md` was **not touched**, per the dispatch
brief. It awaits an operator ruling: the original fold investigated it with
`git log` (not just reading) and found every mdcli-mini phase commit
wall-clock-stamped 2026-08-30 while the brainstorm doc's own prose uses
"2026-08-31" as a self-declared "today" eleven times — a session-local date
convention that conflicts with git history rather than a typo, and the
report's own header uses the same convention. The fold declined to
unilaterally reverse it across files outside its scope; this implementer
made the same call. `design/FOLLOWUPS.md`'s `whole-diff-r1-nit-residue`
entry is marked "4 of 5 CLOSED" with N2 explicitly left OPEN under it,
rather than closing the whole entry.

## Item 3 — `push-staging-script-watches-an-order-dependent-run`

**Commit `df0e893e`.** Source:
`design/agent-reports/push-2026-08-31-mdcli-mini.md` and the FOLLOWUPS
entry.

`scripts/push-via-staging.sh` selected its polled CI run with an unfiltered
`gh run list --commit <SHA> -q '.[0].databaseId'`, order-dependent across
however many workflows fire per SHA. This repo now fires three (`CI`,
`fuzz-smoke`, `bitcoind-differential`) and the push agent measured it
selecting `bitcoind-differential` (no required context could ever appear
there) while `CI` (the run actually carrying both required contexts) sat
unselected — a 1800s stall ending in a false-alarm FATAL, never a wrong
push.

Added `CI_WORKFLOW="${CI_WORKFLOW:-CI}"` (overridable, same pattern as
`REQUIRED_CONTEXTS`) and filtered the selection query to `--workflow
"$CI_WORKFLOW" --branch ci/staging`.

**`--branch` is not optional decoration — this was discovered by running the
verification, not by assuming the workflow filter alone would hold.** The
task's own suggested verification command,

```
gh run list --repo bg002h/descriptor-mnemonic --workflow CI --commit \
  bdb031a4cb54a9f57510af98db81386c360e9b70
```

returns **two** `CI`-workflow runs for this SHA today:

```json
[{"databaseId":33365618376,"headBranch":"main"},
 {"databaseId":33364380379,"headBranch":"ci/staging"}]
```

`.[0]` on that query today resolves to `33365618376` — the WRONG run. This
is because the ritual's own last step pushes the staged SHA to `main`,
which triggers a *second* same-workflow run for the identical commit, so
`--workflow` alone stops being unique the moment the ritual completes (it
was unique only in the brief window between the staging push and the final
push, which is when the script's own query actually executes at runtime —
but a later forensic re-query, like this verification, sees both).

Adding `--branch ci/staging` isolates exactly the target run:

```
$ gh run list --repo bg002h/descriptor-mnemonic --workflow CI \
    --branch ci/staging --commit bdb031a4cb54a9f57510af98db81386c360e9b70 \
    --json databaseId -q '.[0].databaseId'
33364380379
```

This is also the semantically correct selector, not merely a
disambiguator: the run being polled is inherently "the CI run for the
branch this script itself just pushed to."

Fail-closed behavior unchanged: the 30-attempt retry loop and the "no run
appeared" FATAL are untouched, only the query gained two filters. Header
comment updated to document the incident, the fix, and the discovered
`--branch` necessity, in the same style as the existing `THE FREEZE RULE`
and `REQUIRED CONTEXTS` sections.

**Not run end-to-end** — the task explicitly scoped this to read-only
verification of the changed selection line; `bash -n
scripts/push-via-staging.sh` confirmed syntax validity; no `shellcheck`
installed on this machine.

## FOLLOWUPS closures

**Commit `1f519b4`.** All three entries closed in `design/FOLLOWUPS.md`'s
established style (heading annotation + a `**✓ CLOSED (date), commit
\`SHA\`.**` paragraph citing what changed). `whole-diff-r1-nit-residue` is
marked "4 of 5 CLOSED... N2 OPEN" rather than fully closed, per the
dispatch brief's instruction to leave that item open under the entry.
Commit-hash citations use the repo's established 8-hex-char convention
(fixed after the prior three commits landed, since `git commit`'s default
short-SHA output was 7 chars).

## Gate

`./scripts/phase-gate.sh`, run on the tip immediately before the FOLLOWUPS
closure commit — **exit 0, all six steps passed**:

```
=== cargo nextest run --locked --all-features ===
Summary [0.875s] 1195 tests run: 1195 passed, 2 skipped
=== cargo test --workspace --doc --all-features ===
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
=== cargo clippy --locked --all-targets --all-features -- -D warnings ===
clean
=== cargo fmt --check ===
clean
=== cargo doc --workspace --no-deps --document-private-items --all-features ===
clean
=== design/display-grouping-vectors.tsv.sha256 ===
display-grouping-vectors.tsv: OK
phase-gate: all six steps passed
```

Test count: 1195, up from 1186 at the FOLD's own baseline. Verified the
delta by inspecting the one crate-code commit (`c8c3a4fd`) that landed
between the FOLD's baseline and this branch's start (`e5dcb24f`) — its own
gate output recorded 1191 passed — plus this implementer's 4 new tests
(1 in `r3_verify_against.rs`, 3 in `n1_admission_taxonomy.rs`): 1186 + 5 +
4 = 1195, matching exactly.

## Commits on this branch (`close-out-nits`, from `e5dcb24f`)

| commit | what |
| --- | --- |
| `3e3e5903` | Item 1 — `md-address-help-summary-is-blank` |
| `2b935f19` | Item 2 — `whole-diff-r1-nit-residue`, N1/N3/N4/N5 |
| `df0e893e` | Item 3 — `push-staging-script-watches-an-order-dependent-run` |
| `1f519b4`  | FOLLOWUPS closures for all three entries, gate output |
| (this)     | This report |

## Skipped / partial work

- N2 (session-date-vs-git-date convention) — skipped entirely, per the
  dispatch brief; the FOLLOWUPS entry stays open for it.
- Nothing else was partial. All four assigned nits (N1, N3, N4, N5) turned
  out implementable at roughly the scope the FOLD report estimated, though
  N1 touched more files (9) than its "at least 3" estimate, and N5 required
  building an old binary in a scratch worktree (removed after use) rather
  than a lighter synthetic test, because the shape it needs to cover is
  refused by every *current* binary — the same reason the two existing N1
  fixtures are frozen mint-once artifacts rather than generated ones.
