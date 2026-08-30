# IMPL — converter C2 (P2, the S row: the seating engine)

**Phase: COMPLETE.** All seven of plan §3 C2's build-order steps landed, plus
one fix commit for a defect the phase's own end-to-end sweep found. Every
gate below was RUN, and every number in this report was read off a command's
output rather than counted by hand.

Worktree `/scratch/code/worktrees/converter-c2`, branch `impl/converter-c2`,
branched from `impl/converter-c1` at `a3482649`. Nothing pushed; nothing on
`main` or the other `impl/` branches touched; the worktree is left in place.

Final commit: **`b5061b8f`**.

## Commits, in build order

| SHA | step | what landed |
| --- | --- | --- |
| `7aead734` | 1 | P2's normative input pipeline: dedupe → group by chunk-set id → reassemble under `mk decode` |
| `7ee0167d` | 2 | A2 satisfaction, the two door checks, the two card-set checks |
| `6a900d11` | 3 | A3: perfect matchings, compose-canonicalise-compare, the 720 cap, the assignment-vector tie-break; composition, the comparison form, the spend-equality checker |
| `b83c0995` | 4 | A4 completeness (unfilled slots + leftover cards, off a maximum matching) |
| `e59b4da2` | 5 | A5 `--seat '@i=<chunk-set-id>'` |
| `5425d634` | 6 | PHASE B: B1 dispositions, B2 oracles |
| `5691ad94` | 7 | the CLI surface: `--from-mk1`, `--from-mk1-file`, `--seat` on `descriptor` and `address` |
| `b5061b8f` | fix | drive every fixture END TO END; V-LEFTOVER was pinning the wrong refusal |

## Where the spec lives in the code

`crates/md-cli/src/seat/` — the whole engine, under a `mod.rs` whose doc
comment carries plan §1's matrix. **Machine-checked: the six table rows in
`seat/mod.rs` are byte-identical to
`design/IMPLEMENTATION_PLAN_wallet_form_converter.md` §1's** (compared
programmatically after stripping the `//! ` prefix, not by eye).

| SPEC rule | module |
| --- | --- |
| P2 input pipeline (A3(a)) | `seat/input.rs` |
| A2 satisfaction, both door checks, both card-set checks | `seat/satisfy.rs` |
| A3 matchings + compare + cap + tie-break | `seat/matching.rs` |
| composition, the comparison form, spend-equality | `seat/compose.rs` |
| A4 completeness | `seat/complete.rs` |
| A5 `--seat` | `seat/directive.rs` |
| B1 disposition, B2 oracles | `seat/disposition.rs` |
| PHASE-ordered orchestration + `--from-mk1-file` reader | `seat/mod.rs` |

CLI surface in `crates/md-cli/src/main.rs`,
`crates/md-cli/src/cmd/{descriptor,address}.rs`; end-to-end rows in
`crates/md-cli/tests/seating_vectors.rs`.

## The defect the phase found

Recorded because it is the phase's most useful output.

**V-LEFTOVER's row was pinning a refusal the engine never reached.** The
first fixture's extra card carried KEY 5's xpub at `48'/0'/9'/2'`, and KEY 5's
xpub is ALSO in the pathological set at `48'/0'/0'/2'`. So `seat::run` refused
at A3's pairwise-distinct check — *"cards decb1 and e9a3d carry the SAME
extended public key"* — and never got to A4. The unit row went on passing
because it calls `complete::refusal()` directly.

The message was right and the engine could not get to it: a shape a unit row
on the message-builder cannot see. Found by driving every fixture through the
built binary, which is now a permanent 15-row table in `seating_vectors.rs`
(`*_reaches_the_command`). The fixture was regenerated with a depth-5 child
xpub that is deliberately not one of the eleven.

A second, smaller instance of the same class: the V-UNFILLED fixture's first
draft selected the dropped card by ORIGIN PATH, and two of the eleven keys
sit at `48'/0'/3'/2'` under different masters — so it dropped both and left
two slots unfilled instead of one. The generator now matches on both halves
of the origin.

## Gate outputs (verbatim, final tree)

```
$ cargo nextest run --locked
     Summary [   1.267s] 987 tests run: 987 passed, 2 skipped

$ cargo clippy --locked --all-targets -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.66s
    (no warnings, no errors)

$ cargo fmt --check
    (clean, no output)
```

