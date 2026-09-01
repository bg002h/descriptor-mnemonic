# R0 review — IMPLEMENTATION_PLAN_seat_auto_partition.md, round 2 (fold-check)

**Artifact:** `design/IMPLEMENTATION_PLAN_seat_auto_partition.md` @ `4bb3696b`
**Fold diff:** `git diff 7e5c566b..4bb3696b -- design/IMPLEMENTATION_PLAN_seat_auto_partition.md`
**Against:** `design/agent-reports/R0-seat-auto-partition-plan-r1.md` (0C/6I/8M/3N)
**Lens:** fold-check ONLY — does `4bb3696b` discharge each r1 finding, and did
the fold introduce a new defect or internal contradiction? NOT a fresh audit.
Spec design and plan-r1's "what the plan gets right" list are settled and were
not re-opened.

## Verdict

**0 Critical / 1 Important / 2 Minor — NOT GREEN.**

5 of 6 Importants and all 8 Minors + 3 Nits are fully discharged, several
verbatim-close to the remedy text. One Important (I3) is only **partially**
discharged: the fold closes the cap-inversion risk that was I3's actual
"Failing scenario" but drops two of I3's three remedy items — a mixed-totals
fixture with one incomplete class, and the row-7 sub-row split — leaving a
genuinely new, uncovered composition path with documented Critical lineage in
the spec's own history (`r1 C3`).

## Machine-checked before this review (do not re-derive)

| Claim | Result |
| --- | --- |
| `encode_bytecode`, `derive_chunk_set_id`, `decode_string`, `to_5bit_symbols`, `bytes_to_5bit`, `encode_5bit_to_string` | All six are `pub fn` in vendored `mk-codec` 0.5.0 (`vendor/mk-codec/src/{bytecode/encode.rs,string_layer/{chunk.rs,bch.rs,header.rs}}`) — the plan's P0 item 1 "four public calls" (`encode_bytecode`, `derive_chunk_set_id`, `decode_string`, "the string-layer encode entry") resolve to real, existing public API; not a drift from r1's measured chain (M1 check). |
| Existing fixture coverage for spec row 7's variant (iii) | **None found.** `grep -n "fn .*incomplete"` over `crates/md-cli/src/seat/*.rs` finds exactly `r5_incomplete_one_of_two_chunks_classifies_as_incomplete` (`input.rs:613`) — a SINGLE card, single class, `total_chunks = 2`. No existing or plan-listed fixture has **two different declared totals in one id group where one class is incomplete**. `v-mix.txt` is fingerprint-mixed (one fp-bearing/one fp-free), unrelated. |
| Old→new P1 step numbering | Old plan: step 1 = "§1 canonicalisation", step 2 = "§2 engine" (unchanged from r1's baseline). New plan: step 0 (new, M5) prepended; step 1 is STILL "§1 canonicalisation stage wired ... using P0's key fn"; step 2 is STILL "§2 engine". Confirms P0 item 2's "P1 step 2 wires the stage" is off by one against the plan's own step 1 text. |

## Disposition of plan-r1 findings

### I1 — two-layer row map: DISCHARGED

> "**RED at this step = ENGINE-UNIT rows** (plan-r1 I1): admissibility/k/cap/budget arithmetic and V=k decided directly on P0's fixtures, incl. the group-cap set." (P1 step 2)

> "**RED at this step = the END-TO-END rows 1, 3, 4 (floor + boundary), 5, 6, 7, 9, 10a** — the same behaviours re-asserted at command level so the pre-pass ordering is tested (plan-r1 I1: both layers are required)." (P1 step 3)

> "Gate: all 13 spec rows green at **BOTH layers where applicable**" (P1 gate)

Matches remedy option (b) exactly: engine-unit REDs at step 2, e2e reassertion of rows 3–7 (plus 1, 9, 10a) at step 3, and the gate requires both layers. Row 2 (§1, wired directly at step 1) is correctly left as a single e2e RED — it was never part of I1's engine/e2e split, since §1 wiring happens at step 1, not through the step-2/3 engine.

### I2 — row 10(c): DISCHARGED

> "RED: row 8, **row 10b AND row 10c** (plan-r1 I2), permutation invariance." (P1 step 4)

### I3 — group-wide cap: PARTIALLY DISCHARGED (residual Important — see New findings)

> "**group-cap set: 3+3 two-class, one id (plan-r1 I3 — without it the r3-I5 per-class-cap defect ships green)**" (P0 item 3)

> "the group-cap set's Σk = 6" (P0 item 5, shape test)

> "admissibility/k/cap/budget arithmetic and V=k decided directly on P0's fixtures, **incl. the group-cap set**" (P1 step 2)

This closes I3's actual "Failing scenario" — the per-class-vs-group-wide cap-inversion risk from r3-I5 is now unrepresentable-as-green, because Σk = 6 on the 3+3 fixture would fail both an engine-unit and (via row 7's step-3 reassertion) an e2e cap-refusal check.

