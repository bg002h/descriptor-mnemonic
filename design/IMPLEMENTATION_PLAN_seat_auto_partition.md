# IMPLEMENTATION PLAN — seat auto-partition

**Status: DRAFT for R0 r2.** Implements `design/SPEC_seat_auto_partition.md`
@ `230661b6` (GREEN, five R0 rounds); folded once against
`design/agent-reports/R0-seat-auto-partition-plan-r1.md` (0C/6I/8M/3N).
TDD, two phases, one implementer each, worktree-isolated. **Code changes
are single-repo (descriptor-mnemonic); P0's FIXTURE GENERATION also runs
the sibling mnemonic-key `mk` binary** (plan-r1 I6) — the shipped
`tests/fixtures/seating/generate.sh` discipline: regeneration is a
command, and the determinism guard is a clean `git diff` after re-run.

**Baseline:** descriptor-mnemonic `230661b6`; seat path last changed at
`54ab1cd6` — re-confirm `git log 54ab1cd6..HEAD -- crates/md-cli/src/seat/`
shows only design docs before dispatching P0.

## Machine-verified facts (do not re-derive)

- Shipped classifier/warning/wording-pin: `seat/input.rs` @ `54ab1cd6`.
- `decode_cards`: 1 production + 22 test call sites (all
  `#[cfg(test)] mod tests` inside `src/seat/*.rs`).
- Candidate decode: 7.845 µs release; **9.8–10.7 µs on the
  `opt-level = 2` TEST profile the gate runs** (plan-r1 I4).
- The synthetic chunker is fully expressible on mk-codec 0.5.0's PUBLIC
  API (plan-r1 built and ran it for n = 7/11/12/21/32; `ChunkFragment`'s
  `#[non_exhaustive]` blocks nothing). The four public calls are named in
  P0 item 1 — no local mirroring of codec internals (plan-r1 M1).
- 2^32 SHA-256 measured at **15.8 s** on this box (272 MH/s, 24 cores) —
  the AP2 grind is cheap to REGENERATE, not only to commit (plan-r1 I5).
- `v-collide.txt` = same-id (12345) mixed-totals fixture (spec rows 7
  and 10b — plan-r1 N1); the `44444` fixture = missing-index (row 6).
- `--seat` help text lives at `main.rs:432/441` AND `main.rs:666/675`
  (two subcommands); man pages generate from it (`clap_mangen`,
  `man-pages` workflow); `README.md:132` + `CHANGELOG.md` describe the
  grammar (plan-r1 M6).

## Build gate

Per-phase `cargo nextest run --locked -p md-cli` + `cargo fmt --check` +
`cargo clippy --all-targets`. Timing note: the floor row alone runs
177,147 real decodes ≈ 1.7–1.9 s on the test profile and runs on every
suite invocation — a later slowdown there is attributable, not a mystery
(plan-r1 M8).

## P0 — fixtures, helpers, and the canonical key

1. **Synthetic chunker** — `#[cfg(test)]` test-support inside
   `src/seat/` (re-exported for integration rows; `tests/common` is not
   importable from unit tests — plan-r1 M2), built ONLY on mk-codec
   public API: `encode_bytecode`, `derive_chunk_set_id`,
   `decode_string`, and the string-layer encode entry plan-r1 verified.
   It constructs inputs; it is not an oracle.
2. **The §1 canonical-key function** — authored ONCE as shipped code in
   `src/seat/` (unused by production until P1 step 1 wires the stage);
   P0's shape tests call THE SHIPPED FUNCTION, never a second
   implementation (plan-r1 M3).
3. **Fixtures** (generated via generate.sh + the sibling `mk`; each file
   carries provenance; regen = command + clean diff):
   canonical 2×2 pinned pair + unpinned-twin control (row 1) — this pair
   with a matching 2-slot template is ALSO row 12's "new minimal 2-slot
   fixture" for `v_collide_reaches_the_command` (plan-r1 N2); BCH twins
   (row 2); shared-piece pair, ≥13 shared stubs (row 3); floor n=11 +
   boundary n=12 sets, distinct stubs (row 4); over-budget 5^32-scale
   header set via the chunker (row 5); **group-cap set: 3+3 two-class,
   one id (plan-r1 I3 — without it the r3-I5 per-class-cap defect ships
   green)**; **incomplete-class set: one complete 2-chunk card + a
   3-chunk card missing one piece, one id (plan-r2 I3 residue — the
   r1-C3 fail-closed composition rule's separating input: the whole
   group must refuse via arm 1, nothing seats, no pieces dropped)**;
   the AP2 ground fixture + its committed generation script.
