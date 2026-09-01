# R0 review — IMPLEMENTATION_PLAN_seat_auto_partition.md, round 1

**Artifact:** `design/IMPLEMENTATION_PLAN_seat_auto_partition.md` @ `7e5c566b`
**Against:** `design/SPEC_seat_auto_partition.md` @ `230661b6` (GREEN 0C/0I, r1–r5)
**Code baseline:** descriptor-mnemonic `origin/main` `54ab1cd6`
**Lens:** plan correctness ONLY — does the plan correctly and completely
implement the GREEN spec, with a phase order that works and gates that can
fail? Spec design (V=k semantics, budget numbers, AP rulings, cap, ordinals)
was treated as settled and was NOT re-opened.

## Verdict

**0 Critical / 6 Important / 8 Minor / 3 Nit — NOT GREEN.**

No finding disputes the spec or the design. Every Important is a hole in the
plan's own execution surface: a step whose cited RED row it cannot turn green,
a spec row assigned to nobody, a fixture the plan claims to build but does not
list, a constant with no acceptance bounds, a script with no specification, and
a false scoping statement about which repos P0 needs.

## Machine-checked BEFORE this review (do not re-derive)

Everything below was run against `54ab1cd6`, not read from a doc comment.

| Claim | Result |
| --- | --- |
| Baseline staleness anchor | `git log 54ab1cd6..HEAD -- crates/md-cli/src/seat/` → **empty**. Anchor holds. |
| `decode_cards` call sites | **1 production + 22 test**, exactly as the plan says: `seat/mod.rs:143`; `input.rs` ×13 (444,446,470,479,480,504,542,579,618,644,678,710,725), `complete.rs` ×3 (135,153,192), `matching.rs` ×3 (314,348,434), `disposition.rs` ×2 (179,361), `satisfy.rs` ×1 (304). (`grep` reports 31 hits; 9 are doc references or `use` lines.) |
| Spec row-12 code citations | All four resolve: `REMEDIES` at `matching.rs:216-221`; `input.rs:12-20`; `input.rs:490-493`; `directive.rs:23-26`. No drift. |
| mk-codec pin | `Cargo.toml` `mk-codec = "0.5"`, `Cargo.lock` **0.5.0**. |
| Synthetic chunker buildable on mk-codec 0.5.0 PUBLIC API alone | **YES — compiled and run.** `StringLayerHeader::Chunked{..}` literal (the enum is `#[non_exhaustive]`, its *variants* are not, so downstream construction is legal) → `.to_5bit_symbols()` → `+ bch::bytes_to_5bit(fragment)` → `bch::encode_5bit_to_string()`. Emitted valid mk1 strings for **n = 7, 11, 12, 21 and 32**; every one round-trips through `decode_string` + `StringLayerHeader::from_5bit_symbols`. **`ChunkFragment`'s `#[non_exhaustive]` blocks nothing** — the chunker never needs one. No BCH and no trailing-hash logic is reimplemented. |
| 2^32 SHA-256 grind on this box | **15.8 s measured** (2×10^8 trials of a 77-byte input in 0.73 s across 24 cores = 272 MH/s, `sha2` 0.10, release). Python `hashlib`: **2.70 MH/s/core** (→ ~66 min 1-core, a few minutes under `multiprocessing`). |
| `mk encode` supports P0's mints | `--policy-id-stub` **repeatable**; `--chunk-set-id` present (hex). |
| §1's plumbing already exists | `group_key_of` (`input.rs:164`) already calls `decode_string` and `StringLayerHeader::from_5bit_symbols`, and **discards `_consumed`**. The §1 key's payload tail is `decoded.data()[consumed..]` — one extra `ChunkInfo` field, no new parse, no new failure route (matches the spec's claim). |
| Row 1's "two R2 warnings" | Holds unchanged: `seat_chunk_set_id_warnings` (`input.rs:401`) iterates **per card**, not per group id, so two collided cards emit two warnings with no code change. |
| `DecodedCard` / `label()` churn size | `DecodedCard{..}` constructed at **exactly 1 site** (`input.rs:366`); `.label()` called at **16 sites**. |
| CI required checks | Job names are `cargo test (ubuntu-latest)` and `cargo clippy` — the plan is right. Also live: `cargo fmt`, `cargo doc`, freebsd compile-gate, musl, and **`vendor-freshness`** (resolves `Cargo.lock` OFFLINE against the committed `vendor/`, 127 crates; `bitcoin_hashes` present, `sha2` and `rayon` **absent**). |

## Important

### I1 — P1 step 2's cited RED rows (3, 4, 5, 6, 7) assert outcomes that only step 3 delivers

The plan orders P1 as: step 2 = the `seat/partition.rs` engine (RED: rows 3, 4,
5, 6, 7); step 3 = outcomes wiring, "pre-pass before the shipped classifier;
arm-1 entry on no-partition; AP2 refusal".

Every one of those five rows is defined in the spec as an **end-to-end
outcome**, not an engine internal:

- row 3 "→ seats as two cards", row 4 floor "→ seats within budget" — seating
  is step 3's wiring;
- row 4 boundary "→ budget refusal naming AP3's rationale", row 5 "→ static
  refusal" — the refusal must surface as a `CliError` from `decode_cards`,
  which is step 3;
- **row 6 "→ zero-decode arm-1 message"** — "arm-1 entry on no-partition" is
  literally step 3's own deliverable line;
- row 7 "one class incomplete → whole group refuses via arm 1" — same.

**Failing scenario.** The implementer writes row 6's test at step 2. It cannot
go green at step 2 (the pre-pass does not exist yet, so a two-piece 44444 group
still reaches the shipped classifier by the old route and the test's *reason*
for passing is untested). They resolve the contradiction the cheap way: rewrite
rows 3–7 as unit tests over `partition.rs`'s internal outcome type, mark step 2
green, and never re-assert them end-to-end — because the plan lists each row
exactly once. §3's wiring order (partition pre-pass **before** the shipped
classifier) then ships with no test that can fail on it, and the P1 gate's "all
13 spec rows implemented and green" is satisfied by rows that no longer test
what the spec's rows say.

