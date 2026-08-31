# FOLD — mdcli-mini whole-diff review r1, fold report

Responds to `design/agent-reports/REVIEW-mdcli-mini-whole-diff-r1.md` (0C/5I/5M/5N),
persisted verbatim at `00f08a78`. Worktree
`/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`, branch `mdcli-mini`.
Fold commits: `6d8c655a`..`4b9881e0` (8 commits over the persist tip).

Per the dispatch brief: reviewers reproduce defects, not remedies — where a
finding's direction conflicted with what I measured, I report the conflict
below rather than transcribe the direction.

---

## I1 — R-N1a cited the wrong BIP 388 rule

**Files:** `crates/md-cli/src/bip388.rs`, `crates/md-cli/src/parse/reuse.rs`,
`crates/md-cli/src/seat/satisfy.rs`, `crates/md-cli/tests/n1_admission_taxonomy.rs`,
`crates/md-cli/tests/seating_vectors.rs`.

Fetched `bip-0388.mediawiki` fresh from `bitcoin/bips` master and confirmed the
report's line-195 quote verbatim: *"If two `KEY` are `KP/<M;N>/*` and
`KP/<P;Q>/*` for the same key placeholder `KP`, then the sets `{M, N}` and
`{P, Q}` must be disjoint."* Added `bip388::DISJOINTNESS_RULE` (the same
verbatim-quote pattern as the existing `PAIRWISE_DISTINCT_RULE`) and swapped
`Finding::SamePathExpression`'s citation to it, keeping the BIP's own
invalid-example quote (`sh(multi(1,@0/**,@0/**))` — "Repeated keys with the
same path expression") exactly as the report directed. R-N1d's
`PAIRWISE_DISTINCT_RULE` citation was left untouched (correct: two
placeholders really do repeat the key in the vector).

Updated every pinned site: `reuse.rs`'s own unit tests, `satisfy.rs:506`'s
door-check assertion, `n1_admission_taxonomy.rs`'s `MSG_N1A`,
`seating_vectors.rs`'s `v_r5m1_reaches_the_command`. Two more sites turned
out to pin the same text that the report did not enumerate by line number —
found by running the suite, not by re-reading: `cmd_encode.rs` and
`sortedmulti_a_taproot_leaf.rs` only assert the rendered-line *prefix*
("md: unsupported: @N appears at N use sites"), which the citation swap
never touches, so those needed no edit; `n1_admission_taxonomy.rs`'s
`verify_template_warns_and_completes_on_a_refused_shape` derived its
expected WARN text from `MSG_N1A` via `strip_prefix`, which broke the moment
I1 and M4 (below) were folded together — fixed to reference `MSG_N1A_WARN`
directly.

**Evidence:** `cargo nextest run -p md-cli -E 'binary(n1_admission_taxonomy) or
binary(seating_vectors) or binary(cmd_decompose) or binary(cmd_encode) or
binary(sortedmulti_a_taproot_leaf) or binary(duplicate_key_slots)'` — 158/158
passed.

## M4 — folded together with I1 (same match arm)

**File:** `crates/md-cli/src/parse/reuse.rs`.

`Finding::message` now takes a `Disposition` parameter. Only
`SamePathExpression`'s arm reads it — every other finding renders identically
under either disposition, unchanged, which the (renamed) test
`the_disposition_changes_the_outcome_not_the_text_for_most_findings` now
proves using an R-N1b template instead of R-N1a, since R-N1a is the
documented exception. REFUSE tail: unchanged ("md declines to mint or
compose this shape: give each distinct key its own placeholder."). WARN
tail: "This shape can no longer be minted or composed; the card remains
readable." — matching the fold brief's example text, not the report's
"optional, append a clause" framing (the brief's wording overrides: "the
read-side WARNING does not end with 'md declines...'", i.e. replace the
tail, not append after it).

New row `r_n1a_warn_tail_differs_from_refuse_tail_per_m4` pins both tails
structurally. `n1_admission_taxonomy.rs` gained `MSG_N1A_WARN` and its four
call sites (`decode`/`inspect`/`bytecode`/`verify` WARN rows) now reference
it directly instead of deriving it from `MSG_N1A` by prefix substitution.

## I2 — the apply-then-refuse ordering in `build_descriptor` had no pin

**File:** `crates/md-cli/tests/duplicate_key_slots.rs`.

Added two rows using `tr(@0/<0;1>/*,multi_a(2,@1/<0;1>/*,@2/<0;1>/*))` — a
non-canonical wrapper whose slots carry no inline origin, so
`refuse_key_reuse_across_slots` can only expand and compare them once
`apply_path_override_per_slot` has already merged `--path` in.

**Swap-red proof (the mandatory check).** Reordered the two statements in
`cmd/build.rs::build_descriptor` (`refuse_key_reuse_across_slots` before
`apply_path_override_per_slot` instead of after):

```
$ cargo nextest run -p md-cli -E 'binary(duplicate_key_slots) and test(t_row_key_reuse_through_a_noncanonical_wrapper)'
FAIL t_row_key_reuse_through_a_noncanonical_wrapper_sourced_solely_by_path_refuses_at_descriptor
  left: Some(0)  right: Some(1)
  "one xpub filling two multi_a leaf slots was minted:
   tr(xpub…/<0;1>/*,multi_a(2,xpub661…/<0;1>/*,xpub661…/<0;1>/*))#sytk8gy9"
FAIL …refuses_at_address
  "…derived an address: bc1pj9zvvs3el6ved40pa3588nlrv4p2gkzve4jcq90phrpqterw9nksnun7y8"
```

Both numbers match the review's own reproduction exactly. Reverted the swap
(`git diff` over `cmd/build.rs` was empty afterward); rebuilt; both rows
green again: `cargo nextest -p md-cli -E 'binary(duplicate_key_slots)'` —
12/12 passed.

## I3 — `--emit md1` now carries the legacy-P2SH advisory

**Files:** `crates/md-cli/src/cmd/descriptor.rs`, `crates/md-cli/src/cmd/encode.rs`,
`crates/md-cli/tests/fixtures/seating/generate.sh` (new `V-LEGACY-P2SH`
block), `crates/md-cli/tests/fixtures/seating/v-legacy-p2sh.txt` (new),
`crates/md-cli/tests/n2_emit_md1.rs`.

Made `emit_legacy_p2sh_advisory` and `emit_pathless_advisory`
(`cmd/encode.rs`) `pub(crate)` and called both from `emit_md1_card`, in the
same relative order `md encode` uses. F-227 and F-410 were left out
deliberately, per the report's own finding that they are genuinely
inapplicable to this path.

No existing seating fixture used a legacy-P2SH top-level wrapper, so I built
one: `generate.sh` grew a `V-LEGACY-P2SH` block (`sh(sortedmulti(...))`,
fingerprint-bearing, modelled on the existing `V-CE1` block) and I ran the
**whole** generator with the sibling `mk` binary
(`/scratch/code/shibboleth/mnemonic-key/target/debug/mk`) — every pre-existing
fixture regenerated byte-identical (`git status --porcelain` empty except the
one new file), matching the plan's own "the frozen block excepted, every
fixture regenerates clean" invariant.

**Mutation-checked**, not just measured: stashed the two-line fix, reran the
new row — RED (`the F-A4 advisory did not carry across to --emit md1:` with
the mint's normal notes and no "legacy P2SH" line, exit 0). Restored the
fix; `cargo nextest -p md-cli -E 'binary(n2_emit_md1) or
binary(duplicate_key_slots)'` — 24/24 passed.

## I4 — CHANGELOG corrected and extended

**File:** `CHANGELOG.md`.

Corrected the converter section's two now-false paragraphs (measured live
before editing: `md encode "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key
@0=X --key @1=X` now exits 1, contradicting "still composes and still
encodes"; the same-path form now refuses too, contradicting "composing the
BIP-FORBIDDEN same-path form"; `design/FOLLOWUPS.md:2131` confirms
`md-repeated-placeholder-inverts-bip388` closed citing P2+P3). Added a new
stacked `## md-cli [Unreleased]` section (repo convention: one section per
change-set — five already existed) covering the N1 taxonomy, read-side
warnings, `--emit md1`, `--verify-against`, N3's bracket source, R9's
`--from-mk1` arity guards, and decompose's `/**`/`-`. Every specific claim in
the new section (exit codes, `num_args` values, file/line citations) was
grepped against source before writing, not transcribed from memory — see the
commit message for the specific checks. Caught and fixed one own error before
committing: the `/**` shorthand is BIP-**388**'s (the repo's own existing
CHANGELOG entry and `decompose/mod.rs`'s comments confirm this), not
BIP-389 as I first wrote the new section's heading.

Markdown-only; `cargo build`/tests unaffected. Fence-pair count in the file
even (10 `` ``` `` lines) before and after.

## I5 — decompose's D-row disjoint-multipath refusal quoted a dead message

**Files:** `crates/md-cli/src/decompose/mod.rs`, `crates/md-cli/src/parse/reuse.rs`,
`crates/md-cli/tests/cmd_decompose.rs`.

Fixed both sites the report named by line number (`decompose/mod.rs:256`'s
doc comment, `:335`'s rendered message) plus the third site carrying the
identical stale measurement that the report also named:
`cmd_decompose.rs`'s matching doc comment on
`v_d_shape2_disjoint_sets_refuse_naming_mds_narrower_template_surface`.

Took the report's "second-order" note as in-scope rather than optional: made
`parse::reuse::ESCAPE` `pub(crate)` and had the D row quote the identical
runnable escape R-N1c/R-N1d use (`me sysw pack --as descriptor --in <your
export file>`), replacing "Give each position its own key, or engrave this
wallet by another route" — which was not just stale but substantively wrong
advice (the wallet does not need a new key).

`cargo nextest -p md-cli -E 'binary(cmd_decompose)'` — 31/31 passed.

## M1 — N3's row 3

**File:** `crates/md-cli/tests/cli_p1_origin_key.rs`.

Verified the report's reproduction live before writing the row (exit 1,
`non-canonical wrapper requires explicit origin for @1, but none provided`).
Added `v_n3_a_slot_with_no_path_from_any_source_still_refuses`: `@0` sourced
by an N3 bracket (which wins), `@1` with no origin from any source at all.
`cargo nextest -p md-cli -E 'binary(cli_p1_origin_key)'` — 14/14 passed.

## M3 — `--verify-against` now names itself on an empty file

**Files:** `crates/md-cli/src/cmd/mod.rs`, `cmd/repair.rs`, `cmd/bytecode.rs`,
`cmd/verify.rs`, `cmd/decode.rs`, `cmd/inspect.rs`, `cmd/descriptor.rs`,
`crates/md-cli/tests/r3_verify_against.rs`.

Threaded a `flag: &str` parameter through `read_md1_inputs` and its six call
sites: `"--in"` for the five verbs that own that flag,
`"--verify-against"` for `resolve_verify_against`. New row
`r3_an_empty_verify_against_file_names_verify_against_not_in`. Measured
before the fix: `md: --in /tmp/…: no md1 strings…`; after:
`md: --verify-against /tmp/…: no md1 strings…`. `cargo nextest -p md-cli`
over `r3_verify_against` + the five `--in`-owning verbs' binaries —
31/31 passed.

## M5 — corrected the overstated comment on `spend_equal`'s ordering claim

**File:** `crates/md-cli/src/seat/compose.rs`.

Comment-only. The old wording claimed the reorder "does not change which
pairs are equal" unconditionally; it is true only when `expand_per_at_n`
succeeds on both sides — for a pair where expansion fails, the current order
returns `Err` where the structure-first order would have reached
`Ok(SpendEqualVerdict::Structure)`. Reworded to state the qualification and
the report's own reachability note (unreachable through the CLI today: a
`MissingExplicitOrigin` descriptor cannot be minted).

## M2 — NOT taken

Per the dispatch brief: the `--emit md1` output-form gap (no `--out`,
`--group-size`, `--separator`, or engraving card) is left for the controller
to file as a follow-up.

## Nits — none fixed inline; all left for filing

Every one of the five, on inspection, needed more than a pure comment/wording
touch, so none were folded (per the brief: "anything larger, list in your
report as left-for-filing"):

- **N1** ("rule (1)" mislabel in R-N1d). The report itself notes the same
  label is used at `build.rs:293` and `decompose/mod.rs:244` — fixing only
  R-N1d would introduce a NEW inconsistency rather than remove one, so a
  correct fix touches at least 3 source files plus their pinned test
  strings. Larger than a nit.
- **N2** (burndown closure dates). Investigated with `git log`, not just
  read: every mdcli-mini phase commit (`P2` `83885768`, `P4.1` `d72ede51`,
  `P6.1/.2` `1a1983d7`/`3aa38764`, etc.) is timestamped **2026-08-30**, and
  `design/FOLLOWUPS.md`'s "CLOSED by P_ (2026-08-30)" headers match that
  exactly. The "2026-08-31" the report flagged as the correct date instead
  traces to `design/BRAINSTORM_mdcli_mini.md`'s own prose, which uses
  "2026-08-31" as its self-declared "today" **11 times** throughout the
  whole document — a session-local date convention, not a typo — while its
  own git commits are wall-clock-stamped 2026-08-30. The review report's own
  header ("**Date:** 2026-08-31") uses the same convention. This is
  systemic across at least two other design docs, not a 3-line fix
  contained to FOLLOWUPS.md, and I did not touch it — the report's stated
  direction (treat 2026-08-31 as correct) is the one that conflicts with git
  history, so I am reporting the conflict rather than either transcribing it
  or unilaterally reversing it across files outside this fold's scope.
- **N3** (doctests not widened in CI/phase-gate.sh). A behavioral change to
  `.github/workflows/ci.yml` and `scripts/phase-gate.sh`, not a comment.
- **N4** (`--verify-against` existence-routing message hides which branch
  ran). Fixing it means wrapping `decode_md1_string`/`reassemble`'s errors
  with branch context in `resolve_verify_against` — a new code path, not a
  wording edit.
- **N5** (`count_occurrences`'s `Body::Tr` internal-key arm unexercised for
  R-N1a). Requires a new fixture/template exercising a `tr(@0/**,...)`-shaped
  repeat on the internal key — new test construction, not a comment fix.

---

## Gate

`./scripts/phase-gate.sh` — **exit 0, all six steps passed**:

```
cargo nextest run --locked --all-features: 1186 tests run: 1186 passed, 2 skipped
cargo test --workspace --doc: ok (0 doctests)
cargo clippy --locked --all-targets --all-features -- -D warnings: clean
cargo fmt --check: clean
cargo doc --workspace --no-deps --document-private-items --all-features: clean
design/display-grouping-vectors.tsv.sha256: OK
```

Test count rose from the review's baseline 1180 to 1186 (+6: I2 x2, I3 x1,
I5's `reuse.rs` internal row x1 from M4, M1 x1, M3 x1 — I5 itself added no
new test, only corrected existing text).

Independently re-ran `MD=$PWD/target/debug/md MK=<sibling mk>
bash crates/md-cli/tests/fixtures/seating/generate.sh` after all fold
commits landed — exit 0, `git status --porcelain
crates/md-cli/tests/fixtures/` empty, confirming the fixture set (including
the new `v-legacy-p2sh.txt`) is still reproducible from a clean generator
run.

## Commits

| commit | finding(s) |
| --- | --- |
| `6d8c655a` | I1, M4 |
| `3b760895` | I2 |
| `29374e53` | I3 |
| `690a7c7a` | I5 |
| `b6221069` | M1 |
| `966a0914` | M3 |
| `820bc32d` | M5 |
| `4b9881e0` | I4 |

Final SHA at time of writing this report: `4b9881e0`.
