# R0 round 1 — IMPLEMENTATION_PLAN_wallet_form_converter.md

**Artifact:** `design/IMPLEMENTATION_PLAN_wallet_form_converter.md` @ `7f07ecc0`
(311 lines; plan baseline `7e125963`).
**Question:** does this plan faithfully and executably schedule the settled spec?
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED.**
**Counts: 0 Critical / 3 Important / 2 Minor.**

The SPEC is settled (GREEN at r9) and was not re-opened; where the plan and spec disagree
I treat the plan as the defect, per the plan's own rule. The brief's settled facts (spec
contents, the bypass measurement, §8's first-sentence crate-layout facts) were taken as
given. Nothing in any repo was modified.

**Shape of the round.** The plan is well built: the phase spine matches the spec's
dependency structure, D1–D4 are genuinely scheduling decisions with no normative
smuggling, §8's claims audit is real and every fact I spot-checked held, and the roster
covers most of the spec faithfully. Three defects: the roster is missing three
spec-promised rows, two phases' exit gates can pass with their rows unwritten, and one
load-bearing fixture path is wrong and unaudited.

---

## IMPORTANT

### I1 — Three spec-promised vector rows are missing from the §4 roster.

The spec commits, twice, to a row per disjunct ("Every rule above ships as executable
vector rows … disjuncts as rows, not prose"). Three are absent.

**(a) r2's three-orders counterexample.** SPEC lines 138–140: *"Every prior round's
counterexample — **r2's three-orders**, r3's two-groups, r4's repeated-slot and
internal-key, r5's use-site-path swap — ships as a vector row against THIS procedure."*
The roster maps r3→`V-GRP`, r5→`V-USP`, r4's repeated-slot→`V-BOUND-REF` (regrounded as
reuse). **r2's case has no row.** `V-ORD` is not it: its Expect column reads *"identical
descriptor bytes"*, which is the tie-break determinism test on a case that SEATS. r2's
case is `wsh(or_i(multi(2,@0,@1),multi(2,@2,@3)))` with four fingerprint-free cards at one
path, measured giving three different wallets from three supply orders — it must **REFUSE**.
A row expecting a seat and a row expecting a refusal are not interchangeable, and the one
the spec names is the refusal.

**(b) r4's internal-key case, in its reuse-free form.** Also named in that sentence. Its
reuse-bearing form (`tr(@0,{sortedmulti_a(2,@0,@1),pk(@2)})`, `@0` at two positions) is now
covered by `V-BOUND-REF`'s class. But the reuse-free form —
`tr(@0/O,{sortedmulti_a(2,@1/O,@2/O),sortedmulti_a(2,@3/O,@4/O)})`, five distinct keys, all
at one fingerprint-free origin, where swapping the internal key with a leaf key changes the
wallet — is still live, still must refuse, and has no row. It is the case that proves the
internal key is order-sensitive even when every candidate sits in a sorted group.

**(c) A2's safety half.** SPEC A2: *"A fingerprint-free CARD satisfies only a
fingerprint-free declaration by path — a declared fingerprint is a requirement the card
cannot meet blind."* `V-MIX` pins the permissive half (a fingerprint-bearing card at a
fingerprint-free declaration → SEAT). Nothing pins the refusing half. Inverted, a blind
card seats at a slot the policy pinned to a specific master — the funds-shaped direction of
the asymmetry, and the half that exists to be a constraint.

C2's scoped review is a partial backstop ("do the rows assert what the spec's rows
demand"), which is why this is Important and not Critical; but a roster is exactly the
artifact that review reads against, so a gap here propagates.

### I2 — C0's and C3's exit gates can pass with none of their rows written.

"The gate" is defined once, in C0: `cargo nextest run --locked` + `cargo clippy` +
`cargo fmt --check`. All three pass on a tree to which no new test was added. The phases
then differ:

