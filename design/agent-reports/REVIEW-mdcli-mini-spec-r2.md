# REVIEW — SPEC_mdcli_mini.md, R0 round 2 (scoped fold re-review)

| field | value |
| --- | --- |
| artifact | `design/SPEC_mdcli_mini.md` |
| commit | `df420bda` (main tip; fold of `ed2fe9c2`'s r1 report) |
| date | 2026-08-31 |
| reviewer | independent agent, no authorship of the artifact or the fold |
| repo | `/scratch/code/shibboleth/descriptor-mnemonic` |

**Scope (as briefed, and held to).** Two questions only: (1) does the fold in
`df420bda` fix each of the 15 findings in
`design/agent-reports/REVIEW-mdcli-mini-spec-r1.md`; (2) did the folded text
introduce a new defect. NOT in scope and NOT re-done: the shipped converter,
the operator rulings recorded in `BRAINSTORM_mdcli_mini.md`, the BIP-388 line
numbers (r1 fetched and verified them), the 11 code citations in the spec's
Machine-verified block and the two live mint measurements (controller
re-verified).

**Method.** `git diff ed2fe9c2..df420bda -- design/SPEC_mdcli_mini.md` read
against the r1 report finding by finding; every piece of NEW normative content
(R-N1d, the five-row axis table, the placement constraint, the R3 exit-code
scheme, the R9 symmetric guard, the N2 oracle input set) checked against the
tree by opening the cited files and by running the binary.

**Machine-checked by this reviewer before writing (nothing below rests on a
described fact):**

- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`
  → **exit 0**.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
  --document-private-items --all-features` → **exit 0**.
  (Together these confirm R5(b)'s widening to the clippy and doc jobs does not
  turn CI red at this tree — the fold's new claim, previously unmeasured.)
- `resolve_placeholders` signature at `crates/md-cli/src/parse/template.rs:723`
  is `(occs: &[PlaceholderOccurrence]) -> Result<ResolvedPlaceholders, CliError>`
  — **no key material**. `ResolvedPlaceholders` holds `n`, `path_decl`,
  `use_site_path`, `use_site_path_overrides: Vec<(u8, UseSitePath)>` — one
  use-site path per `@i`.
- `crates/md-codec/src/encode.rs:120` `validate_no_duplicate_key_slots(d)?` sits
  **inside `encode_payload`** (between `validate_origin_key_consistency` and the
  first `BitWriter` write, `encode.rs:99-135`).
- `validate_no_duplicate_key_slots` (`crates/md-codec/src/validate.rs:361`)
  iterates `expand_per_at_n(d)` — per-`@N` SLOTS — comparing
  `(xpub, use_site_path)`. A single `@0` referenced twice in the tree is ONE
  slot and is invisible to it.
- Both T-row test names the spec says to flip exist:
  `crates/md-cli/tests/duplicate_key_slots.rs:82` and `:318`.
- `md repair`'s exit-5 doc is real (`crates/md-cli/src/main.rs:652-655`,
  "D26 cross-CLI parity … 5 — REPAIR_APPLIED").
- `.github/workflows/ci.yml`: test `:48`/`:49`, clippy `:65`, doc `:93` — none
  carries `--all-features`. The spec's claim is true.
- `me sysw pack --as <descriptor|md1> --in <your export file>` is the real
  guidance string (`mnemonic-engrave/crates/me-cli/src/main.rs:723`).
- Live at `target/debug/md`, this tree:
  - **Form A** `md descriptor --template "wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))" --key @0=<K> --path "48'/0'/0'/2'"`
    → **exit 1**, `md: template parse error: @0 appears with inconsistent path/multipath/hardening`.
  - **Form B** `md descriptor --template "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=<K> --key @1=<K> --path "48'/0'/0'/2'"`
    → **exit 0**, `wsh(multi(2,X/<0;1>/*,X/<2;3>/*))#3sxca8l0`.
  - **Form B at `md encode`** → **exit 0**, `chunk-set-id: 0x00ee4`, four md1 chunks.
    Engraved plates of the R-N1d disjoint shape are producible today.
  - `md decompose "wpkh([73c5da0a/48'/0'/0'/2']xpub…/<0;1>/*)" --emit template`
    → exit 0, `wpkh(@0/48'/0'/0'/2'/<0;1>/*)`.
  - `md encode "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))" --key @0=A --key @1=B --fingerprint @0=… --fingerprint @1=…`
    → exit 0. **N2's primary oracle form is expressible**; the fold's fallback is
    a hedge, not the only path.
  - `md descriptor --from-mk1 mk1qqqq` (no policy card) → clap group error,
    `error: the following required arguments were not provided: <PHRASES|--template <TEMPLATE>>`.

---

# Part 1 — fix verification (15 r1 findings)

| id | verdict | evidence (one line) |
| --- | --- | --- |
| **C1** — enforcement home breaks the read side | **FIXED** | New NORMATIVE "Placement constraint" section forbids `encode_payload`'s validator set by name, restates it as a Non-goal, adds read-side vector rows for `inspect`/`verify`/`bytecode` (not `decode` alone), and adds Acceptance 5 pinning exit 0 for all four reading verbs on legacy cards. |
| **C2** — nonexistent `md build`, missing `address` | **FIXED** | "Verb dispositions (derived from the `Cmd` enum; there is no `md build`)" enumerates REFUSE = `encode`/`descriptor`/`address` (both inputs) and WARN = `decode`/`inspect`/`bytecode`/`verify`; the vector list replaces the `build` row with `address --template` plus card-input rows. |
| **I1** — R6's desugar-reuse claim is false | **FIXED** | R6 now retracts it in the spec's own words ("is a NO-OP on decompose's concrete-descriptor input — 'reuse the shipped desugar' was false") and replaces it with a one-core requirement citing the `template.rs:53-62` keep-in-sync hazard. |
| **I2** — classification key coarser than the parser's | **FIXED** | The table's key is now the triple `(inline origin path, multipath set, wildcard hardening)`, with R-N1-origin and R-N1-hardening as separate rows carrying explicit "MUST NOT cite the repeated-key rule" / "MUST NOT cite BIP 388" constraints. (Residue filed as new M-b.) |
| **I3** — same xpub in two slots: T mints, S refuses | **FIXED** | New "Family 2 — R-N1d" makes mint/compose refuse it, flips both named tests (verified present at `duplicate_key_slots.rs:82` and `:318`) and directs the stale `build.rs:280-283` / `validate.rs:353-355` rationales to be corrected in the same commit. (Consequences filed as new I-b, I-c.) |
| **I4** — N2's oracle omits fingerprints / decl shape | **FIXED** | Oracle now names the FULL input set incl. per-slot fingerprints and the Divergent-vs-Shared shape, and specifies the concrete `md encode` invocation — which I measured executable (inline divergent origins + `--fingerprint @i=HEX`, exit 0), with a stated fallback if it were not. |
| **I5** — R3 conflates "not equal" with input error | **FIXED** | `0 = spend-equal; 5 = NOT spend-equal` (following the real `md repair` exit-5 precedent, `main.rs:652-655`), `1/2 = errors unchanged`, plus a new garbage-`--verify-against` row asserting exit 1 with a decode error; the admissibility matrix sentence closes the smaller half. |
| **I6** — R9(a) moves the arity edge | **FIXED** | R9(b) is now TWO symmetric guards, and the spec explicitly requires the flag-first ordering to produce the named diagnostic "NOT clap's missing-required-argument error", with three rows pinning both orderings. (Mechanism residue filed as new M-c.) |
| **I7** — R-N1c's goal unreachable via the message body | **FIXED** | New "R-N1c's message, fully specified": the rendered line is normative "FROM THE `md:` PREFIX ONWARD" and it "must NOT render through `CliError::TemplateParse`"; Acceptance 4 lifts row assertions to the rendered line. |
| **M1** — singleton-set clause unrepresentable | **FIXED** | The clause is deleted and the deletion is explained ("md's use-site grammar rejects post-multipath fixed steps at lex, and pre-multipath steps lex as origin"), routing the shape to the ORIGIN axis. |
| **M2** — R5(b) widens tests only | **FIXED** | R5(b) now widens "CI's test, clippy AND doc jobs, and … the same three lines of the phase gate, in the same commit"; the phase-gate paragraph matches. Machine-checked: both are already green under `--all-features` (exit 0, exit 0), so the widening cannot turn CI red. |
| **M3** — `--emit md1` input modes undefined | **FIXED** | New "Input modes" paragraph: admissible ONLY with `--from-mk1`/`--from-mk1-file`; named refusals for `--template` and for a keyed-card positional; the `md decompose --emit` name reuse is declared deliberate with per-verb vocabularies. |
| **M4** — escape-hatch spelling does not exist | **FIXED** | R-N1c now mandates the runnable `me sysw pack --as descriptor --in <your export file>`, which matches the real guidance line at `me-cli/src/main.rs:723`. |
| **N1** — `spend_equal` dead-code comment goes stale | **FIXED** | R3 explicitly "DELETES its `#[allow(dead_code)]` and its now-false 'nothing on the CLI surface calls it' comment". |
| **N2** — citation names the wrapper, not reachability | **FIXED** | Machine-verified block now reads "`crates/md-cli/src/cmd/build.rs:301`, the latter reached via `build_descriptor`'s `--template` branch at `build.rs:66` — the caller that makes it cover `md descriptor` AND `md address`". |

**15 / 15 FIXED. No PARTIAL, no NOT FIXED.**

---

# Part 2 — the two briefed pressure points

**(b) Can one classifier invocation-point serve `verify`'s WARN and `encode`'s
REFUSE without contradicting the single-source requirement? — YES. No finding.**
All four template-parsing verbs reach the same funnel (`md encode` and
`md verify` call `parse_template_ext` directly, `cmd/verify.rs:48`;
`md descriptor` and `md address` reach it via `build_descriptor` →
`parse_template`, `cmd/build.rs:64`, and `parse_template` is a thin wrapper,
`template.rs:2560-2566`). The spec's requirement is "ONE classifier
implementation, invoked per verb with **that verb's disposition**", which
explicitly permits a disposition parameter, and `build_descriptor` already
threads a verb name for exactly this purpose (`refuse_key_reuse_across_slots(&descriptor, args.cmd)`).
The WARN/REFUSE split is satisfiable at one home. What is NOT satisfiable is the
classifier's INPUT — that is finding **I-a**, a different axis.

**(a) Are R-N1c's and R-N1d's message claims mutually consistent, and does any
vector row distinguish them? — NO and NO.** That is finding **I-b**.

---

# Part 3 — NEW findings

## I-a (Important) — the placement constraint's named home cannot see Family 2's input, and no single existing invocation point sees both families

**Severity:** Important (unsound assumption; the fold's remedy for C1 names a
home that structurally cannot carry the family the same fold introduced, and the
fallback recreates the divergence the single-source invariant exists to prevent).

**Evidence.** The spec, "Placement constraint … NORMATIVE":

> Single-source requirement: ONE classifier implementation, invoked per verb
> with that verb's disposition. The template-text funnel (`resolve_placeholders`,
> `template.rs:723`) reaches every template-parsing verb

