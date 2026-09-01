# Fold report — whole-diff-seat-ap-review findings

**Worktree:** `/scratch/code/shibboleth/dm-worktrees/seat-ap`, branch
`impl/seat-auto-partition`, base tip `c2529d13`. Left **dirty/unstaged**
per instruction — nothing committed by this agent.

**Report folded:** `design/agent-reports/whole-diff-seat-ap-review.md`
(0 Critical / 3 Important / 5 Minor / 3 Nit). Every finding below is
addressed except M2, which reproduction found unreachable (see M2).

---

## I1 — Σ vs Π budget (`partition.rs:172-186`)

**Fix:** the fold's `acc.saturating_mul(class_product)` over `classes`
became `acc.saturating_add(class_product)` — sum across classes, product
within a class's own indices, matching SPEC §2.4 and the constant's own
doc comment (both already said Σ; only the arithmetic was Π).

**RED (measured before the fix, real decodable cards — id `0xB2001`, 2
cards × 4 chunks + 3 cards × 11 chunks):**

```
thread '...budget_sums_across_classes_not_products_i1_separating_row' panicked:
SPEC row 7 (Sigma=177163 <= BOUND=200000): expected Seated(5), got
OverBudget { product: 2834352 } -- if this is OverBudget{product} with
product near 2,834,352, the budget fold is still multiplying ACROSS
classes instead of summing (I1)
```

`2,834,352` matches the review's own PROBE (`code_product=2834352`)
exactly.

**GREEN (after the fix):** `budget_sums_across_classes_not_products_i1_separating_row`
passes — `Outcome::Seated(5)`, both cards from both classes present.

**Second row** (`sum_still_refuses_a_genuinely_over_budget_two_class_group_i1`,
2 cards × 12 chunks + 3 cards × 13 chunks, Σ = 4096 + 1,594,323 =
1,598,419 > BOUND): this one was ALSO red before the fix (buggy Π gave
6,530,347,008, not the asserted Σ), and green after — proving the sum
still correctly caps a genuinely over-budget multi-class group, zero
decode calls.

**Files/lines:** `crates/md-cli/src/seat/partition.rs:179-198` (fix),
`:594-738` (2 new tests + the `real_card_at_exact_chunk_count` helper,
which mints real, independently-decodable cards at an arbitrary
`total_chunks` by controlling fragment length, since no `mk encode`
invocation can produce two total-classes under one pinned id).

The refusal's own candidate-count claim (`budget_refusal`, "N candidate
key-card combinations to check") needed no wording change: under the
fixed Σ formula, that number IS what the engine would check (each class
enumerated separately), so the message became accurate as a side effect
of the arithmetic fix.

---

## I2 — vacuous `n_distinct` (`input.rs:355-368`, `:467-499`)

**No code change.** The counting logic (`n_supplied` = post-step-1,
pre-`canonicalize_group`; `n_distinct` = post-`canonicalize_group`) was
already correct — the corpus just never exercised an input where the two
differ, because `v-ap-shared.txt`'s duplicate chunk-0 lines are
byte-identical and step 1's raw dedupe eats one before canonicalisation
ever runs.

**Separating fixture:** row 1's own 4-string, no-sharing pair
(`v-ap-canonical.txt`, id `a1001`) plus ONE line from the already-committed
(but previously unwired) `v-ap-bchtwin.txt` — a BCH-correctable twin
(1 char flipped, well inside t=4) of card 0's chunk 0. The twin is
byte-different from its original, so step 1 keeps both (`n_supplied` = 5);
`canonicalize_group` then collapses it back onto the original
(`n_distinct` = 4); the pair still seats as 2 cards.

**Mutation re-applied and confirmed RED**, per the report's own repro
(`ap1_note(n_supplied, n_supplied, k, set_id)`):

```
seat::input::tests::row3_shared_piece_pair_seats_two_cards_via_reuse ... ok
seat::input::tests::row3b_note_separates_supplied_from_distinct_when_a_bch_twin_duplicates_a_supplied_piece_i2 ... FAILED
  left:  "...these 5 supplied strings are 5 distinct pieces..." (mutated, wrong)
  right: "...these 5 supplied strings are 4 distinct pieces..." (expected)
```