```
C0   "Exit gate: [the three commands]"                       <- rows not required
C1   "Exit: the gate + all rows in the same commits as the code."   <- rows required
C2   "Exit: the gate + a scoped independent review of the SEATING DIFF"  <- review checks rows
C3   "Exit: the gate."                                        <- rows not required
C4   item 5, mandatory whole-diff adversarial review           <- backstopped
```

C3 carries **eight** rows (`V-D-RT`, `-DEPTH`, `-NOORIG`, `-REUSE`, `-SHAPE2`, `-JSON`,
`-PAIR`, `-CHKSUM`) and the phase's largest new piece, the MultiXPub walker; C0 carries the
named-refusal unit rows that kill the measured bare-`base58check` message. Both can close
green with that work absent.

§4's preamble and §7 do state the same-commit rule globally, so a charitable reading covers
C0 and C3 by reference. The reason that reading does not settle it: **C1 bothered to say
"+ all rows"**, which makes the omission elsewhere read as deliberate scoping rather than
as inheritance — and the per-phase exit line is what a controller checks when deciding
whether a phase closed. Either every phase's exit names its rows or none does, and given
the standing rule that a gate which cannot fail is itself blocking, the explicit form is
the safe one.

### I3 — D2's keyed-card fixture provenance is wrong, is load-bearing, and is not among §8's named-unverified items.

D2, fixtures: *"copied in C4 (walks) and C2 (subsets) from mnemonic-engrave
`design/journeys/out/pathological/{backup-strings.txt,keys.txt}` and the 22-string keyed
card **recorded in `design/CONTINUITY_2026-08-29-s2.md`**, each with a provenance note
naming source path + date."*

Measured against the mnemonic-engrave tree today:

```
design/CONTINUITY_2026-08-29-s2.md          exists; contains ZERO md1 strings of any length
                                            (`grep -c 'md1[a-z0-9]\{20,\}'` → 0)
design/journeys/out/pathological/
  journey_pathological.html                 22 unique md1fatzr2* strings   <- the real source
  backup-strings.txt, keys.txt              present, as D2 says
```

The keyed card is not in the file D2 names; it is in the 1.9 MB journey HTML, extractable
with `grep -o 'md1fatzr2[a-z0-9]*' | sort -u` (22 unique, matching the spec's pin of
21×86 + one 59-char tail). C4's acceptance leg (b) — *"the 22-string keyed card composes
SPEND-EQUAL to (a)"* — and rows `W-A/B/C` and `W-PIN` all depend on this fixture, so the
implementer hits a dead path at the phase that closes the cycle.

**This is also the answer to check (4)'s question.** §8 verifies the other two fixture paths
and names two deliberate non-verifications (mk-codec 0.5.0's registry presence, its decode
API surface). The keyed-card provenance is in neither list: it is load-bearing, unverified,
and false. Correct the path, and add it to the audited set — a fixture nobody re-extracts
is precisely the reproduction path that rots.

---

## MINOR

* **M1 — §3's C1→C2 dependency rationale contradicts the plan's own D2.** §3 says *"C1
  before C2 because the origin-notation parser and its refusal texts are P1's small surface
  and **P2 consumes them**."* D2 says `parse/keys.rs` grows that form *"(shared by P1 flags
  and **P3** emission checks)"*. P2's surface is `--from-mk1 <STRING>`,
  `--from-mk1-file <FILE>` and `--seat '@i=<chunk-set-id>'` — none of which is
  `@i=[fp/path]xpub`; A2 compares decoded card and declaration values, not flag text. The
  **ordering is still right** (C1 is a small surface on the same `descriptor`/`address`
  commands C2 extends), but the stated reason is unsupported, and a wrong dependency claim
  is what a "what did the last phase falsify?" re-validation reasons from. C3-after-C2 is
  sound and needs no change: `V-D-RT` re-composes through the C2 engine and the
  ROUND-TRIP/SPEND-EQUALITY checker is P2-owned.

