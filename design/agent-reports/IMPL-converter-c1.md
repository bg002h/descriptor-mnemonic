# IMPL report — converter C1 (P1, the T row)

**Phase**: C1 of `design/IMPLEMENTATION_PLAN_wallet_form_converter.md` §3.
**Worktree**: `/scratch/code/worktrees/converter-c1`, branch `impl/converter-c1`,
off `impl/converter-c0` at `9a4ce318` (matches the dispatch brief's stated tip).
**Implementer**: single agent, TDD, no sub-dispatch (implementation-tight rule).
**Final commit**: `3d62ba35` (five commits total; tree clean).

## What landed

### Task 3 — inline template origin `h`-spelling (commit `c3d34e4b`)

SPEC P1 r2 M4 / the F-420 class: an inline template origin written
`48h/...` used to draw an unrelated "derivation steps after the multipath
group are not representable" complaint. Root cause: the origin-step
capture group in `lex_placeholders`' regex was `(?:/\d+'?)*` (apostrophe
only), so an `h`-spelled step was never consumed and fell through to M5's
post-multipath residue check on a template with no multipath group at all.

**Choice: the pointed refusal, not silent `h` acceptance.** The brief left
this open (task 3), but `IMPLEMENTATION_PLAN_wallet_form_converter.md` §3 C1
had already settled it in prose ("the inline `h`-spelling refusal points at
the `'` requirement") and the V-HSPELL roster row's expected-behaviour
column reads "message points at `'` requirement" — acceptance was never on
the table for this position. Implementation: widened the capture to
`(?:/\d+(?:'|h)?)*` so the `h`-spelled step is CONSUMED (no more stray
residue), then added a dedicated check in the origin-path block that
refuses BY NAME, pointing at the `'` requirement and naming the offending
step, before the value ever reaches `DerivationPath::from_str`. The
origin-notated `--key @i=[fp/path]xpub` bracket form (C0's
`parse_key_with_origin`) is UNCHANGED and still accepts both spellings —
the two syntaxes are deliberately independent (template placeholder origin
vs. `--key` origin bracket).

Updated two pre-existing tests that asserted (or would have gone stale
against) the OLD mechanism: `parse_template_rejects_h_in_origin_funds`
(template.rs) and `m5_multipath_not_last_reject.rs`'s
`encode_h_in_origin_residue_rejects`. Checked every other h-spelling
reference in the test tree (`cli_divergent_origin_encode.rs`'s `BRACKETED`
const, `cmd_encode.rs`'s `--path` value, `cli_bip388_double_wildcard.rs`'s
doc comment) — none exercise this code path: the first hits the separate
descriptor-style-prefix check, the second is `--path`'s own
`DerivationPath::from_str` (already h-tolerant), the third has no h-spelling
assertion at all.

### Tasks 1+2 — origin-notated `--key` wiring + P1 per-datum precedence (commit `ea97c12c`)

New `resolve_keys_fingerprints_and_precedence` in `cmd/build.rs`
(`build_descriptor`, shared by both `descriptor.rs` and `address.rs` — no
change needed to either command file, nor to `main.rs`'s clap wiring, since
`--key`/`--fingerprint` were already untyped `Vec<String>`) parses every
`--key` value with C0's `parse_key_with_origin` instead of the bare
`parse_key`, then reconciles the two data sources per SPEC P1 BEFORE
`parse_template` runs:

- **Paths**: read each slot's inline template origin via
  `parse::template::lex_placeholders` (already `pub`) ahead of the
  Shared/Divergent fold, since that fold cannot answer "did slot i declare
  one at all" after the fact (a slot with no inline origin and one
  explicitly declaring the empty/root origin both fold to the same empty
  `OriginPath`, and there is no template syntax for the latter, so this is
  the ONLY point the distinction is still recoverable). An origin-notated
  `--key`'s bracket path is checked against the inline path ONLY when both
  are non-empty; mismatch refuses naming the slot and BOTH paths; agreement
  is a no-op, never an override.