But I3's remedy asked for three things and the fold does only the first:
1. "(ii) a 3+3 two-class same-id group ... → cap refusal" — **done**, above.
2. "(iii) a mixed-totals group with one class short a piece" — **not present** anywhere in P0's fixture list, P1's RED lists, or the shape tests.
3. "Split row 7 into three named sub-rows and assign each to a step" — **not done**; row 7 is still cited as one undifferentiated item at P1 step 3.

See New findings for why (2) is a live gap, not a discretionary miss.

### I4 — test-profile measurement + bounds: DISCHARGED

> "`PARTITION_DECODE_BOUND` fixed from a re-measurement ON THE TEST PROFILE, with acceptance bounds `177_147 ≤ BOUND < 531_441` (plan-r1 I4 — outside these, row 4's floor/boundary invert); constant comment records measured µs + profile." (P1 step 2)

Profile named, both bounds stated, comment requirement stated. (The remedy's extra procedural clauses — naming the floor fixture as the measurement input, and "if the ≤2s reading falls below 177,147 the target time moves, not the bound" — aren't restated verbatim, but the load-bearing acceptance-bounds gap I4 was actually about is closed.)

### I5 — grind script specification: DISCHARGED

> "**AP2 grind script (plan-r1 I5, specified):** a Rust test-support binary in this crate, deps = the already-vendored `bitcoin_hashes` ONLY (a new dep reddens `vendor-freshness`); implements r3's `[2,3,3]` construction, grinding STUB bytes never xpub bytes (r2); target runtime ≈ 16 s measured; output committed as the fixture with regeneration documented in the file header." (P0 item 4)

All five required elements present: language/runtime (Rust, no new dep), the `[2,3,3]` construction, the stub-bytes-not-xpub-bytes rule, the ~16 s figure, and committed-fixture-plus-regen-doc.

### I6 — sibling-mk generation + "single repo" correction: DISCHARGED

> "**Code changes are single-repo (descriptor-mnemonic); P0's FIXTURE GENERATION also runs the sibling mnemonic-key `mk` binary** (plan-r1 I6) — the shipped `tests/fixtures/seating/generate.sh` discipline: regeneration is a command, and the determinism guard is a clean `git diff` after re-run." (header)

Corrects the false "single repo, no other change" framing, names the sibling binary and the generator, and states the determinism guard. Item 3 also distinguishes generator per fixture (`generate.sh`/`mk` for the mint-able rows; "via the chunker" for row 5; the grind script for AP2) — satisfying I6's "chunker/grind fixtures name their own generator" clause. Residual noted below (gate line).

### M1–M8, N1–N3 — all DISCHARGED

- **M1**: hatch replaced by four named public calls (verified real — see table above); guard sentence "it constructs inputs; it is not an oracle" retained.
- **M2**: "`#[cfg(test)]` test-support inside `src/seat/` (re-exported for integration rows; `tests/common` is not importable from unit tests — plan-r1 M2)" — matches exactly, alternative dropped.
- **M3**: "authored ONCE as shipped code ... P0's shape tests call THE SHIPPED FUNCTION, never a second implementation" — matches; see New findings for the step-number citation attached to this same item.
- **M4**: "`DecodedCard` gains `ordinal: Option<u32>` (set in `decode_cards`; 1 construction site, `input.rs:366` — plan-r1 M4)" — matches exactly.
- **M5**: step 0 is exactly the mechanical signature change, moved to the front — matches exactly.
- **M6**: "`--seat` help at `main.rs:432/441/666/675`, `README.md:132`, `CHANGELOG.md`; man pages regenerate. Grep-assert scope: `crates/md-cli/src/`, `README.md`, `CHANGELOG.md`." — all four sites + scope statement present.
- **M7**: "run the mutated build under `timeout`, with a probe proving the mutated line executed; evidence = probe fired ∧ timeout expired; the mutation is never committed." — matches exactly.
- **M8**: "the floor row alone runs 177,147 real decodes ≈ 1.7–1.9 s on the test profile and runs on every suite invocation — a later slowdown there is attributable, not a mystery" — matches exactly, added to the Build gate section.
- **N1**: "`v-collide.txt` = same-id (12345) mixed-totals fixture (spec rows **7 and 10b** — plan-r1 N1)" — row 7 now cited alongside 10b.
- **N2**: "this pair with a matching 2-slot template is ALSO row 12's 'new minimal 2-slot fixture' for `v_collide_reaches_the_command` (plan-r1 N2)" — states which mint it is.
- **N3**: "the note into `Seating.notes` with **per-GROUP interleaving**: the note precedes ITS OWN group's R2 warnings (plan-r1 N3 — not a global prepend)" — resolves the ambiguity explicitly.