* **M2 — the A5 ambiguous-id roster note defers a question the spec already answers.**
  Check (5): the note is a **real gate, not hypothesis-shaped** — it names the two
  acceptable artifacts (either the row exists, or `V-COLLIDE` carries an assertion plus a
  comment recording that it subsumes the A5 case), and forbids silent dropping, so a
  reviewer can check which was produced. Blessed on that. What it should not do is ask the
  implementer to discover the answer: SPEC A3(a) step 3 already states that a merged
  id-collision group refuses at reassembly *"and the seating engine never sees colliding
  cards"*, which entails the A5 ambiguous-id branch is unreachable. The plan can cite that
  and require the subsuming assertion directly, saving the implementer a re-derivation of
  something measured twice and written down. (SPEC A5 separately still calls that refusal
  "deliberately reachable" — a spec-internal tension I am not re-opening a settled spec
  for, and one the plan routes around simply by citing A3(a).)

---

## Blessed

* **§8's claims audit is real, and every fact I spot-checked held.** Baseline `7e125963` is
  a genuine commit ("report: persist push-agent report — spec cycle pushed, bypass message
  noted"). `crates/md-cli/Cargo.toml:28` is
  `md-codec = { path = "../md-codec", version = "=0.42.0" }` verbatim; the registry-not-path
  comment begins at `:39` as cited. `crates/md-cli/src/seat/` and `src/cmd/decompose.rs` do
  not exist (correctly described as new); `src/parse/keys.rs` does. All four named test
  files — `cmd_gui_schema.rs`, `json_snapshots.rs`, `help_examples.rs`,
  `duplicate_key_slots.rs` — are present.
* **D1's citations are exact and its reasoning holds.** `crates/me-cli/Cargo.toml:39` is
  `mk-codec = "0.4"`, and the lock resolves 0.4.1. The apparent tension — md-cli takes
  md-codec by `path` — is not a contradiction: md-codec is an intra-workspace sibling,
  whereas mk-codec lives in another repository, which is exactly the fresh-checkout case
  the quoted comment forbids. The C0 entry gate (confirm 0.5.0 published; escalate rather
  than vendor around it) is the right escalation shape.
* **D1–D4 smuggle no normative change.** D1 is dependency sourcing, D2 is module and
  fixture location, D3 is snapshot-regeneration discipline, D4 is push process. Each
  schedules or locates; none alters a spec rule. D2's placement of the input pipeline
  inside `seat/` matches the spec's "P2's input pipeline is normative".
* **The matrix travels correctly** — §1 carries it, and D2 requires the fourth home
  (`seat/mod.rs`'s doc comment), which is the operator directive's own wording.
* **Phase ordering is otherwise sound**, and C2's scoped-review brief is well aimed: "does
  any refusal path return before printing its names, is there a false-PASS shape" is the
  right question for the funds-shaped phase.

---

## Gate

**RED — 0 Critical, 3 Important.** No code before these close.

None is structural: the phase spine, the decisions and the audit discipline are all sound,
and the fixes are additive.

1. **I1** — add the three rows: r2's three-orders (REFUSE), r4's internal-key reuse-free
   form (REFUSE), and A2's fingerprint-free-card half (must NOT satisfy a
   fingerprint-bearing declaration). Check `V-ORD`'s Expect column while there — it is a
   determinism row and should say so, so it is not mistaken for r2's case.
2. **I2** — give C0 and C3 the same "+ all rows in the same commits as the code" exit that
   C1 carries, or state once that every phase inherits it and drop C1's restatement.
3. **I3** — correct the keyed-card fixture path to
   `design/journeys/out/pathological/journey_pathological.html` with its extraction command,
   and move it into §8's audited set.
4. **M1, M2** — one corrected sentence each.

Re-dispatch after the fold, scoped to these five items and to whatever the fold newly
writes. The roster's other 38 rows, D1–D4, and §8's spot-checked facts are settled and need
no revisiting.