**Remedy.** Either (a) merge steps 2 and 3 into one step, or (b) state
explicitly that step 2's REDs are `partition.rs` unit tests over a
`PartitionOutcome` value, and re-list rows 3–7 in step 3 as the end-to-end
assertions through `decode_cards` / `md seat`. Also say where each row lives:
rows 3–7 are `decode_cards`-level (`src/seat/` unit tests), while **rows 1 and
8 are integration-level** — they compare descriptor, address AND WalletPolicyId,
which only `md seat`'s output carries (`crates/md-cli/tests/cli_*.rs`).

### I2 — spec row 10(c) is assigned to no phase or step

Spec row 10 has three variants (r4-I1). The plan assigns (a) to step 3 ("RED:
… 10a") and (b) to step 4 ("row 10b"). **(c) — "different-id extra card →
today's leftover path, unchanged" — appears nowhere in the plan.**

**Failing scenario.** P1's gate asserts "all 13 spec rows implemented and
green". It closes with 12.67 of them. Row 10(c) is the regression guard proving
the partition pre-pass did **not** capture a different-id card into a collision
group; it is the cheapest of the three and the only one covering the unchanged
path, so its absence is invisible until a different-id card starts getting a
`#<k>` label.

**Remedy.** Add row 10(c) to step 4's RED list (it shares row 10(b)'s fixture
family), or state that the shipped `v-leftover.txt` row already discharges it
and name that test.

### I3 — the group-wide cap has no fixture in P0 and no decomposed RED

P1 step 2 lists "group cap Σk ≤ 5" as a deliverable. Its only cited row is 7.
But spec row 7 is **three** distinct outcomes — (i) mixed totals, both classes
complete → both seat; (ii) **a 3+3 two-class group → cap refusal** (the whole
point of r3-I5: the cap is group-wide, NOT per-class); (iii) one class
incomplete → whole group refuses via arm 1 — and **P0's fixture list contains
none of (ii) or (iii)**. P0 lists exactly: row 1's pinned pair + control, row 2
BCH twins, row 3 shared-piece pair, row 4 floor/boundary, row 5 over-budget,
row 9 AP2. The shipped `v-collide.txt` serves (i) only.

**Failing scenario.** The implementer writes the cap as `k_class > 5` per class
— the exact defect r3-I5 found and the spec fixed. Every fixture P0 built still
passes: the over-budget row is one class with k = 5 (Σ = 5, cap not reached
either way), the floor row is k = 3, v-collide is k = 1 per class. AP3's
operator ruling ships inverted with a green suite. Nothing in P0 or P1 can
detect it, because the only input that separates the two readings — two classes
of 3 — was never minted.

**Remedy.** Add to P0's fixture list: (ii) a 3+3 two-class same-id group
(6 cards, n = 3 each, distinct stub lists, Σk = 6 > 5 → cap refusal *before*
the budget test), and (iii) a mixed-totals group with one class short a piece.
Split row 7 into three named sub-rows and assign each to a step.

### I4 — the `PARTITION_DECODE_BOUND` procedure states neither the build profile nor the acceptance bounds

