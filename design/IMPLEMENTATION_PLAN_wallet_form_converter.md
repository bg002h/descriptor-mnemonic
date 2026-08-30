# IMPLEMENTATION PLAN — the wallet-form converter (P1 / P2 / P3)

**Status: R0 CLOSED GREEN at r2 (0C/0I/1M, Minor folded same day) —
rounds r1 (RED 0C/3I/2M), r2 (GREEN) in
`design/agent-reports/R0-converter-plan-r*.md`. A GREEN EXPIRES —
re-validate before each phase dispatch (§7).**
Spec: `design/SPEC_wallet_form_converter.md`, **R0 CLOSED GREEN at r9**
(rounds r1–r9 + the 2026-08-30 operator key-reuse rulings all folded).
The spec is normative; this plan only schedules and locates it. Where
this plan and the spec disagree, the spec wins and the plan is the
defect.

**Plan baseline (staleness anchor): descriptor-mnemonic `7e125963`**,
mk-codec crate 0.5.0 (repo `93cebfb` + the parked-draft commit),
rust-miniscript pin `ff4732e`. Every file path, flag, version and test
name cited below was resolved against that tree on 2026-08-30 — the
claims-audit list is §8. A GREEN on this plan EXPIRES: re-validate
against "what did the last phase falsify?" immediately before
dispatching each phase's implementer.

This plan carries **no rust blocks** — its executable content is
CLI-command-shaped (commands quoted here were either run on 2026-08-30
or are marked as a phase's gate to run), so there is nothing for a
code-extractor gate to extract; the build gate for the plan itself is
§8's claims audit.

## 1. The surface: one matrix

**THE MATRIX TRAVELS (operator directive, 2026-08-30): this table is
the cycle's goal-and-gaps statement and is embedded, cells kept
current, in EVERY artifact — brainstorm, spec, this plan, and the
seating engine's module doc comment in the code. A document or module
missing it is incomplete.**

Input forms (any COMPLETE wallet expression):

- **D** — concrete descriptor (miniscript or plain), keys + origins inline
- **T** — BIP-388 template + per-slot keys/origins as flags
- **S** — the split card set: keyless md1 phrases + mk1 strings
- **K** — keyed md1 phrases (Pubkeys TLV)

Output forms: concrete descriptor · addresses · keyed card (via the
existing `md encode --key` bridge) · template + origin-notated key lines.

| in \ out | concrete descriptor | addresses | keyed card | keyless + mk1 cards |
| --- | --- | --- | --- | --- |
| **D** concrete descriptor | — | ✗ P3 | ✗ P3+bridge | ✗ P3 (the decomposer) |
| **T** template + key flags | ⚠ P1 (flag-form gap; inline template origins already work — r1 I8) | ⚠ P1 | ✓ `md encode --key` (Divergent) | ✓ |
| **S** keyless card + mk1 strings | ✗ P2 (the seating engine) | ✗ P2 | ✗ P2+bridge | — |
| **K** keyed card phrases | ✓ (round-tripped live) | ✓ | — | ✗ non-goal (first real need files it) |

✓ measured working; ⚠/✗ the gaps, tagged with the piece that closes
them. On C4 close, the ⚠/✗ cells this cycle owns flip to ✓ in every
embedded copy in the same commit as the acceptance walk that proves
them.

## 2. Decisions this plan settles (the spec left them to scheduling)

**D1 — mk1 decoding arrives via the REGISTRY dep `mk-codec`, never a
path.** Precedent is in-tree: `crates/md-cli/Cargo.toml:39-41` — "the
shared constellation IO crate, taken from the REGISTRY. NOT `path =`:
a path does not resolve in a fresh CI checkout" — and me-cli already
consumes `mk-codec = "0.4"` from the registry
(mnemonic-engrave `crates/me-cli/Cargo.toml:39`, lock 0.4.1). The
converter needs mk-codec's decode surface at 0.5.x (derived
chunk-set-id era). **C0 entry gate: confirm mk-codec 0.5.0 is
published; if it is not, publishing it (dual MIT OR Unlicense, as all
constellation publishes) is an operator-visible pre-C0 step — flag it,
do not vendor around it.** `cargo build --locked` proving resolution
is C0's exit.

**D2 — module locations.**

- `crates/md-cli/src/parse/keys.rs` — grows the origin-notated value
  form `@i=[fp/path]xpub` (shared by P1 flags and P3 emission checks).
