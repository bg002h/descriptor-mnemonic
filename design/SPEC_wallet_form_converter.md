# SPEC — the wallet-form converter (compose / decompose / per-slot origins)

**Status: DRAFT — R0 r1 (RED 3C/9I/8M/2N,
`design/agent-reports/R0-converter-spec-r1.md`) folded 2026-08-30.**
Repo: descriptor-mnemonic (`md`), with one measured touch-point in
mnemonic-key (`mk decode` output is consumed as-is; no mk changes).
Baselines: md `6c4a56fd`, mk `93cebfb` (crate 0.13.0 — r1 M3: the
format version is not a rev), rust-miniscript pin `ff4732e`, evidence
session 2026-08-30 (mnemonic-engrave
`design/CONTINUITY_2026-08-29-s2.md`, the post-S2 bench night). This
repo has no `scripts/` gate helpers; this spec carries no rust blocks,
so its build gate is CLI-command-shaped and the plan must say so.

## Motivation, measured

Working the pathological vault's real backup (36 strings: one keyless
6-chunk md1 policy card + eleven mk1 key cards), the session hit three
refusals that together mean **the split card set cannot be composed into a
concrete descriptor by the shipped CLI**:

1. `md descriptor <keyless phrases>` — *"this card is a keyless TEMPLATE,
   which has no concrete form. Supply --key @i=XPUB"* — and supplying
   `--key` does not change the answer (the flag pairs only with
   `--template`).
2. `md descriptor --template <tpl> --key @i=xpub --fingerprint @i=hex` —
   *"non-canonical wrapper requires explicit origin for @0"* — the only
   path FLAG is the shared `--path`, and this vault's slots declare four
   different accounts. (Per-slot paths DO work inline in the template
   itself — r1 I8 — so the gap is the flag form and the missing
   origin-notated `--key`, not the capability; P1.)
