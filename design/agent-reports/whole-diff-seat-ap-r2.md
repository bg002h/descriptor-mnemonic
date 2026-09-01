# Scoped re-review — seat auto-partition fold (round 2)

**Scope:** did commits `64e2f108` (code) + `7dbc471e` (spec wording) discharge
each finding of `design/agent-reports/whole-diff-seat-ap-review.md`, and did
the fold introduce a new defect? NOT a fresh audit — the whole-diff review's
"verified clean" section (§2.5 identity, evaluation order, fail-closed
composition, budget constant/guard, V-ORD, `--seat #k` binding, etc.) is
accepted as already measured and was not re-derived.

**VERDICT: 0 Critical / 0 Important — the implementation is GREEN. Closes
the loop.**

Diff audited: `git -C /scratch/code/shibboleth/dm-worktrees/seat-ap diff
c2529d13..7dbc471e` (580 insertions / 34 deletions, 5 files — matches the
fold report's file list exactly). Worktree confirmed clean before and after
every experiment (`git status --porcelain` empty).

---

## Dispositions

### I1 — Σ vs Π budget — **FIXED**

`partition.rs:190-196`, `fold(1u64, |acc, c| ... acc.saturating_mul(...))` →
`fold(0u64, |acc, c| ... acc.saturating_add(class_product))` — sum across
classes, product within a class's own indices. Matches SPEC §2.4 and the
constant's own doc comment.

Ran both new engine-unit rows directly:

```
PASS seat::partition::tests::budget_sums_across_classes_not_products_i1_separating_row
PASS seat::partition::tests::sum_still_refuses_a_genuinely_over_budget_two_class_group_i1
```

First row: real, independently-decodable cards (`real_card_at_exact_chunk_count`,
2×4 + 3×11 under one id) — `Outcome::Seated(5)`, all 5 expected cards present.
Second row: 2×12 + 3×13 — `OverBudget { product: 2⁴¹² + 3¹³ = 1,598,419 }`
(arithmetic: `2u64.pow(12) + 3u64.pow(13) = 4096 + 1,594,323 = 1,598,419 >
200,000`), and the test asserts `DECODE_CALLS == 0`. Both pass, confirming the
fix bounds decodes as an upper bound (no wrong seat either direction).

### I2 — vacuous `n_distinct` — **FIXED**

No code change (the counting logic was already correct — the corpus never
separated the two counts). Re-applied the report's exact mutation via `sed`
at `input.rs:499` (`ap1_note(n_supplied, n_distinct, k, set_id)` →
`ap1_note(n_supplied, n_supplied, k, set_id)`) and ran the two affected
tests:

```
FAIL row3b_note_separates_supplied_from_distinct_when_a_bch_twin_duplicates_a_supplied_piece_i2
  left:  "...5 supplied strings are 5 distinct pieces..." (mutated)
  right: "...5 supplied strings are 4 distinct pieces..." (expected)
PASS row3_shared_piece_pair_seats_two_cards_via_reuse   (unaffected, as predicted)
```

Reverted the mutation (`sed` back); `git status --porcelain` confirmed empty
— worktree byte-identical to `7dbc471e` after the experiment. `row3b`'s
separating fixture (row 1's 4-string pair + one BCH-correctable twin of
card 0 chunk 0) genuinely forces `n_supplied=5 ≠ n_distinct=4`; row 1's own
`row3` was upgraded to exact-text assertion alongside it.

### I3 — arm 1/2/3 coverage lost — **FIXED**

Unit level: `row7b_incomplete_class_set_refuses_the_whole_group_via_arm_1`
gained `assert!(msg.contains("`mk inspect`"))` and the re-mint clause
assertion. Command level: three new tests in `seating_vectors.rs`
(`arm1_merged_v_ap_incomplete_reaches_the_command`,
`arm2_incomplete_reaches_the_command`, `arm3_terminal_reaches_the_command`),
each driving the real `md descriptor` binary. `arm1_merged_...` explicitly
asserts `e.contains("`mk inspect`"))` **at command level** — the exact
clause the review found pinned nowhere. Ran all four:

```
PASS seat::input::tests::row7b_incomplete_class_set_refuses_the_whole_group_via_arm_1
PASS md-cli::seating_vectors arm1_merged_v_ap_incomplete_reaches_the_command
PASS md-cli::seating_vectors arm2_incomplete_reaches_the_command
PASS md-cli::seating_vectors arm3_terminal_reaches_the_command
```

### M2 — out-of-range chunk indices — **ACCEPTED-UNREPRODUCIBLE**

Fold's claim: `canonical_piece_key`/`group_key_of` resolve through
`mk_codec::string_layer::StringLayerHeader::from_5bit_symbols`, which itself
refuses `chunk_index >= total_chunks` before a `Piece` can ever be built, so
no by_index entry with an out-of-range index can reach `partition`'s
`k_class`/budget computation. Verified the cited refusal is real:

```
vendor/mk-codec/src/string_layer/header.rs:160:  if chunk_index >= total_chunks {
vendor/mk-codec/src/string_layer/header.rs:162:      "chunk_index = {chunk_index} >= total_chunks = {total_chunks}"
vendor/mk-codec/src/string_layer/header.rs:302:  fn parse_rejects_chunk_index_at_or_above_total_chunks()
```

A dedicated vendor test already pins this exact refusal. The ruling holds —
no code change was warranted and none was made.

### M1 — `--seat` refusal lists bare ids — **FIXED**

`directive.rs:174`, `cards.iter().map(|c| c.set_id.to_string())` →
`cards.iter().map(DecodedCard::label)`. New unit test (`v_seat_unk_no_such
_card_refusal_lists_ordinal_labels_when_collided_cards_exist_m1`) and the
`seating_vectors.rs` fallout fix (parsing only the leading token of each
`label()` entry, since labels now carry a trailing `(stub ...)` clause) both
ran green. Checked for other consumers of the old bare-id format: only two
call sites reference `set_id.to_string()` in test assertions
(`directive.rs:380,411`) — both are non-collided fixtures (`V_MIX`, `V_USP`),
and `label()` (`input.rs:104-115`) always begins with the bare `set_id`
(`"{set_id}#{k} (stub ...)"` or `"{set_id} (stub ...)"`), so
`msg.contains(&card.set_id.to_string())` remains true as a substring match —
neither test needed changing, and neither broke (both are in the 752-count
baseline+delta and passed). No other consumer of the old format exists in
`crates/` or `README.md`.

### M3 — silent drop on uncanonicalisable string — **FIXED**

`partition.rs:127-136`, `continue` → `return Outcome::NoPartition`. New test
`partition_refuses_rather_than_silently_drop_an_uncanonicalisable_string_m3`
ran green.

### M4 — §Security overclaim — **FIXED (doc-only, as instructed)**

`design/SPEC_seat_auto_partition.md:137-148` corrected to state the
guarantee holds only for a candidate assembled from already-supplied pieces;
an injected new piece raises k together with `|V|` and legitimately seats.
New pin `row10d_an_injected_new_card_sharing_the_victim_id_raises_k_with_v_
and_seats` ran green (2 cards, 1 note — SEATS, as the corrected text says).
Seat semantics untouched — confirmed no non-doc lines changed in this hunk.

### M5 — order-key degrades on encode failure — **FIXED**

`partition.rs:225-249`: SPEC §4 sort tail extracted into `order_and_seat`;
`encode_bytecode` failure now returns `Outcome::NoPartition` instead of
`.unwrap_or_default()`. New test
`order_key_failure_refuses_rather_than_default_the_sort_key_m5` ran green.

**New-defect check (M5), traced not just asserted:** `order_and_seat` is
only ever called from `partition()`'s tail with `seated_all`, which is built
exclusively by `verify_class` → `ClassVerdict::Seats(distinct_cards)`
(`partition.rs:286-292`) — and `distinct_cards` is populated only from
`verify_candidate` (`partition.rs:103-107`), whose entire body is
`mk_codec::decode(refs).ok()`. So every `KeyCard` reaching `order_and_seat`
through the real pipeline already round-tripped through `mk_codec::decode`.
Read `vendor/mk-codec/src/bytecode/encode.rs:24-90`: `encode_bytecode`'s
preconditions (non-empty `policy_id_stubs`, `component_count ≤
MAX_PATH_COMPONENTS`, `xpub.depth`/`child_number` agreeing with the path) are
exactly the invariants a card produced by the paired decoder must already
satisfy for the encode/decode round trip to be sound — the same
encode/decode-symmetry argument this codebase already relies on elsewhere
(see the comment at `encode.rs:24-32` about the encoder/decoder previously
disagreeing on `MAX_PATH_COMPONENTS`). So the new `Err(_) => NoPartition` arm
is unreachable for any card that reached `order_and_seat` via
`partition()`'s real call path — confirmed by tracing the call graph, not by
re-asserting the fold's claim. **No new reachable refusal path for any
previously-seating input.** (The unit test exercises `order_and_seat`
directly with a hand-built invariant-violating card, bypassing the pipeline
entirely, which is the only way to reach the new arm at all.)

### N1/N2/N3 — Nits — **NOT FIXED, not explicitly dispositioned by the fold**

None of the three doc/coverage Nits from the review were touched:

- **N1** (`input.rs:380`'s `"no `note: ` prefix"` doc comment is still false
  — `ap1_note` still emits text beginning `"note: these …"`, unchanged by
  the diff).
- **N2** (`dedupe_strings`'s own doc block, `input.rs:118-135`, still has no
  pointer to the §1 canonicalisation stage — unchanged).
- **N3** (`row8_ordinal_assignment_is_invariant_under_supply_order` still
  tests 2 of 24 permutations — unchanged; the review already noted this is
  not a false PASS, only weaker than free).

The fold report's summary line ("every finding below is addressed except
M2") is accurate only for the findings it enumerates in its own body (I1-I3,
M1-M5) — it never mentions N1-N3 at all, so there is no explicit
accept/defer ruling on them anywhere. Per this repo's severity policy, Nits
never gate a fold or a phase, so this does not block closure — flagging it
only because the brief asked for an explicit disposition and none exists in
writing. Recommend a one-line follow-up entry if these are meant to be
carried forward.

