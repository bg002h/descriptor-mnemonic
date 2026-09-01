# P0 implementation — seat auto-partition: fixtures, helpers, canonical key

**Worktree:** `/scratch/code/shibboleth/dm-worktrees/seat-ap`, branch
`impl/seat-auto-partition`, base `a20c17c0` (unchanged — worktree left dirty,
nothing committed per instruction).

**Scope:** `design/IMPLEMENTATION_PLAN_seat_auto_partition.md` P0 (items 1-5 +
Gate) only. No P1 code (no partition engine, no `seat::run` wiring change).

## Files + lines (new/changed, worktree-relative)

| File | Lines | What |
| --- | --- | --- |
| `crates/md-cli/src/seat/canonical.rs` | 163 | NEW — item 2: `CanonicalPieceKey` + `canonical_piece_key()`, shipped `pub(crate)`, `#[allow(dead_code)]` (unused by production until P1 wires it) + 5 unit tests |
| `crates/md-cli/src/seat/synth.rs` | 222 | NEW — item 1: `#[cfg(test)] pub(crate) mod synth`: `synth_string`/`synth_fragment`/`synth_card_strings` + 5 unit tests |
| `crates/md-cli/src/seat/p0_shapes.rs` | 352 | NEW — item 5: 7 shape tests, all via `canonical_piece_key` |
| `crates/md-cli/src/bin/gen_ap2_grind.rs` | 320 | NEW — item 4: the AP2 grind binary (`[[bin]]`, auto-discovered from `src/bin/`) |
| `crates/md-cli/src/seat/mod.rs` | +5 | Registers `canonical`, `synth` (`#[cfg(test)]`), `p0_shapes` (`#[cfg(test)]`) |
| `crates/md-cli/tests/fixtures/seating/generate.sh` | +191 | Item 3: 8 new fixture blocks (mintable ones) |
| `crates/md-cli/tests/fixtures/seating/v-ap-*.txt` (8 files) | — | NEW — item 3 fixtures, minted via `mk` 0.13.0 on PATH |
| `crates/md-cli/tests/fixtures/seating/v-ap2.txt` | 9 mk1 lines / 1620 bytes | NEW — item 3/4: the AP2 ground fixture, committed output of the grind |

No `Cargo.toml`/`Cargo.lock` changes anywhere in the workspace (verified via
`git diff --stat` — empty) — the grind uses `bitcoin::hashes::sha256`,
already reachable through the existing `bitcoin = { workspace = true }`
dependency, so no new dependency was added (vendor-freshness untouched).

## Item 1 — synthetic chunker (`seat/synth.rs`)