- `crates/md-cli/src/seat/` — NEW module: the whole seating engine
  (A1 triage, A2 satisfaction, A3 matchings + compose-canonicalise-
  compare + cap + tie-break, A4 completeness, A5 `--seat`, B1
  disposition, B2 oracles, the spend-equality checker, the input
  pipeline). **Its `mod.rs` doc comment carries the §1 matrix** (the
  operator directive's fourth home).
- `crates/md-cli/src/cmd/descriptor.rs` + `cmd/address.rs` — gain
  `--from-mk1 <STRING>` (repeatable) / `--from-mk1-file <FILE>` and
  `--seat '@i=<chunk-set-id>'` (P2), and the origin-notated `--key`
  form + per-datum precedence (P1).
- `crates/md-cli/src/cmd/decompose.rs` — NEW subcommand (P3), with
  the fresh MultiXPub walker in `crates/md-cli/src/decompose/` (the
  existing substitution machinery is unusable by measurement — r1 M5:
  it strips to synthetic xpubs and its drift guard forbids
  `MultiXPub`, yet every multipath key parses AS `MultiXPub`).
- Vectors: new `crates/md-cli/tests/seating_vectors.rs` (C2),
  `decompose_vectors.rs` (C3), extensions to existing files where a
  row is one flag away (C1). Style: the shipped `assert_cmd` +
  `predicates` pattern of the 40+ existing `cli_*`/`cmd_*` tests.
- Fixtures: `crates/md-cli/tests/fixtures/pathological/` — copied in
  C4 (walks) and C2 (subsets) from mnemonic-engrave
  `design/journeys/out/pathological/{backup-strings.txt,keys.txt}`;
  the 22-string keyed card comes from
  `design/journeys/out/pathological/journey_pathological.html`
  (r1 I3 corrected a wrong CONTINUITY citation; measured 2026-08-30:
  the html carries the card TWICE — 44 `md1fatzr2…` tokens, 42×86 +
  2×59 chars — deduping to exactly 22 unique strings, 21×86 + one
  59-char tail, the W-PIN shape). C4's copy step extracts, dedupes,
  and asserts that shape before any walk runs. Each fixture file
  carries a provenance note naming source path + date.

**D3 — snapshot surfaces move deliberately.** New flags and the new
subcommand will shift `cmd_gui_schema.rs`, `json_snapshots.rs`,
`help_examples.rs` and the gen-man output. The implementer regenerates
insta snapshots per-change with the diff read and named in the commit
message — never a blind `insta accept` sweep.

**D4 — pushes use the staging-ref ritual from this cycle on.**
Measured 2026-08-30: a plain `git push origin main` on THIS repo
printed `remote: Bypassed rule violations for refs/heads/main` — the
required checks bind here exactly as on the siblings, and this repo
has no staging script yet. Until one is committed (follow-up, C4),
follow mnemonic-key's CLAUDE.md ritual inline: push
`main:refs/heads/ci/staging`, watch the run green, then push main,
then delete the staging ref. Freeze main for the window.

## 3. Phases

Ordering: C0 → C1 → C2 → C3 → C4, one implementer per phase (standing
directive: implementation tight, controller folds small post-review
fixes inline). C1 before C2 because C1 is the small standalone surface
— settling the flag grammar and its refusal texts first keeps the big
phase's diff pure engine (r1 M1: the earlier "P2 consumes them" reason
was wrong; what C0's parser is shared by is C1's flags and C3's
emission checks, per D2). C3 after C2 because decompose's acceptance
leg re-composes through the C2 engine.

**C0 — wiring (small).** D1's dependency lands and resolves
(`cargo build --locked`); `parse/keys.rs` gains the
`@i=[fp/path]xpub` value parser with unit rows (accepts the notation;
malformed origin draws a NAMED refusal, not today's bare
"base58check decode" — the measured motivation refusal 3); `seat/mod.rs`
skeleton exists carrying the matrix doc comment. Exit gate:
`cargo nextest run --locked` green, `cargo clippy --locked
--all-targets -- -D warnings` green, `cargo fmt --check` green —
**PLUS, for every phase (r1 I2 — nextest+clippy+fmt all pass on a
tree with no new tests, so alone they cannot prove a phase's work
exists): the phase's named roster rows present and passing, proven by
a row-scoped run (`cargo nextest run --locked -E 'test(<row prefix>)'`)
whose matched-test count is quoted AGAINST THE PHASE'S EXPECTED ROW
COUNT (r2 M1 — "nonzero" is satisfied by 1 of C3's 8 rows; the
expected number comes from the roster) in the phase-close commit
message — an empty or short count is a FAIL, not a pass.** These together are
hereafter "the gate"; C0's named rows are V-KEYORIG-BAD's unit half.

