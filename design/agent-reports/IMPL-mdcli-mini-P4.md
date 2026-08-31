# IMPL report — mdcli-mini P4 (N3 bracket-as-source + R9 `--from-mk1` arity)

Worktree: `/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`, branch
`mdcli-mini`. Started at `f9cfde94` (P1–P3 landed). Two commits, per the
plan's "N3 then R9" ordering:

- `d72ede51` — P4.1: N3
- `d593218c` — P4.2: R9 (final SHA, HEAD)

Plan: `design/IMPLEMENTATION_PLAN_mdcli_mini.md`, "P4 — N3 bracket-as-source
+ R9 arity" steps 1–2. Spec: `design/SPEC_mdcli_mini.md` "N3" and "R9".
FOLLOWUPS: `descriptor-key-bracket-path-as-a-last-resort-source`,
`from-mk1-arity-spills-card-strings-into-the-md1-positional`. Executed
exactly as written; **no deviations, no contradictions found.**

## Step 1 — N3 (commit `d72ede51`)

### Files changed

- `crates/md-cli/src/parse/path.rs` — `apply_path_override_per_slot` gains a
  fourth parameter, `bracket_sourced: &BTreeMap<u8, DerivationPath>`. It no
  longer early-returns when `path` is `None`; it now returns early only when
  BOTH `path` is `None` AND `bracket_sourced` is empty. Per-slot resolution
  order: `inline_declared` (unchanged) → shared `--path` (unchanged when
  `Some`) → `bracket_sourced` (new) → unfilled (unchanged fallthrough to the
  existing non-canonical-wrapper refusal downstream).
- `crates/md-cli/src/cmd/build.rs` — `resolve_keys_fingerprints_and_precedence`'s
  return type grows a fourth element, `bracket_sourced: BTreeMap<u8,
  DerivationPath>`. Its per-key loop's `None` arm (fired when neither inline
  nor `--path` supplies a path for the slot) no longer returns
  `CliError::Mismatch` — it inserts `(ok.i, ok.path.clone())` into
  `bracket_sourced` via `entry().or_insert_with()`, mirroring how
  `inline_paths` already handles first-writer-wins. The `Some(shared)`
  disagreement arm (bracket vs `--path`) is untouched — still refuses. The
  single call site (`build_descriptor`) is updated to thread the new value
  through.
- `crates/md-cli/tests/cli_p1_origin_key.rs` — two existing tests exercised
  exactly the case N3 changes (single winning bracket, no inline, no
  `--path`) and are flipped in place rather than duplicated:
  - `v_patheff_bracket_path_with_no_winning_source_refuses_instead_of_truncating`
    → renamed `v_n3_bracket_path_with_no_other_source_becomes_the_path_source`,
    assertion flipped from refusal to success with the full origin present.
  - `v_patheff_the_divergent_origin_journey_refuses_rather_than_emitting_false_origins`
    → renamed `v_n3_divergent_origin_wallet_composes_and_equals_inline_pasted_origins`,
    the plan's row-1 equality obligation: composes via three
    different-account brackets (`48'/0'/0'/2'`, `48'/0'/1'/2'`,
    `48'/0'/2'/2'`, fingerprint `73c5da0a` — the FOLLOWUPS reproduction's own
    numbers, keys derived via the file's existing `xpub_at()` helper against
    the abandon-mnemonic, matching `keys.txt` records 1–3 by construction)
    and is asserted byte-for-byte equal to the same wallet built by pasting
    the three origins directly into the template with bare `--key` +
    `--fingerprint`.
  - The module doc comment's "V-PATHEFF" section is updated to record that
    N3 supersedes manifestation A's disposition (truncation→refuse became
    truncation→bracket-as-source) while manifestation B (silent override
    with `--path` present) is unchanged.

### Row evidence, red → green

Confirmed by `git stash` of the five N3 source files (tests kept) and
re-running:

```
$ cargo nextest run --locked --all-features -p md-cli --test cli_p1_origin_key -E 'test(v_n3)'
FAIL v_n3_bracket_path_with_no_other_source_becomes_the_path_source
  stderr: md: MISMATCH: @0: origin-notated --key states path `48'/0'/0'/2'`,
  but nothing supplies a path for @0: ...
FAIL v_n3_divergent_origin_wallet_composes_and_equals_inline_pasted_origins
  stderr: md: MISMATCH: @0: origin-notated --key states path `48'/0'/0'/2'`,
  but nothing supplies a path for @0: ...
Summary: 2 tests run: 0 passed, 2 failed
```

After `git stash pop` (implementation restored):

```
$ cargo nextest run --locked --all-features -p md-cli --test cli_p1_origin_key --test cli_path_override_reaches_noncanonical
Summary: 17 tests run: 17 passed, 0 skipped
```