Built ONLY on `encode_bytecode`, `derive_chunk_set_id`, `decode_string`, and
`encode_5bit_to_string` (the string-layer encode entry) — no `ChunkFragment`
construction (confirmed non-constructible externally, matching r3-I1 P-d).
`#[cfg(test)] pub(crate)`, reachable from every `#[cfg(test)] mod tests`
across `src/seat/*.rs` via `crate::seat::synth::…` (interpretation note below).
Self-tests round-trip a REAL card through the chunker's primitive at n=7 and
n=21 (mk-codec's own 255-stub/21-chunk ceiling) via `mk_codec::decode`, and
assert 5 synthetic cards give 5 distinct pieces at every one of 32 indices —
the property SPEC row 5 depends on.

**Interpretation note (non-blocking):** this crate is bin-only (no
`lib.rs`), so `tests/*.rs` integration-test binaries cannot import
`crate::seat` items at all (only `assert_cmd`-drive the compiled `md`
binary) — "re-exported for integration tests" is read here as "reachable
from every `#[cfg(test)] mod` across the `seat/` module tree", which is what
`p0_shapes.rs` (a sibling file, not `input.rs`'s own `mod tests`) exercises
via `super::synth::synth_card_strings(..)`.

## Item 2 — the §1 canonical-key function (`seat/canonical.rs`)

`pub(crate) fn canonical_piece_key(s: &str) -> Result<CanonicalPieceKey, CliError>`,
four fields `(chunk_set_id, total_chunks, chunk_index, symbol_tail: Vec<u8>)`,
`derive(PartialEq, Eq, Hash)` so identity is structural, matching SPEC §1
exactly. `symbol_tail` is `decoded.data()[consumed..]` — the 5-bit symbols,
never re-derived bytes (`five_bit_to_bytes` is not called on this path, per
SPEC §1 / NEW-M4). `SingleString` and any future non-exhaustive variant are
refused by name. `#[allow(dead_code)]` because P0 does not wire it into
`seat::run` — P1 step 1 removes the allow when it does.

## Item 3 — fixtures

All 8 mintable fixtures generated via `generate.sh` + the sibling `mk` 0.13.0
binary on PATH (measured against vendored mk-codec 0.5.0; the r3 report's
`bytecode_len = 80 + 4N` model was for a different path/fp shape and did not
transfer, so N-values were found by direct measurement — documented inline
in `generate.sh`'s new block header).

| Fixture | Shape | mk1 lines | Evidence |
| --- | --- | --- | --- |
| `v-ap-canonical.txt` (+`-control.txt`) | row 1 / row 12's minimal 2-slot fixture: 2 cards, 1 stub, standard path, "2×2" | 4 + 4 | 2 cards × 2 chunks each, pinned vs. unpinned twin (`--chunk-set-id` present/absent); 2-slot fp-free sortedmulti template included |
| `v-ap-bchtwin.txt` | row 2: card 0's 2 chunks, 1-char BCH-correctable flip each (well inside `t=4`, past the 8-symbol header) | 2 | manually diffed against the canonical card's chunks — single-character substitutions confirmed at offset 23 (chunk 0) / 26 (chunk 1) |
| `v-ap-shared.txt` | row 3: 2 cards, 13 IDENTICAL stubs (chunk 0 shared), n=3 | 6 | chunk-0 lines byte-identical (`sed -n '1p;4p'` diff empty); shape test confirms canonical-key equality |
| `v-ap-floor.txt` | row 4 floor: 3 cards, N=120 distinct stubs, n=11 | 33 | shape test: 11 indices × 3 distinct pieces, product = 3^11 = 177,147 |
| `v-ap-boundary.txt` | row 4 boundary: 3 cards, N=128 distinct stubs, n=12 (matches the spec text's own "128-stub mint" exactly) | 36 | shape test: 12 indices × 3 distinct pieces, product = 3^12 = 531,441 |
| `v-ap-groupcap.txt` | 3+3 two-class group, one id (r3-I5's separating shape) | 15 | shape test: class A (2-chunk) k=3, class B (3-chunk) k=3, Σk=6 |
| `v-ap-incomplete.txt` | one complete 2-chunk card + a 3-chunk card missing chunk_index 2 | 4 | shape test: 2-chunk class has both indices, 3-chunk class has exactly {0,1}, index 2 genuinely absent |
| over-budget (row 5) | 5 synthetic cards, n=32 (mk-codec's real ceiling is n=21 — not mintable) | 160 (built at test time via `synth::synth_card_strings`, never a static file) | shape test: 32 indices × 5 distinct pieces, product SATURATES to `u64::MAX` (5^32 ≈ 2.3e22 ≫ u64::MAX) |
| `v-ap2.txt` | row 9 AP2 ground fixture — see item 4 | 9 | see below |

## Item 4 — AP2 grind script (`src/bin/gen_ap2_grind.rs`)

Implements r3-I4's `[2,3,3]` construction exactly: cards A/B share a
13-stub chunk 0 (SPEC row 3's threshold), card C carries a different
13-stub list; grinds ONE stub of C's list (4 bytes = 32 bits, entirely
inside chunk 0's 53-byte range — `assert!`-checked at runtime, not just
commented) until `C0 ++ A1 ++ A2` collides with A's real trailing
cross-chunk hash. Deps: only `bitcoin::hashes::sha256` (already-vendored via
the workspace `bitcoin` dep) — no `ChunkFragment`, no mk-codec internals.

**Measured runtime:** first attempt was single-threaded and took over 120s
because `KeyCard`/`encode_bytecode`/EC-key-derivation were being re-run on
every one of ~4 billion iterations (the EC point multiplication dominates,
not the SHA-256). Rewrote the hot loop to precompute a 136-byte template
once and patch only the 4 counter bytes per iteration (no allocation, no EC
math in the loop), and parallelized across `std::thread::available_parallelism()`
(24 on this box) with `std::thread::scope` + an early-exit `AtomicU64`.

Three timed runs after the fix: **7.671s, 8.474s, 7.665s, 8.280s** (4 runs
total, all found the SAME counter `3994174501`), ~21-24 cores utilized
(2037-2136% CPU). This is *faster* than the plan's informal "~16s" reference
— using all 24 cores rather than 1, consistent with this repo's standing
parallel-execution directive. Reported per the brief's "if wildly different,
report it" instruction: it is different (faster), and the reason is
parallelization, not a shortcut on the search space (still a full ~2^32
brute force, verified: the match counter is ~93% through the 32-bit space,
consistent with a uniform-random hash target).

**Fixture:** `crates/md-cli/tests/fixtures/seating/v-ap2.txt`, SHA-256
`e451eeeffc69abcc22f90edd0a15dda12a6ef03bda6c3cdc625f739a156dd110`, 9 mk1
lines (cards A, B, C × 3 chunks), 1620 bytes.

**Self-correction during the task:** the first committed version embedded
the measured wall-clock elapsed time in the file's provenance comment,
which is NOT deterministic run-to-run (only the grind counter is) — this
would have failed the determinism guard. Caught by actually re-running the
determinism check rather than assuming it; fixed by dropping the timing
line from the emitted file (the counter alone is the deterministic,
load-bearing provenance fact; timings belong in this report, not the
fixture).

## Item 5 — shape tests (`seat/p0_shapes.rs`), all via `canonical_piece_key`

| Test | Asserts |
| --- | --- |
| `shared_piece_pair_chunk_0_is_one_canonical_piece` | chunk-0 keys equal (`PartialEq`) for the shared-piece pair; chunks 1/2 distinct |
| `floor_set_has_three_distinct_pieces_at_every_index_product_3_pow_11` | 11×3 distinct, product = 177,147 |
| `boundary_set_has_three_distinct_pieces_at_every_index_product_3_pow_12` | 12×3 distinct, product = 531,441 (and > floor's product) |
| `over_budget_synthetic_set_has_five_distinct_pieces_at_every_index_product_saturates` | 32×5 distinct (via the chunker), product saturates to `u64::MAX` |
| `group_cap_set_sums_k_across_both_classes_to_six` | k=3 per class, Σk=6 |
| `incomplete_class_set_3_chunk_class_is_missing_one_index` | 2-chunk class complete; 3-chunk class present={0,1}, index 2 genuinely absent |
| `ap2_fixture_hidden_extra_candidate_verifies_under_mk_codec_decode` | F=(C0,A1,A2) reconstructed purely from the fixture's 9 strings; `mk_codec::decode` accepts it; F ≠ A, B, C |

### RED→GREEN evidence (mutation-tested, not just "tests pass")

Two targeted mutations, run and reverted (`diff` confirmed clean restore):

1. **`synth_fragment` mutation** (dropped the `card` parameter, so all
   synthetic cards produce identical fragments): `over_budget_…` test FAILED
   exactly as expected (`left: 1, right: 5` at index 0); the other 6 tests
   were unaffected (correct — they don't touch the chunker).
2. **`canonical_piece_key` mutation** (`symbol_tail: Vec::new()` instead of
   the real payload): 6 of 7 tests FAILED (`shared_piece_pair_…`,
   `floor_set_…`, `boundary_set_…`, `group_cap_set_…`,
   `over_budget_…`, `ap2_fixture_…`); `incomplete_class_set_…` correctly
   stayed green (it only depends on `chunk_index`/`total_chunks`, not the
   payload — not vacuous, a genuinely different property).

Both mutations reverted; `diff` against the pre-mutation file confirmed
byte-identical restoration before continuing.

## Gate results

- **Shape tests RED→GREEN:** see mutation evidence above; all 7 pass on the
  clean tree.
- **`cargo nextest run --locked -p md-cli`:** **715/715 passed**, 0 failed,
  0 skipped. Baseline was 698 (stated in the dispatch brief); 698 + 17 new
  unit tests (5 in `canonical.rs` + 5 in `synth.rs` + 7 in `p0_shapes.rs` =
  17) = 715 — exact match, confirming no regression and no silent test
  drop.
- **`cargo fmt --check`:** clean (workspace-wide and scoped; `cargo fmt`
  was run once to fix initial formatting drift in the new files, then
  re-verified clean).
- **`cargo clippy --all-targets`:** clean (workspace-wide, 0 warnings). One
  `clippy::doc_lazy_continuation` warning was found and fixed (a doc
  comment's wrapped "+"-prefixed line read as an unindented markdown list
  continuation) before this result.
- **Fixture regeneration determinism (the guard):**
  - `generate.sh` re-run (all 31 fixtures, including the 8 new ones):
    `git diff --stat` **empty** — byte-identical.
  - `gen_ap2_grind` re-run twice more after the timing-embedding fix:
    `diff` **empty** against the committed `v-ap2.txt`, same SHA-256
    (`e451ee…`) both times, same grind counter (`3994174501`) across all 4
    runs total.

## Deviations from the plan, with reasons

1. **AP2 grind runtime (7.7-8.5s vs. the informal "~16s")** — faster because
   parallelized across the box's 24 cores per this repo's standing
   directive; not a shortcut (still a full-space brute force over 4-byte
   counter space, first fix attempt was single-threaded and would have
   taken hours due to an unrelated per-iteration EC-key-derivation cost,
   which was removed from the hot loop as part of the same fix).
2. **Determinism-breaking timestamp, caught and fixed mid-task** — see item
   4 above; the final committed fixture and script no longer embed
   wall-clock timing.
3. **Floor/boundary N-values (120/128) found by direct measurement**, not
   derived from the r3 report's `80 + 4N` byte-length model, which was
   measured against a different path/fp shape and did not reproduce here
   (documented inline in `generate.sh`).
4. **`synth` module visibility** interpreted as `pub(crate)` reachable
   across `seat/*.rs`'s own `#[cfg(test)]` blocks, not literally importable
   from `tests/*.rs` integration binaries — see the interpretation note
   under item 1 (this crate has no `lib.rs`, so the literal reading is not
   achievable without restructuring the crate, which is out of P0's scope).

None of these deviations touch P0's substance (the canonical key, the
fixtures' shapes, or the gate's pass/fail outcome).

## Not done (correctly out of scope for P0)

No `seat::run` wiring change, no partition engine (`seat/partition.rs`
does not exist), no `--seat` grammar change, no message/doc churn. Worktree
left dirty and unstaged; nothing committed.
