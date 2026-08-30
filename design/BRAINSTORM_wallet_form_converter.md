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
| **D** concrete descriptor | — | ✓ P3 (shipped this cycle) | ✓ P3+bridge (shipped this cycle) | ✓ P3 (the decomposer, shipped this cycle) |
| **T** template + key flags | ✓ P1 (shipped this cycle; inline template origins already worked — r1 I8) | ✓ P1 (shipped this cycle) | ✓ `md encode --key` (Divergent) | ✓ |
| **S** keyless card + mk1 strings | ✓ P2 (the seating engine, shipped this cycle) | ✓ P2 (shipped this cycle) | ✗ P2+bridge — `md encode --key` needs a depth-3/4 xpub, a card composes depth-0 (measured C4, filed) | — |
| **K** keyed card phrases | ✓ (round-tripped live) | ✓ | — | ✗ non-goal (first real need files it) |

✓ = works today, measured. ⚠/✗ = the gaps, each tagged with the spec
piece that closes it (P1 per-slot origins read-side, P2 the seating
engine, P3 decompose).

**Flipped at C4 close (2026-08-30)**, in the same commit as the acceptance
walks that prove them: the T row's two ⚠ P1 cells, the S row's concrete-
descriptor and addresses cells, and the whole D row. One cell this cycle owned
did NOT close, and is left ✗ carrying its measured reason: **S → keyed card**.
`md encode --key` admits only a depth-3/4 xpub, and a descriptor composed from
mk1 cards carries depth-0 keys (md rebuilds them from the 65-byte TLV, which
has no depth), so the bridge refuses — filed as
`md-cannot-mint-a-keyed-card-from-a-split-set`. Byte-identity of the four
copies is machine-checked by `scripts/matrix-identity-check.sh`.

## Decisions made live, with their reasons

1. **Operator principle (verbatim intent): "accepting on input,
   expressive on output, but we require complete input."** This
   reframed a compose/decompose pair into ONE converter with one
   normative seating core; "complete" is the seating engine's contract.
2. **The keyed card already exists as an output** — `md encode --key
   @i=XPUB --fingerprint @i=HEX` (Divergent mode) minted the journey's
   22-chunk card; no new surface needed, only bridges into that call.
   **Measured FALSE for the S row at C4 (2026-08-30)** — see the
   retraction above: a card composes depth-0 xpubs and `md encode --key`
   admits only depth 3/4, so that bridge refuses. True of the D row,
   which is where the ✓ stayed.
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
   same keypath) isn't allowed."** Repeated (xpub, use-site path) is
   BIP-388-forbidden and UNSUPPORTED in both converter directions (see
   the refinement below — technically valid, refused as policy); the
   repeated-key machinery (collapse/capacity/supply-twice, rounds
   r6-r8) is deleted rather than repaired; fingerprint-bearing
   duplicate declarations are refused at the door as only fillable by
   BIP-forbidden reuse; privacy-preserving same-path
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