The plan says the constant is "fixed HERE from a re-run of the timing
measurement on this machine, target ≤ ~2 s worst case, expected ≈ 255k; record
the measured µs in the constant's comment". That is not reproducible:

1. **No profile named.** r3 measured **7.845 µs/candidate in release** but
   **9.8–10.7 µs on this repo's `opt-level = 2` test profile** — the profile the
   gate actually runs in. `2 s ÷ 7.845 µs = 255k`; `2 s ÷ 10.7 µs = 187k`. Two
   honest re-runs of "the timing measurement" produce constants differing by 27%.
2. **No fixture named.** r3's number is `mk_codec::decode(&[&str])` over a
   3-card × 12-chunk class. "The timing measurement" does not say so.
3. **No acceptance bounds — the real defect.** The constant is not free: the
   spec pins two rows on opposite sides of it. Row 4's floor row must SEAT
   (3^11 = 177,147 candidates) and row 4's boundary row must REFUSE
   (3^12 = 531,441). So the only admissible values are
   **`177_147 ≤ PARTITION_DECODE_BOUND < 531_441`**, and the plan states neither
   end.

**Failing scenario.** The implementer measures on the test profile with a
loaded box, gets 12 µs, sets the bound to 166,000 to honour "≤ ~2 s" — and the
floor row now REFUSES. Spec row 4's floor half inverts, AP3's stated face ("3
cards guaranteed to n = 11 chunks") becomes false, and the failing test looks
like a bug in the engine rather than a bad constant. Note the margin is thin:
the test-profile reading is 187k–204k against a 177,147 floor, i.e. 5–15%.

**Remedy.** State: measured on the `opt-level = 2` test profile (the gate's own
profile) with `mk_codec::decode` over the floor fixture; the constant MUST
satisfy `177_147 ≤ BOUND < 531_441`; record the measured µs/candidate AND both
bounds in the constant's comment; if the measured "≤ ~2 s" value falls below
177,147, the target time is what moves, not the bound.

### I5 — the AP2 grind script has no specification, only an existence claim

The plan's P0 item 2 says: "the AP2 ground fixture + its committed one-grind
generation script (row 9 — the script is committed and documented; the fixture
is committed; regeneration is a command)". That is the entire specification.
Missing: what is ground, where the grind freedom lives, the language, the
runtime, and the dependency impact.

This is the **same gap r1, r2 and r3 each raised at the spec layer**, now
reintroduced one layer down. r2 stated it exactly: *"'Feasibility demonstrated
by the script' is circular: the script is the thing that has to be built, and
r1's finding was that an implementer who cannot build it will weaken or waive
the gate."* The spec discharged it by pointing at r3's `[2,3,3]` construction —
which lives in `design/agent-reports/R0-seat-auto-partition-r3.md`, not in the
plan the implementer executes.

**Failing scenario.** The implementer reaches P0 item 2, finds a one-line
instruction to build a 2^32 grind, has no construction, and takes the shortcut
row 9 explicitly forbids — a BCH twin, or a hand-edited "extra card" that does
not actually verify. Row 9 then passes on a fixture that reaches AP2 for the
wrong reason (or is quietly downgraded to `#[ignore]`), and the single hardest
refusal in the change ships unexercised.

**Answering the plan-feasibility question directly: the grind is cheap and
committing the fixture with documented regeneration is exactly what row 9
asks.** Measured on this box: 2^32 SHA-256 over a ~77-byte input = **15.8 s**
across 24 cores in Rust; even Python `hashlib` (2.70 MH/s/core measured)
finishes in minutes under `multiprocessing`. Feasibility is not the problem —
silence is.

**Remedy.** Put the construction in the plan, not a reference to it:
- r3's shape — one n = 3 class; cards A and B share a 13-stub chunk 0; card C
  carries a different stub list; counts `[2,3,3]`, k = 3. Grind card C's chunk-0
  stub bytes until `F = (C0, A1, A2)` matches A's trailing 4-byte hash, then
  recompute C's own trailing hash. `{A,B,C}` and `{F,B,C}` both cover ⇒ `|V| = 4
  > k = 3` ⇒ AP2.
- r2's validity note — put the grind freedom in **stub bytes** (n = 3 with
  13–26 stubs gives 4-byte-aligned freedom in chunks 0 and 1), never in xpub
  bytes, so every trial is a valid `KeyCard` for free and the grind is pure
  SHA-256.
- Language and runtime: Rust, `bitcoin_hashes::sha256` + `std::thread`, stated
  expected wall time. **Use `bitcoin_hashes` (already vendored) and no `rayon`
  or `sha2`** — this repo vendors its dependencies and CI runs a
  `vendor-freshness` job that resolves `Cargo.lock` offline against the
  committed `vendor/`; a new dev-dependency turns that job red.
- The acceptance the script itself must assert before writing the fixture:
  `mk_codec::decode` accepts the extra candidate, and the fixture is NOT a BCH
  twin of any real card.

### I6 — "Single repo … no mk-codec/md-codec change" is false for P0; the fixture-generation discipline is unaddressed

The plan's header scopes the work to "Single repo (descriptor-mnemonic,
`crates/md-cli`)". P0 then says "mint with mk". But **every existing seating
fixture is generated by `crates/md-cli/tests/fixtures/seating/generate.sh`**,
which (its own header, verified) requires two binaries resolved BY PATH:
`target/debug/md` from this repo and **`mk` from the sibling `mnemonic-key`
repo** (`MK=/scratch/code/shibboleth/mnemonic-key/target/debug/mk`). Its
determinism guard is that a re-run leaves `git diff` clean, and each emitted
file repeats the exact commands that produced it.

The plan mentions none of this. It says only "Each fixture file carries its
generation provenance" — the header convention, not the generator.

**Failing scenario.** The implementer, told this is a single-repo change, mints
the row 1/3/4/7 fixtures ad hoc from a scratch shell invocation and pastes the
strings into new `.txt` files with a hand-written provenance comment. The
fixtures are fine today. `generate.sh` no longer regenerates the directory, so
the "re-run and `git diff`" guard silently stops covering six new files, and the
next mk-codec bump rots them with nothing to notice — the failure mode already
recorded in this constellation as *"reproduction paths decay silently"*.

**Remedy.** State the prerequisite (which sibling repo, which binary, built how),
require every mk-mintable new fixture to be emitted by `generate.sh` with its
commands in the file header, require the synthetic-chunker and grind fixtures to
name their committed generator instead, and add "`generate.sh` re-run leaves
`git diff` clean" to P0's gate. Correct the "Single repo" line to "single repo
under change; `mnemonic-key`'s `mk` is a build-time prerequisite for P0".

## Minor

**M1 — delete the "where the codec does not expose a piece, the helper mirrors
it" escape hatch; it is unnecessary.** *Ruling on the question as posed:* a
test-input **constructor** is categorically different from a verification
**oracle**, and the spec's Out-of-scope rejects the *two-stage oracle* — a
decision procedure inside §2.5 — not test-side input construction. So the hatch
does not contradict the spec. But it is now measured to be unneeded: the
chunker is fully expressible on mk-codec 0.5.0's public API (see the table
above; built and run for n = 7/11/12/21/32). A discretionary hatch is exactly
how a local trailing-hash reimplementation gets written anyway. **Remedy:**
replace the hatch with the four named public calls, and keep only the guard
sentence ("it constructs inputs, it is not an oracle").

**M2 — `tests/common` is not importable where these tests live.** The plan
offers "`#[cfg(test)]` or a `tests/common` module" as equals. All 22
`decode_cards` test sites are `#[cfg(test)] mod tests` blocks **inside
`src/seat/*.rs`**, and a unit test in `src/` cannot import from an integration
test crate under `tests/`. **Remedy:** name `#[cfg(test)]` test-support in
`src/seat/` (re-exported for the `tests/cli_*.rs` integration rows that need it),
and drop the alternative.

**M3 — P0's shape tests must reimplement the §1 canonical key that P1 step 1
also implements.** P0 item 3's "product arithmetic for rows 4/5" is a count of
*distinct canonical pieces per index*, which is §1's key — delivered in P1.
Expressible in P0 from public API, but as a second implementation. **Failing
scenario:** the two keys diverge (say P0's includes the header symbols and P1's
does not); P0's shape test pins 5^32 while the shipped code computes something
else, and row 5's zero-decode refusal plus row 11's "must observably hang"
sizing both pass while measuring a different product. **Remedy:** author the
canonical-key function once and have P0's shape test call the shipped one.

**M4 — §4's ordinal has no named carrier.** Step 4 says "`#<k>` labels in
`label()`", but `label(&self)` (`input.rs:82`) can only see `set_id` and `card`
— the ordinal is a property of the card's position in its id group. **Remedy:**
say "add `ordinal: Option<u32>` to `DecodedCard`, set in `decode_cards`". It is
cheap and the plan should say so: `DecodedCard{..}` has **1** construction site
(`input.rs:366`), whereas threading a `&[DecodedCard]` into `label()` touches
**16** call sites.

**M5 — move the `decode_cards` signature change to the front of P1.** Answering
the ordering question directly: steps 1–2's new tests *do* compile against the
old signature, so the order is coherent — but every test they add is then
rewritten at step 3 when the return type gains the note. **Flipping is
strictly smaller**: do the mechanical 1 + 22-site change first, then write every
new test once against the final signature. It also isolates the churn into one
reviewable commit instead of smearing it across steps 1–4.

**M6 — user-facing surfaces outside `src/seat/` are unassigned, and step 5's
grep has no stated scope.** §4 extends the `--seat` grammar with `#<k>`, but the
flag's help text still reads `@i=<chunk-set-id>` at **`main.rs:432`/`441` and
again at `main.rs:666`/`675`** (two subcommands — and the man pages are
generated from those via `clap_mangen`, with a `man-pages` workflow). `README.md:132`
and `CHANGELOG.md` also describe `--seat '@i=<chunk-set-id>'`. **Remedy:** name
all four sites in step 5, and state that its "grep-assert no stale wording"
covers `crates/md-cli/src/`, `README.md` and `CHANGELOG.md`, not just `src/seat/`.

**M7 — row 11's hang-gate has no stated observation method.** "skip the static
budget → over-budget row hangs" and "mutated-line-RAN evidence for each" are in
tension: a run that never terminates cannot report anything. **Remedy:** state
the method — run the mutated build under `timeout <n>`, with a probe (eprintln
or a counter) proving the mutated line executed *before* the timeout; the
evidence is (probe fired) ∧ (timeout expired), and the mutation is never
committed.

**M8 — the floor row is the slowest test in the suite and is unbudgeted.** Row 4's
floor half runs **177,147 real `mk_codec::decode` calls** — ~1.4 s at r3's
release figure, ~1.7–1.9 s at the `opt-level = 2` test-profile figure. **Remedy:**
say so, and say whether it runs on every `cargo nextest run -p md-cli` or is
gated; if it runs always, add it to the phase-gate timing note so a later
slowdown is attributed rather than investigated.

## Nit

**N1** — the "Machine-verified facts" line reads "`v-collide.txt` = same-id
(12345) mixed-totals fixture; the `44444` fixture = missing-index shape (spec
rows 6, 10b)". Both descriptions are right (v-collide is 2-chunk + 3-chunk,
verified; the 44444 fixture is two 3-chunk cards with only chunk 0 of each, so
indexes 1 and 2 are empty), but v-collide also serves **row 7** — spec row 12
rewrites `v_collide_two_cards_pinned_…` as the mixed-totals row. Cite row 7.