and "Classification": *"The classifier runs on the RESOLVED template and
classifies TWO families."*

Measured signature (`crates/md-cli/src/parse/template.rs:723`):

```rust
pub fn resolve_placeholders(
    occs: &[PlaceholderOccurrence],
) -> Result<ResolvedPlaceholders, CliError>
```

`PlaceholderOccurrence` carries exactly the triple the fold's own table names —
`multipath_alts`, `wildcard_hardened`, `origin_path` (the comparison at
`template.rs:730-741`) — and **no key material**. Family 2's predicate is
"identical key material (public key + chain code) bound to two different
placeholders", so **R-N1d is invisible at the named funnel**.

The two layers that DO carry key material cannot carry Family 1:

- `parse_template_ext(template, keys, fingerprints, experimental)`
  (`template.rs:2589`) has the keys — but it obtains them *after* calling
  `resolve_placeholders` at `:2599`, which has already refused R-N1b, R-N1c,
  R-N1-origin and R-N1-hardening.
- The built `md_codec::Descriptor` has per-`@N` xpubs (`expand_per_at_n`,
  `validate.rs:361`) — but it can never hold R-N1b/c/origin/hardening at all:
  `ResolvedPlaceholders` is one `use_site_path` plus per-`@i`
  `use_site_path_overrides`, i.e. **exactly one use-site path per slot**. That
  is F-417 itself, and it is why the spec calls those shapes "inexpressible".

