# SPEC — post-converter md-cli mini-cycle (admission taxonomy, S→K mint, converter riders)

Status: DRAFT r0 — the R0 loop has not yet run. Authored from
`BRAINSTORM_mdcli_mini.md` as walked with the operator 2026-08-31;
every ruling cited below carries its date and is recorded verbatim in
the brainstorm's walk sections.

Baseline: main = `40f1de65`; suite at the cycle's baseline:
`cargo nextest run --locked` = 1069 passed / 2 skipped.

**Machine-verified this session (2026-08-31), so reviewers need not
re-derive:** `validate_no_duplicate_key_slots` has exactly two call
sites (`crates/md-codec/src/encode.rs:120`,
`crates/md-cli/src/cmd/build.rs:301`); `seat::compose::spend_equal`
ships at `crates/md-cli/src/seat/compose.rs:142` under
`#[allow(dead_code)]`; `desugar_double_wildcard` at
`crates/md-cli/src/parse/template.rs:67`, pinned by
`tests/cli_bip388_double_wildcard.rs`; `.github/workflows/ci.yml`'s
test job runs `cargo test --workspace --all-targets` with no
`--all-features`; the tripwire test at
`crates/md-cli/src/compile.rs:338` has fired. Measured live: the
forbidden same-path repetition MINTS today while the disjoint form
refuses; same-xpub-at-two-paths refuses at seating
(`check_no_repeated_xpub`, key material only); a pathless policy
mints (slots declare `m`) but no path-bearing card can seat into it;
`mk encode` requires `--origin-path` and depth-checks it.

## Principle (operator rulings, verbatim anchors)

"Key reuse (meaning with same keypath) isn't allowed." "Bad ideas can
be valid, but we don't want to support BIP forbidden wallets"
(both 2026-08-30). F-417 (2026-08-28): md1's use-site-path narrowness
is DELIBERATE; the wire format will not be widened. Diagnostics for
BIP-forbidden or wire-inexpressible shapes say "forbidden by BIP 388"
/ "unsupported" — NEVER "invalid".

## N1 — the placeholder-repetition taxonomy at the template surface (ruled Q1+Q2, 2026-08-31)

Every verb that parses a template with placeholders toward a mintable
or spendable artifact (`md encode`, `md build`,
`md descriptor --template`) classifies a placeholder appearing at
more than one use site by its use-site multipath sets A and B, where
a fixed derivation counts as its singleton set:

| case | BIP 388 | md1 wire | disposition |
| --- | --- | --- | --- |
| A = B (identical) | FORBIDDEN (its own invalid-example list: "Repeated keys with the same path expression") | expressible today | **R-N1a: REFUSE at mint** |
| A ∩ B ≠ ∅, A ≠ B (overlapping) | FORBIDDEN (disjointness rule) | inexpressible | **R-N1b: REFUSE — forbidden named as the primary reason** |
| A ∩ B = ∅ (disjoint) | LEGAL | inexpressible (one path per key slot) | **R-N1c: refusal STANDS; message rewritten honestly** |

- **R-N1a** wording: names BIP 388's repeated-key rule, says
  "unsupported" / "forbidden by BIP 388", never "invalid".
- **R-N1b** wording: the disjointness rule is the primary reason;
  wire inexpressibility may be named secondary.
