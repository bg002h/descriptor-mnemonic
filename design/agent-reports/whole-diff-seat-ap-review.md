# Whole-diff adversarial review — seat auto-partition (`impl/seat-auto-partition`)

**Scope:** `git diff a20c17c0..HEAD` in `/scratch/code/shibboleth/dm-worktrees/seat-ap`
(P0 `97fe4c63` + P1 `c2529d13`, 27 files). Contracts: `design/SPEC_seat_auto_partition.md`
@ `230661b6`, `design/IMPLEMENTATION_PLAN_seat_auto_partition.md` @ `a20c17c0`,
`design/agent-reports/impl-seat-ap-p1.md`.

**VERDICT: 0 Critical / 3 Important / 5 Minor / 3 Nit — does NOT ship as-is.**

Baselines measured by me on the branch (not transcribed):

- `cargo nextest run --locked -p md-cli` → **742 run, 742 passed, 1 skipped** (1.76 s).
- `cargo nextest run --locked --no-fail-fast --workspace` → **1209 run, 1209 passed, 3 skipped**.
- `cargo fmt --check -p md-cli` clean; `cargo clippy -p md-cli --all-targets` clean.
- Worktree left byte-identical to `HEAD` after every experiment (`git status --porcelain` empty).

---

## What I verified clean (no finding)

These were the brief's named risk areas; each was checked against the code and,
where a tool could answer it, measured.

- **§2.5 distinct-card identity is DECODED-CARD equality, not piece-tuples.**
  `partition.rs:241` — `distinct_cards.iter().any(|c| c == &card)`; `KeyCard`
  derives `PartialEq` structurally over `policy_id_stubs`/`origin_fingerprint`/
  `origin_path`/`xpub` (`vendor/mk-codec/src/key_card.rs:23`). A duplicate
  candidate reached via piece reuse cannot inflate `|V|`.
- **Evaluation order is §2's order.** admissibility (`:150-157`, zero decodes,
  early `NoPartition`) → group-wide cap `Σk` (`:167-170`) → static saturating
  budget (`:174-183`) → enumerate/verify (`:192-204`). Cap genuinely precedes
  budget; both are proved decode-free by the `DECODE_CALLS` counter rows, which
  assert `0` rather than infer it.
- **Fail-closed composition (r1 C3).** One inadmissible class returns
  `NoPartition` for the whole group before any other class is examined; a
  per-class `Ambiguous` outranks a per-class `NoPartition` (`:199-204`).
- **`k_class = 0` / empty-seat is unreachable.** `by_index` is non-empty for
  every class built, so `k_class ≥ 1`; a `total_chunks = 0` class enumerates the
  empty candidate, `mk_codec::decode(&[])` fails, `|V| = 0 < k` → `NoPartition`.
  There is no path to `Seats(vec![])`.
