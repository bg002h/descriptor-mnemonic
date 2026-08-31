# CONTINUITY — post-converter md-cli mini-cycle (opened 2026-08-31)

## STATUS 2026-08-31 (updated in-cycle): SPEC GREEN, PLAN IN R0

Brainstorm WALKED with the operator and complete (all rulings in
BRAINSTORM_mdcli_mini.md): bracket-path-as-source YES; docs pass at
close-out; item 1 both directions YES+YES; rider 8 parked; rider 9
DISCOVERED in the walk (from-mk1 arity, filed). SPEC_mdcli_mini.md is
**GREEN at b8a64938** — R0 loop r1 (2C/7I) → r2 (15/15 fixed,
0C/3I) → r3 (7/7 fixed, 0C/1I) → r4 fold-check clean; all reports
verbatim in design/agent-reports/REVIEW-mdcli-mini-spec-r*.md.
IMPLEMENTATION_PLAN_mdcli_mini.md drafted at d635458a (7 phases, R5
first to widen the gate, burndown map for 11 slugs); its R0 review
was dispatched and, if this session ended before folding, the report
is design/agent-reports/REVIEW-mdcli-mini-plan-r1.md — persist it
FIRST, machine-check, then fold. Key learned facts: the taxonomy must
NOT live in encode_payload (inspect/verify re-enter it on decoded
cards); R-N1d is the disjoint-use-site DELTA over the shipped F-218
floor; fixture cards for newly-refused shapes must be minted from the
baseline binary BEFORE the refusals land (0xed813, 0x00ee4).

**Mission:** burn down the converter cycle's parked residue in ONE
mini-cycle. Repo: descriptor-mnemonic. Baseline: main = d3676fb1 (+2
local report/continuity commits), converter SHIPPED same day (52
commits, whole-diff review r1 RED 1C/5I → fold → r2 GREEN).

## The 8 items (design/FOLLOWUPS.md, grep the slug)

1. `md-repeated-placeholder-inverts-bip388` — NORMATIVE. md accepts the
   BIP-388-forbidden same-path repetition (`@0/<0;1>,@0/<0;1>`) and
   refuses the legal disjoint form. Decide BOTH directions, vectors.
2. `md-cannot-mint-a-keyed-card-from-a-split-set` — NORMATIVE. Pubkeys
   TLV reconstructs depth-0; `md encode --key` admits depth 3/4 only.
3. `md-verify-against-flag-for-cross-form-comparison` — B2's comparison
   has no command; naive diff reports FALSE difference (254 chars of
   origins+checksum) on a correct restore.
4. `descriptor-key-bracket-path-as-a-last-resort-source` — OPERATOR
   DECISION at brainstorm.
5. `all-features-suite-is-red-and-ungated-by-ci`
6. `md-decompose-rejects-double-wildcard-input` (Minor)
7. `md-decompose-does-not-read-stdin` (Minor)
8. `md-decompose-has-no-json-output` (Nit)

Items 1+2 share md encode's admission surface — take together.
Rule item 4 (and the toolkit-manual pass question) in the brainstorm.

## Facts that must survive the clear (all measured 2026-08-30/31)

- Key-reuse ruling verbatim: "Key reuse (meaning with same keypath)
  isn't allowed" + "Bad ideas can be valid, but we don't want to
  support BIP forbidden wallets." Diagnostics say forbidden-by-BIP-388/
  unsupported, NEVER "invalid". BIP-388 rules: pairwise-distinct key
  vector; same-placeholder use sites need disjoint multipath sets.
- `validate_no_duplicate_key_slots` has exactly TWO call sites
  (cmd/build.rs:301, encode.rs:120) — the three-verbs-cannot-diverge
  invariant; any admission change must keep it.
- The seating engine lives in crates/md-cli/src/seat/ (matrix in its
  mod.rs doc — THE MATRIX TRAVELS, operator directive; update all 4
  homes together, scripts/matrix-identity-check.sh gates byte-identity).
- decompose in crates/md-cli/src/decompose/ (fresh MultiXPub walker;
  placeholder numbering by first appearance in rendering, NOT
  for_each_key order).
- Suite: cargo nextest run --locked = 1069 passed / 2 skipped at
  baseline. Gates: nextest + clippy -D warnings + fmt + CARGO DOC
  (per `phase-gate-omits-cargo-doc` — this cycle's plan MUST add doc
  to its phase gate; that entry is owned by "the next plan that writes
  a phase gate" = THIS one).
- Push ritual: scripts/push-via-staging.sh (first run clean). Freeze
  main during the window.
- mk-codec 0.5.0 is on crates.io; md-cli depends on it (registry).

## Process

Items 1-2 are risk-set (normative codec behavior): brainstorm → spec →
R0 to 0C/0I → plan → R0 → one implementer per phase → whole-diff
adversarial review before merge. Items 3-8 ride the same cycle as
non-gated phases. Persist-before-fold; agents persist own reports to
design/agent-reports/; reviewer tiers sonnet/opus (fable never).
Rust-primary rule: any admission change lands here with vectors before
any Go port follows.
