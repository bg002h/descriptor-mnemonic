# IMPLEMENTATION PLAN — seat auto-partition

**Status: DRAFT for R0.** Implements `design/SPEC_seat_auto_partition.md`
@ `230661b6` (GREEN 0C/0I after five R0 rounds). TDD, two phases, one
implementer each, worktree-isolated. Single repo (descriptor-mnemonic,
`crates/md-cli`); no mk-codec/md-codec change.

**Baseline (staleness anchor — re-validate before each dispatch):**
descriptor-mnemonic `230661b6`; the seat path last changed at `54ab1cd6`
(the shipped csid cycle) — re-confirm `git log 54ab1cd6..HEAD --
crates/md-cli/src/seat/` shows only this cycle's design docs before
dispatching P0.

## Machine-verified facts the plan rests on (do not re-derive)

- Shipped four-arm classifier + `chunk_set_id_mismatch_warning` +
  wording-pin test: `crates/md-cli/src/seat/input.rs` @ `54ab1cd6`.
- `decode_cards` has 1 production call site (`seat/mod.rs:143`) and 22
  test call sites (input.rs ×13, complete.rs ×3, matching.rs ×3,
  disposition.rs ×2, satisfy.rs ×1) — measured, spec row 12.
- Candidate decode cost 7.845 µs (r3 measured); `mk_codec::decode` is
  the oracle; `mk_codec::bytecode::encode_bytecode` is the order key —
  both public in the pinned mk-codec 0.5.0.
- `v-collide.txt` = same-id (12345) mixed-totals fixture; the `44444`
  fixture = missing-index shape (spec rows 6, 10b).
- mk encode cannot mint n = 32 (255-stub cap → n = 21); the over-budget
  fixture needs the committed synthetic chunker (spec row 5).

## Build gate

No extractable algorithm blocks; the gate is per-phase
`cargo nextest run --locked -p md-cli` + `cargo fmt --check` +
`cargo clippy --all-targets` (fmt is in the phase gate, not the push —
standing lesson). State this in review briefs.

## P0 — fixtures and helpers (foundation; everything in P1 asserts on it)

Deliverables, TDD where testable:
1. **Synthetic chunker helper** (test-support, `#[cfg(test)]` or a
   `tests/common` module): split an arbitrary bytecode into n chunks +
   trailing hash and emit mk1 strings (re-uses `mk_codec` public
   encode/BCH surface where possible; where the codec does not expose a
   piece, the helper lives in TESTS ONLY and carries a comment naming
   what it mirrors — it constructs inputs, it is not an oracle).
2. **Fixtures**: the canonical 2×2 pinned pair + unpinned-twin control
   (spec row 1); BCH-twin strings (row 2, ≤4 flips per piece); the
   shared-piece pair (row 3, ≥13 shared stubs — mint with mk, verify
   chunk-0 identity); floor/boundary sets (row 4: 3 cards distinct stubs
   at n=11 and n=12); the over-budget header set (row 5, 5^32-scale);
   the AP2 ground fixture + its committed one-grind generation script
   (row 9 — the script is committed and documented; the fixture is
   committed; regeneration is a command). Each fixture file carries its
   generation provenance.
3. **Reader assertions**: a test proving each fixture has the shape its
   row claims (chunk-0 identity for row 3; product arithmetic for
   rows 4/5; the AP2 fixture's extra candidate actually verifies).
Gate: fixtures' shape tests RED→GREEN; suite green; fmt/clippy clean.

## P1 — the feature

Order inside the phase (TDD per step):
1. **§1 canonicalisation** — new mk1-only stage after `dedupe_strings`
   (documented adjacent, not inside); collapse on the 5-bit symbol-tail
   key; first appearance survives. RED: row 2 (BCH twins seat as one).
2. **§2 engine** (`seat/partition.rs`, new): total-class split;
   admissibility + k_class; group cap Σk ≤ 5; static saturating budget
   `PARTITION_DECODE_BOUND` (constant fixed HERE from a re-run of the
   timing measurement on this machine, target ≤ ~2 s worst case,
   expected ≈ 255k; record the measured µs in the constant's comment);
   candidate enumeration + verify; `|V| = k` + cover seat condition.
   RED: rows 3, 4 (floor + boundary), 5, 6, 7.
3. **§3 outcomes wiring** — pre-pass before the shipped classifier;
   arm-1 entry on no-partition; AP2 refusal; AP1 note as a VALUE:
   `decode_cards` return gains the note (signature change; update the
   1+22 call sites), carried into `Seating.notes` ahead of R2 warnings.
   RED: rows 1 (note+warnings order), 9 (AP2 fixture refuses), 10a.
4. **§4 identity** — order key, `#<k>` labels in `label()`, `--seat`
   grammar + its enumerated refusals, tie-break key extension. RED:
   row 8, row 10b (v-collide seats then leftover with distinguishable
   labels), permutation invariance.
5. **Message/doc churn** per spec row 12 (REMEDIES, directive::parse,
   three doc invariants, dedupe doc) — grep-assert no stale wording.
6. **Mutation gates** (row 11), each with mutated-line-RAN evidence.
Gate: full suite green (`-p md-cli`), fmt/clippy clean, all 13 spec
rows implemented and green, the shipped wording-pin + arms-2/3 rows
untouched.

## Post-implementation (mandatory, non-deferrable)

Whole-diff adversarial review (opus) over the full branch diff —
funds-adjacent normative change; then integrate via the repo's staging
push ritual (required checks `cargo test (ubuntu-latest)` + `cargo
clippy`, whole run green incl. fmt).

## Out of scope

Everything the spec's Out-of-scope names; any change to mk-codec even
if the synthetic chunker would be nicer with one (test-side only).
