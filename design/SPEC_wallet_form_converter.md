# SPEC — the wallet-form converter (compose / decompose / per-slot origins)

**Status: R0 CLOSED GREEN at r9 (0C/0I/3M, Minors folded same day) —
rounds r1 (RED 3C/9I/8M/2N), r2 (RED 2C/6I/5M),
r3 (RED 4C/4I/4M — the wholesale seating-engine rewrite), r4 (RED
1C/2I/4M), r5 (RED 1C/1I/2M — the executable-principle rewrite), r6
(RED 0C/3I), r7 (RED 1C/2I — capacity deleted), r8 (RED 1C/1I/2M —
reviewed against the key-reuse ruling) all folded 2026-08-30, plus the
operator key-reuse rulings of 2026-08-30 (recorded in A3); reports in
`design/agent-reports/R0-converter-spec-r*.md`.**
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
(What checking can and cannot promise here was settled across r1-r4:
see the seating engine's A1/B1 three-disposition stub model.)

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
| **D** concrete descriptor | — | ✓ P3 (shipped this cycle) | ✓ P3+bridge (shipped this cycle) | ✓ P3 (the decomposer, shipped this cycle) |
| **T** template + key flags | ✓ P1 (shipped this cycle; inline template origins already worked — r1 I8) | ✓ P1 (shipped this cycle) | ✓ `md encode --key` (Divergent) | ✓ |
| **S** keyless card + mk1 strings | ✓ P2 (the seating engine, shipped this cycle) | ✓ P2 (shipped this cycle) | ✗ P2+bridge — `md encode --key` needs a depth-3/4 xpub, a card composes depth-0 (measured C4, filed) | — |
| **K** keyed card phrases | ✓ (round-tripped live) | ✓ | — | ✗ non-goal (first real need files it) |

✓ measured working; ⚠/✗ the gaps, tagged with the piece that closes
them. This table is byte-identical to the brainstorm's (r1 I9/N1) and
travels unchanged into the plan and the seating engine's module comment.

**Flipped at C4 close (2026-08-30)**, in the same commit as the acceptance
walks that prove them: the T row's two ⚠ P1 cells, the S row's concrete-
descriptor and addresses cells, and the whole D row. One cell this cycle owned
did NOT close, and is left ✗ carrying its measured reason: **S → keyed card**.
`md encode --key` admits only a depth-3/4 xpub, and a descriptor composed from
mk1 cards carries depth-0 keys (md rebuilds them from the 65-byte TLV, which
has no depth), so the bridge refuses — filed as
`md-cannot-mint-a-keyed-card-from-a-split-set`. Byte-identity of the four
copies is machine-checked by `scripts/matrix-identity-check.sh`.

## NORMATIVE — the seating engine (the core, and the funds-shaped part)

Given a keyless policy (from a card set or a template) and a set of keys
(from mk1 cards or flags), seating produces a total assignment slot→key
or REFUSES. **Two phases (r3 I2): PHASE A runs before any assignment is
chosen (decode, stub triage, origin matching, ambiguity); PHASE B runs
after a total assignment exists (wallet-id computation, stub
confirmation, oracles, output).** No check is cited before its phase can
compute it.

**THE PRINCIPLE (r3 C1; made EXECUTABLE at r5 — three rounds discovered
invariance axes one counterexample at a time (order r2, group-choice
r3, multiplicity and occurrence r4, use-site paths r5); the cure is
that the check IS the principle, so no axis can be missed): a card set
seats without operator input iff every complete candidate assignment
composes to the SAME WALLET — and that is DECIDED BY COMPOSING, never
by structural shortcuts.**

The decision procedure: A2 defines a bipartite SATISFACTION relation
between cards and slot declarations (r5 I1 — it is not an equivalence:
a card can satisfy two unequal declarations, e.g. `[fp/path]` and a
fingerprint-free `path`; "origin-equivalence classes" are the wrong
frame and are gone from this spec). A3 enumerates the PERFECT MATCHINGS
of that graph. Zero matchings ⇒ A4's refusals identify the unsatisfied
side. Exactly one ⇒ seat it. Several ⇒ compose each candidate
assignment, canonicalise FOR COMPARISON (an internal form that
additionally sorts keys within each `sortedmulti`/`sortedmulti_a`
group instance; byte-equality of this form is SOUND for
wallet-equality and deliberately conservative — the converse fails on
e.g. taptree branch commutation, measured, which the engine treats as
inequality and refuses; this form is never emitted — r6 M1), and
byte-compare: all equal ⇒ seat the CANONICAL matching — the one whose
ASSIGNMENT VECTOR (the slot-ordered list of seated chunk-set ids) is
lexicographically least (r7 I1: ordering by the comparison FORM cannot
discriminate — the branch is entered precisely when all forms are
byte-equal, measured identical hashes on the r6 fixture while the
emitted descriptors and WalletPolicyIds still differed; the assignment
vector differs between distinct matchings by construction, so this
order is total AND discriminating). Determinism restores B1's meaning
and the emitted text at zero cost since all candidates are proven
wallet-equal (r6 I1's measurements stand as the motivation); any pair differs ⇒ the ambiguity refusal
naming the cards, the slots, and the remedies (re-mint with
fingerprints, or `--seat`). Enumeration is bounded by TOTAL matchings
enumerated — 720, early-terminating at the 721st (r6 I2: a per-class
k! bound neither bounds the work — two independent 6-card components
are 518,400 matchings with no class over 6 — nor tracks it — an
8-card path component has 2 matchings; and "class" is the frame this
procedure deleted). Over the bound, the engine refuses stating the
bound and printing the cards and their candidate slots — graph
properties it has even when the matchings are uncounted (r6 M3) —
`--seat` being the remedy there too. Every prior round's counterexample — r2's three-orders, r3's
two-groups, r4's repeated-slot and internal-key, r5's use-site-path
swap — ships as a vector row against THIS procedure. r5-M1's
measured over-strictness case (two group instances sharing
placeholders) is REGROUNDED by the 2026-08-30 key-reuse ruling: @0 at
two positions with the same use-site path is exactly the repetition
BIP 388 forbids, so its row ships as a REFUSE row now, and the
anti-over-refusal duty it carried transfers to Acceptance 2's boundary
seat row (the fingerprint-free same-path different-masters family must
SEAT).

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

**A2 — seat by declared origin: the DECLARATION is the CONSTRAINT
(r4 I1 — symmetric equality made a legitimate set unrestorable: a
fingerprint-free policy declaration against fingerprint-bearing cards
failed every slot).** Comparison is over decoded (fingerprint, path)
values — never strings (`h`/`'`, `m/`-prefix and fingerprint case are
measured to vary across the tools' own outputs). A card satisfies a
declaration iff the PATHS match AND, where the declaration states a
fingerprint, the card's matches it; a fingerprint-FREE declaration
accepts any card at that path (the card's extra fingerprint is
information, not a mismatch). A fingerprint-free CARD satisfies only a
fingerprint-free declaration by path — a declared fingerprint is a
requirement the card cannot meet blind. **Named residue (r5 M2): a
fingerprint-free DECLARATION accepts a foreign card with the right
path and any fingerprint — CE-1-adjacent, and it is the policy
AUTHOR's accepted risk, chosen at mint time where `md encode` already
warns that fingerprint-free slots cannot be told apart; the converter
inherits that choice rather than overriding it.**

**A3 — ambiguity: THE PRINCIPLE's decision procedure runs** (perfect
matchings of A2's satisfaction graph; compose-canonicalise-compare;
the cap; the refusal naming cards by full chunk-set id, slots, and
both remedies — all as stated above). Identical declared origin with
DIFFERENT xpubs on fingerprint-bearing cards refuses (impossible from
one master — r1). **KEY REUSE IS UNSUPPORTED — operator rulings 2026-08-30.** First
ruling, verbatim: "Key reuse (meaning with same keypath) isn't
allowed." Refinement, verbatim: "Bad ideas can be valid, but we don't
want to support BIP forbidden wallets" — so the ground is NOT
invalidity: a repeated-key descriptor is technically valid script, and
this is a POLICY refusal of a BIP-forbidden shape. The authority is
BIP 388's "Additional rules" (verified against bitcoin/bips master,
2026-08-30): (1) "The public keys obtained by deserializing elements
of the key information vector must be pairwise distinct" — with the
BIP's own security footnote: "Reusing pubkeys could be insecure in the
context of wallet policies containing miniscript. Avoiding repeated
public keys altogether avoids the problem at the source."; (2) two
KEY expressions on the SAME placeholder must have DISJOINT multipath
sets ({M,N} ∩ {P,Q} = ∅) — and BIP 388's invalid-example list
includes `sh(multi(1,@0/**,@0/**))` ("Repeated keys with the same
path expression") verbatim. The converter refuses BOTH forbidden
shapes in BOTH directions — compose refuses to emit, decompose refuses
as input. Rows: shape (1) both directions; shape (2) DECOMPOSE side
only (rust-miniscript parses all three forms, so that refusal is
reachable and testable), while the compose side has no row — md's
parser refuses shape (2) upstream, so a compose-side row would pass in
both worlds (r9 M2) and the refusal is documented, not row-pinned. The
diagnostic says "forbidden
by BIP 388" / "unsupported", never "invalid". Measured scope note:
md's template surface is NARROWER than BIP 388 here and currently
INVERTS it — `md descriptor` refuses the BIP-LEGAL disjoint form
(`wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))` → "@0 appears with
inconsistent path/multipath/hardening") while composing the
BIP-FORBIDDEN same-path form (`wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))`
composed clean; both measured 2026-08-30) — so shape (2) is
unreachable through md today and its refusal is recorded for
completeness, while shape (1) (the same xpub filling two slots) is the
reachable case where the engine's refusal binds. Three consequences,
each a simplification:
(a) each card fills exactly ONE slot and there is no repeated-key
restore case at all — the r6/r7 collapse/capacity/supply-twice
machinery is DELETED, not repaired. An accidental double-scan is made
harmless BY ORDER OF OPERATIONS, not by assumption (r8 I1 — the naive
"dedupes harmlessly" claim was measured false: `mk decode S1 S2 S1 S2`
→ "error: chunked-header malformed: received 4 chunks, header
declares total_chunks = 2"). P2's input pipeline is normative: (1)
dedupe input strings that name the same card, after normalising
display separators AND case; (2) group the survivors by
declared chunk-set id (r8 M2 — the grouping rule the tie-break's
totality depends on); (3) reassemble each group under `mk decode`
semantics, so a merged id-collision group still refuses at reassembly
(measured: "received 5 chunks, header declares total_chunks = 2") and
the seating engine never sees colliding cards. A full card string set
supplied twice over ships as a must-SEAT row;
**Post-GREEN fold, REVIEW-converter-whole-diff-r1 I2, measured
2026-08-30:** step 1 said BYTE-identical, and mk1 strings are
bech32 — an all-uppercase card set seats to the identical
descriptor, so a byte-identity key did not recognise one card
scanned twice, once in each case. Those two survivors merged into
one group at step 2 and refused at step 3 with "Two DIFFERENT cards
pinned to one chunk-set id … re-mint one of them", which diagnoses
the wrong problem and prescribes re-engraving a good plate. Step 1
normalises display separators AND case; whole-string mixed-case
rejection stays the decoder's.

(b) r7-C1's missing-card fabrication (`sortedmulti(2,X,X,Y)` from a
2-of-3 missing Z — measured, X alone controlling funds) is doubly
dead: A4's unfilled-slot refusal AND the BIP-388-unsupported refusal —
both ship as permanent must-REFUSE rows;
(c) a policy declaring two fingerprint-BEARING slots with the
identical (fingerprint, path) is REFUSED AT THE DOOR — its only
possible fill binds one xpub to two slots, which rule (1) forbids —
refused with that explanation. The legitimate same-path family
survives untouched: fingerprint-free declarations across DIFFERENT
masters (privacy-preserving multisig) are pairwise-distinct keys, not
reuse.
A3 enumerates PERFECT MATCHINGS exactly as its normative sentence says
(r7 I2). The P3 write-side handling of a repeated-key INPUT descriptor
becomes a refusal (BIP-forbidden, unsupported — never "invalid"),
replacing the earlier collapse rule; its row flips accordingly.
`md descriptor`/`md encode`'s current acceptance of the same-path form
predates the ruling and is FILED as an md-side question rather than
changed by this spec (see FOLLOWUPS).

**A4 — completeness is total.** Every slot filled, every supplied key
seated. Unfilled slot: refuse naming the slot and its declared origin.
Leftover key: refuse naming the card AND its stub (the drawer-scan
operator's question is "which wallet do these extras belong to").

**A5 — `--seat`, defined (r3 C4).** `--seat '@i=<chunk-set-id>'`
(repeatable): the referenced card must satisfy slot i's declaration
under A2 — that clause carries the whole safety argument, so a
consistent `--seat` on a NON-ambiguous slot is simply satisfied (r4 M1
dropped the was-it-part-of-the-refusal conjunct: it protected nothing
and broke scripting `--seat` for every slot in a mixed run). `--seat`
can never place a card A2 would not, never suppresses A1/B1 stub
dispositions, and never fills A4 gaps. The
chunk-set id is the card's full decoded set id — the exact label the A3
refusal printed, never a string prefix (r3 measured prefix collisions
at 6 characters on the 11-card fixture). The label is md-side: no mk
surface prints it today (r4 M2 — a follow-up may surface it in
`mk inspect`; until then the refusal text is the operator's source).
`mk encode --chunk-set-id` can PIN two cards to one 20-bit id, so the
ambiguous-id refusal is deliberately reachable, and its vector row says
so rather than relying on birthday luck. Unknown id, ambiguous id, or a
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
override). An origin-notated `--key` path must agree with the path
source that WINS for its slot — the inline template origin where the
slot declares one, else the shared `--path` — and a slot with NEITHER
refuses rather than discarding the bracket path, naming the inline
origin and `--path` as the channels that can state it.
**Post-GREEN fold, REVIEW-converter-whole-diff-r1 I1, measured
2026-08-30:** this sentence bound bracket-vs-INLINE only, and the two
cases it left out both shipped at exit 0 — `--path` silently OVERRODE a
bracket path (slot @1 declared `[73c5da0a/48'/0'/0'/2']` for a key the
operator had said was at `48'/0'/1'/2'`), and a bracket path with no
source at all emitted a truncated `[73c5da0a]` origin on a depth-0
xpub, which BIP-380 reads as "this key IS master 73c5da0a". Whether the
bracket should instead become a last-resort path SOURCE is an
accepting-on-input widening and is FILED, not decided here. Two refusal-message fixes ride here: the inline-origin parse path
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
B2 address to stderr. **P2's input pipeline is normative as stated in
A3(a): dedupe separator- and case-normalised strings, THEN group by declared chunk-set
id, THEN reassemble under `mk decode` semantics — a reader scoping P2
from this paragraph budgets that ordering here (r9 M3).** **P2 also
ships the SPEND-EQUALITY checker**
(r3 M3 — it is B2's split-vs-keyed "agree" and acceptance 1's relation
(a); a reader scoping P2 from this paragraph budgets it here) and the
`--seat` flag (A5). The keyed-card output was SPECIFIED here as the
two-command bridge (compose → `md encode --key`) needing no new
surface — **measured FALSE at C4 (2026-08-30): a descriptor composed
from mk1 cards carries depth-0 xpubs (the Pubkeys TLV holds 65 bytes,
no depth) and `md encode --key` admits only depth 3/4, so the bridge
refuses from both ends.** The S → keyed-card matrix cell stays ✗ with
that reason, and the gap is filed as
`md-cannot-mint-a-keyed-card-from-a-split-set` (post-converter md-cli
mini-cycle).

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
- **Repeated keys — REFUSED as BIP-forbidden (operator rulings
  2026-08-30, superseding r1 M4's collapse):** a concrete descriptor
  carrying the same xpub at two positions is technically valid script
  that BIP 388 forbids (pairwise-distinct rule) — decompose refuses it
  by name as unsupported, never as invalid. A vector row pins the
  refusal.
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
dependency; only `compile` needs the `cli-compiler` feature). No mk CODE changes in this
cycle (r4 M4) — but R0 DID find the mk-side action: the filed
`stub-keyed-wallet-binding-at-mint` three-repo lockstep with its
canonical-origin design obligation; the mirror entries exist in both
FOLLOWUPS.

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
2. Every seating refusal, every B1 disposition, AND every
   PROVEN-FREE-SEAT case (the rows that must SEAT — the
   fingerprint-free same-path different-masters family, which is the
   SEAT side of the key-reuse boundary and carries r5-M1's
   anti-over-refusal duty, its REFUSE side (the same xpub at two
   slots) sitting adjacent so the boundary is pinned from BOTH
   directions; and the mixed-declaration unique matching — r6 M2)
   demonstrated by a vector row that FAILS if
   the behaviour is removed or inverted — including CE-1's
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