### Row-scoped run — expected vs matched

The plan requires the matched count quoted **against the phase's expected row
count**. The roster gives C2 **28 row families**; the filter is
`test(v_dup_) + test(v_collide_) + …` over all 28 prefixes.

```
families: 28
     Summary [   0.018s] 78 tests run: 78 passed, 911 skipped
families with zero rows: 0
```

| family | N | family | N | family | N | family | N |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `v_dup_` | 6 | `v_collide_` | 3 | `v_imposs_` | 3 | `v_door_` | 3 |
| `v_ord_` | 3 | `v_r2_ord_` | 2 | `v_r4_ik_` | 2 | `v_fpfree_card_` | 3 |
| `v_grp_` | 2 | `v_usp_` | 1 | `v_r5m1_` | 3 | `v_bound_seat_` | 4 |
| `v_bound_ref_` | 3 | `v_mix_` | 2 | `v_amb_` | 1 | `v_cap_` | 3 |
| `v_seat_ok_` | 4 | `v_seat_bad_` | 5 | `v_seat_unk_` | 4 | `v_unfilled_` | 2 |
| `v_leftover_` | 2 | `v_b1_wallet_` | 1 | `v_b1_shape_` | 2 | `v_b1_warn_` | 3 |
| `v_b1_cross_` | 1 | `v_ce1_` | 2 | `v_spendeq_` | 4 | `v_msg_keyless_` | 4 |

**Expected: 28 families, ≥1 row each. Matched: 28 families, 0 short.** The
per-family counts sum to **78**, exactly the number of tests the run
executed, so no test is double-counted across two families — an earlier
arrangement summed to 64 against 63 tests because one test's name contained a
second family's token, and it was renamed.

### Snapshot surfaces (plan D3), read and named

**No committed snapshot changed** — `git status` shows no `.snap` or
`.snap.new` anywhere, and `crates/md-cli/tests/snapshots/` is untouched.
`cmd_gui_schema.rs` asserts subcommand NAMES rather than an exhaustive flag
list, and `help_examples.rs` validates the EXAMPLES blocks, so neither pins a
surface the new flags shift.

The three DERIVED surfaces did move, and each was checked directly rather
than assumed:

- `md gui-schema` — `descriptor` and `address` each gained `--from-mk1`,
  `--from-mk1-file`, `--seat` (read out of the emitted JSON).
- `md gen-man` — `md-descriptor.1` and `md-address.1` are the two pages
  carrying `from\-mk1`; `md-descriptor.1` mentions `seat` 4 times.
- `--help` — the flags carry the doc comments written above them.

### Mutation checks (run, then reverted)

A green suite proves little on its own, so two of the funds-shaped lines were
broken to see whether anything noticed:

| mutation | rows that FAILED |
| --- | --- |
| delete `sort_sorted_group_instances` from `comparison_form` | `v_bound_seat_fp_free_same_path_different_masters_seats`, `v_bound_seat_the_choice_is_deterministic_under_reordered_input`, `comparison_form_absorbs_a_swap_inside_one_sorted_group` |
| flip A2's restrictive half `(Some(_), None) => false` to `true` | `v_fpfree_card_cannot_satisfy_a_fingerprint_bearing_declaration`, `v_fpfree_card_leaves_its_slot_unfilled_and_itself_over` |

Both reverted; `git diff` confirms the committed tree carries neither.

### Numbers measured while building, which confirm the spec's own

- The 11-card pathological split set composes to address 0
  `bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64` — SPEC
  acceptance 1's `bc1qkuknuy6…`.
- Its composed WalletPolicyId top-4 is `ced22709` and the policy's
  WalletDescriptorTemplateId top-4 is `5b48af35` — the exact pair SPEC A1/B1
  cite for the fixture's three legitimate values.
- `generate.sh` is reproducible: re-running it against the committed tree
  leaves `git status` empty.

## Fixture provenance

Real material, COPIED (not generated), each with a provenance header naming
source path and date:

| file | source | contents |
| --- | --- | --- |
| `tests/fixtures/pathological/backup-strings.txt` | `mnemonic-engrave/design/journeys/out/pathological/backup-strings.txt`, 2026-08-30 | 6 md1 chunks (keyless policy, template id `5b48af35…`) + 30 mk1 chunks = 11 key cards |
| `tests/fixtures/pathological/keys.txt` | `…/pathological/keys.txt`, 2026-08-30 | 11 BIP-380 origin-notated key records, 3 masters |