- **Fingerprints**: `--fingerprint` and an origin-notated `--key`'s bracket
  fingerprint are merged into one map; when both name slot i they must
  match or the command refuses naming the slot and both hex values.

New `apply_path_override_per_slot` in `parse/path.rs` replaces
`apply_path_override` on the descriptor/address path only: it fills the
shared `--path` into ONLY the slots lacking an inline origin, rather than
`apply_path_override`'s whole-descriptor `Shared` overwrite.
`encode`/`verify`/`vectors` keep the old function and its old behaviour,
unchanged — out of C1's scope. Checked this is safe:
`cli_path_override_reaches_noncanonical.rs`'s fixtures (`TR_WITH_LEAF`)
declare no inline origins at all, so the new function is a no-op-equivalent
to the old one for every green test there; `origin_key_contradiction.rs`
and `duplicate_key_slots.rs` exercise `encode`, untouched.

Removed the now-live `#[allow(dead_code)]` markers C0 left on
`OriginNotatedKey`, `parse_key_with_origin` (keys.rs) and
`CliError::BadOrigin` (error.rs) — all three are reachable from `main` as
of this commit.

### Docs (commit `03a992b1`)

Updated `--key`/`--fingerprint`/`--path` doc comments and `--key`'s
`value_name` on `descriptor`/`address` in `main.rs` to describe the new
form and precedence — the old text (in particular Address's `--path`
comment, "flattens Divergent mode to Shared") was now factually wrong for
these two commands. `encode`'s identical-looking comment is untouched and
still accurate (its behaviour did not change). Checked for D3 snapshot
drift before editing: `insta::assert` count is 0 in `cmd_gen_man.rs`,
`help_examples.rs`, `cmd_gui_schema.rs` (only `json_snapshots.rs` uses
insta, for `decode`/`inspect` JSON shapes, unrelated); `help_examples.rs`
has zero references to the changed strings. No snapshot files changed —
confirmed by the full suite staying green before and after.

### v_keyorig also exercises `md address` (commit `3d62ba35`)

`descriptor.rs` and `address.rs` share `build_descriptor` structurally, but
the brief names both commands explicitly and "same code path" is a claim
about the implementation, not a substitute for exercising the second
command. Added `v_keyorig_address_also_accepts_the_origin_notated_key`.

## Rows (`crates/md-cli/tests/cli_p1_origin_key.rs`, new file per plan D2, plus `parse::template::lex_tests` for V-HSPELL)

- **V-KEYORIG** (2 tests): a real multi-slot template with per-slot inline
  origins composes end-to-end via origin-notated `--key` alone (no
  `--fingerprint` flags) — asserted by cross-checking BYTE-FOR-BYTE against
  the already-proven bare-key + `--fingerprint` route, not a hand-written
  expected string; plus the `md address` coverage test above.
- **V-FPAGREE** (2 tests): agreeing fingerprints (flag-only, key-only, and
  both-together) all produce the identical descriptor; disagreeing
  fingerprints refuse naming `@0` and both hex values.
- **V-PATHAGREE** (2 tests): disagreeing origin-notated key path refuses
  naming `@0` and BOTH paths; agreeing succeeds and the descriptor still
  carries exactly the one agreed path.
- **V-PRECEDENCE** (2 tests): inline path wins over a conflicting shared
  `--path` on a single slot; a second, 2-slot template (`@0` bare, `@1`
  inline) proves the PER-SLOT half — `--path` fills only `@0`, `@1` keeps
  its inline origin. This second test is the one that falsifies the OLD
  `apply_path_override` behaviour (whole-descriptor `Shared` overwrite),
  which would either wipe `@1`'s inline path or leave `@0` unfillable.
- **V-HSPELL** (3 tests, `parse::template::lex_tests`, landed with task 3):
  named refusal points at the `'` requirement and the old "multipath"
  wording is absent; a mixed `'`/`h` path still names the specific
  offending `h` step; the `'`-only spelling is unaffected (positive
  control for the widened capture).