**C1 — P1, the T row (small).** Origin-notated `--key` accepted on
`descriptor`/`address`; per-datum precedence exactly as SPEC P1 (paths:
inline template origin, else shared `--path`, else today's
non-canonical-wrapper refusal; fingerprints: `--fingerprint` or
origin-notated `--key`, both naming slot i must AGREE or refuse —
never silent override; origin-notated key path must agree with an
inline path where both exist); the inline `h`-spelling refusal points
at the `'` requirement (kills the measured F-420-class misdirect).
Rows: V-KEYORIG, V-KEYORIG-BAD, V-FPAGREE, V-PATHAGREE, V-PRECEDENCE,
V-HSPELL. Exit: the gate + all rows in the same commits as the code.

**C2 — P2, the S row (the core; funds-shaped).** Build order inside
the phase, TDD throughout, rows in the SAME commit as each behaviour:

1. Input pipeline (normative, A3(a)): dedupe byte-identical strings →
   group by declared chunk-set id → reassemble under `mk decode`
   semantics. Rows V-DUP, V-COLLIDE.
2. A2 satisfaction (decoded values, never strings; declaration is the
   constraint). Rows V-IMPOSS, V-DOOR.
3. A3: perfect-matching enumeration; compose-canonicalise-compare
   (the comparison form sorts within `sortedmulti`/`sortedmulti_a`
   instances, is never emitted); 720-total cap with early
   termination; lexicographically-least assignment-vector tie-break.
   Rows V-ORD, V-R2-ORD, V-R4-IK, V-GRP, V-USP, V-R5M1, V-BOUND-SEAT,
   V-BOUND-REF, V-MIX, V-AMB, V-CAP; V-FPFREE-CARD lands with step 2.
4. A4 completeness. Rows V-UNFILLED, V-LEFTOVER.
5. A5 `--seat`. Rows V-SEAT-OK, V-SEAT-BAD, V-SEAT-UNK (see roster
   note on the ambiguous-id case).
6. B1 dispositions + B2 oracles + the SPEND-EQUALITY checker. Rows
   V-B1-WALLET, V-B1-SHAPE, V-B1-WARN, V-B1-CROSS, V-CE1, V-SPENDEQ.
7. CLI surface: `--from-mk1`/`--from-mk1-file` on `descriptor` and
   `address`; stdout stays the machine contract, notes + B2 address
   to stderr; the keyless-phrases refusal now points at `--from-mk1`
   (row V-MSG-KEYLESS — the r1 M8 self-contradicting message dies).

Exit: the gate + a scoped independent review of the SEATING DIFF
(sonnet, mechanical brief: do the rows assert what the spec's rows
demand, does any refusal path return before printing its names, is
there a false-PASS shape) before C3 dispatch.

**C3 — P3, the D row.** `md decompose <DESCRIPTOR|--in FILE>`:
rust-miniscript parse (UNGATED — measured, parsing needs no feature);
the NEW MultiXPub walker (the phase's largest piece); emissions
(keyless template, round-trip-grade origin-notated key lines — keys AS
PARSED, never re-serialised depth-0; per-slot `--fingerprint` flags;
`--emit commands`); refusals per spec P3 (depth-inconsistent input;
origin-less keys excluded from the mintable set with `--emit commands`
refusing by name; same-xpub-twice REFUSED as "forbidden by BIP 388",
never "invalid"; shape-2 non-disjoint multipath REFUSED — reachable
here because rust-miniscript parses it; Core JSON and receive/change
pairs refused with guidance, not the bare checksum error). Rows:
V-D-RT, V-D-DEPTH, V-D-NOORIG, V-D-REUSE, V-D-SHAPE2, V-D-JSON,
V-D-PAIR, V-D-CHKSUM. Exit: the gate (its row-scoped run covers all
eight V-D-* rows) + the same scoped independent review pattern as
C2's (sonnet, mechanical: do the eight rows assert what the spec's
P3 bullets demand; any false-PASS shape) — r2 M1's second half: C3
was the one phase with rows and no review before C4's backstop.

**C4 — acceptance, docs, close-out.**

1. The pathological walks as executable tests over the copied
   fixtures: (a) 36-string split set composes, address 0
   `bc1qkuknuy6…`; (b) 22-string keyed card composes SPEND-EQUAL to
   (a); (c) decompose of the pinned depth-consistent fixture (first
   three `keys.txt` lines) round-trips ROUND-TRIP-EQUAL through
   `md encode --key` + `mk encode --keys`. The keyed-card-derived
   descriptor stays out of leg (c) BY NAME (depth-0 keys, r1 C3).
   Reproduction pins asserted: 22 strings = 21×86 + one 59-char tail;
   composed keyed descriptor 1,648 chars.