The two plan-named regression rows were already present and required no
change, confirmed still green in the same run:
`v_patheff_bracket_path_disagreeing_with_shared_path_refuses` (disagreeing
bracket with `--path` present still refuses) and
`address_without_path_still_refuses_a_noncanonical_wrapper` in
`cli_path_override_reaches_noncanonical.rs` (a slot with no path from any
source — bare `--key`, no bracket at all — still refuses; unaffected by N3
because `bracket_sourced` stays empty when `ok.path` is empty, which is
`ok.path.as_ref().is_empty()`'s existing `continue` at the top of the loop,
untouched).

Full-suite check on the N3-only tree (R9 not yet staged, via `git stash
push --keep-index`): `cargo build -p md-cli --all-features` clean;
`cargo nextest run --locked --all-features`: 1154 run, 1154 passed, 2
skipped — identical count to the P1–P3 baseline (two tests renamed, none
added or removed at this step).

## Step 2 — R9 (commit `d593218c`)

### Files changed

- `crates/md-cli/src/main.rs` — both `--from-mk1` declarations
  (`Descriptor` at what was line 400, `Address` at what was line 560) gain
  `num_args = 1..`. Both `descriptor_input`/`address_input` `ArgGroup`s
  widen from `.args(["phrases", "template"])` to `.args(["phrases",
  "template", "from_mk1"]).multiple(true)`.
- `crates/md-cli/src/cmd/build.rs` — new `pub fn check_from_mk1_arity(phrases,
  from_mk1, template, cmd) -> Result<(), CliError>`, three sequential guards
  (order is load-bearing, see mechanics below).
- `crates/md-cli/src/cmd/descriptor.rs`, `crates/md-cli/src/cmd/address.rs`
  — each calls `crate::cmd::build::check_from_mk1_arity(args.phrases,
  args.from_mk1, args.template, "descriptor"|"address")?;` as the first
  statement in `run`, before the `if args.from_mk1.is_empty()` branch, so
  the mk1-in-positional guard also catches the plain
  `md descriptor mk1card` case (from_mk1 empty, no swallow involved).
- `crates/md-cli/tests/seating_vectors.rs` — new `r9_cmd()` helper (builds
  ONE `--from-mk1` occurrence carrying every value, unlike the file's
  existing `seat_cmd()` which always repeats the flag) and
  `assert_one_rendered_line()` helper, plus four new test functions
  covering six rows total (two loop over both verbs).

### The R9 mechanics chosen, and why they satisfy the named outcomes

The plan authorized "relaxing the group when `--from-mk1` is present and
refusing in code" without prescribing the exact shape. Chosen mechanics:

1. **`num_args = 1..`** makes a single `--from-mk1` occurrence greedily
   consume every following bare token, which is what makes the natural
   paste (`--from-mk1 mk1a mk1b mk1c`) work — and is exactly the mechanism
   that can swallow a trailing md1 positional in the flag-first ordering.
2. **`ArgGroup` widened to include `from_mk1`, with `.multiple(true))`.**
   Membership makes the group's `required(true)` satisfied by `from_mk1`
   alone, so a swallowed-empty `phrases` no longer trips clap's own
   missing-required-argument error (satisfying the spec's explicit "NOT
   clap's missing-required-argument error" constraint for the flag-first
   row). `.multiple(true)` is required alongside it: without it, the
   group's default at-most-one-member constraint would make the S row's
   required combination (`phrases` + `from_mk1` together) a clap-level
   conflict, breaking the pre-existing working spelling. `template` stays
   mutually exclusive with both via its own pre-existing `conflicts_with`
   attributes, which are independent of group membership.
