# BRAINSTORM — post-converter md-cli mini-cycle

Draft for the operator walk, 2026-08-31. No spec exists yet. Items 1–2
are risk-set (normative admission behavior); the R0 chain starts only
after this walk rules the open decisions below.

Baseline: main = `3efab622` (converter SHIPPED; suite 1069 passed /
2 skipped under `cargo nextest run --locked`). Every code citation in
this document was re-verified against this tree on 2026-08-31.

**Walk status (2026-08-31):** decisions (a) and (b) RULED below. The
operator confirmed in the walk: item 1's read-side rule, item 2's
mint-from-seating shape, rider 6's desugar, rider 8's park. Item 1's
two directions stand pending the operator's word after the same-key
probe was answered by measurement (see item 1's walk measurement).
Rider 9 was DISCOVERED by the walk itself.

## OPERATOR DECISION (a) — should an origin-notated `--key`'s bracket path become a PATH SOURCE of last resort?

(`descriptor-key-bracket-path-as-a-last-resort-source`)

**RULED 2026-08-31 (operator, in the walk): YES — option 1.** The
bracket becomes a last-resort PATH SOURCE. Precedence: inline template
origin > `--path` > `--key` bracket; the bracket fills a slot only
where nothing else spoke, and stays a cross-check (a disagreement
still refuses) whenever they do. The spec binds this.

Today (SPEC P1, folded from whole-diff r1 I1): the bracket path is a
CHECK, never a source — paths come from the inline template origin,
else the shared `--path`. Consequence, measured: a wallet whose slots
sit at DIFFERENT accounts (`48'/0'/0'/2'`, `48'/0'/1'/2'`, …) cannot
be composed from the T row at all — the shared `--path` cannot express
it, and the inline origins are not in the operator's hand (`md decode`
prints the template WITHOUT them; they go to a stderr note).

**Option 1 — widen: bracket becomes a last-resort source.**
Precedence: inline template origin > `--path` > `--key` bracket. The
bracket only fills a slot where nothing else spoke; every agreement
check stays — when anything else does supply a path, a disagreeing
bracket still refuses. Cost measured cheap: one arm in
`resolve_keys_fingerprints_and_precedence` plus a per-slot override in
`apply_path_override_per_slot` (which already takes a per-slot set).

**Option 2 — keep the refusal; add a documentation line** telling the
operator to paste the stderr-note origins inline into the template.

The tradeoff in two sentences. This is NOT F-417's wire-format class:
the bracket-source option changes only CLI path resolution — the
composed card is identical either way — so option 1 buys a wider CLI
admission, not a wider wire. And the safety delta is nil: an operator
holding a WRONG bracket path under option 2 would paste that same
wrong path inline and compose the same wrong wallet — the options
differ only in friction on the natural journey.

**Lean: option 1**, because the natural journey (template from
`md decode`, keys already in the `[fp/path]xpub` form that both
`md decompose --emit keys` and `mk encode --keys` use) then composes
first try.

## OPERATOR DECISION (b) — when does the toolkit-manual cross-repo docs pass run?

(`sibling-toolkit-md-manual-lockstep-for-the-converter`, repo
mnemonic-toolkit)

**RULED 2026-08-31 (operator, in the walk): option 1.** The docs pass
folds into this cycle's close-out phase — one cross-repo pass covers
the converter surface plus this cycle's additions.

The converter cycle added `md decompose` (with `--emit`, `--in`) and
`--from-mk1`, `--from-mk1-file`, `--seat`, origin-notated `--key`, and
`--path` on `md descriptor`/`md address` — none yet reflected in
mnemonic-toolkit's `docs/manual/src/40-cli-reference/42-md.md` (gated
there by `tests/lint.sh flag-coverage`). THIS cycle adds more surface:
at minimum `--verify-against`; likely `-` on decompose; possibly
`--emit md1` and the bracket-source arm.

- **Option 1 — fold into this cycle's close-out phase (lean):** one
  cross-repo pass covers converter + mini-cycle; the manual stays
  stale until then.
- **Option 2 — a pass now and again at close-out:** the manual is
  correct sooner; the first pass is invalidated within days.
- **Option 3 — leave parked for a separate operator-scheduled pass.**

## THE NORMATIVE CORE — items 1+2 (risk-set, full R0 chain)