- **Pre-pass fires exactly on `Failure::Merged`** (`input.rs:481-517`); the
  `Incomplete` arm, the `None`/clean arm and the terminal `mk_codec::decode`
  arm are byte-unchanged. **No input that used to reach arms 2/3 changes path**:
  §1 collapse only removes same-`(id,total,index,tail)` pieces, and a group
  classified `None` or `Incomplete` has no same-index duplicates by definition,
  so nothing collapses in either. The only intended movement is Merged→
  Incomplete/None for BCH twins (spec §1's stated consequence, row 2).
- **`GroupId::Single` untouched.** `canonicalize_group` passes `info.is_none()`
  entries through one-for-one (`input.rs:400-403`); `partition` never sees them
  (`infos.is_empty()` skips the classifier entirely).
- **`--seat #k` binds the labelled card.** Ordinals are assigned from the same
  sorted `Outcome::Seated` vector `label()` renders (`input.rs:492-496`), and
  `apply` resolves by `cards[idx].ordinal == Some(k)` (`directive.rs:187`) — not
  by position. Bare-collided / non-collided-`#k` / `#0` / out-of-range / `#`
  without digits all refuse by name, each with its own test.
- **V-ORD holds under EXHAUSTIVE permutation** (I ran it, the shipped row-8 test
  only checks 2 orderings): `v-ap-canonical` 24/24 permutations and `v-collide`
  120/120 produce an identical `(label, xpub)` list. The A3 tie-break extension
  (Deviation 8) cannot change the emitted wallet in any case — it runs only
  after every matching has byte-compared equal under `comparison_form`.
- **AP1 note is per-group, success-only, ahead of its own group's R2 warnings.**
  Measured on the two-group case the corpus never exercises (ids `11111` +
  `a1001`): output is `note(11111) → warn → warn → note(a1001) → warn → warn`.
- **Budget constant + guard.** `PARTITION_DECODE_BOUND = 200_000`,
  `177_147 ≤ 200_000 < 531_441` ✓. The compile-time guard is a real gate, not a
  hypothesis: setting the constant to `600_000` fails the build with
  `error[E0080] … assertion failed` at `partition.rs:63`. Floor (177,147) seats,
  boundary refuses naming `531441`, both green.
- **Mutation gate 2 re-executed by me** (`verify_class`'s `|V| > k` branch →
  `if false`): `seat::partition::tests::ap2_fixture_is_ambiguous_v_greater_than_k`,
  `seat::input::tests::row9_ap2_fixture_hard_refuses_nothing_seats` and
  `row10a_ground_extra_verified_candidate_is_the_row9_ap2_construction` all FAIL
  (`742 run: 739 passed, 3 failed`). The implementer's claim holds.
- **Row 1's byte-identity row compares descriptor AND address AND
  WalletPolicyId** (`seating_vectors.rs:864-897`), each against a success-asserted
  control, plus the note-before-warnings ordering. The substituted control is
  legitimate: `v-ap-row1-e2e.txt` reuses `v-bound-seat.txt`'s **exact md1 policy
  line** and the same two keys, fingerprints and stub — a true pinned twin.
- **Card-set refusals name collided cards unambiguously**:
  `check_no_impossible_card_pair` and `check_no_repeated_xpub` both render
  through `DecodedCard::label()` (`satisfy.rs:222-223`, `:256-257`), so `#<k>`
  appears. Doc-invariant sweep for stale "unreachable"/"never sees colliding"
  claims across `crates/md-cli/src/` + `README.md` came back clean.

---

## Findings

### I1 — Important. §2.4's budget is implemented as a PRODUCT across classes; the spec (and the code's own doc comment) say a SUM

**File:** `crates/md-cli/src/seat/partition.rs:174-180`

```rust
let product: u64 = classes.iter().fold(1u64, |acc, c| {
    let class_product = c.by_index.values().fold(1u64, |a, v| a.saturating_mul(v.len() as u64));
    acc.saturating_mul(class_product)          // <-- Π across classes
});
```

SPEC §2.4: *"refuse when `Σ_classes Π_indexes count_i > PARTITION_DECODE_BOUND`"*.
`partition.rs:32-33`'s own doc comment for the constant also states
`Σ_classes Π_indexes count_i`. The fold multiplies.

**Failing scenario (measured, not argued).** One id, two total-classes:
2 cards × 4 chunks (class product 2⁴ = 16) and 3 cards × 11 chunks (3¹¹ =
177,147). `Σk = 5`, inside the AP3 cap, so the cap does not intervene. Probe run
on the branch:

```
PROBE strings=41 spec_sum=177163 code_product=2834352
PROBE OUTCOME=OverBudget product=2834352
```

Spec: `177,163 ≤ 200,000` → proceed, enumerate ≈1.74 s, seat (this is exactly
spec row 7's *"mixed-totals rows: both classes complete → both seat"*).
Code: `2,834,352 > 200,000` → budget refusal. A far more ordinary shape diverges
too: 2 cards × 2 chunks + 2 cards × 16 chunks gives `Σ = 65,540` (seats) versus
`Π = 262,144` (refuses), with `Σk = 4`. Both are mintable with real `mk encode`
(16- and 11-chunk cards exist; the floor fixture is 3 × 11).

**Why it is not Critical.** `Π ≥ Σ` whenever every class product is ≥ 2, so the
engine fails closed and the ≈2 s worst-case still holds *a fortiori*. No wrong
seat, no funds surface. (One corner in the other direction: when one class has
product 1, `Π < Σ`, so the guard is not a strict upper bound on decodes —
immaterial in practice, but it means the constant no longer bounds what it
claims to bound.)

**Why it is nonetheless blocking.** (a) It refuses a large family of inputs spec
row 7 requires to seat. (b) The refusal misreports its own reason: `budget_refusal`
prints `{product}` as *"candidate key-card combinations to check"*, and for a
multi-class group that number is not what the engine would check — the engine
enumerates each class separately, i.e. the **sum**. A refusal that overstates
its own cause is the class this repo already gates on.

**Why no test caught it.** `v-collide.txt` is the only multi-class fixture that
reaches the engine, and its two classes have products 1 and 1 — `Σ = 2`,
`Π = 1`, both far under the bound. No acceptance row separates the two formulas.

**Minimal remedy.** Sum across classes, multiply within:

```rust
let product: u64 = classes.iter().fold(0u64, |acc, c| {
    let class_product = c.by_index.values().fold(1u64, |a, v| a.saturating_mul(v.len() as u64));
    acc.saturating_add(class_product)
});
```

plus one engine-unit row pinning a two-class group whose `Σ ≤ BOUND < Π` does
NOT bound-refuse, and one whose `Σ > BOUND` does.

---

### I2 — Important. The AP1 note's `n_distinct` is never separated from `n_supplied`: the mutation survives the whole suite, and spec row 3's own requirement is unmet

**Files:** `crates/md-cli/src/seat/input.rs:499` (call site), `:355` (`ap1_note`),
`crates/md-cli/tests/fixtures/seating/v-ap-shared.txt`.

**False PASS, measured.** Replacing the call site with
`ap1_note(n_supplied, n_supplied, k, set_id)` — i.e. deleting the distinct-piece
count entirely — leaves the suite fully green:

```
Summary [2.465s] 743 tests run: 743 passed, 1 skipped
```

Nothing in the corpus constrains the second count.

**Root cause.** `n_supplied != n_distinct` requires a §1-collapsible duplicate
(BCH twin, or a case/whitespace variant that survives step 1) to coexist with a
**genuine** collision. No fixture has both: row 2's twin collapses to ONE card
and emits no note at all; rows 1, 3 and 4 all yield equal counts.

**Spec row 3 is materially unmet, not merely untested.** Row 3 requires *"note
distinguishes supplied strings from distinct pieces"*. `v-ap-shared.txt`'s two
chunk-0 strings are **byte-identical** (file lines 9 and 12 — verified equal),
so step-1 `dedupe_strings` removes one *before the group is formed* and the §1
stage has nothing left to collapse. Measured note for that fixture:

```
PROBE raw mk1 lines = 6
PROBE cards=2 note=note: these 5 supplied strings are 5 distinct pieces (chunks) carrying one stamped chunk-s…
```

Six strings in the file, "5 supplied … 5 distinct" in the note: the two numbers
are identical and distinguish nothing. The shipped row-3 test asserts only
`notes[0].text.contains("2 different key cards")`, which is why this is
invisible. (Secondary consequence of the same mechanism: `n_supplied` is the
post-step-1 count, so the note's *first* number is also not literally "supplied
strings" when the operator pastes a duplicate.)

**Minimal remedy.** Add one row where the two counts differ — the row-1 4-string
pair plus a ≤ t=4-flipped twin of one chunk gives 5 supplied / 4 distinct /
2 cards — and assert the note's **exact text** there and in row 3 (exact text,
not `contains`, as row 1 already does).

---

### I3 — Important. Arm 1's merged refusal lost its only command-level row, and its `mk inspect` clause is now pinned by no test at any layer

**Files:** `crates/md-cli/tests/seating_vectors.rs` (the repurposed
`v_collide_reaches_the_command`), `crates/md-cli/src/seat/input.rs:311-326`
(`merged_refusal`).

At base `a20c17c0`, `seating_vectors.rs:846-848` asserted three things about the
arm-1 message reaching the CLI:

```
assert!(e.contains("chunk-set 12345"));
assert!(e.contains("piece order does not matter"));
assert!(e.contains("`mk inspect`"));
```

Deviation 7 repurposed that test to row 1. Measured on the branch:

```
$ grep -n "declare piece\|piece order\|mk inspect\|do not form one key card\|scan the missing" crates/md-cli/tests/*.rs
(no output)
```

**Zero integration tests now exercise the R5 classifier's arm-1, arm-2 or arm-3
refusals.** At unit level the inheritance is partial: `row7b_…` picks up
*"piece order does not matter"* and *"Re-scan one card's pieces alone"*, and
`r5_classification_order_prefers_merged_over_incomplete` picks up the
piece-evidence clause — but **`` `mk inspect` `` and the `re-mint (re-encoding
without --chunk-set-id)` clause of `merged_refusal` are asserted nowhere**.
`mk inspect` survives in the corpus only inside the *AP1 note*, which is a
different string. That clause is W15(d)'s named-command remedy — the spec's own
reasoning is that *"an unnamed check is decoration"*.

Spec row 12 requires *"Every retired assertion's inheriting row is named."* The
implementation report (Deviations 6/7) names inheritors for the classification
*behaviour* but not for these three assertions, so an acceptance row ships unmet.

**Minimal remedy.** Add `assert!(msg.contains("`mk inspect`"))` and the re-mint
clause to `row7b_incomplete_class_set_refuses_the_whole_group_via_arm_1`, and
add one `seat_cmd`-level row driving a merged group (the `44444` fixture) so
arm 1 has an end-to-end pin again.

---

### M1 — Minor. `--seat`'s no-such-card refusal lists bare ids, not labels

`crates/md-cli/src/seat/directive.rs:174`:

```rust
let known: Vec<String> = cards.iter().map(|c| c.set_id.to_string()).collect();
```

With a collided group seated, this prints `The cards supplied are: 12345, 12345,
…` — the same token twice, and never the `#<k>` the operator needs. SPEC §4 says
a stale `#<k>` gets *"the no-such-card refusal listing current labels"*. This is
precisely the path an operator lands on while hunting the right label.
**Remedy:** `.map(|c| c.label())`.

### M2 — Minor. Out-of-range chunk indices inflate `k_class` and the budget product, replacing arm 1's diagnosis with a cap/budget refusal

`partition.rs:158` computes `k_class` as the max over **all** `by_index` entries,
including indices ≥ `total_chunks`; `:174-180` folds those counts into the
product too. Enumeration only walks `0..total_chunks` (`:229`), so such a piece
is never covered and the class correctly ends at `NoPartition` — *if it gets
that far*. The inflated `Σk` can trip the cap at `:168` and the inflated product
the budget at `:181` first, so the operator gets "would need more than 5 key
cards" or "admits N candidate combinations" instead of arm 1's accurate
diagnosis. `classify` treats `chunk_index >= total_chunks` as reachable
(`input.rs`, the `out_of_range` predicate), so this is not a dead branch.
Fail-closed in every case. **Remedy:** derive `k_class` and the product from
indices `< total_chunks` only, while keeping out-of-range pieces in
`total_pieces` so the cover check still refuses them (dropping them from
`total_pieces` too would be worse — it would let a group seat while silently
ignoring a supplied piece).

### M3 — Minor. `partition()` silently drops a string whose canonical key fails

`partition.rs:127-129`: `let Ok(key) = canonical_piece_key(s) else { continue; };`.
Unreachable from `decode_cards` (every string is pre-validated by `group_key_of`,
identical call chain), but `partition` is `pub(crate)` and its own unit tests
call it on **raw fixture text**. A malformed fixture line would silently shrink
the piece set — lowering `k`, lowering the cover requirement — and the test would
still assert a seat. Fail-open shape in a test-facing entry point.
**Remedy:** return `Outcome::NoPartition` (or `expect` with the contract) rather
than `continue`.

### M4 — Minor. §Security's surplus bullet is narrower than it reads

SPEC §Security: *"a GROUND same-id extra verified candidate hits `|V| > k` ⇒ AP2
refusal"*. That holds only for a candidate assembled from **already-supplied**
pieces. An adversary who injects a *new* piece under the victim's id raises `k`
as well as `|V|` — e.g. index 0 gains a ground piece `b` such that `(b, c)`
verifies, giving `|V| = k = 2` — and the group **seats**, exposing the injected
card to the ordinary satisfy/complete machinery. Net exposure equals the
pre-existing different-id surplus path (row 10c), so this is **not a new attack
class** and the AP1 ruling knowingly traded the old hard refusal away; but the
sentence reads as an absolute guarantee and a future reader will rely on it.
Documentation only — the spec's design is settled and I am not re-opening it.

### M5 — Minor. `Outcome::Seated`'s order key degrades silently on encode failure

`partition.rs:210-211`: `encode_bytecode(a).unwrap_or_default()`. Two cards that
both fail to re-encode tie on an empty key; the stable sort then falls back to
enumeration order, which is supply-order dependent — i.e. `#<k>` would become
supply-order dependent, the one thing §4 exists to prevent. Unreachable in
practice: a card produced by `mk_codec::decode` satisfies every encoder invariant
(`vendor/mk-codec/src/bytecode/encode.rs:24-80` — non-empty stubs, `depth ==
path length`, path ≤ `MAX_PATH_COMPONENTS`). **Remedy:** treat an encode failure
as `NoPartition` rather than defaulting the key.

### N1 — Nit. `PartitionNote.text`'s doc comment is false

`input.rs:380` says *"no `note: ` prefix"*; `ap1_note` (`:355`) emits a format
string that begins `"note: these …"`. The rendered output is correct (and pinned
byte-exact by row 1), only the doc is wrong.

### N2 — Nit. Spec row 12's "§1 stage documented adjacent to `dedupe_strings`" is half-done

The §1 documentation lives on `canonicalize_group` (`input.rs:389-395`) and in
the `decode_cards` body comment. `dedupe_strings`'s own doc block
(`input.rs:~119-165`) carries no pointer to the §1 stage, and the module doc's
"Step 1" paragraph does not mention it either — so a reader arriving at step 1
still sees the pre-P1 story.

### N3 — Nit. Row 8 permutes 2 of 24 orderings

`row8_ordinal_assignment_is_invariant_under_supply_order` tests supply order and
its reverse. The property does hold exhaustively (I measured 24/24 and 120/120),
so this is not a false PASS — just weaker than it could be for free.

---

## Per-deviation verdicts

| # | Deviation | Verdict |
| --- | --- | --- |
| 1 | Row-1 e2e uses new `v-ap-row1-e2e.txt`, control is `v-bound-seat.txt` | **ACCEPT.** Verified the new fixture reuses `v-bound-seat.txt`'s exact md1 policy line and the same two keys/fingerprints/stub — a genuine pinned twin. The e2e row compares descriptor, address and the composed-wallet-id line. (Controller already settled the underlying `check_no_impossible_card_pair` refusal as correct.) |
| 2 | `canonicalize_group` runs per-group, after grouping | **ACCEPT — the equivalence proof holds.** `CanonicalPieceKey` embeds `chunk_set_id` (`canonical.rs:37`) and groups are keyed by that same wire field, so no two pieces can collapse across groups; first-appearance order survives either way. Two points in its favour the report does not make: a global pre-grouping collapse would shift `GroupId::Single(position)` keys, so per-group is strictly safer; and `canonical_piece_key` cannot fail where `group_key_of` succeeded (identical `decode_string` → `from_5bit_symbols` chain), so the `Err(_) => pass through` arm is genuinely unreachable rather than merely unhit. |
| 3 | `PARTITION_DECODE_BOUND = 200_000`, not the spec's ≈255,000 | **ACCEPT.** In the plan's window; the compile-time guard is a real gate (measured `E0080` at 600_000); floor seats, boundary refuses at 531,441. Note I1 makes the "worst case ≈1.96 s" claim conservative rather than wrong. |
| 4 | `Vec<PartitionNote>` instead of `Option<String>`, for multi-group ordering | **ACCEPT — and I verified the case the corpus never exercises.** Two colliding groups (`11111`, `a1001`) emit `note → warn → warn → note → warn → warn`, correctly per-group. Untested in the shipped suite; that gap is the reason I could not take the justification on faith. |
| 5 | `seat_chunk_set_id_warnings` kept as a `#[allow(dead_code)]` wrapper | **ACCEPT.** It delegates to `seat_notes(cards, &[])`, so the frozen R2 wording is still exercised through the production function, and row 1's command-level row independently asserts both warnings reach stderr. |
| 6 | `r5_merged_…` rewritten in place to row 1 | **ACCEPT with I3 attached.** The rewrite is correct and the fixture genuinely seats now; the arm-1 *wording* inheritance is incomplete. |
| 7 | `v_collide_reaches_the_command` repurposed; new row-10b test | **ACCEPT with I3 attached.** Row 10b's new construction (1-slot policy at an origin neither card declares, so both labels appear in one message) is strictly stronger than the alternative and is correct. This is, however, where the command-level arm-1 row was lost. |
| 8 | `assignment_vector` → `Vec<(GroupId, Option<u32>)>` | **ACCEPT.** Correct and total. Worth recording that V-ORD was never at risk from the un-extended key: the tie-break runs only after every matching has byte-compared **equal** under `comparison_form`, so it selects which assignment is reported, never which wallet is emitted. `Option<u32>` orders `None < Some`, consistent with `GroupId`'s derived ordering. |

---

## Recommended gate action

I1, I2 and I3 are all cheap (a fold each, plus three test rows) and none requires
re-opening the spec's design. I2 and I3 are both "the test cannot fail" defects,
so they should be fixed and re-run before the next gate rather than deferred;
I1 changes one fold expression and needs the two new engine-unit rows to pin the
Σ/Π distinction that the corpus currently cannot see.