4. **AP2 grind script (plan-r1 I5, specified):** a Rust test-support
   binary in this crate, deps = the already-vendored `bitcoin_hashes`
   ONLY (a new dep reddens `vendor-freshness`); implements r3's
   `[2,3,3]` construction, grinding STUB bytes never xpub bytes (r2);
   target runtime ≈ 16 s measured; output committed as the fixture with
   regeneration documented in the file header.
5. **Shape tests** (RED→GREEN): chunk-0 identity for row 3; per-index
   distinct-canonical-piece counts and products for rows 4/5 VIA the
   shipped key fn; the group-cap set's Σk = 6; the AP2 fixture's extra
   candidate verifies under `mk_codec::decode`.
Gate: shape tests green; suite green; fmt/clippy clean; **fixture
regeneration re-run yields a clean `git diff` (the determinism guard,
in the gate itself — plan-r2)**.

## P1 — the feature (steps ordered so every test is written once)

0. **`decode_cards` signature first (plan-r1 M5):** the mechanical
   1 + 22-site change — return gains the AP1-note value and
   `DecodedCard` gains `ordinal: Option<u32>` (set in `decode_cards`;
   1 construction site, `input.rs:366` — plan-r1 M4). One reviewable
   mechanical commit; all later tests target the final signature.
1. **§1 canonicalisation stage** wired after `dedupe_strings` using
   P0's key fn. RED: row 2 e2e.
2. **§2 engine** (`seat/partition.rs`): class split; admissibility +
   k_class; **group-wide cap Σk ≤ 5**; static saturating budget;
   enumerate/verify; `|V| = k` + cover. `PARTITION_DECODE_BOUND` fixed
   from a re-measurement ON THE TEST PROFILE, with acceptance bounds
   `177_147 ≤ BOUND < 531_441` (plan-r1 I4 — outside these, row 4's
   floor/boundary invert); constant comment records measured µs +
   profile. **RED at this step = ENGINE-UNIT rows** (plan-r1 I1):
   admissibility/k/cap/budget arithmetic and V=k decided directly on
   P0's fixtures, incl. the group-cap set.
3. **§3 outcomes wiring** — pre-pass before the classifier; arm-1
   entry; AP2 refusal; the note into `Seating.notes` with **per-GROUP
   interleaving: the note precedes ITS OWN group's R2 warnings**
   (plan-r1 N3 — not a global prepend). **RED at this step = the
   END-TO-END rows 1, 3, 4 (floor + boundary), 5, 6, 7 (BOTH sub-rows:
   7a both-classes-complete seats; 7b one-class-incomplete refuses via
   arm 1 on the P0 incomplete-class set — plan-r2), 9, 10a** —
   the same behaviours re-asserted at command level so the pre-pass
   ordering is tested (plan-r1 I1: both layers are required).
4. **§4 identity** — order key; `#<k>` via the ordinal; `--seat`
   grammar + enumerated refusals; tie-break extension. RED: row 8,
   **row 10b AND row 10c** (plan-r1 I2), permutation invariance.
5. **Message/doc churn** (spec row 12 + plan-r1 M6): REMEDIES,
   directive::parse, the three doc invariants, dedupe doc, `--seat`
   help at `main.rs:432/441/666/675`, `README.md:132`, `CHANGELOG.md`;
   man pages regenerate. Grep-assert scope: `crates/md-cli/src/`,
   `README.md`, `CHANGELOG.md`.
6. **Mutation gates** (row 11). The skip-the-budget hang gate's method
   (plan-r1 M7): run the mutated build under `timeout`, with a probe
   proving the mutated line executed; evidence = probe fired ∧ timeout
   expired; the mutation is never committed.
Gate: all 13 spec rows green at BOTH layers where applicable; shipped
wording-pin + arms-2/3 rows untouched; fmt/clippy clean.

## Post-implementation (mandatory)

Whole-diff adversarial review (opus) over the branch; then the staging
push ritual (required `cargo test (ubuntu-latest)` + `cargo clippy`,
whole run green incl. fmt and vendor-freshness).

## Out of scope

Everything the spec names; any mk-codec change (the chunker needs none —
measured); new dependencies (vendor-freshness).