- **R-N1c** (today's "@0 appears with inconsistent
  path/multipath/hardening") is rewritten to state: the wallet is
  BIP-legal; md1 deliberately cannot express it (one path per key,
  F-417); keep it as a descriptor (engraving path:
  `me … --as descriptor`). Behavior unchanged — message only.

**Single-source invariant.** The taxonomy is enforced by ONE
implementation reachable from every verb above, extending the
`validate_no_duplicate_key_slots` discipline (its two call sites are
the model). Verbs must not be able to diverge. Exact placement is the
plan's decision; the invariant is normative.

**Read side (walk-confirmed).** `decode`, `inspect`, `bytecode` still
READ a card carrying R-N1a's shape, printing a warning that names the
BIP-388 violation; composing verbs (`descriptor`, `address`) REFUSE
it — they produce a spendable artifact. R-N1b/c shapes are believed
unrepresentable in the wire; the plan carries a verification
obligation: attempt to craft such a card at the byte level, and if
one is craftable, decode's disposition for it gets its own row.

**Vectors.** R-N1a refusal at `encode`, at `descriptor --template`,
and at `build`; R-N1b refusal; R-N1c diagnostic row (asserts the new
message names BIP-legality, inexpressibility, and the descriptor
escape, and does NOT contain "invalid"); a decode-warn row from a
hand-built md1 vector string carrying R-N1a (no shipped binary will
mint one after this lands); a V-BOUND-REF sibling row pinning
same-xpub-at-DIFFERENT-declared-paths refusing at seating (measured
2026-08-31, currently unpinned).

**Reconciliation note.** Converter SPEC A3 (r9 M2) asserts md's
parser refuses shape (2) upstream — measured FALSE today at the
template surface. R-N1a makes that premise true. The shipped
converter spec is not edited; this paragraph is the record.

## N2 — mint a keyed card from a seating result (S → K)

`md descriptor <keyless md1…> --from-mk1 … --emit md1` mints the
keyed card directly from the seating result. Measured ground: a keyed
card's Pubkeys TLV holds 65 bytes (chain code ‖ compressed point) and
no depth field, so depth-0 seated keys lose nothing. The depth-3/4
admission rule on `md encode --key` is UNCHANGED — it guards
hand-pasted keys. The minted card carries the origin metadata learned
from seating (walk-confirmed).

**Oracle.** The minted card must be byte-identical to the card
`md encode` mints given the same template and the fixtures' real
account-level keys with the same origins — the primary row. Secondary
rows: `spend_equal` and address-0 equality against the keyed fixture
card.

`--emit md1` composes with `--seat` and changes ONLY the output form:
every A2/A3/A4 seating rule and refusal is untouched, and one vector
row pins a seating refusal surviving under `--emit md1`.

**Matrix duty.** Flips the S→K cell (the one C4 did not). THE MATRIX
TRAVELS — all 4 homes in the same commit,
`scripts/matrix-identity-check.sh` gates byte-identity.

## N3 — the `--key` bracket path becomes a last-resort source (ruled 2026-08-31)

Precedence: inline template origin > `--path` > `--key` bracket. The
bracket fills a slot ONLY where neither of the first two supplies a
path; wherever they do, it remains a cross-check and a disagreement
refuses exactly as today. The wire is untouched — this is CLI path
resolution only.

**Vectors.** The measured different-accounts wallet (the FOLLOWUPS
reproduction) now composes, and its descriptor equals the one
produced by pasting the same origins inline (equality row); a
disagreeing bracket with `--path` present still refuses (regression
row); a slot with no path from any source still refuses.

## Riders

**R3 — `--verify-against <md1|FILE>` on `md descriptor`.** Wires
`seat::compose::spend_equal` (its `#[allow(dead_code)]` names this
channel). Output states SPEND-EQUAL or NOT, names the failing half
(structure, values, use-sites), and states plainly that origin
metadata is excluded and why. Exit 0 equal / 1 not. Rows: an equal
cross-form pair; a one-xpub-off negative reporting NOT; an
origins-differ pair reporting EQUAL.

**R5 — the all-features suite.** (a) Delete `render_tr_template` and
its fired tripwire; render via upstream `Display`. (b) Add
`--all-features` to CI's test job. Sequence (a) before (b) so CI is
never red; from (b) onward the phase gate's nextest line carries
`--all-features` in the same commit, keeping gate = CI.

**R6 — decompose desugars `/**`.** A deliberate amendment to the
converter spec P3's D-row input boundary: decompose applies the same
desugar the template surface ships, `--help` names the accepted
spelling. Row: a `/**` descriptor decomposes identically to its
`/<0;1>/*` rewrite.

**R7 — decompose reads `-`.** The positional accepts `-`
(≡ `--in /dev/stdin`), and the not-a-descriptor refusal names `--in`
and `-`.

**R8 — decompose `--json`: PARKED (ruled in the walk).** Trigger
recorded at close-out in the FOLLOWUPS entry: the first front-end
consumer doing the listdescriptors-extraction job designs the
envelope. No code this cycle.

**R9 — `--from-mk1` arity (discovered by this walk).** (a)
`num_args = 1..` so the natural paste works — the plan verifies the
positional/greedy-flag interaction with a mixed vector (md1
positionals and mk1 strings in one invocation); (b) the md1
positional refuses an `mk1…`-prefixed string BY NAME, pointing at
`--from-mk1`. Both land.

**Docs (ruled).** The mnemonic-toolkit manual pass
(`docs/manual/src/40-cli-reference/42-md.md`, gated by
`tests/lint.sh flag-coverage`) runs in this cycle's close-out phase,
covering the converter cycle's surface plus this cycle's.

## Non-goals

- No wire-format change of any kind (F-417 binds; R-N1c refuses
  instead of widening).
- No relaxation of `md encode --key`'s depth rule.
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

**Phase gate (closes `phase-gate-omits-cargo-doc`), quoted verbatim
by the plan:** `cargo nextest run --locked` (plus `--all-features`
from R5(b) onward) + `cargo clippy --locked --all-targets -- -D
warnings` + `cargo fmt --check` + `RUSTDOCFLAGS="-D warnings" cargo
doc --workspace --no-deps --document-private-items`. The plan
re-validates against the tree immediately before each implementer
dispatch (a plan's GREEN expires).

Rust-primary: every admission change lands here with vectors before
any Go port follows. Push via `scripts/push-via-staging.sh`; `main`
frozen for the window.

## Acceptance

1. Every vector row named above exists as an executable test in the
   same commit as its implementation — disjuncts as rows, not prose.
2. The suite is green under the full phase gate, including
   `--all-features` (R5).
3. The S→K matrix cell is flipped in all 4 homes with the identity
   check green (N2).
4. No diagnostic introduced by this cycle contains the word "invalid"
   for a BIP-forbidden or wire-inexpressible shape — asserted by the
   vector rows, not by convention.
