# SPEC — post-converter md-cli mini-cycle (admission taxonomy, S→K mint, converter riders)

Status: **GREEN — 0C/0I, R0 loop closed 2026-08-31.** Rounds:
r1 (2C/7I/4M/2N) → fold → r2 (15/15 fixed; 0C/3I/3M/1N new) → fold →
r3 (7/7 fixed; 0C/1I/1M/1N new) → fold → r4 fold-check (3/3 fixed,
0 contradictions). All reports verbatim in `design/agent-reports/`. Authored from `BRAINSTORM_mdcli_mini.md` as
walked with the operator 2026-08-31; every ruling cited below carries
its date and is recorded verbatim in the brainstorm's walk sections.

Baseline: main = `ed2fe9c2`; suite at the cycle's baseline:
`cargo nextest run --locked` = 1069 passed / 2 skipped;
`--all-features --no-fail-fast` = 1106 run / 1105 passed / 1 failed
(the fired tripwire only) / 2 skipped.

**Machine-verified (2026-08-31, controller + r1 reviewer
independently), so later reviewers need not re-derive:**
`validate_no_duplicate_key_slots` call sites
(`crates/md-codec/src/encode.rs:120`,
`crates/md-cli/src/cmd/build.rs:301`, the latter reached via
`build_descriptor`'s `--template` branch at `build.rs:66` — the
caller that makes it cover `md descriptor` AND `md address`);
`md inspect` and `md verify` re-enter `encode_payload` on a DECODED
card (`cmd/inspect.rs:32` → `identity.rs:40`; `cmd/verify.rs:52`);
the `Cmd` enum holds no `build` verb; `seat::compose::spend_equal` at
`seat/compose.rs:142` under `#[allow(dead_code)]`;
`desugar_double_wildcard`'s regex is anchored on `@i` placeholders
(`parse/template.rs:73`); the parser's repeated-placeholder check
compares the triple (origin path, multipath set, wildcard hardening)
(`template.rs:730-741`) and raises `CliError::TemplateParse`, whose
`Display` prefix is "template parse error:" (`error.rs:87`);
`.github/workflows/ci.yml`: test, clippy and doc jobs all lack
`--all-features`; the seated card inherits the policy card's
`Fingerprints` TLV (`seat/compose.rs:219`); every non-`BadArg`
`CliError` exits 1 (`main.rs:695-703`) and `md repair` reserves exit
5 for its non-error non-default answer. Measured live at exit 0
(controller re-ran both): `md address --template
"wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" --key @0=<K> --path …
--count 1` prints a receive address; `md descriptor --template
"wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=<K> --key @1=<K>`
composes. BIP-388 quotes below were verified by the r1 reviewer
against bip-0388.mediawiki (fetched 2026-08-31): the repeated-key
invalid example is line 308, the disjointness rule line 195, the
pairwise-distinct rule line 193, the disjoint-use-site VALID example
line 291, the explicit-path-on-placeholder invalid example line 305.
The r2 reviewer additionally verified: `cargo clippy --all-features
--all-targets -- -D warnings` and `RUSTDOCFLAGS="-D warnings" cargo
doc --all-features --workspace --no-deps --document-private-items`
both exit 0 at this tree, so R5(b) cannot turn CI red; `md encode`
accepts a template with inline divergent origins plus per-slot
`--fingerprint @i=HEX` (N2's primary oracle form is executable, not
resting on its fallback); and the two-placeholder disjoint same-key
form MINTS today (`md encode`, exit 0, chunk-set-id `0x00ee4`) —
plates carrying R-N1d's disjoint half can exist.

## Principle (operator rulings, verbatim anchors)