3. **`check_from_mk1_arity`, three ordered guards, called first in both
   `run` functions:**
   - Guard 1 (`CliError::Seat`, exit 1): any `phrases` entry prefixed
     `mk1` (after `strip_display_separators`) refuses by name pointing at
     `--from-mk1`. Runs regardless of `from_mk1`'s own state, so it also
     catches a bare `md descriptor mk1card` typed directly — the general
     shape the FOLLOWUPS remedy (b) asked for ("also catches a card string
     arriving there by any other route").
   - Guard 2 (`CliError::Seat`, exit 1): any `from_mk1` entry prefixed
     `md1` refuses by name pointing at the positional — this is the
     swallow's own signature.
   - Guard 3 (`CliError::BadArg`, exit 2): `phrases.is_empty() &&
     template.is_none() && !from_mk1.is_empty()` refuses naming the
     missing policy input. This is what closes the gap the `ArgGroup`
     widening in (2) reopened: `from_mk1`'s bare presence now satisfies the
     group, so without this guard a genuinely policy-less
     `--from-mk1 <keys only>` invocation would fall through into
     `seat::run`'s `reassemble(&[])`, surfacing as the bare "chunk set is
     empty" codec error the FOLLOWUPS class is about. Exit 2 (not 1) to
     match the exit code clap's own group-required error would have
     produced for the same shape, and to match the sibling defense-in-depth
     check already in `build_descriptor` (`"{cmd} requires either
     positional... — clap should have caught this"`).
   - **Order 1 → 2 → 3 is load-bearing, not stylistic.** In the flag-first
     swallow scenario, `from_mk1` is non-empty AND `phrases` is empty AND
     `from_mk1` contains an md1 string — Guards 2 and 3's preconditions
     both hold simultaneously. Guard 2 must run first so the more specific,
     more actionable diagnosis wins; if the order were reversed the
     operator would be told "no policy card" instead of "here is the
     policy card, it's in the wrong flag."
   - Guard scope (r1 M6, honored by construction): the function only ever
     inspects `phrases` and `from_mk1` — no other flag's values are in
     scope, so P5's future literal `--emit md1` cannot trip it.

### Row evidence, red → green

RED confirmed by `git stash` of the R9 source files (main.rs, build.rs,
descriptor.rs, address.rs — tests kept), each test function failing with
the pre-fix symptom the FOLLOWUPS entry describes:

```
$ cargo nextest run --locked --all-features -p md-cli --test seating_vectors -E 'test(v_r9)'
FAIL v_r9_mk1_string_in_the_positional_refuses_naming_from_mk1
  left:  "md: codec error: codex32 decode error: string does not start with HRP md1"
FAIL v_r9_positional_first_natural_paste_composes
  "positional-first natural paste did not compose: md: codec error:
   codex32 decode error: string does not start with HRP md1"
FAIL v_r9_from_mk1_with_no_policy_card_anywhere_refuses_naming_the_missing_policy
  left: Some(1)   right: Some(2)   (fell through to seat::run's reassemble
  on 3 leftover mk1 strings spilled into phrases by the old num_args=1
  default — the FOLLOWUPS' own spillage bug, still present pre-fix)
FAIL v_r9_flag_first_trailing_md1_string_refuses_naming_the_positional
  left:  "md: codec error: codex32 decode error: string does not start with HRP md1"
Summary: 4 tests run: 0 passed, 4 failed
```

After `git stash pop` (implementation restored):

```
$ cargo nextest run --locked --all-features -p md-cli --test seating_vectors -E 'test(v_r9)'
PASS v_r9_from_mk1_with_no_policy_card_anywhere_refuses_naming_the_missing_policy
PASS v_r9_mk1_string_in_the_positional_refuses_naming_from_mk1
PASS v_r9_flag_first_trailing_md1_string_refuses_naming_the_positional
PASS v_r9_positional_first_natural_paste_composes
Summary: 4 tests run: 4 passed, 40 skipped
```

Row → test map (spec's four descriptor rows + at-minimum-two duplicated on
address, all satisfied):

| Spec row | Test | Verbs |
| --- | --- | --- |
| positional-first composes | `v_r9_positional_first_natural_paste_composes` | descriptor |
| flag-first trailing md1 → named refusal | `v_r9_flag_first_trailing_md1_string_refuses_naming_the_positional` | descriptor + address |
| mk1 in positional → named refusal | `v_r9_mk1_string_in_the_positional_refuses_naming_from_mk1` | descriptor + address |
| `--from-mk1` with no policy card anywhere → named refusal | `v_r9_from_mk1_with_no_policy_card_anywhere_refuses_naming_the_missing_policy` | descriptor |

All four refusal rows assert exactly one rendered `md: ` line via
`assert_eq!` against the literal message text (Acceptance 4 — copied
verbatim from the source `format!` strings into the test file rather than
retyped, to rule out transcription drift).

## Gate (final, `d593218c`)

`./scripts/phase-gate.sh`, all six steps passed:

```
cargo nextest run --locked --all-features: 1158 tests run, 1158 passed, 2 skipped
  (P1-P3 baseline was 1154; +4 new R9 test functions = 6 rows; N3 renamed
  2 tests in place, net 0)
cargo test --workspace --doc: 0 doctests, ok
cargo clippy --locked --all-targets --all-features -- -D warnings: clean
cargo fmt --check: clean
cargo doc --workspace --no-deps --document-private-items --all-features: clean
design/display-grouping-vectors.tsv.sha256: OK
phase-gate: all six steps passed
```

## Deviations from the plan/spec

None. Both steps executed as specified; the only judgment calls were the
mechanics the plan explicitly deferred to this phase (R9's guard shapes and
exit codes, both reasoned above) and the exact diagnostic wording (not
specified by spec or plan beyond "refuses by name" — chosen to match the
codebase's existing tone, verified via Acceptance-4 exact-line tests).

## Final SHA

`d593218c` (HEAD of `mdcli-mini` in this worktree).