Synthetic material, GENERATED by the committed
`tests/fixtures/seating/generate.sh` (both binaries resolved BY PATH: `md`
from `target/debug/md`, `mk` from
`/scratch/code/shibboleth/mnemonic-key/target/debug/mk`). Every file repeats
the exact commands that produced it in its own header.

| file | cards | policy / stub | what it is |
| --- | --- | --- | --- |
| `v-collide.txt` | 2 (colliding) | — / `5b48af35` | two DIFFERENT cards pinned to `--chunk-set-id 0x12345` |
| `v-imposs.txt` | 2 | 2-slot fp-bearing / `5b48af35` | KEY 2's xpub minted at KEY 1's origin — a lie the format cannot catch |
| `v-door.txt` | 1 | two identical fp-bearing slots / `5b48af35` | refused at the door |
| `v-fpfree-card.txt` | 2 | 2-slot fp-bearing / `5b48af35` | one privacy-preserving card against a fingerprint-bearing declaration |
| `v-r5m1.txt` | 3 | `tr(@2,{sortedmulti_a(2,@0,@1),sortedmulti_a(1,@0,@1)})` / `5b48af35` | r5-M1's construction, regrounded as a REFUSE row |
| `v-bound-seat.txt` | 2 | fp-free same-path `sortedmulti` / `5b48af35` | the key-reuse boundary's SEAT side, two different masters |
| `v-bound-ref.txt` | 2 | same policy / `5b48af35`, `00000000` | the same xpub offered as two cards |
| `v-usp.txt` | 2 | `sortedmulti(2,@0/…/<0;1>,@1/…/<2;3>)` / `5b48af35` | r5's use-site-path swap; also the V-AMB / V-SEAT-* fixture |
| `v-mix.txt` | 2 | one fp-bearing slot, one fp-free / `5b48af35` | r6-M2's mixed declarations, unique matching |
| `v-r2-ord.txt` | 4 | two `multi` groups, fp-free / `5b48af35` | r2(a)'s three-orders counterexample |
| `v-r4-ik.txt` | 5 | `tr(@0,{sortedmulti_a(2,…),sortedmulti_a(2,…)})` / `5b48af35` | the internal-key hazard, reuse-free five-distinct-key form |
| `v-grp.txt` | 5 | same with a 1-of-2 second leaf / `5b48af35` | r3's two-group repartition |
| `v-cap.txt` | 12 | 12 slots, two independent 6-card components / `5b48af35` | 6!×6! = 518,400 matchings, no component over 6 |
| `v-b1-wallet.txt` | 2 | shared B1 policy / `6a801edb` | stub = the COMPOSED WalletPolicyId top-4, extracted by the generator from `md inspect` |
| `v-b1-shape.txt` | 2 | shared B1 policy / `aad0e0e0` | stub = that policy's own template id top-4 |
| `v-b1-warn.txt` | 2 | shared B1 policy / `232214e4` | SPEC A1's measured value — matches neither id |
| `v-b1-cross.txt` | 2 | shared B1 policy / `5b48af35` | the PATHOLOGICAL policy's real template id: a card from another wallet |
| `v-spendeq-keyed.txt` | — | keyed md1 card | same wallet as `v-b1-wallet.txt`, minted with DIFFERENT declared fingerprints |
| `v-ce1.txt` | 2 | fp-free 2-slot / `a235ee75` | the genuine pair |
| `v-ce1-foreign.txt` | 1 | — / `a235ee75` | another master, @0's path, the SAME stub |
| `v-leftover.txt` | 1 | — / `5b48af35` | a depth-5 child at `m/48'/0'/9'/2'/0`, privacy-preserving — deliberately not one of the eleven |
| `v-unfilled.txt` | 10 | pathological policy | the pathological set minus the card for `[73c5da0a/48'/0'/3'/2']` |