**Failure construction.** A plan follows the placement constraint literally and
puts the classifier at `resolve_placeholders`. R-N1a, R-N1b, R-N1c,
R-N1-origin and R-N1-hardening land there. R-N1d cannot be written there — there
is no key to compare — so it lands as a **second** implementation at
`cmd/build.rs` / `cmd/encode.rs`. Two classifiers is precisely what
"Verbs must not be able to diverge" forbids, and it is the defect the tree
already names as its own design rule (`cmd/build.rs:277`: *"THE DETECTION IS THE
ENGINE'S OWN CALL, NOT A SECOND COPY"*). The mirror choice — put everything on
the built `Descriptor` — leaves four of the five Family-1 rows with nothing to
classify, because those templates never build.

**Direction (one line).** Specify the classifier's **input** — the occurrence
list plus the resolved per-`@i` key bindings, both available at
`parse_template_ext`, and reconstructible from a decoded card — rather than
naming a funnel that carries half of it.

---

## I-b (Important) — R-N1d's stated authority contradicts row 3 of the same table for the disjoint sub-case, and R-N1d is the one refusal in this cycle with no wording mandate and no rendered-line row

**Severity:** Important (the cycle's central guarantee is diagnostic honesty;
this ships two contradictory BIP verdicts for one measured wallet, and nothing in
the acceptance set can see it).

**Evidence.** The spec's Family-1 table, row 3:

> triples differ ONLY in multipath sets, sets disjoint | **BIP 388 LEGAL** (its
> own valid example, line 291) | **R-N1c** …

and R-N1c's mandated body: *"the wallet is BIP-388-legal; md1 deliberately
cannot express it … keep it as a descriptor"* with the runnable
`me sysw pack --as descriptor` escape.

The spec's Family 2:

> Identical key material (public key + chain code) bound to two different
> placeholders is **forbidden by BIP 388's pairwise-distinct rule (line 193)
> REGARDLESS of use sites** … The operator's standing ruling decides it:
> **mint/compose refuses R-N1d.**

**These two rows are about the same wallet.** Measured at this tree:

```
Form A: md descriptor --template "wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))" --key @0=<K> --path "48'/0'/0'/2'"
        exit 1 — md: template parse error: @0 appears with inconsistent path/multipath/hardening   [→ R-N1c]

Form B: md descriptor --template "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=<K> --key @1=<K> --path "48'/0'/0'/2'"
        exit 0 — wsh(multi(2,X/<0;1>/*,X/<2;3>/*))#3sxca8l0                                        [→ R-N1d]

Form B at mint: md encode <same> → exit 0, chunk-set-id 0x00ee4, four md1 chunks.
```

Form A and Form B denote the byte-identical concrete descriptor. After this
cycle the operator who typed Form A is told the wallet is **legal under BIP 388**
and handed a descriptor escape; the operator who typed Form B is told the same
wallet is **forbidden by BIP 388** under the ruling "we don't want to support BIP
forbidden wallets", with no escape named.

The honest reconciliation exists and the fold dropped it. The brainstorm's
Direction 2 states it: *"there is no workaround spelling: the same xpub under a
second placeholder violates BIP 388's pairwise-distinct rule, so the wallet is
genuinely inexpressible as an md1 card."* The **wallet** is legal; **both md1
spellings** are inadmissible — one by the pairwise-distinct rule, one by F-417.
R-N1d's text attributes the prohibition to the wallet.

**Failure construction.** R-N1d is the only new refusal with no wording
constraint — R-N1a must cite the repeated-key rule, R-N1b must name disjointness
primary, R-N1c is fully specified, R-N1-origin and R-N1-hardening carry explicit
MUST-NOTs; R-N1d carries none. The implementation therefore reuses the refusal
already sitting at the site R-N1d extends — `refuse_key_reuse_across_slots`'s
`CliError::KeyReuse` (`cmd/build.rs:302-311`):

> `@{a}` and `@{b}` were given the SAME extended public key **at the same
> use-site** … `md encode` and `md decompose` already refuse this wallet, so **a
> card minted from it could never be read back**.

For Form B every clause is false: the use sites are not the same, `md encode`
mints it (measured above, exit 0, four chunks), and the card **can** be read back
— Acceptance 5 of this very spec *requires* it to stay readable. No row detects
this: R-N1d's only named row is "the two flipped tests" (a refusal row, not a
message row), and the fold **deleted** r0's Acceptance-4 clause "asserted by the
vector rows, not by convention" (see M-a).