3. `--key '@i=[fp/path]xpub'` (origin notation, mk's own file format) —
   *"base58check decode"* — the value is parsed as a bare xpub only.

Meanwhile the KEYED card (22 chunks, Pubkeys TLV) round-trips today:
`md descriptor <keyed phrases>` → the full 1,648-char concrete descriptor
(`…#xn3k4jmt`). And the reverse entrance — a concrete miniscript
descriptor as INPUT — does not exist in any tool (`me` stops at the seven
plain forms by the S2 seam rule; `md encode` takes templates).

A session-written composition script (decode cards → seat by declared
origin → substitute) produced the correct descriptor, and is the working
sketch of the engine below. It also exposed the danger class: a
composition that seats a wrong key card reconstructs a DIFFERENT
wallet — "same-path keys cannot be seated", now on the compose path.
(What checking can and cannot promise here was settled by r1 C1 and
r2 I1: see the seating engine's two-tier rule 1.)

## The principle (operator, 2026-08-30, verbatim intent)

> "Our goal is to be accepting on input and expressive on output, but we
> require complete input."

## The surface: one matrix

**THE MATRIX TRAVELS (operator directive, 2026-08-30): this table is the
cycle's goal-and-gaps statement and is embedded, cells kept current, in
EVERY artifact — brainstorm, this spec, the implementation plan, and the
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
them. This table is byte-identical to the brainstorm's (r1 I9/N1) and
travels unchanged into the plan and the seating engine's module comment.

## NORMATIVE — the seating engine (the core, and the funds-shaped part)

Given a keyless policy (from a card set or a template) and a set of keys
(from mk1 cards or flags), seating produces a total assignment slot→key
or REFUSES. **Two phases (r3 I2): PHASE A runs before any assignment is
chosen (decode, stub triage, origin matching, ambiguity); PHASE B runs
after a total assignment exists (wallet-id computation, stub
confirmation, oracles, output).** No check is cited before its phase can
compute it.

**THE PRINCIPLE (r3 C1, replacing per-position classification): a card
set seats without operator input iff EVERY complete candidate assignment
yields the SAME wallet.** The hazard unit is the assignment, not the
position: two `sortedmulti_a` groups of different thresholds are each
internally order-free, yet a card's choice BETWEEN them was measured
giving three wallets — one placing a card in a 1-of-2 unilateral-spend
leaf. Practically: cards and slots are grouped by declared origin
(decoded values, rule A2); within one origin-equivalence class, free
seating requires every candidate slot to lie in ONE sorted group
(`sortedmulti`/`sortedmulti_a` — permutation-invariance measured for
the former, a vector row proves the latter before the rule relies on
it). Candidates spanning two groups (sorted or not), touching any
`multi`/`multi_a` position, a taproot internal key, or any position the
classifier cannot place (r2 I3 — the classification is exhaustive over
pk/pkh fragments, sorted groups, unsorted groups, and the internal-key
position; unplaceable refuses) ⇒ the ambiguity refusal.

### PHASE A

**A1 — stub TRIAGE (records, never refuses alone — r3 C3).** Decode
each card's `policy_id_stubs` (`Vec<[u8;4]>`, any-of). A stub matching
the TOP 4 BYTES of the policy card's 16-byte template id marks the card
*shape-matched*. A stub matching nothing yet is *unconfirmed* — NOT
refused, because legitimate mismatches are measured: a card minted
`--from-md1 <keyed card>` carries a WalletPolicyId-rooted stub
(`232214e4` on the fixture) that matches neither the template id
(`5b48af35`) nor the WalletPolicyId the composer will compute under the
split set's own origin declarations (`ced22709`) — same wallet, three
values, all legitimate (WalletPolicyId is origin-sensitive and this
fixture carries two origin declarations). Final stub disposition is
B1's.

**A2 — seat by declared origin, compared as DECODED VALUES.** Slot
declarations and card origins compare as decoded (fingerprint, path)
values — never strings (`h`/`'`, `m/`-prefix and fingerprint case are
measured to vary across the tools' own outputs). A card seats at a slot
iff the values match; fingerprint-less cards match fingerprint-free
declarations by path.

**A3 — ambiguity: the PRINCIPLE decides.** Where all of an
origin-class's candidates lie in one sorted group, seat in supplied
order (any order is the same wallet — that is the proof obligation,
carried by vector rows). Everywhere else, ambiguity REFUSES, naming the
cards (by full chunk-set id), the candidate positions, and both
remedies: re-mint with per-slot fingerprints, or `--seat` (A5).
Identical declared origin with DIFFERENT xpubs on fingerprint-bearing
cards refuses (impossible from one master — r1). Identical
(origin, xpub) pairs collapse to one key, on the policy side too
(r2 M3), pinned by a row.

**A4 — completeness is total.** Every slot filled, every supplied key
seated. Unfilled slot: refuse naming the slot and its declared origin.
Leftover key: refuse naming the card AND its stub (the drawer-scan
operator's question is "which wallet do these extras belong to").

**A5 — `--seat`, defined (r3 C4).** `--seat '@i=<chunk-set-id>'`
(repeatable) resolves A3 ambiguity ONLY: the referenced card must
already origin-match slot i under A2 and be part of the refused
ambiguity — `--seat` can never place a card A2 would not, never
suppresses A1/B1 stub dispositions, and never fills A4 gaps. The
chunk-set id is the card's full decoded set id — the exact label the A3
refusal printed, never a string prefix (r3 measured prefix collisions
at 6 characters on the 11-card fixture). Unknown id, ambiguous id, or a
`--seat` contradicting A2 refuses by name. Vector rows: an A3 refusal
resolved by `--seat`; a `--seat` contradicting A2 refused (r3 M2).

### PHASE B

**B1 — stub DISPOSITION.** With the assignment total, compute the
composed wallet's WalletPolicyId under the supplied origin
declarations. Per card: stub matches the WalletPolicyId top-4 ⇒
*wallet-confirmed* (true binding — a foreign card cannot reach this
tier; CE-1 impossible for it). Else stub shape-matched (A1) ⇒
*shape-confirmed* (CE-1's accepted limitation applies). Else ⇒ a
WARNING, never a hard refusal (r3 C3): *"card `<set-id>`'s stub
matches neither this policy's shape id nor the composed wallet id —
minted under different origin metadata (legitimate), or a different
wallet; verify address 0 before trusting."* Both readings named, the
human check directed. Vector rows pin all three dispositions.

**B2 — oracles where they exist, the human where none does.** Input D:
the composed result must match the input descriptor's own derivation
(automated; mismatch is an internal error). Split set WITH a keyed card
supplied: the two compositions must be SPEND-EQUAL (the acceptance's
relation (a); the checker ships in P2). Otherwise: address 0 printed on
stderr (stdout stays the machine contract) with the standing
instruction *"compare against your wallet software before trusting"* —
this SURFACES the CE-1 residue for human comparison; nothing in this
engine can catch it alone (r3 M1 — "caught" was an overstatement and is
withdrawn).

Every rule above ships as executable vector rows in the SAME commit as
its implementation — disjuncts as rows, not prose. Acceptance 2
(mutation-shaped) covers each, including CE-1's accepted-limitation row
(scoped to cards that are not wallet-confirmed).

## The three pieces

**P1 — per-slot origins on the read side: DOCUMENT and DEFINE, not build
(r1 I8; premise corrected again by r3 I1).** Inline template origins
already work on `md descriptor`/`md address` — measured on all 11
pathological slots — in the spelling md actually parses: a PATH-only
inline origin with `'` hardening (`@i` plus the template's origin
note); the bracketed `[fp/path]@i/...` spelling does NOT parse and is
not the mechanism. What is missing is (a) the
origin-notated `--key '@i=[fp/path]xpub'` FLAG form (today parsed as a
bare xpub — measured refusal), and (b) a defined PRECEDENCE. Corrected
per r2 I2: an inline template origin carries a PATH only, never a
fingerprint (and the r1-fold's `[fp/path]@i/...` spelling does not
parse — md refuses it by name), so the sources do not overlap on the
same datum and precedence is per-DATUM: paths come from inline template
origins where present, else shared `--path`, else refuse for
non-canonical wrappers (today's rule); fingerprints come from
`--fingerprint @i=` or the origin-notated `--key` form, and when BOTH
name slot i they must agree (mismatch refuses — never silent
override). An origin-notated `--key` path must agree with the slot's
inline path when both exist. Two refusal-message fixes ride here: the inline-origin parse path
accepts `h` spellings or refuses POINTING AT the `'` requirement (r2
M4 — today `48h/...` inline draws an unrelated multipath complaint,
the F-420 class, and P1 makes this path load-bearing); and P2's
keyless-phrases message fix (r1 M8): the keyless-phrases refusal today
prescribes `--key @i=XPub`, which its own constraint rejects — once P2
exists the message points at `--from-mk1`.

**P2 — the split set composes (the S row).** `md descriptor` and
`md address` accept `--from-mk1 <STRING>` (repeatable; or
`--from-mk1-file <FILE>`, one string per line) TOGETHER WITH keyless md1
phrases. The seating engine runs; descriptor to stdout, notes and the
B2 address to stderr. **P2 also ships the SPEND-EQUALITY checker**
(r3 M3 — it is B2's split-vs-keyed "agree" and acceptance 1's relation
(a); a reader scoping P2 from this paragraph budgets it here) and the
`--seat` flag (A5). The keyed-card output needs no new surface:
the spec documents the two-command bridge (compose → `md encode --key`),
sugar filed only if the bridge proves annoying.

**P3 — the concrete descriptor becomes an entrance (the D row).**
`md decompose <DESCRIPTOR|--in FILE>`: parses a concrete descriptor via
rust-miniscript — **UNGATED (r1 I4): parsing needs no feature; only
`compile` needs `miniscript/compiler`; verified with a compiler-free
probe** — and emits: the keyless template, origin-notated key lines
(one per line, a valid `mk encode --keys` file), and the per-slot
`--fingerprint` flags. Details forced by measurement:

- **Key emission is round-trip-grade (r1 C3):** key lines carry the key
  AS PARSED — true depth, child number and origin from the input, never
  a re-serialised depth-0 form (mk's own depth-consistency check
  refuses those, measured). A key whose depth/origin are inconsistent
  IN THE INPUT refuses at decompose with mk's constraint named.
- **Origin-less keys (r1 I5):** emitted as bare key lines, EXCLUDED
  from the mk-mintable set, and `--emit commands` refuses when any key
  lacks an origin — naming the keys and the reason (mk1 cards bind key
  to origin by design; a card cannot be minted for an origin the input
  never stated). The template and descriptor outputs still work.
- **Repeated keys (r1 M4):** identical (origin, xpub) appearing in
  multiple positions collapses to ONE slot referenced multiply
  (matching `md encode`'s accepted `@0…@0` form) — on read AND write,
  so compose∘decompose is stable for such descriptors; a vector row
  pins the choice and the WalletPolicyId consequence.
- **New walker, not `compile`'s (r1 M5):** the existing substitution
  machinery strips placeholders to bare synthetic xpubs and its drift
  guard forbids `MultiXPub` — but every multipath key parses AS
  `MultiXPub`. The plan budgets a fresh walker; this is the largest
  single piece of P3.
- **Input boundary (r1 I6):** D accepts a bare descriptor string (with
  or without checksum; multipath or fixed-path). Core's
  `listdescriptors` JSON and separate receive/change descriptor PAIRS
  refuse with guidance (extract the string; combine the pair to
  `<0;1>`) — extraction stays the operator's or a future front-end's
  job, and the refusal must not be the bare checksum error measured
  today (the F-420 class).

## Canonicalisation

Emitted descriptors use the `'` hardened spelling — md's measured,
shipped emission (44 apostrophes, zero `h`-forms in the composed
pathological descriptor) — preserve input key order, and always
recompute and append the checksum. **The r1 draft claimed `h`-spelling
and byte-comparability with `me`'s canonical form; both were FALSE by
measurement (r1 I3: `me` emits `h`, md emits `'`, rust-miniscript's
Display emits `'`, and the spelling changes the checksum).** Stability
promise is WITHIN md only. Cross-repo canonical unification, if ever
wanted, is its own breaking-change decision and is out of scope.

## Non-goals

- `me sysw pack --as md1` accepting miniscript (touches the S2-settled
  admission predicate; its own spec-amendment + R0 cycle if ever).
- Crowning either card form: keyed (shorter, monolithic) and split
  (distributable custody) are peers; the converter makes moving between
  them cheap and recommends neither.
- Any device/fork change. Any wire-format change (Tag/TLV untouched).
- K→S splitting (keyed card → keyless + mk1 cards): mechanical but no
  motivating journey yet; follow-up on first real need.

## Gates and process

Seating semantics are restore-correctness — funds-shaped — so: R0 review
of this spec to 0C/0I before code; vectors from the first implementation
commit; one implementer; adversarial review before merge. ALL three pieces ship
UNGATED (r2 I4 unified the three contradictory statements to P3's
measured ruling: parsing needs `miniscript`, an unconditional
dependency; only `compile` needs the `cli-compiler` feature). mk is untouched; the
cross-repo mirror rule applies only if R0 finds an mk-side action.

## Acceptance

1. **The pathological round trip, with TWO equality relations (r3 C2 —
   one relation was doing two irreconcilable jobs: the fixture's split
   set and keyed card declare DIFFERENT legitimate origin metadata, so
   an origin-including relation fails nine of eleven slots on the
   spec's own walk).**
   **SPEND-EQUALITY** (cross-form): canonicalised template structures
   equal AND per-slot xpub values and use-site paths equal — origin
   metadata EXCLUDED (it is seating/signing guidance, not script
   content). **ROUND-TRIP-EQUALITY** (decompose∘compose): spend-equality
   AND origin metadata preserved exactly.
   The walk: (a) the 36-string split set and (b) the 22-chunk keyed
   card compose to SPEND-EQUAL wallets, both matching the journey
   derivation's addresses (`bc1qkuknuy6…` receive 0) as confirmation;
   (c) `md decompose` of the PINNED depth-consistent fixture (r2 M5:
   the wallet built from the first three lines of the pathological
   `keys.txt`) emits a template + key file that `md encode --key` +
   `mk encode --keys` accept and that re-compose ROUND-TRIP-EQUAL.
   (The keyed-card-derived descriptor stays excluded from the decompose
   leg BY NAME — depth-0 re-serialised keys, r1 C3.)
2. Every seating refusal AND every B1 disposition demonstrated by a
   vector row that FAILS if the behaviour is removed — including CE-1's
   accepted-limitation row, scoped to cards that are not
   wallet-confirmed (r3 I3): a same-stub foreign card seats and the
   derived address differs; the row asserts BOTH halves.
3. The three B1 dispositions each proven on a fixture (r3 I3, replacing
   the r2 wording C3 falsified): a cross-shape card draws the
   unconfirmed WARNING (not a refusal); a same-shape shape-tier card
   seats; a keyed-mint card whose declarations match the composition is
   wallet-confirmed; and the fixture's own keyed-mint card
   (`232214e4` vs `ced22709`) draws the warning with both readings
   named — the C3 counterexample as a permanent row.
4. Reproduction notes pinned once: the keyed card is 22 strings (21×86
   chars + one 59-char tail, r1 N2); the composed keyed-card descriptor
   is 1,648 characters (1,649 bytes with trailing newline, r1 M1).