## New findings

### Important — I3's residual: spec row 7's third variant (mixed totals, one class incomplete) has zero fixture or test at either layer

Spec Acceptance row 7: *"mixed-totals rows: both classes complete → both seat (group cap applies across classes ...); **one class incomplete → whole group refuses via arm 1, nothing seats.**"* This third outcome exercises §2.6's composition rule — *"Composed fail-closed across classes (**r1 C3**): every class must seat or the whole group fails"* — a rule whose citation (`r1 C3`) marks it as a former **Critical** finding in the spec's own review history.

Nothing in the fold builds this scenario:
- P0 item 3's fixture list (canonical pair, BCH twins, shared-piece pair, floor/boundary, over-budget, group-cap set, AP2) has no fixture with **two different declared `total_chunks` in one id group where one class is incomplete**.
- P1 step 3's e2e RED list cites "row 7" as a single undifferentiated item, backed only by `v-collide.txt` (variant i, both-seat) and the group-cap set (variant ii, cap refusal).
- No pre-existing fixture covers it either — the only "incomplete" test in the codebase (`r5_incomplete_one_of_two_chunks_classifies_as_incomplete`, `input.rs:613`) is a single card in a single class, not a two-class group.

**Failing scenario.** The new partition pre-pass is genuinely new composition logic (no prior code path split a same-id group by declared total before deciding class-by-class success). If an implementation bug causes the pre-pass to seat the complete class alone while silently dropping the incomplete one, or to mis-route to a wrong message, no fixture in P0 or P1 can catch it — the exact "ships green because the separating input was never minted" pattern I3 already named for the cap defect, now reopened one variant later.

**Remedy.** Add a mixed-totals fixture with one class short a piece (e.g., a 3-chunk complete class + a 2-chunk class missing one index, same id) to P0's list; add its shape test; cite it explicitly as row 7(iii) at P1 step 3's e2e list (and, if desired, split row 7's citation into (i)/(ii)/(iii) throughout, per I3's original remedy item 3 — optional now that (ii) is separately named via the group-cap set, but still removes the ambiguity of a bare "7").

### Minor — P0 item 2 cites the wrong P1 step number for when the canonical-key function gets wired

> "The §1 canonical-key function — authored ONCE as shipped code in `src/seat/` (**unused by production until P1 step 2** wires the stage)" (P0 item 2)

But P1 step 1's own text is unambiguous: *"§1 canonicalisation stage wired after `dedupe_strings` using P0's key fn."* The function P0 item 2 is describing is wired at **step 1**, not step 2 (step 2 is the separate `partition.rs` engine). This reads as a renumbering artifact from prepending step 0 — the old plan's "§1 canonicalisation" was already step 1 before the fold, so the "step 2" reference in P0 item 2 was never correct, old or new. Low-consequence (P1 step 1's own text resolves it for a careful reader) but worth a one-word fix.

### Minor — I6's "add to P0's gate" clause not literally in the Gate line

I6's remedy said: *"add 'generate.sh re-run leaves git diff clean' to P0's gate."* The discipline is stated twice — in the plan header and in P0 item 3 ("regen = command + clean diff") — but P0's own `Gate:` line still reads only *"shape tests green; suite green; fmt/clippy clean"* with no explicit generate.sh/clean-diff criterion. The false-scoping defect I6 was actually about (implementer not knowing a sibling repo/binary is needed) is fixed; this is a residual completeness gap in the gate's own checklist, not a comprehension risk.

## No orphaned or double-assigned rows found otherwise

Walked all 13 spec Acceptance rows against every RED/gate citation in P1 steps 0–6: rows 1, 2, 3, 4, 5, 6, 8, 9, 10a, 10b, 10c, 11, 12, 13 each resolve to at least one step with no contradictory duplicate assignment; row 7 is the sole exception (cited, but only 2 of its 3 variants are backed — see the Important above). Step 0 is purely mechanical (no RED rows cited, none needed) and does not depend on values only later steps produce; `ordinal: Option<u32>` defaulting to `None` until §4 (step 4) computes real values is coherent with the type and requires no special-casing.

## What GREEN requires

Fold the Important (row 7 variant iii — fixture + citation) and, optionally, the two Minors (step-number citation fix; gate-line completeness), then re-review scoped to *"did this fold add the fixture and close the composition-path gap, and did it introduce anything new"* — not a fresh audit. Everything else in this report is settled and should not be re-derived.