The four-, five- and twelve-card fingerprint-free families are built from
depth-5 children (`mk derive --path m/N`) minted privacy-preserving at ONE
declared path. `mk encode` checks only the xpub's depth and the path's LAST
component (measured 2026-08-30: *"xpub origin-path mismatch: xpub depth 5 /
child 7 vs origin_path depth 5 / last Normal{0}"*), so N distinct cards at one
origin are constructible from 11 keys.

## Deviations from the spec or plan, each with its reason

1. **Most rows are unit rows inside `src/seat/`, not in
   `tests/seating_vectors.rs`.** Plan D2 says "Vectors: new
   `crates/md-cli/tests/seating_vectors.rs` (C2)". `md-cli` has NO `[lib]`
   target, so an integration test can only drive the binary through
   `assert_cmd` and cannot reach the engine's types at all — a row that needs
   a decoded `KeyCard`, a constructed candidate graph or a `Descriptor`
   cannot live there. `tests/seating_vectors.rs` exists and carries the 30
   command-level rows; every row name carries its family prefix, so the
   row-scoped gate sees both halves as one set.

2. **B2's split-vs-keyed branch ships as the CHECKER, with no CLI channel.**
   Plan §3 C2 step 7 defines the CLI surface and names no way to supply a
   keyed card alongside a split set, and inventing one is scope the plan
   parks in C4 (§3 C4 item 1(b) drives exactly that comparison). So
   `compose::spend_equal` ships — SPEC P2's "P2 also ships the SPEND-EQUALITY
   checker" — and is row-pinned four ways, including a real cross-form row
   against the `v-spendeq-keyed.txt` card. B2's other branch, the one that
   DOES have a CLI channel ("otherwise: address 0 on stderr"), ships wired.
   `spend_equal` therefore carries an `#[allow(dead_code)]` with that reason
   stated in place.

3. **New `CliError::Seat` variant, exit code 1 for every seating refusal.**
   One variant for the whole engine, because a seating refusal is a statement
   about the card set and the policy together and the MESSAGE is what the
   rows pin; splitting the taxonomy across variants would move it into the
   type where the rows would stop checking it. Exit 1 (a content refusal) not
   2 (a usage error): the flags were spelled correctly.

4. **`--from-mk1-file` skips blank lines and `#` comments.** SPEC P2 says only
   "one string per line". Skipping follows `mk`'s own `--from-md1-set`
   precedent and lets a card file carry provenance. Any OTHER non-mk1 line is
   REFUSED by line number rather than dropped — a truncated line is exactly
   the input a restore must not quietly ignore. Row:
   `v_dup_from_mk1_file_refuses_a_line_it_cannot_read`.

5. **The input pipeline's step 1 is applied to the md1 side too.** SPEC A3(a)
   is written about the mk1 strings, but V-DUP is "a full split set supplied
   twice over", and a drawer scan repeats whatever it repeats. Deduping the
   policy card's phrases as well is within the rule's words and is what makes
   the row true end to end.

6. **A1 is not a separate pass.** It "records, never refuses alone" and its
   only consumer is B1's shape tier, which recomputes the template id from
   the policy. Carrying a triage result across the phases would be a second
   copy of a value B1 already holds.

7. **B1's confirmed tiers are summarised one line per TIER, listing every
   card's set id, rather than one line per card.** On the eleven-card fixture
   the per-card form repeated a three-line sentence eleven times and buried
   the line that matters. WARNINGS stay one per card — each is a separate
   thing to go and check. Every card is still named.

8. **Two guards ship without a named roster row, folded under the nearest
   family**: `--seat` supplied with no cards (`v_seat_bad_…`), and a KEYED
   md1 card supplied with `--from-mk1` (`v_msg_keyless_…`). Neither is in the
   roster; both are reachable through the new surface, and a silent
   composition would be worse than saying so.

9. **On BIP 388 shape (2).** SPEC A3 says the compose side gets no row
   because md's parser refuses the DISJOINT-multipath spelling upstream. What
   `check_no_repeated_placeholder` refuses is the reachable case — a
   placeholder at two positions with the SAME use-site path, which md
   composes clean today — and that is V-R5M1, which the roster does place in
   C2. The disjoint spelling never reaches the engine. The two statements are
   about different halves of shape (2); no compose-side row was added for the
   half the spec excludes.

## Follow-up worth filing (not blocking, not this phase's)

`md descriptor --from-mk1` cannot be handed a keyed card as a cross-check
oracle, so SPEC B2's split-vs-keyed branch has no operator-reachable path —
only the checker and C4's test. If the acceptance walk finds the comparison
worth having at the command line, it wants a flag (`--verify-against <md1>`
or similar) and its own row. Owning phase: C4.