Shared surface: md's admission at mint time. The enforcement home is
`md_codec::validate::validate_no_duplicate_key_slots`, whose TWO call
sites are re-verified on this tree — `crates/md-codec/src/encode.rs:120`
(inside the codec's encode path, so EVERY minting verb inherits it)
and `crates/md-cli/src/cmd/build.rs:301` — the
three-verbs-cannot-diverge invariant. Any new admission rule lives in
the same place, so the invariant survives by construction. Reading
verbs do NOT run the encode-path validators, which is what keeps
already-minted cards readable (see the read-side rule below).

### Item 1 — `md-repeated-placeholder-inverts-bip388`: decide both directions, with vectors

Measured inversion (2026-08-30): md COMPOSES the BIP-388-FORBIDDEN
same-path repetition — `md descriptor --template
"wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))"` clean, and `md encode`
accepts it too (r4 M4) — while REFUSING the BIP-388-LEGAL disjoint
form `wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))` ("@0 appears with
inconsistent path/multipath/hardening").

Settled ground (SPEC A3, operator rulings verbatim): "Key reuse
(meaning with same keypath) isn't allowed"; "Bad ideas can be valid,
but we don't want to support BIP forbidden wallets". Diagnostics say
forbidden-by-BIP-388 / unsupported, NEVER "invalid".

**The taxonomy the spec must pin** — same placeholder at two use
sites, multipath sets A and B (a fixed derivation counts as its
singleton set for the disjointness test):

| case | BIP 388 | md1 wire | disposition |
| --- | --- | --- | --- |
| A = B (identical) | FORBIDDEN (the invalid-example class) | expressible today | **refuse: forbidden/unsupported** (direction 1) |
| A ∩ B ≠ ∅, A ≠ B (overlapping) | FORBIDDEN (disjointness rule) | inexpressible | **refuse: forbidden** — forbidden is the primary reason named, inexpressibility secondary |
| A ∩ B = ∅ (disjoint) | LEGAL | inexpressible (one path per key slot) | **refuse BY DESIGN with an honest diagnostic** (direction 2) |

**Direction 1 — the forbidden same-path repetition: REFUSE at mint.**
The ruling already decides this; the walk confirms scope:

- New validator beside `validate_no_duplicate_key_slots` in
  `md_codec::validate`, reached from the same two call sites.
- Diagnostic names BIP 388's repeated-key rule, says "unsupported" /
  "forbidden by BIP 388", never "invalid".
- **Read-side rule (proposal):** reading verbs (`decode`, `inspect`,
  `bytecode`) still read a card carrying the forbidden shape — reading
  is not endorsing, and the operator needs decode to learn what a
  plate says — but print a WARNING naming the BIP-388 violation.
  Composing verbs (`descriptor`, `address`) refuse: they produce a
  spendable artifact. This mirrors where the validators already sit
  (encode path only).
- Vectors: an encode refusal row, a `descriptor --template` refusal
  row, a build refusal row, and a decode-warn row from a hand-built
  md1 vector string carrying the forbidden shape (no shipped binary
  will mint one after this lands).

**Direction 2 — the BIP-legal disjoint form: BY-DESIGN REFUSAL with an
honest diagnostic (proposal).** Admitting it means the wire carries a
DIFFERENT multipath per use site of one key; md1's wire holds one path
per key slot, so admission is a wire-format widening — exactly the
class F-417 ruled against (2026-08-28: never widen the wire format for
arbitrary use-site paths). And there is no workaround spelling: the
same xpub under a second placeholder violates BIP 388's
pairwise-distinct rule, so the wallet is genuinely inexpressible as an
md1 card. Keep the refusal; the text stops reading as a user error.
Proposed shape: "BIP-388-legal, but not expressible in md1's wire
format (one path per key); keep this wallet as a descriptor" (naming
`me … --as descriptor` as the engraving path). A vector row pins each
diagnostic in the taxonomy — three rows.

**Consistency flag for the spec to reconcile:** SPEC A3 (r9 M2)
asserts the converter's compose-side shape-(2) refusal is unreachable
because "md's parser refuses shape (2) upstream" — measured FALSE at
the template surface today (the forbidden form composes clean).
Direction 1 makes A3's premise true. The mini-cycle spec states that
reconciliation rather than silently inheriting it.

**Walk measurement (2026-08-31), answering the operator's probe** ("a
two-slot template populated with the same key at different paths via
two mk1 strings — refused currently?"): YES — measured live, not
recited. A freshly minted pair (same xpub declared at `48'/0'/0'/2'`
and `48'/0'/1'/2'`) fed to `md descriptor --from-mk1` refuses: "cards
… carry the SAME extended public key … forbidden by BIP 388 …
UNSUPPORTED here, not a malformed input", exit 1. The check is
`check_no_repeated_xpub` (`crates/md-cli/src/seat/satisfy.rs:294`,
wired at `seat/mod.rs:140`, BEFORE matching) and it compares key
material only (public key + chain code) — declared paths never enter.
So item 1's gap is confined to the TEMPLATE surface; the card-seating
side is already closed. Vector gap for the spec: `V-BOUND-REF` pins
only the same-path variant — add a different-paths sibling row so the
case the operator asked about is pinned, not measured once.

### Item 2 — `md-cannot-mint-a-keyed-card-from-a-split-set`: mint from the seating result; do NOT relax the depth rule

Measured (2026-08-30): the S → keyed-card bridge refuses from both
ends — the Pubkeys TLV reconstructs depth-0 xpubs, `md encode --key`
admits depth 3/4 only, and a descriptor composed from cards fails
decompose's depth-consistency check.

**The information is not lost.** A keyed card's Pubkeys TLV holds
exactly 65 bytes (chain code ‖ compressed point) — NO depth field — so
minting from depth-0 seated keys loses nothing. (Claim to
machine-check in the spec: byte-compare a card minted from the seating
result against `md encode` fed the same template + account-level
keys.)

**Proposal (the entry's own analysis):** add an emission on the
compose path — mint the keyed card directly from the seating result —
and keep `md encode --key`'s depth rule, which catches a genuinely
wrong key pasted at an account slot. Surface spellings for the walk:

1. `md descriptor <keyless md1…> --from-mk1 <mk1…> --emit md1` (lean —
   precedented by decompose's `--emit`, and the seating result is
   already in hand on this path);
2. a new verb (`md rekey` / `md mint-keyed`);
3. `--from-mk1` on `md encode`.

Open question for the walk: does the minted keyed card carry the
origin declarations (known from seating), matching what `md encode`
would emit — or none, matching the keyed fixture's convention? The
oracle either way is `spend_equal` plus an address row against the
keyed fixture; if minted WITH origins, a byte-equality oracle against
a fresh `md encode` is also available and stronger. Lean: carry
origins, take the byte oracle.

Matrix duty: this flips the one cell C4 did not (S → keyed card, ✗ →
✓). THE MATRIX TRAVELS — all 4 homes together,
`scripts/matrix-identity-check.sh` gates byte-identity.

The Rust-primary rule binds both items: admission changes land here
with vectors first; any Go-port counterpart follows, never leads.

## RIDERS (items 3–8) — same cycle, non-gated phases

### 3. `--verify-against` on `md descriptor` (`md-verify-against-flag-for-cross-form-comparison`)

Wire `seat::compose::spend_equal` (ships row-pinned at
`crates/md-cli/src/seat/compose.rs:142`; its `#[allow(dead_code)]`
names this missing channel as its reason). Surface:
`md descriptor … --verify-against <md1|FILE>` → states SPEND-EQUAL or
NOT, names which half failed (structure, values, use-sites), states
plainly that origin metadata is excluded and why; exit 0 equal / 1
not. Vectors (from the entry): an equal cross-form pair; a
one-xpub-off negative; an origins-differ pair that must report EQUAL.
Closes the measured false-difference trap: naive `diff` reports 253
chars of origins+checksum as a difference on a correct restore — the
worst direction for a funds-shaped check, since it invites re-cutting
plates that are fine.

### 5. all-features suite red + ungated (`all-features-suite-is-red-and-ungated-by-ci`)

Two pieces, sequenced (a) then (b) so CI never goes red
(never-skip-jobs):
(a) delete `render_tr_template` and its fired tripwire
(`crates/md-cli/src/compile.rs:338`,
`upstream_display_is_still_broken_delete_local_renderer_when_this_fails`
— it did its exact job: upstream `Display` no longer flattens the
depth-2 taptree); render via upstream directly.
(b) add `--all-features` to CI's test job (`ci.yml` runs
`cargo test --workspace --all-targets` today — verified on this
tree), which is what stops the next one. The entry says do (b) even
if (a) slips; sequencing (a) first makes that moot.

### 6. decompose refuses BIP-389 `/**` (`md-decompose-rejects-double-wildcard-input`)

The decision C4 correctly declined to smuggle: desugar on input
(WIDENS SPEC P3's D-row input boundary — this cycle amends the spec
deliberately) vs keep the refusal and name `/**` with its rewrite in
the refusal text. **Lean: desugar.** md's own template surface already
desugars (`crates/md-cli/src/parse/template.rs:67`,
`desugar_double_wildcard`, pinned by
`tests/cli_bip388_double_wildcard.rs` — both verified on this tree),
so the asymmetry is an accident of the upstream parser
(rust-miniscript at `ff4732e` does not parse `/**`), not a design. The
fix reuses the shipped desugar; the spec amendment is one sentence on
P3's input boundary plus a `--help` note. Vector: a `/**` descriptor
decomposes identically to its `/<0;1>/*` rewrite.

### 7. decompose stdin (`md-decompose-does-not-read-stdin`)

Both halves, both cheap: (a) accept `-` on the positional (≡ `--in
/dev/stdin`), matching the convention every other reading verb gained
in P3 §6b; (b) the refusal names `--in` (and `-`) instead of only
saying what `-` is not.

### 8. decompose `--json` (`md-decompose-has-no-json-output`) — Nit

The entry's own analysis: the envelope should carry per-slot
origin/fingerprint/key STRUCTURE, and the moment to design it is when
a front-end consumer exists. **Lean: decide-and-park** — close the
item by recording the trigger ("the first front-end doing the
listdescriptors-extraction job designs the envelope") rather than
inventing an API with no consumer. Alternative: a minimal envelope now
while the walker is warm, accepting envelope churn later.

### 9. `--from-mk1` arity + mistargeted diagnostic (`from-mk1-arity-spills-card-strings-into-the-md1-positional`) — NEW, from this walk

Discovered 2026-08-31 while measuring the operator's same-key probe.
`--from-mk1` takes one string per occurrence, so the natural paste of
a card set sends every string after the first to the md1 positional,
which fails with the bare codec error "wire-format version mismatch:
got 10, expected 4" — the F-420 class on the converter's most-trodden
entrance. Two composable remedies (full entry in FOLLOWUPS):
`num_args = 1..` on the flag, and an mk1-prefix refusal in the md1
positional that names `--from-mk1`.

## PROCESS AND GATES

- Items 1+2: this walk → SPEC (amending A3's md-side note and P3's
  input boundary as decided) → R0 to 0C/0I → plan → R0 → one
  implementer per phase → whole-diff adversarial review before merge.
  Riders ride the same plan as non-gated phases. Reviewer tiers
  sonnet/opus; fable never proposed.
- **The phase gate (closes `phase-gate-omits-cargo-doc`):**
  `cargo nextest run --locked` + `cargo clippy --locked --all-targets
  -- -D warnings` + `cargo fmt --check` + **`cargo doc --workspace
  --no-deps --document-private-items` with
  `RUSTDOCFLAGS="-D warnings"`** — the full CI surface, named job by
  job. Once rider 5(b) lands, the nextest line gains `--all-features`
  in the same commit that widens CI, so gate and CI never diverge
  again. The plan MUST quote this gate; a phase gate narrower than CI
  reports green for a tree CI will reject.
- Push via `scripts/push-via-staging.sh`; freeze main for the window.
- Persist-before-fold; agents persist their own reports to
  `design/agent-reports/`.

## SETTLED GROUND THE SPEC MUST NOT CONTRADICT

- A3's key-reuse rulings (quotes above); diagnostics never say
  "invalid".
- P3's D-row input boundary — rider 6 widens it only by deliberate
  spec amendment here.
- F-417 (md1 narrow paths): the wire never widens for use-site paths.
  Binds item 1 direction 2. Does NOT bind decision (a), which is
  CLI-side path resolution with the wire untouched.
- The `validate_no_duplicate_key_slots` two-call-site invariant
  (`encode.rs:120`, `build.rs:301` — re-verified on this tree).
- THE MATRIX TRAVELS: item 2's cell flip updates all 4 homes together,
  gated by `scripts/matrix-identity-check.sh`.
