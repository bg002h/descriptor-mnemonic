# IMPL — converter C4 (acceptance, docs, close-out)

**Phase:** C4 of the wallet-form converter (plan §3 C4; SPEC "Acceptance").
**Worktree:** `/scratch/code/worktrees/converter-c4`, branch `impl/converter-c4`,
branched from `impl/converter-c3` (`32eeca02`).
**Final SHA:** `541f5005` (this report's own commit follows it).
**Not pushed. `main` and the other `impl/` branches untouched. Worktree left in
place.**

`git diff --stat impl/converter-c3..HEAD` — 15 files, **1,270 insertions, 100
deletions**. The 100 deletions are one refactor (C3's copy of the equality
oracle moved into a shared test module) plus the four matrix rows and three
captions that were rewritten.

## Commits

| SHA | What |
| --- | --- |
| `d75214f7` | gate: `cargo doc` was RED on entry — the `--seat` help text tripped rustdoc |
| `caa3b0f1` | c4: the acceptance walks, and the matrix cells they prove — one commit |
| `ac354d80` | c4 docs: CHANGELOG entry for the converter, README section placing it |
| `4e9af312` | c4: `scripts/push-via-staging.sh` — the staging ritual this repo lacked |
| `541f5005` | c4: follow-ups reconciled — one decision, one decline, four new, one triggered |

## Exit gate — outputs verbatim, final tree

```
$ cargo nextest run --locked
     Summary [   0.973s] 1047 tests run: 1047 passed, 2 skipped
  exit 0

$ cargo clippy --locked --all-targets -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.06s
  exit 0

$ cargo fmt --check
  (no output)
  exit 0

$ cargo test --workspace --doc --locked                    # CI runs this
    test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  exit 0

$ RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items
   Generated /scratch/code/worktrees/converter-c4/target/doc/md/index.html and 1 other file
  exit 0                                                   # RED on entry — see below

$ cargo nextest run --locked -E 'test(walk_)'
     Summary [   0.009s] 10 tests run: 10 passed, 1039 skipped
  exit 0

$ cargo nextest run --locked -E 'binary(acceptance_walks)'
     Summary [   0.013s] 9 tests run: 9 passed, 0 skipped
  exit 0

$ cargo nextest run --locked -E 'test(v_d_)'               # C3's rows, regression check
     Summary [   0.028s] 30 tests run: 30 passed, 1019 skipped
  exit 0

$ scripts/matrix-identity-check.sh
matrix identical across 4 homes, 6 lines each:
  design/BRAINSTORM_wallet_form_converter.md
  design/SPEC_wallet_form_converter.md
  design/IMPLEMENTATION_PLAN_wallet_form_converter.md
  crates/md-cli/src/seat/mod.rs
sha256: 0527deee4a60b60385b07d7bfda8d5987672d2d5195ced56db0780836a5baab1
  exit 0
```

### Expected vs matched — the row-scoped run

The roster's C4 entry is `W-A/B/C, W-PIN`. C4 lands **9 rows in 4 families**,
0 short:

| family | rows | what |
| --- | --- | --- |
| `walk_pin_` | 2 | the two SPEC Acceptance 4 pins |
| `walk_a_` | 2 | leg (a): composition + the address pin; the 1,901-vs-1,648 arithmetic |
| `walk_b_` | 4 | leg (b): spend-equality, the address, the deliberate round-trip INequality, the negative half |
| `walk_c_` | 1 | leg (c)'s pointer-integrity grep (asserts nothing about the round trip itself) |

`-E 'test(walk_)'` matches **10**, not 9: the tenth is a PRE-EXISTING unrelated
unit row whose name starts with the same token —
`md-cli::bin/md parse::template::root_tests::walk_rawpkh_wsh_check_emits_rawpkh_node`.
The binary-scoped run above isolates the phase's own 9. Suite growth: C3's tip
ran **1038**, this runs **1047** — **+9**, all of them the walks.

## What landed

### 1. The acceptance walks (`crates/md-cli/tests/acceptance_walks.rs`, 474 lines)

**Leg (a), SPEC Acceptance 1(a).** The 36-string split set composes through
`md descriptor <md1…> --from-mk1 <mk1…>`; no `@` placeholder survives, eleven
xpubs are present, and address 0 is the journey's own
`bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64` — asserted
**three ways**: derived from the composed STRING by rust-miniscript inside the
test (md not in the loop), printed by `md address` over the same cards, and
volunteered by md on stderr (SPEC B2's note).

**Leg (b), SPEC Acceptance 1(b).** The 22-string keyed card composes SPEND-EQUAL
to leg (a)'s wallet, derives the same address 0, and is deliberately **not**
round-trip-equal — the split cards declare `[fingerprint/path]` origins and the
keyed card declares none, which is exactly why SPEC Acceptance 1 carries two
relations (r3 C2). Both halves are asserted separately.

**Leg (c).** Nothing new is asserted. C3's eleven `v_d_rt_*` rows discharge it,
and are named in the module doc. The one thing added is
`walk_c_leg_c_rows_still_exist_in_cmd_decompose_roundtrip`, which greps those
eleven names out of the other file's source so the pointer cannot rot into a
comment describing tests nobody kept. **This is a deviation from the brief's
"add one comment"** — taken because a comment naming eleven tests is precisely
the kind of claim that goes stale silently, and the grep costs ten lines.

### 2. The oracle does not grade itself

`seat::compose::spend_equal` is part of what the acceptance grades, so both
relations are computed in `crates/md-cli/tests/common/facts.rs` from a
rust-miniscript parse of the emitted descriptor STRING. C3's copy of that oracle
**moved** into that file rather than being duplicated;
`cmd_decompose_roundtrip.rs` now includes it with `#[path]` and its eleven rows
pass unchanged (30/30 `v_d_` rows above). Two copies of a funds-shaped equality
relation is a drift hazard; one copy, used by both acceptance files, is not.

### 3. The extracted fixture (plan D2)

`crates/md-cli/tests/fixtures/pathological/keyed-card.txt`, with
`extract-keyed-card.sh` beside it. Measured on the journey page 2026-08-30:

| | |
| --- | --- |
| md1 tokens on the page (all cards) | 78 |
| `md1fatzr2…` tokens (this card, rendered twice) | 44 = 42×86 + 2×59 |
| after order-preserving dedupe | 22 = 21×86 + one 59-char tail |

The `md1fatzr2` prefix filter is load-bearing, not decoration: the same page
carries the KEYLESS policy card (`md1fkl3cz…`, 6 chunks × 5 renders) and a third
unrelated 4-chunk card (`md1fveszps…`), so an unfiltered `md1[0-9a-z]+` sweep
would splice three cards together. **The W-PIN shape is asserted before the file
is written** (the script exits non-zero and leaves the committed fixture
untouched) **and again by the test helper before any walk runs** — a failed
extraction fails, it does not skip. Determinism verified: re-running the
extractor leaves `git status` empty.

### 4. The matrix (plan §1, all four homes) — and the one cell that did not flip

Flipped to ✓: the T row's two ⚠ P1 cells, the S row's concrete-descriptor and
addresses cells, and the whole D row. **Every flipped cell was RUN first**, not
inferred:

| cell | how it was proved |
| --- | --- |
| T → descriptor / addresses | `md descriptor\|address --template … --key '@i=[fp/path]xpub'`, exit 0, same wallet as the D-row route |
| S → descriptor / addresses | the walks above |
| D → addresses | `decompose --emit template/keys`, then `md address --template --key`, exit 0, address matches |
| D → keyed card | `decompose --emit commands`, route 1 eval'd, exit 0, 6 md1 chunks out |
| D → keyless + mk1 cards | route 2's md half exit 0 (`policy.md1` + `keys.txt`); mk's half measured in C3 |

**`S → keyed card` stays ✗.** The brief said to flip all three S-row cells; the
measurement says the third is not true. A descriptor composed from mk1 cards
carries DEPTH-0 extended keys (md's `Pubkeys` TLV holds 65 bytes with no depth,
so md-codec rebuilds a depth-0 xpub), and `md encode --key` admits only an
account-level xpub. Measured both directions:

```
$ md encode "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,…))" --key "@0=<composed xpub>"
md: --key @0: expected an account-level xpub at depth 3 or 4 (this script
context conventionally uses 4), got 0

$ md decompose "<the descriptor composed from the split set>" --emit template
md: decompose: key @0 is depth-inconsistent IN THE INPUT: the extended key
states depth 0, but its origin path `[73c5da0a/48'/0'/0'/2']` has 4
component(s) — depth 4. …
```

So the bridge refuses from either end. The cell now carries that reason inside
the table, and `md-cannot-mint-a-keyed-card-from-a-split-set` is filed. The
information is not lost (a keyed card needs exactly the 65 bytes each mk1 card
already carries) — only the surface is missing, and the natural fix is an
`--out`/`--emit md1` on `md descriptor --from-mk1`, **not** a relaxation of the
depth rule.

Flipping that cell would have written a false claim into four artifacts and the
code, in the same commit that claims to prove the cells.

`scripts/matrix-identity-check.sh` is committed so the four-way identity is a
command rather than a discipline. It reported identical **before** the flip
(sha256 `8fbd6287…`) and **after** (sha256 `0527deee…`). It checks IDENTITY, not
truth, and its own header says so.

### 5. Docs

**CHANGELOG.md** — a new `## md-cli [Unreleased] — the wallet-form converter`
section at the top, following the file's existing convention (crate-prefixed
Unreleased headings, newest first, prose with the measurement). Covers the four
things the brief names: the seating engine (`--from-mk1` / `--from-mk1-file` /
`--seat` on `descriptor` and `address`), `md decompose`, the origin-notated
`--key` form with per-datum precedence, and the key-reuse refusal policy on
BIP-388 grounds — quoting the shipped refusals' own words ("the public keys
obtained by deserializing elements of the key information vector must be
pairwise distinct", "forbidden by BIP 388", "UNSUPPORTED", never "invalid") and
naming the boundary's SEAT side so the entry cannot be read as banning the
fingerprint-free same-path family.

**README.md** — one new section, "Moving between wallet forms", before "Status":
the four forms, four commands, why seating is the funds-shaped step, and a LINK
to the matrix in the three design docs. Not a fifth copy — a fifth copy is a
fifth thing to keep in sync, and the identity script only guards four.

Machine-checked before commit: all four README command shapes RUN against the
committed fixtures (exit 0 each); `--emit`'s six values read out of
`md decompose --help`; the 720 bound read from `seat::matching::MATCHING_BOUND`;
the hardened spelling is `'` (md's shipped emission) inside double quotes so the
snippets are runnable as printed.

**Snapshot surfaces (plan D3): nothing moved, and that is checked, not assumed.**
C4 adds no flag, no subcommand and no help text — the one source change
(`d75214f7`) is an `#[allow]` attribute, and `md descriptor --help` still prints
the identical `--seat` line. `git status` shows no `.snap` or `.snap.new`
anywhere in the tree. Nothing was regenerated because nothing needed to be.

### 6. `scripts/push-via-staging.sh` (plan D4's follow-up)

Adapted from mnemonic-engrave's, which was read first. Three things are
repo-specific and were resolved against the LIVE rule rather than copied
(`gh api repos/bg002h/descriptor-mnemonic/branches/main/protection`):
**two** required contexts here — `cargo test (ubuntu-latest)` and
`cargo clippy` — with `strict:false` and `enforce_admins:false`; the branch is
`main`; `--repo bg002h/descriptor-mnemonic` on every `gh` call. A single-context
loop copied from the sibling would have pushed with clippy still running.

Beyond the sibling's: a clean-tree precondition (the freeze depends on it, so it
is enforced rather than assumed), per-JOB conclusions instead of the run-level
one, and the FREEZE rule in the header with the measurement behind it.

**NOT RUN, deliberately** — running it pushes. Verified by `bash -n` and by
exercising the required-context parsing in isolation (the `IFS='|'` split yields
exactly the two contexts, including the one with spaces and parentheses; the awk
lookup returns `success`/`pending` against a mocked job table). The `gh` calls
themselves are unexercised.

## The gate that was RED on entry

C4 opened by running the WHOLE validation surface rather than the plan's three
commands, and CI's `doc` job was already failing at the C3 tip:

```
error: unclosed HTML tag `chunk-set-id`
   --> crates/md-cli/src/main.rs:336:56
error: unclosed HTML tag `chunk-set-id`
   --> crates/md-cli/src/main.rs:417:56
error: could not document `md-cli`
```

Verified pre-existing by stashing C4's work and re-running against the bare C3
tip: same two errors, same lines. Introduced with C2's `--seat` flag; invisible
to nextest, clippy and fmt because none of them runs rustdoc, and the plan's
"gate" is exactly those three. **C2 closed green and C3 closed green with a CI
job red the whole time.**

Fixed at `d75214f7` by silencing the lint at the two fields rather than
rewording the text: it is a CLAP help string, so rewording would change
`--help`, the man pages and the gui-schema. Help output verified byte-unchanged.
The GAP is filed as `phase-gate-omits-cargo-doc` — a phase gate narrower than CI
reports green for a tree CI will reject.

## Walk measurements

| measurement | value |
| --- | --- |
| address 0, both forms | `bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64` |
| keyed-card composition | **1,648** chars / **1,649** bytes with the newline (SPEC Acceptance 4 ✓) |
| keyed card | **22** strings = 21×86 + one 59-char tail (SPEC Acceptance 4 ✓) |
| split-set composition | **1,901** chars = 1,648 + 11 origins × 23 |
| split composition, origins stripped | **1,648** chars; identical to the keyed form up to the `#` checksum |
| the two checksums | `#s5a2k003` (split) vs `#xn3k4jmt` (keyed) — computed over different text |
| composed wallet id / policy shape id | `ced22709` / `5b48af35` (the pair SPEC A1/B1 cite) |
| B1 disposition | 11 cards SHAPE-CONFIRMED, each named by its five-hex chunk-set id |

**The brief's 1,648 for leg (a) is a misattribution, and this is the correction.**
SPEC Acceptance 4 pins 1,648 to "the composed **keyed-card** descriptor", and
plan §3 C4 item 1 says "composed **keyed** descriptor 1,648 chars". The split
form is 1,901 because it carries the origin metadata the keyed card does not.
Rather than assert a number the measurement contradicts,
`walk_a_the_split_composition_is_the_keyed_form_plus_eleven_origins` asserts the
arithmetic against the real strings: 1,901 and 1,648 exactly, each of the eleven
origins 23 characters, and the origin-stripped split form equal to the keyed
form character-for-character up to the checksum.

### Mutation check — run, then reverted

A green suite proves little, so the seating was broken to see whether the walks
noticed. `seat::compose::compose` was mutated to seat slots 0 and 1 into each
other's positions — a **valid, complete, but WRONG** wallet, all eleven slots
filled, no refusal path involved:

| mutation | walk rows that FAILED |
| --- | --- |
| slots 0↔1 exchanged in `compose` | 4 of 9 — `walk_a_the_split_set_composes_and_derives_the_pinned_address_zero`, `walk_a_the_split_composition_is_the_keyed_form_plus_eleven_origins`, `walk_b_the_keyed_card_composes_spend_equal_to_the_split_set`, `walk_b_the_two_forms_are_not_round_trip_equal_because_only_one_declares_origins` |

Reverted; `git diff` confirms the committed tree carries no mutation. The
relation itself carries its own negative half in
`walk_b_spend_equality_fails_when_two_slots_exchange_their_keys` — an EXCHANGE
rather than an overwrite, because an overwrite leaves ten distinct keys where
there were eleven and the STRUCTURE half catches it, proving nothing about the
value half.

## The B2 decision, with its evidence

C2's report parked one question here: `md descriptor --from-mk1` cannot be
handed a keyed card, so SPEC B2's split-vs-keyed branch has no
operator-reachable path.

**Decision: file `--verify-against` as post-converter; do not add it in C4.**

The walk's evidence is stronger than "inexpressible". The comparison IS
expressible — `md address` on each form prints the same address 0, and `--count`
widens it. What the walk proved is that the form an operator reaches for FIRST
is wrong:

```
$ md descriptor <6 keyless md1> --from-mk1 <30 mk1>  > split.txt   # 1,901 chars
$ md descriptor <22 keyed md1>                       > keyed.txt   # 1,648 chars
$ diff split.txt keyed.txt
Files split.txt and keyed.txt differ                               # exit 1
```

Same wallet; `diff` says otherwise, over 254 characters of origin metadata and a
checksum computed across it. A false NEGATIVE on a correct restore is the worst
direction for a funds-shaped check — it invites re-cutting plates that are fine.
That is the case for a flag rather than a documentation line.
`seat::compose::spend_equal` already ships row-pinned four ways, carrying the
`#[allow(dead_code)]` whose stated reason is this exact gap; the flag is the
wiring. Not added here because C4 is close-out, and a surface shipped at
close-out is a surface no R0 round reviewed.

## Follow-ups sweep — plan §6, every entry resolved

| entry | owning phase | state |
| --- | --- | --- |
| `md-repeated-placeholder-inverts-bip388` | post-converter md admission mini-cycle | **post-converter**, untouched (its own text forbids a converter side effect) |
| `stub-keyed-wallet-binding-at-mint` | mnemonic-key's next mint cycle | **post-converter, other repo**, untouched |
| `scripts/push-via-staging.sh` (D4) | C4 | **DONE this cycle** (`4e9af312`) |
| `mk inspect` chunk-set-id surface (r4 M2) | C4, conditional | **NOT NEEDED — not filed** |
| `md-decompose-rejects-double-wildcard-input` | post-converter md-cli mini-cycle (heading offered C4 the option) | **CONSIDERED AND DECLINED**, reason recorded in the entry |

The `mk inspect` item was conditional — "if the `--seat` UX proves it needed". It
did not: md prints the full five-hex-digit chunk-set label beside every card
BOTH in a seating refusal AND in the B1 disposition note on a **successful**
composition (measured on the pathological set: `043d3, 13da0, 3fc7c, 69f0e,
7fa26, 94eb4, ab645, d1427, dd78b, decb1, e03a5`). A separate mk surface would
be a second way to learn what md already volunteers. Recorded as a decision in
plan §6 rather than filed as an item.

The double-wildcard entry was declined for the reason C3 filed it: the fix
WIDENS the D-row input boundary SPEC P3 states, which would be a spec change
smuggled past the R0 loop inside a close-out commit. Its heading now records the
decision instead of leaving the offer open.

**Filed by C4 (four new, none gating):**

| entry | severity | owning phase |
| --- | --- | --- |
| `md-verify-against-flag-for-cross-form-comparison` | Minor | post-converter md-cli mini-cycle |
| `md-cannot-mint-a-keyed-card-from-a-split-set` | Minor | post-converter md-cli mini-cycle |
| `phase-gate-omits-cargo-doc` | Minor | the next plan that writes a phase gate |
| `push-ritual-not-discoverable-from-claude-md` | Nit | operator's call |

**Marked TRIGGERED:** `manual-cli-surface-mirror` — a standing cross-repo
invariant this cycle made concrete. `md-cli` gained one subcommand (`decompose`,
with `--in`/`--emit`/`--network`) and three flags on each of `descriptor` and
`address`, plus the origin-notated `--key` value form;
`mnemonic-toolkit/docs/manual/src/40-cli-reference/42-md.md` must mirror them or
the manual-side `tests/lint.sh flag-coverage` step fails on its next push. Not
done in-cycle: the manual is a SIBLING repo this worktree does not reach.

## Deviations from the brief, each with its reason

1. **The `S → keyed card` matrix cell was NOT flipped.** The brief said "S-row
   ✗ P2 cells → ✓"; the bridge measurably refuses from both ends (both
   diagnostics quoted above). Flipping it would have written a false claim into
   four artifacts and the code. Filed instead, with the cell carrying its
   reason.
