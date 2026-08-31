# CONTINUITY — post-converter md-cli mini-cycle (opened 2026-08-31)

## STATUS 2026-08-31 FINAL: CYCLE COMPLETE — merged to main, push in flight

Operator said "Proceed"; all 7 phases implemented (P1-P6 one agent
each, controller-verified gates + live probes after every phase; P7 =
sweep + toolkit docs pass + whole-diff review). Whole-diff review:
0C/5I/5M/5N → fold (9 commits, I2 swap-red proven) → r2 foldcheck
9/9, 0 survivors, gate PASS → **0C/0I, loop closed**. Final suite:
1186/1186, all six gate steps, matrix identical across 4 homes.
Merged --ff-only to main at c589054d. Toolkit docs on branch
mdcli-mini-docs (commit 95e3723d) in mnemonic-toolkit — AWAITS MERGE
in that repo, gate green locally, CI pin gap filed. New FOLLOWUPS
filed at close: emit-md1-has-no-transcribe-ready-form (operator's
call), toolkit-manual-gate-pinned-to-stale-md-release (next publish),
md-address-help-summary-is-blank, whole-diff-r1-nit-residue. Go
vendor sync for 15 regenerated corpus files outstanding (fork
follows, flagged in IMPL-mdcli-mini-P2.md). Worktrees
descriptor-mnemonic-mdcli-mini and mnemonic-toolkit-mddocs clean up
after push confirms.

## (superseded) earlier same day: SPEC GREEN + PLAN GREEN — AWAITING OPERATOR GO FOR P1

Both R0 loops CLOSED at 0C/0I. Spec GREEN at b8a64938 (4 rounds).
Plan GREEN at the r4-foldcheck persist commit (4 rounds: 2C/7I →
0C/2I → 0C/1I → clean). Operator rulings recorded verbatim in spec
Principle incl. post-walk "No carve out for reused keys unless
different origin paths". NEXT ACTION: on the operator's go,
re-validate the plan against the tree (staleness scope: what moved),
then dispatch the P1 implementer (all-features closure +
scripts/phase-gate.sh). The section below predates this and stands
as history.

## (superseded) STATUS earlier same day: SPEC GREEN, PLAN IN R0

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