Row 3 (shared-piece) still passes under the mutation, exactly as the
report measured — it is `row3b` (this fold's new row) that catches it.
Mutation reverted; both tests GREEN on the real code.

**Also upgraded** row 3 (`row3_shared_piece_pair_seats_two_cards_via_reuse`)
from `.contains("2 different key cards")` to the exact note text (5
supplied / 5 distinct / a1002 — both numbers equal, because step 1 already
ate the one true duplicate in that fixture), matching row 1's rigor per
the report's remedy.

**Files:** `crates/md-cli/src/seat/input.rs:1262-1277` (row 3 exact-text),
`:1288-1320` (new `row3b`).

---

## I3 — arm 1/2/3 coverage lost

**Unit level** (`row7b_incomplete_class_set_refuses_the_whole_group_via_arm_1`,
`input.rs`): added assertions for `` `mk inspect` `` and the re-mint
clause, GREEN immediately (the implementation was already correct — only
the assertion was missing).

**Command level** (`seating_vectors.rs`), three new tests, each measured
directly against the real binary before being written into Rust:

- `arm1_merged_v_ap_incomplete_reaches_the_command` — `v-ap-incomplete.txt`
  (complete 2-chunk class + incomplete 3-chunk class, one id) through
  `md descriptor`: piece-count evidence (`declares piece 1 of 2` /
  `declares piece 1 of 3`), `piece order does not matter`, `` `mk inspect` ``,
  and the re-mint clause.
- `arm2_incomplete_reaches_the_command` — the same single-string literal as
  `r5_incomplete_one_of_two_chunks_classifies_as_incomplete` (chunk-set
  `33333`): `should be 2` / `you supplied 1` / `scan the missing piece(s)`,
  and NOT arm 1's wording.
- `arm3_terminal_reaches_the_command` — the same two literals as
  `r5_terminal_cross_chunk_hash_mismatch_classifies_as_terminal`
  (chunk-set `22222`): human sentence (`do not form one key card`) before
  the `error:` line (`cross-chunk integrity hash mismatch`).

Named as the inheritors of the retired `v_collide_reaches_the_command`
arm-coverage per spec row 12, as instructed.

**Files:** `crates/md-cli/src/seat/input.rs:1137-1148`,
`crates/md-cli/tests/seating_vectors.rs:935-1040`.

---

## M1 — `--seat` no-such-card refusal lists bare ids

**Fix:** `directive.rs:174`, `cards.iter().map(|c| c.set_id.to_string())`
→ `cards.iter().map(DecodedCard::label)`.

**RED (new unit test, `V_COLLIDE` + a throwaway 1-slot policy):**

```
panicked: lists the ORDINAL label, not a bare duplicated id (M1):
seating refused: --seat @0: no supplied card has chunk-set id 00000.
The cards supplied are: 12345, 12345.
```

**GREEN after the fix**, labels now carry `#<k>`.

**Fallout, fixed:** `seating_vectors.rs`'s
`v_seat_bad_a_contradicting_seat_refuses_from_the_command_line` parses
the "are:" list and retries `--seat @0=<id>` with each entry — it broke
under the label change (labels now carry a trailing `(stub ...)` clause,
not a bare id). Fixed by taking only the leading token before the first
space (V_MIX has no collision, so the label's leading token is still a
valid bare id). Verified RED→GREEN:

```
FAIL (before fix): assertion `left == right` failed: exactly one of the
  two cards may sit in @0: [false, false]  (left: 0, right: 1)
PASS (after fix)
```

**Files:** `crates/md-cli/src/seat/directive.rs:173-178` (fix), `:417-452`
(new test); `crates/md-cli/tests/seating_vectors.rs:433-446` (parsing fix).

---

## M2 — out-of-range chunk indices — **NOT REPRODUCIBLE, NO CODE CHANGE**

Attempted the report's construction (6 garbage pieces at an out-of-range
`chunk_index` under an otherwise-legitimate 2-card class) directly against
`partition()`. Result was `Seated(2)` under the UNFIXED code, not
`CapExceeded` as the report predicted — because the garbage pieces never
reached `by_index` at all.

**Root cause, confirmed empirically with a throwaway probe:**
`canonical_piece_key`/`group_key_of` both resolve a chunked string through
`mk_codec::string_layer::StringLayerHeader::from_5bit_symbols`, which
itself **refuses** `chunk_index >= total_chunks`
(`vendor/mk-codec/src/string_layer/header.rs:160-163`, error text
`"chunked-header malformed: chunk_index = N >= total_chunks = M"`).
Probe output:

```
PROBE string=mk1qp5yqppfqurswpcy0lpmfx7denlx
PROBE canonical_piece_key result: false
PROBE err=seating refused: malformed mk1 string-layer header:
  chunked-header malformed: chunk_index = 9 >= total_chunks = 2
```

So no `Piece`/`ChunkInfo` with `chunk_index >= total_chunks` can ever be
constructed from any string — real or hand-crafted — that survives
string-layer decode. `partition`'s `k_class`/budget computations can
never see one, and `classify`'s own `out_of_range` flag in `input.rs`
(the thing the report cites as proof this is reachable) is dead for the
identical reason: `group_key_of` runs the same validated decode path. No
test anywhere in the suite has ever driven `out_of_range` true.