2. **Leg (a) is asserted at 1,901 chars, not 1,648.** The brief attached SPEC
   Acceptance 4's 1,648 to leg (a); the spec and the plan both attach it to the
   KEYED-card composition, and the split form measures 1,901. Both numbers are
   asserted exactly, plus the arithmetic that relates them.
3. **A `walk_c_` row exists.** The brief said leg (c) gets "one comment". It
   gets the comment AND a ten-line grep asserting the eleven named rows still
   exist — a pointer-integrity check, not a re-assertion of the round trip.
4. **`cmd_decompose_roundtrip.rs` was refactored** to share the equality oracle
   rather than leaving a second copy of it. C3's file is reviewed and green;
   the change is mechanical and its eleven rows pass unchanged (`v_d_` 30/30).
   The alternative — two copies of a funds-shaped relation that must agree — is
   the worse hazard.
5. **A fifth commit, `d75214f7`, fixes a gate that was red before C4 started.**
   Out of the brief's five items, but the exit gate is the whole validation
   surface and a phase cannot close green on a tree CI rejects.
6. **`scripts/matrix-identity-check.sh` was committed.** The brief asked for the
   check to be run and its output quoted; committing it makes the check a
   command rather than a thing to remember, which is what the repo's own
   build-gate rule prescribes for a check that will be re-run.
7. **CLAUDE.md was NOT edited**, though the push script's discoverability
   depends on it. An implementer editing the instructions it is operating under
   is the operator's call; filed as a Nit with the exact one-line fix.

## What C4 did NOT do

* **No post-implementation adversarial review was dispatched.** Plan §3 C4 item
  5 names a mandatory, non-deferrable opus review over the WHOLE cycle diff
  (C0..C4) before merge. That is the controller's to run; merge waits for it.
* No push. `main` and the other `impl/` branches untouched; the staging script
  is committed but unrun.
* No new CLI surface, no flag, no wire-format change, no mk or md-codec change.
* No `--verify-against`, no `/**` desugar, no keyed-card mint from a split set —
  all three deliberately filed rather than taken (above).