2. Matrix cells flip to ✓ in all four embedded copies, same commit.
3. CHANGELOG, README surface note, regenerated man/gui-schema
   snapshots (D3 discipline).
4. Follow-ups reconcile (§6) + commit `scripts/push-via-staging.sh`
   for this repo (D4's follow-up).
5. **Mandatory post-implementation adversarial review over the WHOLE
   cycle diff (opus)** — seating semantics are restore-correctness,
   squarely in the risk set; this review is non-deferrable and merge
   waits for it. Then push via D4's ritual.

## 4. The vector-row roster

Every row below FAILS if its behaviour is removed or inverted; rows
land in the same commit as the behaviour they pin. "Named" means the
refusal text carries the identifiers the spec demands (cards by full
chunk-set id, slots, remedies).

| Row | Case | Expect | Phase |
| --- | --- | --- | --- |
| V-KEYORIG | `--key '@0=[fp/48h…]xpub'` on a template | seats; concrete descriptor | C1 |
| V-KEYORIG-BAD | malformed origin notation | named refusal (not bare base58) | C0/C1 |
| V-FPAGREE | `--fingerprint` + origin-notated `--key` disagree on slot | refuse | C1 |
| V-PATHAGREE | origin-notated path ≠ inline template path | refuse | C1 |
| V-PRECEDENCE | inline path present + shared `--path` | inline wins (per-datum) | C1 |
| V-HSPELL | inline `48h/…` origin | message points at `'` requirement | C1 |
| V-DUP | full split set supplied twice over | SEAT (dedupe precedes grouping) | C2 |
| V-COLLIDE | two cards pinned to one chunk-set id | refuse at reassembly (merged group) | C2 |
| V-IMPOSS | identical fp-bearing origin, different xpubs | refuse (impossible from one master) | C2 |
| V-DOOR | policy with two identical fp-bearing (fp,path) slots | refuse at the door (BIP 388 rule 1) | C2 |
| V-ORD | three supply orders of a SEATABLE set | identical descriptor bytes (determinism) | C2 |
| V-R2-ORD | the r2 three-orders counterexample fixture | REFUSES identically in all three orders (r1 I1 — the verdict, not just the bytes, is order-invariant) | C2 |
| V-R4-IK | r4's internal-key case, reuse-free five-distinct-key form | refuse (internal-key/leaf repartition composes unequal wallets) | C2 |
| V-FPFREE-CARD | fp-free CARD against a fp-BEARING declaration | cannot satisfy (A2's restrictive half); slot otherwise unfillable → unfilled-slot refusal naming the declared origin | C2 |
| V-GRP | two-group repartition (r3) | refuse (wallet-unequal candidates) | C2 |
| V-USP | use-site-path swap (r5) | refuse (comparison-form inequality) | C2 |
| V-R5M1 | two group instances sharing placeholders, same path | refuse — "forbidden by BIP 388" | C2 |
| V-BOUND-SEAT | fp-free same-path DIFFERENT masters | SEAT (the boundary's seat side) | C2 |
| V-BOUND-REF | same xpub offered for two slots | refuse — "forbidden by BIP 388" | C2 |
| V-MIX | mixed declarations, unique matching | SEAT (r6 M2) | C2 |
| V-AMB | genuine ambiguity | refuse naming cards/slots/both remedies | C2 |
| V-CAP | two independent 6-card components (>720) | refuse stating bound + candidates | C2 |
| V-SEAT-OK | V-AMB + consistent `--seat` | seats | C2 |
| V-SEAT-BAD | `--seat` contradicting A2 | refuse by name | C2 |
| V-SEAT-UNK | `--seat` with unknown id | refuse by name | C2 |
| V-UNFILLED | missing card (r7-C1 half) | refuse naming slot + declared origin | C2 |
| V-LEFTOVER | extra foreign card | refuse naming card + stub | C2 |
| V-B1-WALLET | keyed-mint card matching composition | wallet-confirmed | C2 |
| V-B1-SHAPE | shape-tier card | seats, shape-confirmed | C2 |
| V-B1-WARN | fixture card `232214e4` vs `ced22709` | WARNING, both readings named | C2 |
| V-B1-CROSS | cross-shape card | unconfirmed WARNING, no refusal | C2 |
| V-CE1 | same-stub foreign card | seats AND derived address differs — both asserted | C2 |
| V-SPENDEQ | split vs keyed compositions; + one-xpub-off negative | equal / not equal | C2 |
| V-MSG-KEYLESS | keyless phrases alone | refusal points at `--from-mk1` | C2 |
| V-D-RT | pinned fixture decompose | outputs re-compose ROUND-TRIP-EQUAL | C3 |
| V-D-DEPTH | depth-inconsistent input key | refuse naming mk's constraint | C3 |
| V-D-NOORIG | origin-less key | bare line; `--emit commands` refuses by name | C3 |
| V-D-REUSE | same xpub at two positions | refuse — "forbidden by BIP 388" | C3 |
| V-D-SHAPE2 | same placeholder, non-disjoint multipath | refuse (reachable via rust-miniscript) | C3 |
| V-D-JSON | Core `listdescriptors` JSON | guidance refusal | C3 |
| V-D-PAIR | separate receive/change descriptors | guidance refusal (combine to `<0;1>`) | C3 |
| V-D-CHKSUM | missing/wrong checksum | accept / non-F-420 error | C3 |
| W-A/B/C, W-PIN | §3 C4 item 1 | the acceptance walks | C4 |

Roster note: A5's "ambiguous id" refusal is UNREACHABLE, settled by
SPEC A3(a) step 3 (r1 M2 — no implementer determination is open):
colliding cards refuse at reassembly and never reach `--seat` parsing.
V-COLLIDE's test carries a comment stating it subsumes the A5
ambiguous-id case.

## 5. What is deliberately NOT here

The spec's non-goals (no `me` admission change, no crowning either
card form, no device/fork/wire change, no K→S split) plus: no mk CODE
changes (r4 M4); no md `encode` admission change for the `@0,@0` form
(filed, §6); no cross-repo canonical-spelling unification.

## 6. Follow-ups reconciliation (per-phase, by ownership)

- `md-repeated-placeholder-inverts-bip388` — **NOT this cycle's**, by
  its own text ("do not change either as a converter side effect");
  owning phase re-tagged to a dedicated post-converter md-codec/cli
  admission mini-cycle so the grep stays honest.
- `stub-keyed-wallet-binding-at-mint` (three-repo lockstep) — mint-side,
  owned by mnemonic-key's next mint cycle; the converter only inherits
  today's stub semantics (A1/B1 already encode them).
- C4 files: this repo's `scripts/push-via-staging.sh` (D4), and a
  possible `mk inspect` chunk-set-id surface (r4 M2) if the `--seat`
  UX proves it needed.

## 7. Process gates (restated so the plan is self-carrying)

R0 this plan → 0C/0I. Then per phase: re-validate plan freshness →
dispatch ONE implementer (worktree `impl/converter-cN` under
`/scratch/code/worktrees/` — NEVER under `/tmp`, 32 GB tmpfs, measured
kill) → TDD, rows same-commit → the gate → scoped review where §3
names one → controller folds small findings inline. After C4: the
mandatory whole-diff adversarial review (opus), then merge, then D4's
staged push. UC stays OFF for implementation (executing a green plan
is transcription); reviews before irreversible/costly actions stay on.

## 8. Claims audit (what was machine-resolved against `7e125963`)

Measured 2026-08-30: `crates/md-cli/src/parse/keys.rs`,
`cmd/{descriptor,address}.rs`, `format/`, `parse/` exist as named;
NO `seat/` or `cmd/decompose.rs` exists (they are new);
`md-codec = { path = "../md-codec", version = "=0.42.0" }` and the
registry-not-path comment at `crates/md-cli/Cargo.toml:28,39-41`;
me-cli's `mk-codec = "0.4"` (lock 0.4.1); mk-codec crate 0.5.0 in the
mnemonic-key tree; dev-deps assert_cmd/predicates/insta/tempfile;
test files `cmd_gui_schema.rs`, `json_snapshots.rs`,
`help_examples.rs`, `duplicate_key_slots.rs` exist (the implementer
reads the last before writing V-BOUND-REF — it may be the seed);
fixtures `backup-strings.txt`/`keys.txt` at the mnemonic-engrave path
in D2; the keyed card in `journey_pathological.html` (44 tokens, 22
unique, 21×86 + one 59-char tail — measured 2026-08-30, r1 I3); the bypass message on today's plain push (verbatim in
`design/agent-reports/push-2026-08-30-converter-spec.md`). NOT yet
verified: mk-codec 0.5.0's presence on the registry (C0's entry gate)
and the exact mk-codec decode API surface (C0 resolves it against the
crate docs, not this plan).
