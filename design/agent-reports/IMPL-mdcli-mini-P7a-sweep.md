# IMPL-mdcli-mini-P7a: FOLLOWUPS burndown sweep

Executed: P7 step 3 (design/IMPLEMENTATION_PLAN_mdcli_mini.md, "## Follow-up
burndown" table). Worktree: `/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`,
branch `mdcli-mini`, starting at `88a22da8`. Closes over the eleven rows in
that table (the plan's "eight originals" + the walk-discovered R9 entry +
`phase-gate-omits-cargo-doc` + `sibling-toolkit-…`).

All commit SHAs below were verified to exist on `mdcli-mini` via `git log -1
<sha>` before citing, and their subjects/diffs were spot-checked against the
closure text (not merely assumed from the dispatch brief).

## Per-slug actions

| slug | action | citation |
| --- | --- | --- |
| `all-features-suite-is-red-and-ungated-by-ci` | CLOSED — heading `owning phase` annotated `✓ CLOSED P1, 2026-08-30`; body paragraph added | P1: `1b36bf6b` (deleted `render_tr_template`, routed Tap rendering through upstream `Display`) + `400a2f77` (CI widened to `--all-features`, `scripts/phase-gate.sh` added) |
| `phase-gate-omits-cargo-doc` | CLOSED — heading annotated `✓ CLOSED P1 (mdcli-mini), 2026-08-30`; body paragraph added, quotes the six gate steps verified against the live script | P1: `400a2f77` — verified `scripts/phase-gate.sh` content directly (`cat`), confirms all six named steps (nextest --all-features, cargo test --doc, clippy --all-features, fmt --check, cargo doc --all-features, vectors-checksum) and its stated blind spot (freebsd/musl/windows/macos) |
| `md-repeated-placeholder-inverts-bip388` | CLOSED — heading annotated `✓ CLOSED P2+P3, 2026-08-30`; body paragraph added | P2 final commit `83885768` (N1 taxonomy classifier, both directions, corpus replacement) + P3 final commit `f9cfde94` (card-input refusals, read-side warnings, door-check unification) |
| `descriptor-key-bracket-path-as-a-last-resort-source` | CLOSED — heading annotated `✓ CLOSED P4, 2026-08-30`; body paragraph added | P4: `d72ede51` ("N3 -- the --key bracket path becomes a last-resort PATH SOURCE"); operator ruling cited per dispatch brief (brainstorm walk, 2026-08-31) |
| `from-mk1-arity-spills-card-strings-into-the-md1-positional` | CLOSED — heading annotated `✓ CLOSED P4, 2026-08-30`; body paragraph added, both remedies (a) and (b) confirmed present in the commit diff before citing | P4: `d593218c` ("R9 -- --from-mk1 arity, on both verbs") — inspected the diff directly: `num_args = 1..` on both verbs' `--from-mk1`, `ArgGroup` widened, and `check_from_mk1_arity` names both an `mk1`-prefixed positional string and an `md1`-prefixed `--from-mk1` value by name |
| `md-cannot-mint-a-keyed-card-from-a-split-set` | CLOSED — heading annotated `✓ CLOSED P5, 2026-08-30`; body paragraph added | P5: `cd7785a2` ("the matrix travels -- S -> keyed card flips to ✓ in all four homes") |
| `md-verify-against-flag-for-cross-form-comparison` | CLOSED — heading annotated `✓ CLOSED P6, 2026-08-30`; body paragraph added | P6: `1a1983d7` (`--verify-against` wired to `spend_equal`; exit codes 0/5/1-2) |
| `md-decompose-rejects-double-wildcard-input` | CLOSED — heading annotated (appended to existing "C4 CONSIDERED AND DECLINED" text) `✓ CLOSED P6, 2026-08-30`; body paragraph added | P6: `3aa38764` (decompose desugars `/**` via the shared core) |
| `md-decompose-does-not-read-stdin` | CLOSED — heading annotated `✓ CLOSED P6, 2026-08-30`; body paragraph added | P6: `3aa38764` (`-` accepted; refusal names `--in`) |
| `md-decompose-has-no-json-output` | PARKED, not implemented — verified the entry already states the trigger ("a future front-end" needing the envelope) before adding; one-line closure appended, no duplication of the trigger text | Parked by the mini-cycle walk ruling 2026-08-31 (operator; `BRAINSTORM_mdcli_mini.md` rider 8, verified present at that file's line ~298) |
| `sibling-toolkit-md-manual-lockstep-for-the-converter` | LEFT OPEN — one paragraph appended noting the docs pass is running in `bg002h/mnemonic-toolkit` as part of this cycle's P7; explicitly states closure will cite that repo's commit, not discharged here | N/A — pending cross-repo commit |

Total: 9 closed, 1 parked, 1 left open. All closures are additive — no
entry body was deleted or rewritten; only heading `owning phase`
parentheticals were annotated and closure paragraphs appended at the end of
each entry's existing body.

## Heading / owning-phase mismatch check

Checked each closed entry's heading `owning phase` field against where the
plan's burndown table says it actually closed. No true mismatches found:

- All nine closed entries' pre-existing `owning phase` text ("post-converter
  md-cli mini-cycle" / variants) is consistent with closing somewhere inside
  this same mini-cycle plan; none contradicts the phase that actually closed
  it.
- `phase-gate-omits-cargo-doc`'s heading names a *trigger condition* ("the
  next plan that writes a phase gate") rather than a specific phase number.
  Confirmed this is not a mismatch: P1 of *this* plan (`mdcli-mini`) is that
  next plan — `400a2f77`'s subject line is literally "P1.2/gate: widen CI to
  --all-features, add scripts/phase-gate.sh". Noted in the closure text as
  "satisfying the 'next plan' this entry's heading anticipated" rather than
  a phase-number clash.
- One chronology oddity, not a heading/phase mismatch: the
  `descriptor-key-bracket-path-as-a-last-resort-source` and
  `from-mk1-arity-…` entries' dispatch-brief closure text cites an operator
  ruling / brainstorm-walk date of 2026-08-31, while the commits that
  implement them (and every other commit in this plan, including HEAD) are
  dated 2026-08-30 per `git log --date=iso-strict`. This predates the
  repo's own pre-existing text — the `from-mk1-arity…` entry's own "Filed"
  line already reads "Filed 2026-08-31" while sitting atop 2026-08-30
  commits. Pre-existing inconsistency in the repo's internal chronology, out
  of this sweep's scope to fix; carried through faithfully rather than
  silently corrected.

## Verification performed

- `git log -1 <sha>` for all 9 cited commit SHAs — all exist on `mdcli-mini`,
  all dated 2026-08-30.
- `git show d593218c` — read in full to confirm both remedy (a) and remedy
  (b) landed together before writing the closure text (not just trusting the
  dispatch brief's summary).
- `cat scripts/phase-gate.sh` — read in full to confirm the six named steps
  and stated blind spot before citing them verbatim in the closure.
- `grep -n "rider 8"` against `design/BRAINSTORM_mdcli_mini.md` — confirmed
  present (line 13 cross-reference, line 298 the rider itself) before citing
  it in the parked entry.
- Re-read the full `git diff design/FOLLOWUPS.md` after all edits: confirmed
  no entry body text was deleted, only heading annotations and appended
  closure paragraphs.

## Final state

- Commit: `c75b58d` — "followups: P7 burndown sweep -- 9 closed citing phase
  commits, 1 parked by ruling, 1 pending the toolkit docs pass"
- `design/FOLLOWUPS.md`: 78 insertions, 10 deletions (deletions are all
  heading-line replacements, matched 1:1 with a heading-line addition —
  no body content removed).
