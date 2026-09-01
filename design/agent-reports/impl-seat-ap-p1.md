# P1 implementation report — seat auto-partition

**Worktree:** `/scratch/code/shibboleth/dm-worktrees/seat-ap`, branch
`impl/seat-auto-partition`. P0 committed at `97fe4c63`, untouched (HEAD
unchanged, nothing amended). All P1 work below is **uncommitted** in the
worktree, as instructed. `git diff --stat`:

```
 CHANGELOG.md                           |  22 ++
 README.md                              |   2 +-
 crates/md-cli/src/main.rs              |  32 +-
 crates/md-cli/src/seat/canonical.rs    |  14 +-
 crates/md-cli/src/seat/complete.rs     |   6 +-
 crates/md-cli/src/seat/directive.rs    | 271 +++++++++++++--
 crates/md-cli/src/seat/disposition.rs  |   4 +-
 crates/md-cli/src/seat/input.rs        | 609 ++++++++++++++++++++++++++++-----
 crates/md-cli/src/seat/matching.rs     |  28 +-
 crates/md-cli/src/seat/mod.rs          |  11 +-
 crates/md-cli/src/seat/satisfy.rs      |   9 +-
 crates/md-cli/tests/seating_vectors.rs | 139 +++++++-
 12 files changed, 975 insertions(+), 172 deletions(-)
```
New files (untracked): `crates/md-cli/src/seat/partition.rs` (499 lines, the
engine), `crates/md-cli/tests/fixtures/seating/v-ap-row1-e2e.txt` (a P1-cycle
fixture, see Deviation 1).

## Final gate

`cargo nextest run --locked -p md-cli`: **742 tests run, 742 passed, 1
skipped** (the `#[ignore]`d budget-measurement test, run explicitly below).
Whole-workspace: `cargo nextest run --locked --workspace`: **1209 run, 1209
passed, 3 skipped**. `cargo fmt --check -p md-cli`: clean. `cargo clippy -p
md-cli --all-targets`: clean, zero warnings.

---

## Per-step files + lines

**Step 0 (signature, mechanical):** `seat/input.rs:83-116` (`DecodedCard`
gains `ordinal: Option<u32>`, `label()` renders `#<k>` when `Some`);
`decode_cards` at `input.rs:434` returns
`Result<(Vec<DecodedCard>, Vec<PartitionNote>), CliError>`
(`PartitionNote` at `input.rs:377`). One production call site
(`seat/mod.rs:149`) + 22 test call sites across `input.rs` (13),
`satisfy.rs` (1, the `fixture::cards` funnel), `complete.rs` (3),
`disposition.rs` (2), `matching.rs` (3) — all updated to destructure the
tuple or discard the notes half.

**Step 1 (§1 canonicalisation):** `canonicalize_group` at `input.rs:396`,
wired into `decode_cards`'s per-group loop (`input.rs:434` body) immediately
after grouping, before `classify`. Runs PER GROUP rather than once globally
(deviation from the plan's literal phrasing — see Deviation 2). RED→GREEN
below.

**Step 2 (§2 engine):** new module `seat/partition.rs`. `GROUP_CAP: usize =
5` (`:30`); `PARTITION_DECODE_BOUND: u64 = 200_000` (`:57`) with a
compile-time acceptance-window guard (`:63`); `pub fn partition(strings:
&[&str]) -> Outcome` (`:121`); `Outcome` enum (`:76`) —
`Seated`/`Ambiguous`/`CapExceeded`/`OverBudget`/`NoPartition`; per-class
verification in `verify_class` (`:228`); refusal builders
`cap_refusal`/`budget_refusal`/`ap2_refusal` (`:282`/`:293`/`:303`). 21
engine-unit tests, all against P0's own fixtures, none reimplementing
canonicalisation (reuses `canonical::canonical_piece_key`).

**Step 3 (outcomes wiring):** `decode_cards`'s `Some(Failure::Merged)` arm
(`input.rs`, inside the per-group loop) now calls `partition::partition`
before falling through to the unchanged `merged_refusal`; `ap1_note`
(`input.rs:355`) renders the AP1 note; `seat_notes` (`input.rs:584`)
interleaves AP1 notes with R2 warnings PER GROUP; `seat::run`
(`mod.rs:149,161`) calls `decode_cards` and `seat_notes` in place of the old
`seat_chunk_set_id_warnings` call — which is KEPT, unchanged signature and
behaviour, as a thin wrapper (`input.rs`, `#[allow(dead_code)]`, its own
tests untouched).

**Step 4 (§4 identity):** order key (ascending `encode_bytecode`) computed
inside `partition()` (`partition.rs:121` body, near the `Outcome::Seated`
return); ordinals assigned in `decode_cards`'s partition-seat branch
(`Some(i as u32 + 1)`); `label()` already renders `#<k>` (step 0);
`directive.rs` rewritten: `SeatDirective` gains `ordinal: Option<u32>`
(`:41`), `parse` (`:56`) accepts `@i=<id>#<k>`, `apply` (`:132`) resolves
bare-id-ambiguous / non-collided-`#<k>` / out-of-range / `#`-without-digits,
each with its own named refusal; `matching.rs`'s `assignment_vector`
(`:164`) extended to `(GroupId, Option<u32>)` so the A3 tie-break
discriminates same-id collided carriers.

**Step 5 (message/doc churn):** `matching.rs`'s `REMEDIES` (`:228`) gains
the `#<k>` mention; `directive.rs`'s module doc comment (top) corrected
(the pre-P1 "ambiguous id is unreachable" claim is now a documented,
reachable grammar); `input.rs`'s top module doc comment (`:8-30`, was
`:12-20` pre-edit) and the `V-COLLIDE` unit test's own doc comment (was
`input.rs:490-493` pre-edit, now the `row1_canonical_collision...` test's
doc comment) both corrected; `main.rs:432-441` and `:666-675` (`--seat`
help text, both subcommands, `replace_all`) mention `#<k>`;
`README.md:132` and a new `CHANGELOG.md` `## md-cli [Unreleased]` section
added. Grep-swept `crates/md-cli/src/`, `README.md`, `CHANGELOG.md` for
`"never sees colliding"`, `"pinned to two"`, `"UNREACHABLE"` etc. — clean.