Every precedence-sensitive test supplies `--fingerprint`: module doc
comment notes and verified directly (manual `md descriptor` run) that
`md-codec/src/to_miniscript.rs`'s `assemble_origin_and_xkey` gates the
WHOLE origin bracket — including the path — on fingerprint presence, so a
path-precedence assertion is unobservable in the rendered descriptor
without one.

## Mutation checks (inline, not part of the formal gate)

Verified each new refusal/precedence path actually gates its test, not
just "some assertion happens to hold":

1. Disabled the fingerprint-mismatch branch
   (`Some(existing) if false && *existing != fp`) →
   `v_fpagree_disagreeing_fingerprints_refuse_naming_the_slot` went RED
   (accepted the disagreement and printed a descriptor).
2. Disabled the path-mismatch branch the same way →
   `v_pathagree_disagreeing_key_path_refuses_naming_both_paths` went RED.
3. Reverted `apply_path_override_per_slot` to the old blind
   whole-descriptor `PathDeclPaths::Shared` overwrite → BOTH
   `v_precedence_*` tests went RED (one on the single-slot conflict, one on
   the two-slot per-slot split), each with the shared `--path` visibly
   overriding the inline origin in the failure message.

All three mutations reverted before committing; the full suite was
re-confirmed green after each revert and again at the final commit.

## Deviations from the brief

None material. One addition beyond the brief's letter: task 1 named both
`descriptor` and `address`; the row set as drafted exercised only
`descriptor` directly (relying on the shared `build_descriptor` for
`address` coverage), so a dedicated `md address` test was added in the
final commit to close that gap explicitly rather than leave it implicit.

## Exit gate (final tree, commit `3d62ba35`)

**`cargo build --locked -p md-cli`**: clean, no warnings.

**`cargo nextest run --locked`** (full suite):
```
     Summary [   0.894s] 897 tests run: 897 passed, 2 skipped
```
(897 = 887 at C0's tip + 2 net new from V-HSPELL's test rename/expansion +
7 from the first `cli_p1_origin_key.rs` commit + 1 from the `md address`
addition; the 2 skipped are the pre-existing `cli-compiler`-feature-gated
tests, unaffected by this phase.)

**`cargo clippy --locked --all-targets -- -D warnings`**: clean, no
warnings (one `clippy::type_complexity` finding during development, fixed
with a `ResolvedKeysAndFingerprints` type alias — see commit `ea97c12c`'s
diff).

**`cargo fmt --check`**: clean, no diff.

**Row-scoped gate — `cargo nextest run --locked -E 'test(v_keyorig) or test(v_fpagree) or test(v_pathagree) or test(v_precedence) or test(v_hspell)'`**:
```
     Summary [   0.009s] 14 tests run: 14 passed, 885 skipped
```
**Matched count: 14, against expected count: 14** — this phase's 11 new
tests (2 V-KEYORIG + 2 V-FPAGREE + 2 V-PATHAGREE + 2 V-PRECEDENCE +
3 V-HSPELL) plus C0's 3 `v_keyorig_bad` unit tests (a different row,
V-KEYORIG-BAD, matched incidentally by the `v_keyorig` substring — the same
overlap C0's own report already accounted for its own count). Non-empty,
non-short, matches the row list the brief named.

**`vendor-freshness`**: not re-run — this phase touched no
`Cargo.toml`/`Cargo.lock`/`vendor/`, confirmed by `git diff --stat` against
those paths showing no changes since C0's tip.

## Commits

- `c3d34e4b` — `c1: inline template origin refuses h-spelling, points at ' requirement`
- `ea97c12c` — `c1: origin-notated --key on descriptor/address, P1 per-datum precedence`
- `03a992b1` — `c1: --help text for descriptor/address --key/--fingerprint/--path`
- `3d62ba35` — `c1: v_keyorig row also exercises md address directly`
- this report, committed separately (repo convention: report lands as its
  own commit).

Working tree is clean at `3d62ba35`; nothing left uncommitted or unstaged.