**Direction (one line).** Split R-N1d into its same-use-site half (already
shipped; wording stands) and its disjoint half (authority = F-417
inexpressibility, message = R-N1c's including the descriptor escape), and give
the disjoint half a rendered-line vector row like every other refusal in the
taxonomy.

---

## I-c (Important) — the Non-goal "no placement inside `encode_payload`'s validator set" cannot hold together with R-N1d's stated scope and the single-source requirement

**Severity:** Important (three normative statements in one spec, at most two of
which can be honoured; one resolution silently removes a shipped codec-level
mint guarantee).

**Evidence.** Three statements in `df420bda`:

1. *"Single-source requirement: ONE classifier implementation"* (Placement
   constraint).
2. *"No placement of the taxonomy inside `encode_payload`'s validator set (the
   C1 constraint, restated)"* (Non-goals) — and the constraint section's
   "MUST NOT be enforced inside `encode_payload`'s validator set".
3. R-N1d covers identical key material at two placeholders **"REGARDLESS of use
   sites"** — which subsumes the same-use-site case.

The same-use-site case is `validate_no_duplicate_key_slots`
(`crates/md-codec/src/validate.rs:361`), and its call at
`crates/md-codec/src/encode.rs:120` sits **inside `encode_payload`** — verified
by reading `encode.rs:99-135`: it is the last validator before the first
`BitWriter` write, tagged `// F-218: refuse to mint a policy that names more
cosigners than it has.` So part of the taxonomy is already inside the set the
fold declares off-limits, and it must stay there: `cmd/build.rs:277-287`
documents that calling the engine's own validator rather than a second copy is
what stops `md descriptor` and `md encode` from diverging.