**Step 6 (mutation gates):** all four executed and reverted; see below.

---

## Measured budget constant

`cargo test -p md-cli --bin md -- --ignored --nocapture budget_measurement`
(profile: this repo's `[profile.test]`, `opt-level = 2`,
`debug_assertions` on — the profile the gate actually runs):

```
MEASURED: 20000 decodes in 196.33435ms = 9816 ns/candidate (9.816 us/candidate)
```

Floor (177,147 candidates) ≈ 1.74 s; boundary (531,441) ≈ 5.22 s.
`PARTITION_DECODE_BOUND = 200_000` → worst case ≈ 1.96 s, inside SPEC
§2.4's ~2 s target and inside the plan's acceptance window
`177_147 ≤ BOUND < 531_441`, guarded at compile time
(`partition.rs:63`). See Deviation 3 for why this differs from SPEC's own
quoted ≈255,000.

---

## RED→GREEN evidence

**Row 1** (canonical collision, unit level, `row1_canonical_collision_two_cards_seat_with_ap1_note`):
RED, captured by mechanically reverting the `Some(Failure::Merged)` dispatch
to its pre-P1 direct `merged_refusal` call (Mutation 1's edit, run before
step 3 was implemented and again as Mutation 1's own evidence), panic at
`input.rs:821`:
```
a clean 2-card collision must seat: Seat("chunk-set 11111: 2 strings declare
piece 1 of 2 and 2 strings declare piece 2 of 2. A duplicated piece number is
proof this chunk-set id is pinned ... — check each card alone first with
`mk inspect`.")
```
GREEN (current code): `PASS seat::input::tests::row1_canonical_collision_two_cards_seat_with_ap1_note`.
Also GREEN at the command level:
`PASS md-cli::seating_vectors row1_canonical_collision_reaches_the_command_byte_identical_to_the_unpinned_control`
(descriptor, address and the `composed wallet id` line all byte-identical
between the pinned pair and the unpinned control; AP1 note precedes both
R2 warnings).

**Row 5** (over-budget synthetic, unit level, `row5_over_budget_synthetic_set_refuses_statically`):
RED (engine disabled):
```
the saturated product names itself: seating refused: chunk-set 77777: 5
strings declare piece 1 of 32 and 5 strings declare piece 2 of 32 ... [arm-1
merged message, 32 clauses] ... check each card alone first with `mk inspect`.
```
GREEN: `PASS seat::input::tests::row5_over_budget_synthetic_set_refuses_statically`
— message is now `chunk-set 77777: these pieces (chunks) admit
18446744073709551615 candidate key-card combinations to check, more than
auto-separation's budget of 200000 ...`.

**Row 7b** (incomplete-class, whole group refuses via arm 1,
`row7b_incomplete_class_set_refuses_the_whole_group_via_arm_1`): this row
does NOT discriminate old vs. new code (both produce arm 1's message for
this shape — noted honestly, not claimed as a false positive). What it DOES
prove: the wiring routes `merged_refusal(set_id, &infos)` with the correct,
UNCHANGED `infos` after canonicalisation — a wiring bug (e.g. passing stale
`infos`) would break it. Engine-level discrimination for this exact case is
`seat::partition::tests::incomplete_class_set_is_no_partition_the_whole_group_fails_closed`,
which calls `partition()` directly and could not exist/pass without the
engine. GREEN: both pass.

**Row 9** (AP2 fixture, `row9_ap2_fixture_hard_refuses_nothing_seats`):
RED (engine disabled):
```
seating refused: chunk-set a1999: 2 strings declare piece 1 of 3 and 3
strings declare piece 2 of 3 and 3 strings declare piece 3 of 3. A
duplicated piece number is proof ... [arm 1's message]
```
GREEN:
```
seating refused: chunk-set a1999: these pieces (chunks) verify as more key
cards than they can belong to, and the tool will not guess which cards are
your wallet. This is not expected from accidental damage — treat the
strings as untrusted and re-scan one card's pieces alone, from a source you
trust.
```

**Row 10b** (surplus variant b, command level,
`v_collide_surplus_variant_b_seats_then_refuses_leftover_with_distinguishable_labels`):
RED (engine disabled):
```
md: seating refused: chunk-set 12345: 1 string declares piece 1 of 2 and 1
string declares piece 1 of 3 and 1 string declares piece 2 of 2 and 1
string declares piece 2 of 3 and 1 string declares piece 3 of 3. [arm 1]
```
GREEN:
```
seating refused: this card set does not seat. Completeness is total: every
slot must be filled and every supplied card must be seated. 1 slot(s)
unfilled, 2 card(s) left over (1 slots, 2 cards supplied).
Unfilled slots — no supplied card satisfies the declared origin:
  @0 [48'/0'/9'/2'] (no fingerprint declared)
Cards left over — which wallet do these belong to? Each is named by its
full chunk-set id and the policy-id stub it was minted against:
  12345#1 (stub 5b48af35) declaring origin [73c5da0a/48'/0'/0'/2']
  12345#2 (stub 5b48af35) declaring origin [73c5da0a/48'/0'/1'/2']
```

All four RED captures were produced by mechanically reverting the
`Some(Failure::Merged)` dispatch to its pre-P1 form
(`return Err(merged_refusal(set_id, &infos))`), running the target tests,
then restoring from a byte-identical snapshot (`diff` confirmed empty
before continuing each time).

---

## Mutation gates (row 11), before/after, each reverted

**1. Disable the partition attempt** (`input.rs`, `Some(Failure::Merged)`
arm forced back to the direct `merged_refusal` call): row-1 unit test
fails — `called ... on an Ok value` becomes `Err(Seat("chunk-set 11111: 2
strings declare piece 1 of 2 ...`. Reverted; `diff` against the pre-mutation
snapshot: empty.

**2. Force-seat when `|V| > k`** (`partition.rs::verify_class`, the
`if distinct_cards.len() > class.k_class { Ambiguous }` branch replaced
with `if false`): row-9 test fails —
`ap2_fixture_is_ambiguous_v_greater_than_k` reports
`expected Ambiguous, got Seated([...4 cards...])` and
`row9_ap2_fixture_hard_refuses_nothing_seats` panics on an unexpected `Ok`
with 4 seated cards (the 4th being the GROUND forged extra candidate `F` —
the exact wrong-seating the check exists to prevent). Reverted; diff empty.

**3. Skip canonicalisation** (`decode_cards`, `canonicalize_group` call
replaced with the identity): row-2 test fails —
`row2_bch_correctable_twin_collapses_to_one_card_silently` panics on
`not a genuine collision, so no AP1 partition note: [PartitionNote { ...
these 4 supplied strings are 4 distinct pieces ... }]`. Note: the FINAL
card count is still correct (1 card) because `partition()` internally
re-canonicalises (a deliberate, documented idempotent redundancy — see
Deviation 2) — the observable defect is a spurious AP1 note where SPEC row
2 requires silence. Reverted; diff empty.

**4. Skip the static budget** (`partition.rs`, the
`if product > PARTITION_DECODE_BOUND` check replaced with `if false`, plus
a probe writing `/tmp/mutation4_probe.txt` immediately before the
now-unguarded fallthrough): run under
`timeout 8 ./target/debug/deps/md-<hash> seat::input::tests::row5_over_budget_synthetic_set_refuses_statically --exact --test-threads=1`.
Result: `TIMEOUT_EXIT_CODE=124` (GNU `timeout`'s documented "command timed
out" code — the process never returned "ok"/"FAILED", confirmed by the
captured stdout ending mid-line after `test ... `), AND the probe file
contains `probe fired: product=18446744073709551615` (the SATURATED
product, proving the mutated line executed and genuinely entered the
unbounded 5^32-scale enumeration). **Probe fired ∧ timeout expired,
both required, both true.** Reverted; diff empty. This mutation was never
committed at any point.

---

## Deviations from the plan, each with its reason

**1. `v-ap-canonical.txt` / `v-ap-canonical-control.txt` (P0's own row-1
fixtures) cannot reach a seating at all — a P0 fixture-authoring defect,
not a P1/engine defect.** Measured directly: `md descriptor` on EITHER
file refuses with `cards ... both declare origin [73c5da0a/48'/0'/0'/2']
yet carry DIFFERENT xpubs ... impossible from one master`
(`satisfy::check_no_impossible_card_pair`, a PRE-EXISTING, orthogonal
check that runs before A2/A3 for every seating, collided or not). Root
cause, verified against `keys.txt`: every one of the 16 test keys records
the SAME placeholder fingerprint (`73c5da0a`), so P0's
`--origin-fingerprint <KEY N's fp>` mint recipe put two DIFFERENT xpubs at
the IDENTICAL declared `(fingerprint, path)` — `v-bound-seat.txt`
demonstrates the correct pattern (explicit, DISTINCT fingerprints,
`73c5da0a`/`b8688df1`). The auto-partition ENGINE itself correctly seats
`v-ap-canonical.txt`'s pair — proven independently at the engine-unit
level (`seat::partition::tests::canonical_pair_seats_two_cards_v_equals_k`)
and equivalently at the decode_cards-unit level
(`row1_canonical_collision_two_cards_seat_with_ap1_note`, on an inline
2-card literal with the same shape). Per the brief's instruction to STOP
and report rather than silently reshape the engine or the fixture: I did
not touch `check_no_impossible_card_pair`, `v-ap-canonical.txt`, or
`v-ap-canonical-control.txt`. For the full-command-level row-1 proof I
minted a new, minimal fixture (`v-ap-row1-e2e.txt`, provenance in its own
header) using `v-bound-seat.txt`'s existing policy + explicit distinct
fingerprints, whose UNPINNED control is `v-bound-seat.txt` itself (same
two keys, same fingerprints, same path — natural ids instead of a pinned
one). Full trace, including the exact refusal text from both original
files, is in the new fixture's header comment.

**2. `canonicalize_group` runs PER GROUP, after step 2's grouping, not as
one global pass before grouping (plan's literal phrasing).** Provably
equivalent: `CanonicalPieceKey` embeds `chunk_set_id`, so two pieces can
never canonicalise together across different groups — collapsing within a
group after grouping produces an IDENTICAL result to collapsing globally
then grouping. The reorder buys two things without cost: (a) it naturally
yields `n_supplied` (pre-collapse count) and `n_distinct` (post-collapse
count) per group for the AP1 note, which a global pre-grouping pass would
have to re-derive by other bookkeeping; (b) `partition()` independently
re-canonicalises its OWN input (documented, deliberate idempotent
redundancy — mirrors the "P0's shape tests call the shipped function"
discipline), which is what makes it directly unit-testable on raw fixture
text without routing through `decode_cards` first, and what Mutation 3
above measured directly (skipping `decode_cards`'s own canonicalisation
still leaves the final card SET correct, only the note becomes spurious).

**3. `PARTITION_DECODE_BOUND = 200_000`, not SPEC §2.4's own quoted
≈255,000.** SPEC's number is derived from the R0 rounds' RELEASE-profile
measurement (7.845 µs/candidate); the plan explicitly requires
re-measuring ON THE TEST PROFILE, which measured 9.816 µs/candidate — about
25% slower. Carrying the release-derived 255,000 forward would have put
the worst case at ≈2.50 s, over SPEC's own ~2 s target. 200,000 keeps the
worst case at ≈1.96 s while staying inside the plan's numeric acceptance
window `177_147 ≤ BOUND < 531_441`.

**4. `decode_cards`'s AP1-note return type is `Vec<PartitionNote>`
(`{set_id, text}`), not the plan's literal example `Option<String>`.**
The plan itself offers "or a small struct" as an alternative. A bare
`Option<String>` cannot support SPEC §5's per-GROUP interleaving
correctly when more than one id group collides in one `decode_cards` call
(a case the acceptance corpus never exercises, but the pipeline itself
does not rule out) — without the `set_id` tag, `seat::run` would have no
way to know WHERE in the R2-warning stream to place a note whose group is
not the numerically-first one. This costs nothing at the 22 call sites
(all of which just discard or forward the tuple) and is exercised by
`row1_canonical_collision_reaches_the_command_byte_identical_to_the_unpinned_control`'s
ordering assertions.

**5. `seat_chunk_set_id_warnings` kept as a thin wrapper over the new
`seat_notes`, rather than changed/removed.** Its own pinned tests
(`csid_warning_fires_on_a_pinned_mismatch...`,
`csid_warning_wording_is_pinned_against_literals...`) are UNTOUCHED and
still pass; `seat::run` now calls `seat_notes` directly. Marked
`#[allow(dead_code)]` with a comment, mirroring the exact precedent P0 set
for `canonical_piece_key` before its own production wiring landed.

**6. `r5_merged_two_cards_pinned_to_one_id_classify_as_merged` REWRITTEN
in place** (not just `v_collide_two_cards_pinned_to_one_chunk_set_id_*`) —
per the spec's own row-12 churn note ("→ canonical row"), confirmed
necessary by running it: this fixture (chunk-set `11111`, two distinct
2-chunk cards, no shared pieces) now auto-partitions and seats instead of
refusing, exactly SPEC row 1's shape. Renamed to
`row1_canonical_collision_two_cards_seat_with_ap1_note`; arm 1's own
message stays independently pinned by the UNTOUCHED
`r5_classification_order_prefers_merged_over_incomplete` (the `44444`
admissibility-failure fixture, SPEC row 6).

**7. `v_collide_reaches_the_command` REPURPOSED to test row 1** (using the
new `v-ap-row1-e2e.txt`, per Deviation 1) **rather than kept as an arm-1
refusal test; a NEW test
`v_collide_surplus_variant_b_seats_then_refuses_leftover_with_distinguishable_labels`
added for row 10b**, using `v-collide.txt` against a freshly-minted
1-slot policy at an origin NEITHER collided card declares (so both
`12345#1`/`12345#2` appear together as leftover, the clearest possible
demonstration of "distinguishable labels" — verified this is stronger
than a single-leftover construction, which I tried first and confirmed
also works but shows only one label). The OLD test's setup (V_USP policy +
V_USP cards + `v-collide.txt`'s cards) was measured to ALSO demonstrate
variant (b) correctly (V_USP's 2 cards fill V_USP's 2 slots via Kuhn's
matching regardless of the 2 extra collided cards, which have no candidate
slot at all) — I did not use it only because the new minimal 1-slot
construction is simpler and the R0 report's proposed vehicle (V_USP) was
never actually the deciding factor once row 10b needed its own dedicated
test. Row 10c (different-id extra card) needed no new test: the existing
`v_leftover_reaches_the_command_naming_the_card` (PATHOLOGICAL + an
unrelated foreign card, no shared id anywhere) re-confirmed green,
unmodified.

**8. `matching.rs`'s `assignment_vector` return type changed from
`Vec<GroupId>` to `Vec<(GroupId, Option<u32>)>`** (SPEC §4's explicit
"extends the A3 tie-break key"). Without the ordinal, two candidate
matchings that assign DIFFERENT physical collided cards sharing one
`set_id` to the same slot would render identical tie-break keys, silently
losing the total-ordering guarantee SPEC A3(a)'s principle claims for
every axis.

No other deviations. Every fixture cited by the plan/spec behaved exactly
as claimed once reached through the real engine, with the one exception
above (Deviation 1), which was a PRE-EXISTING P0 defect orthogonal to
everything P1 built.

---

## Files touched (absolute paths)

- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/partition.rs` (new)
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/input.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/directive.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/matching.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/canonical.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/mod.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/complete.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/disposition.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/seat/satisfy.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/src/main.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/tests/seating_vectors.rs`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/crates/md-cli/tests/fixtures/seating/v-ap-row1-e2e.txt` (new)
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/README.md`
- `/scratch/code/shibboleth/dm-worktrees/seat-ap/CHANGELOG.md`

Worktree left dirty/unstaged as instructed; no commits made.