Per the dispatch instruction ("any finding you could NOT reproduce or fix
as described — report, don't improvise"), **no code change was made**. A
comment recording this trace was left at
`crates/md-cli/src/seat/partition.rs` (in the tests module, where the M2
test would otherwise have lived) so a future reader does not re-derive it.

---

## M3 — `partition()` silently drops an uncanonicalisable string

**Fix:** `partition.rs:127-129`, `continue` on `canonical_piece_key`
failure → `return Outcome::NoPartition`.

**RED (row 1's 4-string pair + one malformed literal, `"mk1notavalidstring"`):**

```
panicked: an uncanonicalisable string must refuse the group, not
silently drop it and seat on the remaining pieces (M3): got
Seated([KeyCard { ... }, KeyCard { ... }])
```

**GREEN after the fix.**

**Files:** `crates/md-cli/src/seat/partition.rs:127-136` (fix), `:543-552`
(test).

---

## M4 — §Security surplus overclaim

**Fix:** doc-only, `design/SPEC_seat_auto_partition.md`'s §Security bullet
corrected to state the guarantee's real scope (holds for a candidate
assembled from already-supplied pieces; an attacker injecting a wholly
new self-consistent piece raises k together with `|V|` and the class can
legitimately seat — not a new attack class, the pre-existing different-id
surplus exposure). Seat semantics untouched, per instruction.

**Pin, GREEN immediately** (already-shipped behaviour, same construction
as row 1): `row10d_an_injected_new_card_sharing_the_victim_id_raises_k_with_v_and_seats`
(`input.rs`), asserting `v-ap-canonical.txt`'s pair SEATS (2 cards, 1 AP1
note) under its corrected §Security name, so a reader of that section has
a named row rather than only prose.

**Files:** `design/SPEC_seat_auto_partition.md:137-148`,
`crates/md-cli/src/seat/input.rs:1189-1215`.

---

## M5 — order key silently defaults on `encode_bytecode` failure

**Fix:** extracted the SPEC §4 sort tail into `fn order_and_seat(seated_all:
Vec<KeyCard>) -> Outcome` (`partition.rs:225-244`); an `encode_bytecode`
error now returns `Outcome::NoPartition` instead of `.unwrap_or_default()`.

Report calls this unreachable through the real pipeline (every card
already round-tripped through `mk_codec::decode`, which enforces every
`encode_bytecode` invariant) — confirmed, so the separating test exercises
`order_and_seat` directly on a hand-built card with empty `policy_id_stubs`
(`encode_bytecode`'s own rejected shape).

**RED (before the fix — `order_and_seat` extracted first with the OLD
`.unwrap_or_default()` behaviour, as an intermediate, behaviour-preserving
refactor step):**

```
panicked: an order-key encode failure must refuse the group, not
silently default the sort key (M5) -- #<k> would become supply-order
dependent
```

**GREEN after the fix.**

**Files:** `crates/md-cli/src/seat/partition.rs:225-249` (extraction +
fix), `:739-775` (test).

---

## Gate results

- `cargo nextest run --locked -p md-cli`: **752 run, 752 passed, 1
  skipped** (baseline 742 + 10 new tests: 4 in `partition.rs` (I1×2, M3,
  M5), 2 in `input.rs` (I2's `row3b`, M4's `row10d`), 1 in `directive.rs`
  (M1), 3 in `seating_vectors.rs` (I3's arm1/2/3)). Count matches exactly.
- `cargo nextest run --locked --no-fail-fast --workspace`: **1219 run,
  1219 passed, 3 skipped** (baseline 1209 + 10, same arithmetic).
- `cargo fmt --check -p md-cli`: clean (after one `cargo fmt` pass over
  the new test bodies).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  (the exact CI invocation, `.github/workflows/ci.yml:65`): clean.

## Constraints honoured

- No design reopened: V=k, the cap, AP rulings, and the pre-pass placement
  are untouched. `mk-codec` untouched (only read, for the M2
  investigation). The shipped wording-pin test and the arms-2/3 unit rows
  (`r5_incomplete_...`, `r5_terminal_...`) are byte-unchanged.
- Nothing committed; worktree left dirty (`git status --porcelain` shows
  5 modified files, no new untracked files outside what's listed above).

## Findings NOT fixed as described

- **M2** — see above. Reproduction found the cited code path unreachable
  given the vendored `mk-codec`'s own string-layer validation; no fix
  applied.
