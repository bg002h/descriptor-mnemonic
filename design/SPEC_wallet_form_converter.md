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
   path flag is the SHARED `--path`, and this vault's slots declare four
   different accounts. (`md encode` has per-slot Divergent mode; its
   read-side siblings do not — an asymmetry, not a design.)
3. `--key '@i=[fp/path]xpub'` (origin notation, mk's own file format) —
   *"base58check decode"* — the value is parsed as a bare xpub only.

Meanwhile the KEYED card (22 chunks, Pubkeys TLV) round-trips today:
`md descriptor <keyed phrases>` → the full 1,649-char concrete descriptor
(`…#xn3k4jmt`). And the reverse entrance — a concrete miniscript
descriptor as INPUT — does not exist in any tool (`me` stops at the seven
plain forms by the S2 seam rule; `md encode` takes templates).

A session-written composition script (decode cards → seat by declared
origin → substitute) produced the correct descriptor, and is the working
sketch of the engine below. It also exposed the danger: it did NOT verify
the policy-id stubs. A composition that seats an unbound key card can
reconstruct a DIFFERENT wallet — the "same-path keys cannot be seated"
class, now on the compose path.

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
(from mk1 cards or flags), seating produces a total assignment slot→key or
REFUSES. Rules, in order:

1. **Stub check — SHAPE binding, stated honestly (r1 C1's ruling).** The
   mk1 `policy_id_stubs` field is derived from the keyless template id
   (WalletDescriptorTemplateId), which is invariant to keys, origins and
   fingerprints — measured: two different wallets minted the same stub
   `a235ee75`. So the stub proves only *"this card was minted against a
   policy of this SHAPE"*. The check: at least one of the card's stubs
   (the field is `Vec<[u8;4]>`, r1 I1 — any-of) matches the policy
   card's template id. A mismatch refuses — *"key card `<prefix>` was
   minted for a different policy SHAPE (stub `xxxx`; this template is
   `yyyy`)"* — catching cross-shape mistakes cheaply. **What it does NOT
   catch, and the spec says so where the operator reads it: a foreign
   card from a same-shape wallet seats cleanly.** That residual is
   IDENTICAL to physical-card restore and is caught the same way: the
   address-0 verification in rule 6. CE-1 (the reviewer's same-stub
   foreign card) ships as a PERMANENT vector row asserting exactly this
   accepted behaviour — it seats, and the derived address differs from
   the intended wallet's (r1 M6: the sharing case, not the trivial one).
   The true-binding upgrade (mint-time keyed-WalletPolicyId stubs) is
   FILED as `stub-keyed-wallet-binding-at-mint` (primary: mk's
   FOLLOWUPS; companion: this repo's) — and per the operator ruling
   recorded there (2026-08-30: no engraved plates exist besides test
   plates; backward compatibility does not matter until v1.0), it is
   compat-FREE if landed pre-v1.0. When it lands, this rule tightens
   and CE-1's row flips to a refusal, in lockstep. Out of scope here.
2. **Seat by declared origin, compared as DECODED VALUES (r1 I7).**
   Slot `@i` declares an origin; a key seats at `@i` iff its decoded
   (fingerprint, path) equals the slot's decoded declaration. Never
   string comparison: `h`/`'`, `m/` prefix presence, and fingerprint
   case are measured to vary across md's and mk's own outputs.
3. **Ambiguity: refuse where assignment changes the wallet, seat
   deterministically where it cannot (r1 I2).** Identical declared
   origin with DIFFERENT xpubs on fingerprint-bearing cards refuses
   (impossible from one master: a collision or corruption — r1 confirmed
   this half). For fingerprint-less (privacy-preserving) or shared-path
   cards matching a group of interchangeable slots: within a
   `sortedmulti` group, assignment is immaterial by construction — seat
   in supplied order silently. Within a `multi` group, order changes the
   script: seat in SUPPLIED order, say so on stderr — *"multi(): key
   order taken from input order; verify address 0 before trusting"* —
   because the cards themselves carry no tiebreak and only the address
   check can confirm intent. Identical (origin, xpub) pairs deduplicate
   to one key (see repeated-slot rule in P3).
4. **Privacy-preserving cards** (no fingerprint) seat by path under rule
   3's group rules; refuse only when the path matches slots in more than
   one non-interchangeable group, naming `--privacy-preserving` as the
   reason the tiebreak is unavailable.
5. **Completeness is total.** Every slot filled and every supplied key
   seated. An unfilled slot refuses naming the slot and its declared
   origin; an unseated leftover key refuses naming the card AND its
   stub (r1 M7 — the drawer-scan operator's question is "which wallet do
   these extras belong to", and the stub is already decoded).
6. **Post-seat verification — automated where an oracle exists, human
   where none does (r1 M2).** For input D, the composed result's address
   0 MUST equal the input descriptor's own derivation (automated; a
   mismatch is an internal error, refuse loudly). For the split set,
   when a keyed card of the same wallet is also supplied the two
   compositions MUST agree (automated cross-check). Otherwise nothing
   external exists: the output prints address 0 **on stderr** (stdout
   remains the machine contract — descriptor only) with the standing
   instruction *"compare against your wallet software before trusting"*.
   The me §5.4 discipline, inherited.

Every rule above ships as executable vector rows in the SAME commit as
its implementation — disjuncts as rows, not prose (the S2
classifier-precision lesson). Acceptance 2 (mutation-shaped: every
refusal's row FAILS if the refusal is removed) covers each, INCLUDING
CE-1's accepted-limitation row.

## The three pieces

**P1 — per-slot origins on the read side: DOCUMENT and DEFINE, not build
(r1 I8 — the premise was wrong).** Inline template origins
(`[fp/path]@i/...`) already work on `md descriptor`/`md address` —
measured on all 11 pathological slots. What is missing is (a) the
origin-notated `--key '@i=[fp/path]xpub'` FLAG form (today parsed as a
bare xpub — measured refusal), and (b) a defined PRECEDENCE among the
three origin sources. The rule: inline template origins are
authoritative; `--fingerprint @i=` must AGREE with an inline origin when
both name slot i (mismatch refuses — never silent override); the shared
`--path` applies only to slots with no inline origin. P2's refusal
message fix rides here too (r1 M8): the keyless-phrases refusal today
prescribes `--key @i=XPub`, which its own constraint rejects — once P2
exists the message points at `--from-mk1`.

**P2 — the split set composes (the S row).** `md descriptor` and
`md address` accept `--from-mk1 <STRING>` (repeatable; or
`--from-mk1-file <FILE>`, one string per line) TOGETHER WITH keyless md1
phrases. The seating engine runs; descriptor to stdout, notes and the
rule-6 address to stderr. The keyed-card output needs no new surface:
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

## Non-goals## Non-goals

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
commit; one implementer; adversarial review before merge. `decompose`
sits behind `cli-compiler`; P1/P2 ship ungated. mk is untouched; the
cross-repo mirror rule applies only if R0 finds an mk-side action.

## Acceptance

1. **The pathological round trip, as WALLET equality — never descriptor
   string equality (r1 C2: the split set composes to 1,901 chars
   `#s5a2k003` with 11 origin brackets, the keyed card to 1,648 chars
   `#xn3k4jmt` with none — same wallet, different spellings, both
   measured).** The walk: (a) the 36-string split set composes; (b) the
   22-chunk keyed card composes; (c) EQUALITY = address 0 and 1 on both
   chains agree across (a), (b), and the original journey derivation
   (`bc1qkuknuy6…` receive 0), and the decoded key sets agree as values;
   (d) `md decompose` of a depth-consistent coordinator-grade concrete
   descriptor emits a template + key file that `md encode --key` +
   `mk encode --keys` accept and that re-compose to the same wallet by
   the same equality. (The keyed-card-derived descriptor is a known
   DEGENERATE decompose source — depth-0 re-serialised keys, r1 C3 — and
   is excluded from (d) by name.)
2. Every seating refusal in the NORMATIVE section demonstrated by a
   vector row that FAILS if the refusal is removed — including CE-1's
   accepted-limitation row (a same-stub foreign card seats and the
   derived address differs; the row asserts BOTH halves).
3. The stub-shape refusal proven with a cross-shape foreign card, AND
   the same-shape case proven to seat (r1 M6: the trivial mismatch
   alone passes in both worlds and tests nothing).
4. Reproduction notes pinned once: the keyed card is 22 strings (21×86
   chars + one 59-char tail, r1 N2); the composed keyed-card descriptor
   is 1,648 characters (1,649 bytes with trailing newline, r1 M1).
