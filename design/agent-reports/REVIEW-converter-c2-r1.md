# REVIEW — converter C2 seating engine, round 1 (mechanical)

Scope: the SEATING DIFF only (`git diff a3482649..16374b72`), against
`design/SPEC_wallet_form_converter.md`'s "NORMATIVE — the seating engine"
section + P2, and `design/IMPLEMENTATION_PLAN_wallet_form_converter.md` §3 C2
/ §4 roster. SETTLED per the dispatch brief and NOT re-run: the four exit
gates (987/987 nextest, clippy -D warnings clean, fmt clean, 28/28 families
with 78 rows) and `seat/mod.rs`'s byte-identical matrix. This round's budget
went only to the six questions below.

**Result: 0 Critical, 0 Important, 2 Minor.**

## Q1 — ROWS vs SPEC

**(a) A3 ambiguity refusal — full chunk-set id, slots, both remedies.** PASS.
`matching.rs:216-221`'s `REMEDIES` const states both remedies verbatim
(re-mint with `--fingerprint`; `--seat '@i=<chunk-set-id>'`) and is shared by
`ambiguity_refusal` (`:223-235`) and `over_bound_refusal` (`:237-248`), both
built from `multi_slot_cards` (`:196-214`), which names every multi-candidate
card by `card.label()` (full set id + stub) and every one of its candidate
slots by `decls[si].label()`. Unit-level `v_usp_use_site_path_swap_refuses`
(`matching.rs:389-404`) asserts each card's full `set_id.to_string()` is
present plus `"@0 ["`/`"@1 ["`; CLI-level
`v_amb_the_ambiguity_refusal_reaches_the_operator_with_exit_1`
(`seating_vectors.rs:158-174`) asserts exit 1, the match count, both remedy
strings, and empty stdout. Coverage is split unit/CLI by design (stated in
the test file's header) but nothing the spec demands is missing across the
pair.

**(b) Key-reuse diagnostics never say "invalid".** PASS. Grepped
`invalid`/`Invalid` case-insensitively across `crates/md-cli/src/seat/` and
`seating_vectors.rs`. Every hit is either a doc comment quoting BIP 388's own
"invalid-example list" (`satisfy.rs:174,181,293` — describing the BIP text,
not an emitted string) or a test assertion checking the word's *absence*
(`satisfy.rs:509-513,536-538,587-589`; `seating_vectors.rs:465`). Read the
three key-reuse diagnostic builders in full
(`check_no_repeated_placeholder`, `check_no_identical_fp_bearing_declarations`,
`check_no_repeated_xpub`, `satisfy.rs:188-315`): none interpolates a
lower-layer error string, so none can ever emit "invalid" through any input.
Confirmed live: captured the actual V-COLLIDE stderr text (a *different*,
non-key-reuse refusal that does wrap a lower-layer `mk_codec` error) —
`"chunk-set 12345: the 5 string(s) declaring this id do not reassemble into
one key card: chunked-header malformed: received 5 chunks, header declares
total_chunks = 2. Two DIFFERENT cards..."` — no "invalid". Separately
confirmed that a genuinely malformed mk1 string *does* surface `"invalid
HRP: ..."` via `group_key_of`'s decode-failure path (`input.rs:114-120`) —
correctly so, since that path describes actually malformed wire data, not a
BIP-forbidden-but-valid shape, so it sits outside the ruling's scope.

**(c) B1's WARNING names both readings + directs the check.** PASS.
`disposition.rs:131-139` builds: `"card <id>'s stub matches neither this
policy's shape id nor the composed wallet id — minted under different origin
metadata (legitimate), or a different wallet; verify address 0 before
trusting."` — matching SPEC B1's quoted message. Test
`v_b1_warn_the_232214e4_card_warns_with_both_readings_named`
(`disposition.rs:267-294`) asserts all four required substrings.

**(d) B2's else-branch: address 0 on stderr, stdout stays the machine
contract.** PASS. `disposition.rs:146-155`'s `address_zero_note` builds the
standing instruction; `cmd/descriptor.rs:131-135` and the address command's
equivalent call site emit every note via `eprintln!` only, while stdout gets
exactly `rendered` (`println!("{rendered}")`, `descriptor.rs:119`) or the
address rows. `v_ord_stdout_carries_the_descriptor_and_nothing_else`
(`seating_vectors.rs:121-144`) asserts stdout is exactly one line with no
`"note:"`/`"warning:"`, and stderr carries the standing instruction
verbatim.

## Q2 — REFUSAL-BEFORE-NAMING

PASS. Walked every refusal construction site in `src/seat/`:
`input.rs::group_key_of`, `::decode_cards` (empty-input, reassembly);
`satisfy.rs`'s two door checks + two card-set checks; `matching.rs`'s
ambiguity/over-bound refusals; `complete.rs::refusal` (A4); `directive.rs`'s
five `--seat` refusal branches (malformed shape, out-of-range slot, unknown
id, contradicting A2, conflicting directives). Every A2/A3/A4/A5-class
refusal — the ones SPEC requires to name cards/slots/remedies — does so on
every branch; there is no early-return that skips the naming code (each
refusal is built as a single `format!` call with the names already
substituted, not assembled incrementally with an escape hatch). The
remaining refusal sites (malformed `--seat` syntax, an undecodable mk1
string, an unreadable `--from-mk1-file` line) are about the *input itself*
before any card set exists, which is outside the spec's naming requirement
and is reasonably scoped that way in the code.

## Q3 — FALSE-PASS SHAPES

**(a) Bare nonzero-exit / generic-substring rows.** PASS — none found.
Grepped every `unwrap_err()`/`is_err()`/`status.code()`/`status.success()`
call across `src/seat/*.rs` and `seating_vectors.rs`; every refusal-path test
pairs the exit/error check with fixture-specific message content in the same
test body. The one bare `.is_err()` (`directive.rs:203`) is a setup
precondition ("without `--seat` this fixture is ambiguous") inside a test
whose real assertions follow, not a roster row's own verdict.

**(b) The 15-row `*_reaches_the_command` table.** PASS, with one Minor.
`v_r5m1`/`v_bound_ref` assert the BIP-388 wording; `v_r2_ord`/`v_r4_ik`/`v_cap`
assert exact matching counts (`"24"`, `"120"`, `"more than 720"`);
`v_unfilled`/`v_leftover`/`v_fpfree_card` assert exact slot/card counts and
origin labels; `v_collide` asserts the exact colliding id; the four
must-SEAT rows assert `status.success()` plus the descriptor's tag prefix.
`v_door` (`"declare the IDENTICAL origin"`) and `v_grp` (`"do NOT all compose
to the same wallet"`) assert a phrase specific to their refusal class rather
than a count or full message — weaker than their siblings, though not a
false-PASS shape (the phrase can't fire from a different refusal class), and
each fixture's fuller content is independently pinned by its unit-level
sibling test in `satisfy.rs`/`matching.rs`. **Minor.**

**(c) Stream misdirection (stdout/stderr).** PASS. Cross-checked every
`out_of(&o)`/`err_of(&o)` call site in `seating_vectors.rs` (grep, all ~35
sites read): stdout assertions are always about the descriptor/addresses or
its emptiness on refusal; stderr assertions are always about notes,
warnings, or refusal text. No site checks descriptor content on stderr or
note content on stdout.

## Q4 — PIPELINE ORDER IN THE CODE

PASS. `input.rs::decode_cards` (`:155-185`) runs, in this literal order:
step 1 `dedupe_strings` (`:90-102`) → step 2 group into a `BTreeMap<GroupId,
Vec<String>>` (`:164-168`) → step 3 `mk_codec::decode` per group
(`:170-183`). No special-casing exists for either V-DUP or V-COLLIDE — both
are generic consequences of this one pipeline: V-DUP's doubled set collapses
at step 1 before grouping ever sees it; V-COLLIDE's two DIFFERENT cards
pinned to one id merge into one group at step 2 and fail generically at step
3's `mk_codec::decode` call. Confirmed live by capturing V-COLLIDE's actual
refusal text (above, Q1b) — it is exactly the generic reassembly-failure
wrap around `mk_codec`'s own "chunked-header malformed" error, not a bespoke
"colliding id" message.

## Q5 — THE 720 CAP

PASS. `matching.rs::dfs` (`:111-146`) checks, at the point it would record
the `MATCHING_BOUND`-th+1 (721st) complete assignment: `if found.len() ==
MATCHING_BOUND { *over = true; return; }` (`:122-127`) — this fires *before*
the 721st matching is pushed, so the search never materializes more than 720
matchings before stopping. Confirmed by
`v_cap_the_bound_is_on_total_matchings_not_per_component`
(`matching.rs:488-497`), which calls `enumerate()` directly on the 12-card,
518,400-matching V-CAP fixture and asserts `Enumerated::OverBound` (not a
fully materialized `Vec`), and by the from-below/from-above boundary test at
exactly 720/721 (`matching.rs:499-515`, asserting 6! = 720 enumerates fully
and 7! = 5040 trips the bound). `over_bound_refusal` (`matching.rs:237-248`)
prints `multi_slot_cards` — every card with more than one candidate slot,
alongside its labelled candidate slots — the same graph-property function
`ambiguity_refusal` uses, so the refusal states the bound and prints cards +
candidate slots exactly as SPEC A3 requires, available even though the
matchings themselves are uncounted past 720.

## Q6 — DEVIATIONS 3-8 vs SPEC TEXT

**3 (single `CliError::Seat`, exit 1).** No contradiction — SPEC and plan
are both silent on exit codes; this is a plan/implementation-level choice.

**4 (`--from-mk1-file` skips blank/`#` lines, refuses any other unrecognized
line by name).** No contradiction of SPEC TEXT (SPEC only says "one string
per line," silent on comment/blank handling) — but the report's stated
justification is inaccurate. I read the cited precedent directly:
`mnemonic-key/crates/mk-cli/src/cmd/encode.rs::read_md1_set` (`:458-476`)
does *not* refuse per-line on an unrecognized line; it silently drops
*every* line that doesn't start with `md1` (blank, comment, or garbage
alike) and only refuses if the whole file yields zero matches. The C2
behavior (refuse by name on anything that isn't blank/comment/mk1) is
stricter than that precedent, not modeled on it — arguably the safer choice
for a restore tool, so not a functional defect, but "skipping follows mk's
own `--from-md1-set` precedent" mischaracterizes what that precedent does.
**Minor** (report-accuracy, not a code or spec defect).

**5 (dedupe applied to the md1 policy-card side too).** No contradiction —
SPEC A3(a)'s literal words are written about mk1 strings, but "a full card
string set supplied twice over" (P2/A3(a)) is naturally read to include a
drawer-scanned policy card, and V-DUP's own test doubles both halves
deliberately (`seating_vectors.rs:62-95`, comment states so). A defensible
reading, not an inversion.

**6 (A1 not a separate pass; B1 recomputes the template id inline).** No
contradiction — SPEC states "Final stub disposition is B1's," and A1's
shape-matched criterion (stub vs. the policy's `WalletDescriptorTemplateId`
top-4) is invariant of the assignment, so B1 recomputing it
(`disposition.rs:69,75`) yields the identical value a stored A1 pass would
have carried forward. Verified the two computations are literally the same
call (`compute_wallet_descriptor_template_id`), not a re-derivation that
could drift.

**7 (confirmed tiers summarised one line per tier, not per card).** No
contradiction — SPEC's only format requirement is on the WARNING message
(B1's quoted text, one per card, which the code does — `disposition.rs:131-
139`); it does not prescribe a line-per-card format for the confirmed
tiers, and `notes()` (`:92-141`) still names every card's set id within the
tier line via `.join(", ")`.

**8 (two unrostered guards: `--seat` without cards; a keyed card with
`--from-mk1`).** No contradiction — both are additive refusals SPEC doesn't
discuss, reachable through the new surface, each named and tested
(`v_seat_bad_seat_without_cards_says_what_it_needs`,
`v_msg_keyless_a_keyed_card_with_from_mk1_says_there_is_nothing_to_seat`).

*(Checked but not counted as a finding: whether `--key`/`--fingerprint`
could silently no-op alongside `--from-mk1`. Verified this is unreachable —
`--key`/`--fingerprint` both carry `requires = "template"`
(`main.rs:298,305`) and `--template` carries `conflicts_with = "from_mk1"`
et al. (`main.rs:398-424`), so clap itself refuses the combination as a
usage error before the seating engine ever runs.)*

## Summary

| # | Severity | Finding |
| --- | --- | --- |
| 1 | Minor | `v_door_reaches_the_command`/`v_grp_reaches_the_command` assert a class-specific phrase rather than an exact count or full message, unlike their sibling rows in the same table (Q3b) |
| 2 | Minor | Deviation 4's stated precedent ("follows mk's own `--from-md1-set` precedent") mischaracterizes `read_md1_set`, which silently drops unrecognized lines rather than refusing per-line (Q6) |

No Critical, no Important. No secret-handling findings encountered in this
scope.