**Failure construction.** Pick any two:

- **(1)+(2)** — one classifier, none of it in `encode_payload` — requires
  deleting `validate_no_duplicate_key_slots` from `encode_payload`. md-codec
  then loses its wire-level F-218 refusal, and any consumer of `encode_payload`
  outside `md-cli` can mint the card the check exists to prevent. That is a
  funds-shaped regression introduced by a Non-goal.
- **(2)+(3)** — R-N1d keeps its full scope and stays out of `encode_payload` —
  yields two implementations of the same predicate, violating (1).
- **(1)+(3)** — one classifier covering the same-use-site case, left where it is
  — violates (2), the C1 constraint this fold made normative.

**Related over-claim, same section.** The placement constraint says the hazard is
that the bad placement "makes already-engraved **R-N1a/R-N1d** plates
uninspectable". Only half of R-N1d has engraved plates: the same-use-site half
has been unmintable since F-218 landed in `encode_payload`, while the disjoint
half mints today (measured, `chunk-set-id 0x00ee4`). The sentence is true of
R-N1a and of R-N1d's disjoint half only.

**Direction (one line).** Scope R-N1d explicitly to the **disjoint-use-site
delta**, and state that `validate_no_duplicate_key_slots` stays inside
`encode_payload` and sits outside the single-source scope as the codec-layer
floor.

---

## M-a (Minor) — Acceptance 4 lost its enforcement clause in the fold

r0's criterion 4 read: *"No diagnostic introduced by this cycle contains the word
'invalid' … **asserted by the vector rows, not by convention**."* The folded
criterion 4 keeps the prohibition and replaces the enforcement clause with a rule
about *how* diagnostic rows assert (the rendered line). Read strictly, a
diagnostic that has **no** row is now compliant by convention — which is exactly
what the deleted clause forbade, and R-N1d is exactly such a diagnostic (I-b).
Restore the clause alongside the new rendered-line requirement; they are
complementary, not alternatives.

## M-b (Minor) — R-N1-origin's cited authority condemns md's own emitted template form

The Family-1 table grounds the origin axis in *"outside BIP 388's KEY grammar
(line 305 lists an explicit path on a placeholder as invalid)"*. Inline origins
on placeholders are md's **normal** template spelling, not an error class:
`md decompose … --emit template` prints `wpkh(@0/48'/0'/0'/2'/<0;1>/*)`
(measured, exit 0) and `md encode` mints from that form (measured, exit 0). The
disposition text correctly mandates the md-side wording ("one origin per key in
md1") and bars the repeated-key citation, but it does **not** bar citing line
305 — and a diagnostic that did would tell the operator that md's own
`--emit template` output is BIP-invalid, in a cycle whose Principle is
"NEVER 'invalid'". Add a MUST-NOT for the line-305 citation, or move that note
out of the authority column into a spec-internal aside.

## M-c (Minor) — R9's prescribed mechanism opens a case no row names

R9 says the mechanics are the plan's, "e.g. relaxing the `descriptor_input`
group when `--from-mk1` is present and refusing in code". Today that group is
what catches a `--from-mk1` invocation with **no policy card at all** — measured:
`md descriptor --from-mk1 mk1qqqq` → `error: the following required arguments
were not provided: <PHRASES|--template <TEMPLATE>>`. Relax the group and that
invocation reaches seating code with an empty policy input and whatever error
falls out of it. R9's three named rows (positional-first composes; flag-first
with a trailing md1; mk1 in the positional) do not include it. Name a fourth row,
or name a mechanism that does not relax the group.

## N-a (Nit) — a conditional row versus Acceptance 1's "Every"

R-N1-hardening's row "lands only if reachable", while Acceptance 1 requires
"**Every** vector row named above exists as an executable test in the same commit
as its implementation". The hedge is explicit and adjacent, so this will not
mislead, but criterion 1 should exempt the conditional row by name rather than
leaving a reader to reconcile them.

---

COUNTS (new): 0C / 3I / 3M / 1N; r1 findings: 15/15 FIXED
