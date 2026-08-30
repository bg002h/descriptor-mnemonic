# SPEC — the wallet-form converter (compose / decompose / per-slot origins)

**Status: DRAFT — R0 not started.** Repo: descriptor-mnemonic (`md`), with
one measured touch-point in mnemonic-key (`mk decode` output is consumed
as-is; no mk changes). Baselines: md `6c4a56fd`, mk `mk1 v0.1 HEAD`,
evidence session 2026-08-30 (mnemonic-engrave
`design/CONTINUITY_2026-08-29-s2.md`, the post-S2 bench night).

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

| in \ works today | descriptor | addresses | keyed card |
| --- | --- | --- | --- |
| D | — (this spec: `md decompose` feeds the others) | ✗ | ✗ |
| T | ⚠ shared-path only | ⚠ same | ✓ `md encode --key` (Divergent) |
| S | ✗ (this spec: the seating engine) | ✗ | ✗ |
| K | ✓ | ✓ | — |

This spec closes every ✗/⚠ with THREE pieces, one normative core.

## NORMATIVE — the seating engine (the core, and the funds-shaped part)

Given a keyless policy (from a card set or a template) and a set of keys
(from mk1 cards or flags), seating produces a total assignment slot→key or
REFUSES. Rules, in order:

1. **Stub binding first.** When keys arrive as mk1 cards alongside an md1
   policy card: every card's `policy_id_stub` MUST match the policy card's
   stub. A mismatched card is refused by name — *"key card `<prefix>` is
   bound to a different wallet policy (stub `xxxx` ≠ `yyyy`)"* — before
   any seating is attempted. A key that would seat perfectly but fails the
   stub check is the attack/mistake this rule exists for.
2. **Seat by declared origin, exactly.** Slot `@i` declares
   `[fingerprint/path]` (Divergent) or path-only; a key seats at `@i` iff
   its origin equals the declaration. String-exact after `h`/`'`
   normalisation; no fuzzy matching, no path suffixing.
3. **Ambiguity refuses.** Two unseated keys matching one slot, or one key
   matching two unfilled slots with no exact tiebreak, is a refusal naming
   the colliding origins — never a guess. (Cards with identical origin AND
   identical xpub deduplicate silently; identical origin with different
   xpubs is the unseatable-backup defect and the refusal says so.)
4. **Privacy-preserving cards** (no fingerprint) seat by path alone iff
   exactly one unfilled slot declares that path fingerprint-free;
   otherwise refuse, naming `--privacy-preserving` as the reason the
   tiebreak is unavailable.
5. **Completeness is total.** Every slot filled and every supplied key
   seated. An unfilled slot refuses naming the slot and its declared
   origin; an unseated leftover key refuses naming the card. "Accepting on
   input" never means partial: a wallet with a hole is not a wallet.
6. **Post-seat verification.** The composed descriptor re-derives address
   0; when the input was a card set, nothing external exists to compare —
   so the output ALWAYS prints address 0 beside the descriptor, and the
   operator instruction ("compare against your wallet software before
   trusting") travels with it. The me §5.4 discipline, inherited.

Every rule above ships as executable vector rows in the SAME commit as
its implementation — disjuncts as rows, not prose (the S2
classifier-precision lesson).

## The three pieces

**P1 — per-slot origins on the read side** (closes the ⚠ row). `--key`
learns origin notation: `--key '@i=[fp/path]xpub'` — the exact syntax
`mk encode --keys` files already use — accepted by `md descriptor` and
`md address` in template mode. Bare-xpub `--key` stays valid where the
wrapper is canonical or `--path` covers it. (Write side already has
Divergent; this is symmetry, not new semantics.)

**P2 — the split set composes** (closes the S row). `md descriptor` and
`md address` accept `--from-mk1 <STRING>` (repeatable, or one
space/newline-separated file via `--from-mk1-file`) TOGETHER WITH keyless
md1 phrases. The seating engine runs; output as today. The keyed-card
output needs no new surface: the spec documents the two-command bridge
(compose → `md encode --key …`), and a follow-up may later add
`--emit keyed-card` sugar if the bridge proves annoying.

**P3 — the concrete descriptor becomes an entrance** (closes the D row).
`md decompose <DESCRIPTOR|--in FILE>`: parses a concrete descriptor
(rust-miniscript; feature-gated `cli-compiler` like `compile`), extracts
each key with its origin, substitutes `@0..@N` in first-appearance order,
and emits: the keyless template, the origin-notated key lines
(`[fp/path]xpub`, one per line — a valid `mk encode --keys` file), and
the per-slot `--fingerprint` flags. Output is EXECUTABLE: with
`--emit commands` it prints the exact `md encode` + `mk encode`
invocations that mint the card set. Round trip is the acceptance:
`decompose(compose(S)) ≡ S`'s wallet and `compose(decompose(D)) ≡ D`
canonicalised.

## Canonicalisation

Emitted descriptors use `h` hardened spelling and preserve input key
order (no sorting — `sortedmulti` sorts at spend time, not in the
descriptor text); checksum always recomputed and appended. These follow
`me`'s S1 cascade canonical form so the two repos' canonical descriptors
are byte-comparable. R0 may challenge.

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
commit; one implementer; adversarial review before merge. `decompose`
sits behind `cli-compiler`; P1/P2 ship ungated. mk is untouched; the
cross-repo mirror rule applies only if R0 finds an mk-side action.

## Acceptance

1. The pathological round trip, both directions, as the walk: the
   36-string split set composes to the same wallet the 22-chunk keyed
   card yields today (same descriptor modulo canonicalisation, same
   address 0 `bc1qkuknuy6…`, checksum recomputed), and `md decompose` of
   that descriptor re-emits a template + key set that re-encode to cards
   naming the same wallet.
2. Every seating refusal in the NORMATIVE section demonstrated by a
   vector row that FAILS if the refusal is removed.
3. The stub-mismatch refusal proven with a real foreign card (any mk1
   card from another wallet in the corpus).