"Key reuse (meaning with same keypath) isn't allowed." "Bad ideas can
be valid, but we don't want to support BIP forbidden wallets"
(both 2026-08-30). F-417 (2026-08-28): md1's use-site-path narrowness
is DELIBERATE; the wire format will not be widened. Post-walk ruling
(operator, 2026-08-31, verbatim, on the degrading-multisig
consequence surfaced by plan review): "No carve out for reused keys
unless different origin paths" — the same-path reuse refusals stand
with no exception, and one master at different origin paths
(different derived xpubs) remains the legitimate, control-row-pinned
family. Diagnostics for
BIP-forbidden or wire-inexpressible shapes say "forbidden by BIP 388"
/ "unsupported" — NEVER "invalid".

## N1 — the placeholder/key-reuse taxonomy (ruled Q1+Q2 2026-08-31; r1 C1/C2/I2/I3/I7/M1 folded)

### Classification

The classifier runs on the RESOLVED template and classifies TWO
families:

**Family 1 — one placeholder at more than one use site.** The key is
the triple the parser already compares: (inline origin path,
multipath set, wildcard hardening).

| case | authority | disposition |
| --- | --- | --- |
| triples identical | BIP 388 FORBIDDEN (invalid-example list, line 308: "Repeated keys with the same path expression") | **R-N1a: mint/compose REFUSE**, citing the repeated-key rule |
| triples differ ONLY in multipath sets, sets overlap | BIP 388 FORBIDDEN (disjointness rule, line 195) | **R-N1b: REFUSE** — disjointness named primary, wire inexpressibility secondary |
| triples differ ONLY in multipath sets, sets disjoint | BIP 388 LEGAL (its own valid example, line 291) | **R-N1c: refusal STANDS** (F-417: one path per key slot); message rewritten honestly, see below |
| triples differ in inline ORIGIN | md representability (one origin per key) | **R-N1-origin: REFUSE** naming the origin axis — one placeholder cannot carry two origins; one origin per key in md1. MUST NOT cite the repeated-key rule, and MUST NOT cite BIP 388 at all: inline origins are md's own normal template spelling (`--emit template` prints them, `md encode` mints them — both measured), not an error class. Spec-internal aside, never for diagnostics: BIP 388's KEY grammar carries no explicit paths (its line-305 invalid example), which is why this axis is md-side |
| triples differ in wildcard HARDENING | md derivability, not BIP 388 | **R-N1-hardening: REFUSE** naming the hardening axis; MUST NOT cite BIP 388. The plan verifies whether this case is reachable past the single-site hardened-wildcard refusal; its row lands only if reachable |

There is no "fixed derivation counts as its singleton set" clause
(r1 M1): md's use-site grammar rejects post-multipath fixed steps at
lex, and pre-multipath steps lex as origin — so that shape belongs to
the ORIGIN axis above.

**Family 2 — one key at more than one placeholder (R-N1d; folds
r1 I3, reshaped by r2 I-b/I-c).** Two halves with different standing:

- **The same-use-site half is ALREADY SHIPPED and unchanged.**
  `validate_no_duplicate_key_slots` (md-codec, called inside
  `encode_payload` at `encode.rs:120`, tagged F-218) refuses two
  placeholders holding identical key material at the same use site.
  It is the CODEC-LAYER FLOOR: it stays where it is, sits outside
  this cycle's single-source scope, and is exempt from the C1
  placement constraint (which binds NEW checks; the floor's read-side
  behavior is shipped and out of this cycle's scope). Its wording
  stands for its own case.
- **R-N1d proper is the DISJOINT-USE-SITE DELTA:** identical key
  material (public key + chain code) at two placeholders whose use
  sites differ. The two-placeholder POLICY FORM puts the same key
  twice in BIP 388's key information vector, and the
  pairwise-distinct rule (line 193) forbids that FORM. Measured
  today: `md descriptor` composes it and `md encode` mints it (exit
  0, chunk-set-id `0x00ee4`), with an in-code rationale — "BIP 388
  permits it", `cmd/build.rs:280-283` — that conflates the wallet
  with its spelling. **Mint/compose refuses the delta.** The two
  pinning tests flip
  (`duplicate_key_slots.rs::one_key_at_two_different_use_sites_is_not_a_duplicate`,
  `::t_row_one_key_at_two_disjoint_use_sites_still_composes`), and
  the stale comments (`build.rs:280-283`, `validate.rs:353-355`) are
  corrected in the same commit.

**R-N1d's message mandate (r2 I-b — the honest reconciliation).** The
WALLET — one key at two disjoint path sets — is BIP-legal: it is
R-N1c's wallet in its one-placeholder spelling. What BIP 388 forbids
is THIS two-placeholder spelling (its key vector repeats the key);
what F-417 makes inexpressible is the one-placeholder spelling. Both
md1 spellings are inadmissible while the wallet itself is fine as a
descriptor. The rendered line therefore: attributes the
pairwise-distinct violation to this SPELLING's key vector, not to the
wallet; states the wallet is expressible as a descriptor; names the
same runnable escape as R-N1c (`me sysw pack --as descriptor --in
<your export file>`); never says "invalid"; and MUST NOT reuse the
shipped same-use-site wording — "at the same use-site" and "a card
minted from it could never be read back" are both false for the
delta, the latter contradicting Acceptance 5. A full rendered-line
row is mandated, exactly as for R-N1c.

**Untouched — the two legitimate families, both control-row pinned
(the second measured at the operator's probe, 2026-08-31):**
(i) distinct keys at ONE path across DIFFERENT masters (privacy
multisig) — different key material, no reuse; (ii) ONE master (same
fingerprint) at DIFFERENT account paths — different derived xpubs, no
reuse; measured composing and seating at exit 0. A must-COMPOSE T-row
control (same fingerprint, different accounts, different xpubs) and
the existing S-row control pin the anti-over-refusal duty.

### R-N1c's message, fully specified (r1 I7 folded)

The rendered stderr line is normative FROM THE `md:` PREFIX ONWARD,
so the refusal cannot ship under a prefix that blames the input. It
must NOT render through `CliError::TemplateParse` ("md: template
parse error: …"); the variant (new or repurposed) renders a prefix
that reads as a policy/capability statement, and the body states: the
wallet is BIP-388-legal; md1 deliberately cannot express it (one path
per key, F-417); keep it as a descriptor, with the RUNNABLE escape
spelling `me sysw pack --as descriptor --in <your export file>`
(r1 M4: that is the real surface, `me-cli/src/main.rs:723`).
Behavior unchanged — refusal stands; message and variant only.

### Placement constraint (r1 C1 folded) — NORMATIVE

No check THIS CYCLE ADDS may sit inside `encode_payload`'s validator
set: `md inspect` and `md verify` re-enter `encode_payload` on the
decoded card, so a new check there makes already-engraved plates of
newly-refused shapes uninspectable and unverifiable — and such plates
exist for R-N1a and for R-N1d's disjoint half, both mintable today
(the same-use-site half has been refused at the codec floor since
F-218, r2 I-c's correction of r1's broader claim). The brainstorm's
premise "reading verbs do not run the encode-path validators" is
FALSE for those two verbs and is corrected there. The shipped floor
(`validate_no_duplicate_key_slots` and its sibling validators inside
`encode_payload`) is out of this cycle's scope and stays.

**Single-source, stated as the classifier's INPUT (r2 I-a).** The
classifier consumes (i) the per-placeholder occurrence list — the
triple per use site — and (ii) the resolved per-`@i` key bindings.
Both are in hand at `parse_template_ext` time on the template path,
and both are reconstructible from a decoded card on the card path.
Family 1 needs only (i); R-N1d's delta needs (ii) — so no single
existing code location sees both, and this spec does NOT name one
(r1's `resolve_placeholders` naming was wrong for Family 2: its
signature carries no key material). The normative requirements are:
each predicate has ONE implementation (no per-verb second copy —
`build.rs:277`'s own rule); per-verb disposition (refuse vs warn) is
a parameter of the invocation, never a re-implementation; and the
constraint that identity computation and verify's re-encode of a
decoded card keep working is row-pinned. The plan chooses the
invocation points under those constraints. The S row's shipped
`check_no_repeated_xpub` (`seat/satisfy.rs:294`) is a third,
card-set-side implementation of the Family-2 predicate; it stays as
shipped (r3 M-1) — the single-source rule binds the mint/compose
surface, and T-row/S-row PARITY (each side refuses the wallet the
other refuses) is pinned behaviorally by the row set, not by code
unification.

### Verb dispositions (r1 C2 folded — derived from the `Cmd` enum; there is no `md build`)

- **REFUSE (mint/compose):** `encode`; `descriptor` (both `--template`
  and card input); `address` (both inputs).
- **WARN and proceed (read):** `decode`, `inspect`, `bytecode`,
  `verify` — including `verify`'s `--template` argument, so an
  operator can verify a legacy plate that carries a refused shape.
- **Obligation:** the plan determines whether `md compile`
  (feature-gated) can EMIT a Family-1/2 shape; if it can, it routes
  through the classifier with the refuse disposition.

### Vectors

R-N1a refusal at `encode`, `descriptor --template`, and
`address --template`; card-input composing refusals (`descriptor` and
`address` on a hand-built R-N1a card); R-N1b refusal; R-N1c full
rendered-line row (prefix + the three content statements + absence of
"invalid" + the runnable escape spelling); R-N1-origin row (names the
origin axis, does not cite the repeated-key rule); R-N1-hardening row
if reachable; R-N1d T-row refusal with its full rendered-line row (the two flipped
tests become refusal rows; the message row asserts the mandate
above); R-N1d CARD-INPUT refusals — `descriptor` and `address` on a
minted delta card refuse, mirroring R-N1a's card-input rows (r3 I-1:
both measured at exit 0 today; the card branch at `build.rs:69-77`
runs no reuse check, and `refuse_key_reuse_across_slots` has its one
call site inside the `--template` branch); the R-N1d must-COMPOSE
control (same fingerprint, different
accounts, different xpubs — composes); the V-BOUND-REF sibling row
pinning same-xpub-at-DIFFERENT-declared-paths refusing at seating
(measured 2026-08-31, previously unpinned);
read-side rows on the hand-built R-N1a card — `decode` warns at exit
0, `inspect` completes at exit 0, `verify` completes at exit 0,
`bytecode` completes (the C1 guarantee, pinned).

**Reconciliation note.** Converter SPEC A3 (r9 M2) asserts md's
parser refuses shape (2) upstream — measured FALSE today at the
template surface. R-N1a makes that premise true. The shipped
converter spec is not edited; this paragraph is the record.

## N2 — mint a keyed card from a seating result (S → K; r1 I4/M3 folded)

`md descriptor <keyless md1…> --from-mk1 … --emit md1` mints the
keyed card directly from the seating result. Measured ground: a keyed
card's Pubkeys TLV holds 65 bytes (chain code ‖ compressed point) and
no depth field, so depth-0 seated keys lose nothing. The depth-3/4
admission rule on `md encode --key` is UNCHANGED. The minted card
carries the origin metadata learned from seating (walk-confirmed).

**Oracle (r1 I4).** Byte-identity is decided by the FULL input set:
template, keys, per-slot origins, per-slot FINGERPRINTS (the seated
card inherits the policy card's `Fingerprints` TLV —
`compose.rs:219`), and the path-declaration SHAPE (Divergent vs
Shared). The primary row therefore compares the minted card against
`md encode` invoked with the template carrying INLINE per-slot
origins and one `--fingerprint @i=HEX` per declared fingerprint —
matching the policy card's declarations exactly. If `md encode`'s
flag surface cannot express the fixture card's declaration shape, the
primary row instead pins the minted card's TLV section field-by-field
(fingerprints, pubkeys, path declarations) against the seating
result, and says which form it took. Secondary rows: `spend_equal`
and address-0 equality against the keyed fixture card.

**Input modes (r1 M3).** `--emit md1` is admissible ONLY with
`--from-mk1`/`--from-mk1-file` input: with `--template` it refuses
naming `md encode` as the tool for that job; on a keyed-card
positional it refuses as a re-emit, by name. The flag-name reuse with
`md decompose --emit` is deliberate (per-verb value vocabularies,
each documented in its own `--help`). `--emit md1` composes with
`--seat` and changes ONLY the output form: every A2/A3/A4 seating
rule is untouched, and one row pins a seating refusal surviving under
`--emit md1`.

**Matrix duty.** Flips the S→K cell (the one C4 did not). THE MATRIX
TRAVELS — all 4 homes in the same commit,
`scripts/matrix-identity-check.sh` gates byte-identity.

## N3 — the `--key` bracket path becomes a last-resort source (ruled 2026-08-31)

Precedence: inline template origin > `--path` > `--key` bracket. The
bracket fills a slot ONLY where neither of the first two supplies a
path; wherever they do, it remains a cross-check and a disagreement
refuses exactly as today. The wire is untouched — CLI path resolution
only.

**Vectors.** The measured different-accounts wallet (the FOLLOWUPS
reproduction) now composes, and its descriptor equals the one
produced by pasting the same origins inline (equality row); a
disagreeing bracket with `--path` present still refuses (regression
row); a slot with no path from any source still refuses.

## Riders

**R3 — `--verify-against <md1|FILE>` on `md descriptor` (r1 I5 and
nit-1 folded).** Wires `seat::compose::spend_equal`; the wiring
DELETES its `#[allow(dead_code)]` and its now-false "nothing on the
CLI surface calls it" comment. Output states SPEND-EQUAL or NOT,
names the failing half (structure, values, use-sites), and states
plainly that origin metadata is excluded and why. **Exit codes: 0 =
spend-equal; 5 = NOT spend-equal (following `md repair`'s reserved-5
precedent for a non-error, non-default answer); 1/2 = errors,
unchanged** — so a mistyped input can never read as "not equal", the
exact false signal the FOLLOWUP warns invites re-cutting a good
plate. `--verify-against` is admissible on every `md descriptor`
input mode that composes a descriptor. Rows: an equal cross-form pair
(exit 0); a one-xpub-off negative (exit 5, message names the values
half); an origins-differ pair (exit 0, EQUAL); a garbage
`--verify-against` argument (exit 1, and stderr is a decode error,
not a spend-equality verdict).

**R5 — the all-features suite (r1 M2 folded).** (a) Delete
`render_tr_template` and its fired tripwire; render via upstream
`Display`. (b) Add `--all-features` to CI's test, clippy AND doc
jobs, and to the same three lines of the phase gate, in the same
commit. Sequence (a) before (b) so CI is never red.

**R6 — decompose desugars `/**` (r1 I1 folded).** The shipped
desugar is anchored on `@i` placeholders and is a NO-OP on
decompose's concrete-descriptor input — "reuse the shipped desugar"
was false. Normative: ONE desugar core serves both spellings —
either generalised off the `@`-anchor or two thin front-ends over a
shared component — so no pair of look-alike regexes carries a
keep-in-sync obligation (`template.rs:53-62` documents that hazard).
`--help` names the accepted spelling. Row: a `/**` descriptor
decomposes identically to its `/<0;1>/*` rewrite.

**R7 — decompose reads `-`.** The positional accepts `-`
(≡ `--in /dev/stdin`), and the not-a-descriptor refusal names `--in`
and `-`.

**R8 — decompose `--json`: PARKED (ruled in the walk).** Trigger
recorded at close-out in the FOLLOWUPS entry: the first front-end
consumer doing the listdescriptors-extraction job designs the
envelope. No code this cycle.

**R9 — `--from-mk1` arity (r1 I6 folded).** (a) `num_args = 1..` so
the positional-first natural paste works. (b) TWO guards, symmetric:
an `mk1…`-prefixed string in the md1 positional refuses by name
pointing at `--from-mk1`; an `md1…`-prefixed string among
`--from-mk1`'s values refuses by name pointing at the positional.
Clap's greedy multi-value consumption means the flag-first ordering
swallows a trailing md1 positional — the spec requires that
invocation to produce the symmetric guard's named diagnostic, NOT
clap's missing-required-argument error; the mechanics (e.g. relaxing
the `descriptor_input` group when `--from-mk1` is present and
refusing in code) are the plan's. Rows: positional-first composes;
flag-first with a trailing md1 string → the named refusal; an mk1
string in the positional → the named refusal; `--from-mk1` with NO
policy card anywhere → a refusal naming the missing policy input
(r2 M-c: today the clap group catches this case; a relaxed group must
not let it fall through to seating with an empty policy).

**Docs (ruled).** The mnemonic-toolkit manual pass
(`docs/manual/src/40-cli-reference/42-md.md`, gated by
`tests/lint.sh flag-coverage`) runs in this cycle's close-out phase,
covering the converter cycle's surface plus this cycle's.

## Non-goals

- No wire-format change of any kind (F-417 binds; R-N1c refuses
  instead of widening).
- No relaxation of `md encode --key`'s depth rule.
- No NEW check inside `encode_payload`'s validator set (the C1
  constraint) — and no removal of the shipped floor there:
  `validate_no_duplicate_key_slots` stays at `encode.rs:120` (r2 I-c;
  deleting it would strip md-codec's wire-level F-218 refusal from
  every consumer outside md-cli).
- No JSON envelope (R8 parked).
- No change to pathless-declaration seating: a slot declaring `m`
  never takes its path from a card; card paths remain verification,
  never a source (measured 2026-08-31; the refusal names both sides
  and is judged sufficient).
- No mk-repo changes.
- No edit to the shipped converter spec (the A3 reconciliation is
  recorded here).

## Gates and process

N1 and N2 are risk-set (normative admission; a new minting surface):
R0 loop on this spec to 0C/0I before any plan; plan R0; one
implementer per phase; whole-diff adversarial review before merge. N3
and the riders ride the same plan as non-gated phases.

**Phase gate (closes `phase-gate-omits-cargo-doc`), quoted and
EXTENDED by the plan (plan r2 N-b: the plan adds the doctest and
conformance-checksum lines per the FOLLOWUP's name-every-CI-job
criterion):** `cargo nextest run --locked` + `cargo clippy --locked
--all-targets -- -D warnings` + `cargo fmt --check` +
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
--document-private-items` — and from R5(b) onward the nextest, clippy
and doc lines all carry `--all-features`, matching the widened CI.
The plan re-validates against the tree immediately before each
implementer dispatch (a plan's GREEN expires).

Rust-primary: every admission change lands here with vectors before
any Go port follows. Push via `scripts/push-via-staging.sh`; `main`
frozen for the window.

## Acceptance

1. Every vector row named above — except R-N1-hardening's, which is
   explicitly conditional on reachability — exists as an executable
   test in the same commit as its implementation; disjuncts as rows,
   not prose.
2. The suite is green under the full phase gate, including
   `--all-features` on all three widened lines (R5).
3. The S→K matrix cell is flipped in all 4 homes with the identity
   check green (N2).
4. Diagnostic rows assert the RENDERED stderr line from the `md:`
   prefix onward — never body substrings alone — and no diagnostic
   introduced by this cycle contains the word "invalid" for a
   BIP-forbidden or wire-inexpressible shape. Every diagnostic this
   cycle introduces or rewrites HAS such a row — asserted by the
   vector rows, not by convention (r2 M-a restores r0's clause).
5. Reading verbs (`decode`, `inspect`, `bytecode`, `verify`) complete
   at exit 0 on already-engraved cards carrying shapes THIS CYCLE
   newly refuses — row-pinned, per the C1 constraint. (r3 N-1: the
   pre-existing codec floor's shape is out of scope and reads per
   shipped behavior.)