---

## New-defect sweep beyond M5

Read the full diff hunk-by-hunk (5 files, 580/34). Beyond M5's traced
call-graph check:

- `directive.rs`'s M1 fix and its fallout repair in `seating_vectors.rs`
  are the only other behavior-affecting change outside the I1/I2/I3/M3/M5
  fixes already covered above — no other call site depends on the old
  bare-id rendering (swept above).
- `partition.rs`'s I1 fix touches only the `product` fold; the cap check
  at `:168-170` (Σk) and the enumerate/verify loop are byte-unchanged, so
  evaluation order (cap → budget → enumerate) proved clean by the whole-diff
  review still holds — nothing in the diff touches that ordering.
- All new test-only additions (I1's two rows, I2's `row3b`, I3's three
  command rows, M1/M3/M4/M5's rows) are additive; no existing test body was
  weakened, only `row3`'s assertion was strengthened (`contains` →
  exact-text).

No new defect found.

---

## Machine-checked, independently

- `cargo nextest run --locked -p md-cli`: **752 run, 752 passed, 1 skipped**
  (measured directly, matches the fold report's count exactly).
- `cargo nextest run --locked --no-fail-fast --workspace`: **1219 run, 1219
  passed, 3 skipped** (baseline 1209 + 10, matches).
- Exactly **one** `#[ignore]` test in the touched surface:
  `partition.rs:369`, `#[ignore = "timing measurement, run explicitly to
  re-derive PARTITION_DECODE_BOUND"]` — confirmed by grepping for the
  attribute itself (not doc-comment mentions, of which there are two).
- `cargo fmt --check -p md-cli`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  (verified this is CI's own invocation: `.github/workflows/ci.yml:65`):
  clean, zero warnings.
- Ran all 13 fold-added/modified tests by name filter directly: 13/13 pass.
- I2 mutation re-applied and reverted; `git status --porcelain` empty
  after — worktree left exactly as found.

## Recommended gate action

0 Critical / 0 Important. **Closes the implementation loop.** N1-N3 remain
open as ownerless Nits (doc/coverage polish) — file a follow-up if they
should be carried forward; they do not block.
