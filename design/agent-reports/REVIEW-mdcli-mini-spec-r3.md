# REVIEW — SPEC_mdcli_mini.md, R0 round 3 (scoped fold re-review)

| field | value |
| --- | --- |
| artifact | `design/SPEC_mdcli_mini.md` |
| commit | `9d348f68` (main tip; fold of `f4023e8d`'s r2 report) |
| date | 2026-08-31 |
| reviewer | independent agent, no authorship of the artifact or the fold |
| repo | `/scratch/code/shibboleth/descriptor-mnemonic` |

**Scope (as briefed, and held to).** Two questions only: (1) does the fold in
`9d348f68` fix each of the 7 findings in
`design/agent-reports/REVIEW-mdcli-mini-spec-r2.md`; (2) did the fold's NEW text
introduce a defect — a contradiction between sections, an unexecutable or
cannot-fail vector row, or a claim the tree contradicts. The fold's new text is
`git diff f4023e8d..9d348f68 -- design/SPEC_mdcli_mini.md`, read against enough
surrounding spec text to judge consistency. NOT re-derived, per the brief: r1's
15 findings, the operator rulings, the BIP-388 line numbers, the Machine-verified
block's code citations, and r2's own live measurements (`0x00ee4`, the two
`--all-features` exit-0 runs, N2's oracle form).

**Machine-checked by this reviewer before writing (nothing below rests on a
described fact).** All at `target/debug/md`, this tree:

- `parse_template_ext` body (`parse/template.rs:2589-2599`): `let occs =
  lex_placeholders(template)?;` then `reject_unreferenced_bindings(&occs, keys,
  fingerprints)?;` then `resolve_placeholders(&occs)?` — the occurrence list AND
  the `keys` parameter are both in hand in that body, before the Family-1
  refusal fires. The fold's input claim (i)+(ii) is TRUE at that location.
- `md_codec::Descriptor` (`encode.rs:17-27`) carries `n`, `path_decl`,
  `use_site_path`, `tree`, `tlv`; `tlv.pubkeys` holds the per-slot key material
  and `expand_per_at_n` already reconstructs per-`@N` `(xpub, use_site_path)`
  (`validate.rs:361-380`). Both classifier inputs really are reconstructible
  from a decoded card.
- **R-N1a mints today:** `md encode "wsh(multi(2,@0/<0;1>/*,@0/<0;1>/*))" --key
  @0=<K> --path 48'/0'/0'/2' --force-chunked` → **exit 0**, `chunk-set-id
  0xd2d7e`, two chunks. (The fold's new "such plates exist for R-N1a … mintable
  today" is true, not merely asserted.)
- **The R-N1d delta card composes from CARD INPUT:** minting
  `wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))` with `--key @0=<K> --key @1=<K>` (exit 0,
  four chunks), then feeding those four md1 strings back:
  - `md descriptor <cards>` → **exit 0**,
    `wsh(multi(2,xpub661…pvZG2s/<0;1>/*,xpub661…pvZG2s/<2;3>/*))#3sxca8l0`
  - `md address <cards> --count 1` → **exit 0**, `bc1qsa6qq…lqyh88`
  - `md decode <cards>` → exit 0, template `wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))`
- **The new must-COMPOSE control row is executable and non-vacuous today:**
  `md descriptor --template
  "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))" --key @0=<K0>
  --key @1=<K1> --fingerprint @0=73c5da0a --fingerprint @1=73c5da0a` → **exit 0**,
  two DIFFERENT rendered xpubs under one fingerprint.
- `refuse_key_reuse_across_slots` has exactly ONE call site,
  `cmd/build.rs:66` — inside the `if let Some(template)` branch. The phrase/card
  branch (`build.rs:70-77`) returns `decode_md1_string` / `reassemble` with no
  reuse check at all.
- `seat::satisfy::check_no_repeated_xpub` (`seat/satisfy.rs:294-315`, called at
  `seat/mod.rs:140`) compares `(public_key, chain_code)` across decoded CARDS
  with **no use-site term** — the S row already refuses R-N1d's delta today.
  Its anti-over-refusal control exists: `satisfy.rs:594`
  `v_bound_ref_control_different_masters_at_one_path_pass`.
- `md decompose` is not a hole: both disjoint shapes already refuse
  (`cmd_decompose.rs:474` overlapping sets, `:486` disjoint sets).
- Tree-wide grep for other pins of the old one-key-two-use-sites behavior
  (`one_key_at_two|two_disjoint_use_sites|different_use_sites|not_a_duplicate`
  over `*.rs`/`*.md`/`*.sh`): the only *code* hits are the two tests the spec
  names (`duplicate_key_slots.rs:82`, `:318`) and the doc comment
  `cmd/build.rs:280` the spec already directs to be corrected. No unmentioned
  test or comment pins it.

---

# Part 1 — fix verification (7 r2 findings)

| id | verdict | evidence (one line) |
| --- | --- | --- |
| **I-a** — placement constraint's named home cannot see Family 2's input | **FIXED** | The funnel naming is withdrawn in the spec's own words ("this spec does NOT name one … r1's `resolve_placeholders` naming was wrong for Family 2: its signature carries no key material") and replaced by an INPUT statement — (i) occurrence list, (ii) resolved per-`@i` key bindings — which I verified are both in hand inside `parse_template_ext` (`occs` at `:2590`, `keys` a parameter, both before `resolve_placeholders` at `:2599`) and both reconstructible from a decoded card (`tlv.pubkeys` + `expand_per_at_n`). Single-source survives as two requirements (one implementation per predicate; disposition as a parameter) that a single invocation point can satisfy. |
| **I-b** — R-N1d's authority contradicts table row 3 for the disjoint sub-case; no wording mandate, no rendered-line row | **FIXED** | New "R-N1d's message mandate" separates WALLET from SPELLING: the wallet is R-N1c's and is BIP-legal (row 3 stands, line 291); the two-placeholder SPELLING repeats the key in the key-information vector (line 193); the one-placeholder spelling is F-417-inexpressible. The mandate bans reuse of the shipped `CliError::KeyReuse` wording by naming both false clauses ("at the same use-site", "could never be read back"), requires the R-N1c escape, and Vectors now mandate "its full rendered-line row". The two rows are no longer about the same claim. |
| **I-c** — Non-goal vs R-N1d's scope vs single-source: at most two can hold | **FIXED** | R-N1d is re-scoped to the disjoint-use-site DELTA; the same-use-site case is named the CODEC-LAYER FLOOR, exempted, and pinned in place; C1 is re-worded to bind only checks *this cycle adds*; the Non-goal now forbids BOTH a new check inside `encode_payload` AND removal of the shipped floor. The three statements hold simultaneously: floor predicate `xa==xb && use_site==`, delta predicate `xa==xb && use_site!=` — disjoint predicates, one implementation each, the new one outside `encode_payload`. The over-claim is corrected too, with the correct half-truth spelled out ("the same-use-site half has been refused at the codec floor since F-218") — and I measured that R-N1a plates really are mintable (exit 0, `0xd2d7e`), so the retained half of the claim is true. |
| **M-a** — Acceptance 4 lost its enforcement clause | **FIXED** | Criterion 4 now carries both: the rendered-line rule AND "Every diagnostic this cycle introduces or rewrites HAS such a row — asserted by the vector rows, not by convention". No conflict with criterion 1's new exemption (an unreachable R-N1-hardening introduces no diagnostic). |
| **M-b** — R-N1-origin's authority condemns md's own emitted form | **FIXED** | The authority column is now "md representability (one origin per key)"; the disposition adds "MUST NOT cite BIP 388 at all", states the measured reason (`--emit template` prints inline origins, `md encode` mints them), and demotes line 305 to an explicitly non-diagnostic aside. |
| **M-c** — R9's mechanism opens a case no row names | **FIXED** | A fourth row is added verbatim for it: "`--from-mk1` with NO policy card anywhere → a refusal naming the missing policy input", with the reason the group must not simply be relaxed. |
| **N-a** — conditional row vs Acceptance 1's "Every" | **FIXED** | Criterion 1 now exempts R-N1-hardening's row by name. |

**7 / 7 FIXED. No PARTIAL, no NOT FIXED.**

---

# Part 2 — the four briefed pressure points

**(a) Does the floor/delta split dissolve I-c's three-way contradiction? — YES.
No finding.** The two predicates are disjoint on the use-site term, so "one
implementation per predicate", "no new check inside `encode_payload`", and
R-N1d's (now narrowed) scope can all hold at once against the real tree: the
floor stays at `encode.rs:120`, the delta lands above it, and neither is a copy
of the other.

**(b) Is "reconstructible from a decoded card" implementable? — YES. No
finding.** A decoded `Descriptor` carries the key material in `tlv.pubkeys` and
the per-slot triple across `path_decl` / `use_site_path` / the use-site override
TLV; `expand_per_at_n` already performs exactly that reconstruction for the
shipped floor. The bytecode tree is not an obstacle: placeholder occurrences are
tree references, and md1's one-path-per-slot narrowness (F-417) means every
occurrence of `@N` carries `@N`'s triple. For a keyless policy card (ii) is the
empty set, which makes R-N1d vacuous rather than unimplementable.

**(c) Does anything else pin the OLD one-key-two-use-sites behavior? — Nothing
unmentioned. No finding.** Tree-wide grep returns only the two named tests and
the `build.rs:280` comment the spec already directs to be corrected in the same
commit. Two details the plan will meet, not spec defects: the named test at
`duplicate_key_slots.rs:82` also asserts `md address` derives an address from
the split-use-site form (`:114-160`), so the flip covers an `address` assertion
too, not only `encode`; and `md decompose` already refuses both disjoint shapes
(`cmd_decompose.rs:474`, `:486`), so it needs no flip.

**(d) Does the must-COMPOSE control catch a fingerprint-keyed misimplementation?
— YES. No finding.** I ran the row: same declared fingerprint, different account
paths, different xpubs → exit 0 today. An implementation keying Family 2 on the
master fingerprint instead of `(public key, chain code)` refuses that row, so
the row fails and the defect is caught. (The refusal rows alone would not catch
it — a fingerprint-keyed implementation reaches the right verdict there for the
wrong reason.) `validate_no_duplicate_key_slots`' own doc comment
(`validate.rs:347-350`) states this exact hazard, so the control is aimed at a
real mistake.

---

# Part 3 — NEW findings

## I-1 (Important) — R-N1d is row-pinned on the T row only, while the dispositions require the CARD route to refuse it; measured, that route composes the delta at exit 0 today and no named row can fail

**Severity:** Important (missing case; the whole R-N1d vector set passes against
an implementation that leaves the divergence R-N1d exists to close still open on
a second route). Structurally this gap predates the fold, but the fold rewrote
the sentence that carries it and newly supplied the premise that makes it
consequential, without propagating it.

**Evidence.** Verb dispositions (unchanged text, `SPEC:185-187`):

> **REFUSE (mint/compose):** `encode`; `descriptor` (both `--template`
> and card input); `address` (both inputs).

The fold's rewritten Vectors sentence (`SPEC:202-205`):

> R-N1d T-row refusal with its full rendered-line row (the two flipped
> tests become refusal rows; the message row asserts the mandate
> above); the R-N1d must-COMPOSE control …

Every R-N1d row named there is on the template path. The card path gets rows for
**R-N1a only** — "card-input composing refusals (`descriptor` and `address` on a
hand-built R-N1a card)" — so the spec plainly knows the card route needs its own
rows and omits them for exactly the family whose predicate (key material) a card
can carry pre-seated.

And the fold's OWN new text supplies the premise that makes this reachable, in
the Machine-verified block (`SPEC:51-53`):

> the two-placeholder disjoint same-key form MINTS today (`md encode`, exit 0,
> chunk-set-id `0x00ee4`) — plates carrying R-N1d's disjoint half can exist.

Measured at this tree, minting that form and feeding the four md1 chunks back:

```
md descriptor <4 md1 chunks>   → exit 0
   wsh(multi(2,xpub661…pvZG2s/<0;1>/*,xpub661…pvZG2s/<2;3>/*))#3sxca8l0
md address    <4 md1 chunks> --count 1 → exit 0
   bc1qsa6qqvkypr9v8ve5z54t8yjtn99ws48w8umyyzyhxy8n6p4msemslqyh88
```

Structural confirmation of why: `refuse_key_reuse_across_slots` has exactly one
call site, `cmd/build.rs:66`, inside the `if let Some(template)` branch; the
phrase/card branch returns `decode_md1_string` / `reassemble` with no reuse
check at all.

**Failure construction.** The plan implements R-N1d's delta as a classifier on
the template path — the only path any named R-N1d row exercises. The two flipped
tests refuse, the rendered-line row asserts the mandate, the must-COMPOSE
control composes, the V-BOUND-REF sibling refuses at seating. **Acceptance 1 and
Acceptance 4 are both satisfied.** Ship it, and:

```
md descriptor --template "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=X --key @1=X   → REFUSED
md descriptor <the card md encode minted from that same template>                      → exit 0, composes
```

One binary, one wallet, two answers, differing only by input form — the exact
defect `cmd/build.rs:262-287` documents as the reason the T-row guard was added
in the first place ("`md decompose` then refused the exact string `md
descriptor` had just printed"). No row in the spec fails when this ships.

**Direction (one line).** Name R-N1d card-input rows the way R-N1a already has
them — `descriptor` and `address` on a minted delta card must refuse — and say
which side of the seat/compose boundary owns them, since the card route today
runs neither `refuse_key_reuse_across_slots` nor `check_no_repeated_xpub`.

---

## M-1 (Minor) — the single-source exemption covers the codec floor but not the S row's shipped `check_no_repeated_xpub`, whose only mention the fold deleted

**Evidence.** The fold's new clause (`SPEC:175-178`):

> The normative requirements are: each predicate has ONE implementation (no
> per-verb second copy — `build.rs:277`'s own rule) …

and the exemption it grants, which reaches one place only (`SPEC:163-165`):

> The shipped floor (`validate_no_duplicate_key_slots` and its sibling
> validators **inside `encode_payload`**) is out of this cycle's scope and stays.

The fold simultaneously DELETED r1's sentence "The seating engine already
refuses exactly this (`check_no_repeated_xpub`, key material only)" — after which
the spec contains no mention of that function at all (`grep` over the spec: zero
hits). It is real and it is a second implementation of R-N1d's predicate:
`seat/satisfy.rs:294-315` compares `(public_key, chain_code)` across decoded
cards with **no use-site term**, called at `seat/mod.rs:140`, i.e. the S row is
already at R-N1d's target behaviour and by a different code path with different
wording. Its anti-over-refusal control already exists at `satisfy.rs:594`, so
the fold's other new claim — "the existing S-row control" — is TRUE.

**Why it is Minor, not Important.** Nothing forces its removal: it sits outside
`encode_payload`, so the C1 constraint does not reach it, and its S-row verdict
already agrees with R-N1d's. The risk is that a plan applying "each predicate has
ONE implementation" literally deletes a shipped funds-safety refusal whose
diagnostic names the two colliding CARDS (the useful referent on that route) —
the same shape as r2's I-c, one layer up, and the fold fixed I-c by writing an
explicit exemption rather than leaving it to the plan.

**Direction (one line).** Extend the exemption sentence to name
`seat::satisfy::check_no_repeated_xpub` as the seat-layer floor that stays, and
restore the one-clause fact that the S row already refuses the delta — which
also scopes R-N1d's convergence work to the T row and the card route.

---

## N-1 (Nit) — Acceptance 5's "refused shapes" is unqualified, and the floor's shape is a refused shape the fold just declared out of scope

Acceptance 5 requires the reading verbs to complete at exit 0 "on already-engraved
cards carrying refused shapes". In the fold's own vocabulary the same-use-site
duplicate IS a refused shape (Family 2's first bullet says the floor "refuses"
it), and a card carrying it — hand-built, or minted before F-218 landed — does
NOT read at exit 0, because `md inspect` and `md verify` re-enter
`encode_payload` (the spec's own Machine-verified block). The chain that
reconciles it exists — criterion 5 says "per the C1 constraint", C1 now binds
only new checks, and the floor is exempt from C1 — but it takes three sentences.
Narrow criterion 5 to "shapes this cycle newly refuses" and the reconciliation
is local.

---

COUNTS (new): 0C / 1I / 1M / 1N; r2 findings: 7/7 FIXED
