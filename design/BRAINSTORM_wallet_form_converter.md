# BRAINSTORM — the wallet-form converter (captured from the live session, 2026-08-30)

The brainstorm happened live with the operator during the post-S2 bench
night; this document is its record. The decisions below feed
`SPEC_wallet_form_converter.md` directly.

## THE MATRIX — the goal and the gaps, in one table

**Standing directive (operator, 2026-08-30, verbatim): "Make sure that
'in \\ out' table stays embedded in all design documents … it embodies
the ultimate goal well and points out the gaps we are trying to fill so
we don't want to forget. From brainstorm to spec to plan (and maybe
even in the code as a comment)."** Every artifact of this cycle carries
the matrix, cells updated as they close.

Input forms (rows — any COMPLETE wallet expression) × outputs (columns):

| in \ out | concrete descriptor | addresses | keyed card | keyless + mk1 cards |
| --- | --- | --- | --- | --- |
| **D** concrete descriptor | — | ✗ P3 | ✗ P3+bridge | ✗ P3 (the decomposer) |
| **T** template + key flags | ⚠ P1 (flag-form gap; inline template origins already work — r1 I8) | ⚠ P1 | ✓ `md encode --key` (Divergent) | ✓ |
| **S** keyless card + mk1 strings | ✗ P2 (the seating engine) | ✗ P2 | ✗ P2+bridge | — |
| **K** keyed card phrases | ✓ (round-tripped live) | ✓ | — | ✗ non-goal (first real need files it) |

✓ = works today, measured. ⚠/✗ = the gaps, each tagged with the spec
piece that closes it (P1 per-slot origins read-side, P2 the seating
engine, P3 decompose).

## Decisions made live, with their reasons

1. **Operator principle (verbatim intent): "accepting on input,
   expressive on output, but we require complete input."** This
   reframed a compose/decompose pair into ONE converter with one
   normative seating core; "complete" is the seating engine's contract.
2. **The keyed card already exists as an output** — `md encode --key
   @i=XPUB --fingerprint @i=HEX` (Divergent mode) minted the journey's
   22-chunk card; no new surface needed, only bridges into that call.
   Keyed (shorter, monolithic) vs split (distributable custody) are
   PEERS — different custody shapes; the converter crowns neither.
3. **Concrete descriptor IS an entry point** (the operator asked; the
   principle demands it) — `md decompose`, UNGATED (r2 I4: parsing
   needs only the unconditional `miniscript` dependency).
4. **Seating is funds-shaped** — a wrong seat reconstructs a different
   wallet (the "same-path keys cannot be seated" class); hence the
   seating engine (the assignment-invariance PRINCIPLE, phases A/B,
   the three-disposition stub model — see the SPEC, which supersedes
   this line's earlier wordings per r1-r4), completeness total, R0
   before code, refusals as executable vectors from draft one.
5. **Measured gaps that motivated all of this** (session evidence, three
   verbatim CLI refusals + one working round trip): see the spec's
   "Motivation, measured".
6. **Non-goals fenced**: `me --as md1` miniscript (S2-settled admission,
   own cycle); K→S splitting (no journey yet); any wire change.
7. **Operator ruling (2026-08-30, verbatim): "Key reuse (meaning with
   same keypath) isn't allowed."** Repeated (xpub, use-site path) is an
   invalid wallet in both converter directions; the repeated-key
   machinery (collapse/capacity/supply-twice, rounds r6-r8) is deleted
   rather than repaired; fingerprint-bearing duplicate declarations are
   invalid policies at the door; privacy-preserving same-path
   different-master slots remain the one legitimate same-path family.
   `md encode`'s acceptance of `@0,@0` templates is filed as an md-side
   question, not changed by this cycle.
   **Refinement (operator, 2026-08-30, verbatim): "Bad ideas can be
   valid, but we don't want to support BIP forbidden wallets."** The
   refusal ground is BIP 388 policy (pairwise-distinct keys; disjoint
   multipath sets per placeholder — both verified against bitcoin/bips
   master), NOT invalidity — the diagnostics say "unsupported", never
   "invalid". Measured the same day: md refuses the BIP-legal disjoint
   form ("@0 appears with inconsistent path/multipath/hardening") while
   composing the BIP-forbidden same-path form — md currently inverts
   BIP 388 on this point; that inversion is the filed md-side question.