**N2** — spec row 12 requires `v_collide_reaches_the_command` be rewritten "with
a new minimal 2-slot fixture". That fixture is not in P0's list. Say whether it
IS the row-1 canonical pinned pair or a distinct mint; P0 claims to build
everything P1 asserts on.

**N3** — the plan says the AP1 note is "carried into `Seating.notes` ahead of R2
warnings"; the spec says "ahead of **the group's** R2 warnings". At
`seat/mod.rs:177` the composition is `let mut notes = csid_warnings;`, so the
natural edit is a global prepend. Row 1's fixture has a single group, so **the
acceptance row cannot distinguish a global prepend from per-group interleaving**
— worth one sentence in the plan fixing which it is.

## What the plan gets right (so a fold does not re-litigate it)

- The two-phase split (fixtures first, feature second) is the correct shape, and
  "everything in P1 asserts on P0" is the right invariant — I3 and N2 are it not
  being *complete*, not it being wrong.
- The `decode_cards` churn is enumerated exactly and measured correctly (1 + 22),
  and the citations behind it have not drifted.
- The baseline staleness anchor is real and currently clean.
- Naming the build gate as `nextest --locked -p md-cli` + `fmt --check` +
  `clippy --all-targets`, with fmt in the phase gate rather than the push, is
  right; the post-implementation required-check names are accurate.
- P1's internal order 1 → 2 → 3 → 4 (canonicalise, engine, wire, identity)
  matches §1–§4 and is the right dependency order; I1 is about which ROWS are
  cited per step, not about the steps' sequence, and M5 only moves the mechanical
  signature change forward.
- The AP2 P0 reader assertion ("the extra candidate actually verifies") IS
  expressible with `mk_codec::decode` alone — no P1 machinery required. Of P0's
  three reader assertions, only the "product arithmetic" one has a dependency
  problem (M3); chunk-0 identity and AP2 verification are clean.

## What GREEN requires

Fold I1–I6, then re-review scoped to *"did the fold fix each finding, and did it
introduce a new one"* — not a fresh audit. The Minors are all one-or-two-sentence
edits and can ride the same fold. Nothing here needs a spec change.
