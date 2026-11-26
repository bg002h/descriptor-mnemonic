# descriptor-mnemonic fold review r4 (2026-11-25)

Branch `claude/project-thread-utm1aa`, fold range `a7680cf2..6ca7e05c`:
- `f8b4a675`: the fix;
- `6ca7e05c`: docs.

The review was read-only. I added one temporary probe file, `crates/md-codec/tests/zz_r4_probe.rs`, and deleted it afterwards. `git status` is clean.

**Counts: 0 Critical, 1 Important, 2 Minor, 2 Nit.**

**Verdict:** the fold closes r3 I-1 for single-leaf trees, multi-leaf trees, and NUMS, slot and Liana internal keys. It adds no regression to decoding, ids, keys or the verdict table. Its new normative rule 6 ("some satisfaction exists") is false for tap leaves. md encode mints a wholly unspendable taproot wallet, and md still keys it and gives it an address (I-1 below).

## What I ran

All runs used `CARGO_INCREMENTAL=0` and `-j 2`, never `--release`.

- **md-codec, full suite:** `cargo test --locked -p md-codec --no-fail-fast`. 726 passed, 0 failed, 3 ignored, 58 binaries.
  - It includes `policy_shape_typecheck`, `policy_shape_cases` (freshness test and verdict test), `coordinator_thresh` and `policy_shape_thresh` (both read `minted.tsv`), and `skeleton_key`.
  - Log: `scratchpad/r4/codec-full.log`.
- **md-cli:** `policy_shape_cases_tree_schema` passed (95 cases).
- **Verdict table:** I rebuilt `xtask` and ran `target/debug/xtask verdicts`. It wrote 548 rows: 246 cells, 206 refusals, 2 unkeyable. `git status` showed no diff.
- **`cases.json`:** the old and new files differ in one existing case only. `thresh_neg_ori_two_zero` changed `typed` from true to false. Its `complete` was already false, so its summary does not change. The other 14 cases are additions.
- **Real-world policies** (`md encode --path bip48` then `md shape-key`, about 45 templates, script `scratchpad/r4/probe.sh`):
  - `wsh` multi and sortedmulti with 2-of-3, 15-of-20 and 20-of-20;
  - `sh(multi 15)` and `sh(sortedmulti 15)`, `sh(wsh)`;
  - `tr` with `multi_a` and `sortedmulti_a` up to 31 keys, including 16-of-31 and 31-of-31;
  - the BIP-388 examples, `thresh` with `sln:after` and `snl:older`, `thresh` 16-of-32, 20-of-32 and 31-of-32 of `pk` in `wsh` and `tr`;
  - Liana-style decaying wallets in `wsh` and `tr`, `or_d(multi 20, and_v(v:multi 10, older))`, `andor(multi, older, multi 20)`;
  - `pkh`, `wpkh`, `sh(wpkh)`, `tr(@0)`.
- **Probe of `policy_type_checks`, `policy_shape` and `skeleton`** over hand-built trees, and the conversion or check that refuses each `untyped_*` tree.

## Important

### I-1. Rule 6 ("some satisfaction exists") is not checked for tap leaves. md encode mints a wholly unspendable taproot wallet, and md keys it and gives it an address.

Where:
- `crates/md-codec/src/to_miniscript.rs:149-166`, and its doc at `:122-130` ("a satisfiable script");
- `crates/md-codec/src/policy_shape.rs:439-441` (normative rule 6: "some satisfaction exists").

Why the check misses it:
- `within_resource_limits` is `Ctx::check_local_validity`.
- For `Segwitv0`, `Legacy` and `BareCtx`, `check_local_consensus_validity` returns `ImpossibleSatisfaction` when `sat_op_count()` is `None`.
- For `Tap` it does not:
  - `vendor/miniscript/src/miniscript/context.rs:650`: `if let Some(data) = ms.ext.sat_data { … }` silently passes `None`;
  - `:668-672`: `check_local_policy_validity` is `Ok(())`.
- So an unsatisfiable tap leaf passes `script_ok`.

Measured:

| tree | typed | complete | skeleton |
|---|---|---|---|
| `tr(Slot(0), 0)` | true | true | key |
| `wsh(0)` | false | | NotMiniscript |

Reach is wider than r3's hand-built card. `md encode` mints these with no flag, because the mint `sanity_check` has the same tap gap:

| template | `md encode` | `md shape-key` |
|---|---|---|
| `tr(50929b74…3ac0, and_v(v:pk(@0/<0;1>/*),0))` | rc 0 | rc 0, `…␟[[]][]␟Nums` (one 1-key path) |
| `tr(50929b74…, {pk(@0),and_v(v:pk(@1),0)})` | rc 0 | rc 0, `[[][]]` (two paths, one does not exist) |
| `tr(@0/<0;1>/*, 0)` | rc 0 | rc 0, `[[][]]` |

`md address --template 'tr(50929b74…,and_v(v:pk(@0/<0;1>/*),0))' --key @0=xpub6BemYiVN…` prints `bc1peksku0tsrtcvs05mh0z8h4n6m3fvxz3z9qvduea36vavjy8psqzs0yx4fh`. Funds sent there are lost, because the only leaf always ends with 0 on the stack.

This is r3 I-1's class: a complete, keyed summary that overstates spendability.

- **Not introduced by the fold:** the behaviour predates it.
- **Defect the fold adds:**
  - The fold's normative text says this rule is enforced in every context.
  - The Rust code and the code doc ("a satisfiable script") claim it.
  - A port that implements the doc answers `typed: false` for these trees, and Rust answers `true`.
  - `cases.json` has no vector, so the disagreement goes undetected.

Related, and also pre-existing: a branch that can never be satisfied inside an otherwise satisfiable script is still shown as a path. For example, `wsh(or_d(pk(@0),0))` is typed and complete, and shape-key shows `[[][]]`. `within_resource_limits` only asks whether *some* satisfaction exists, so it can never catch that.

Fix, in order:
1. In `script_ok` for Tap, also require `ms.ext.sat_data.is_some()`. Equivalent: `Tap::max_satisfaction_size(ms).is_some()`, or a satisfiability walk. This makes `tr(NUMS, and_v(v:pk,0))` and `tr(@0,0)` NotMiniscript.
2. Add `untyped_tap_leaf_unsatisfiable` and a two-leaf case with one unsatisfiable leaf.
3. Either make the walk drop or refuse never-satisfiable branches (`0`, `and_v(…,0)`, `or_i(0,0)`), or state in rule 6 that only whole-script satisfiability is checked.
4. Log a FOLLOWUP for the mint side: `md encode` and `md address` accept a wholly unspendable `tr`. That is an admission and address change, so it is the owner's call.

## Minor

### m-1. Rule 6's `thresh` sentence misstates rust-miniscript, and ports are told to follow it

Where: `policy_shape.rs:442-447`.

The doc says rust-miniscript "counts a `thresh` with a never-satisfiable child … as never satisfiable, even when `k` other children could satisfy it". The real behaviour is an upstream off-by-one:

- `vendor/miniscript/src/miniscript/types/extra_props.rs:899-906` gives the satisfaction branch to children `i <= k`. That is **k+1** children.
- Unsatisfiable children sort last (their sort key is `None`).
- So `sat_data` is `None` exactly when fewer than k+1 children are satisfiable and n > k.

Measured with `wsh`, where `X` is `s:or_i(0,0)`:

| tree | typed |
|---|---|
| `thresh(1, pk, s:pk, X)` | true |
| `thresh(2, pk, s:pk, X)` | false (the doc's example) |
| `thresh(2, pk, s:pk, s:pk, X)` | **true**, although it has a never-satisfiable child |
| `thresh(3, pk, s:pk, s:pk, X)` | false, although it is satisfiable (3 signatures, then dissatisfy X with `[1]` or `[]`) |
| `tr` with `thresh(2, pk, s:pk, X)` | **true** (the tap gap from I-1) |

Effects:
- A port that follows the doc refuses the `thresh(2, pk, s:pk, s:pk, X)` row, which Rust accepts.
- Every such case is a hand-built tree: `md encode` refuses each one as malleable or over resource limits.
- In each case Rust is stricter than the truth, so this is a parity and wording defect, not a safety one.

Fix: state the rule as "fewer than k+1 satisfiable children (rust-miniscript 13.0.0, `extra_props.rs` `i <= k`), wsh/sh only". Add a vector for each side of the off-by-one. Or override the upstream quirk with an md-side satisfiability rule.

### m-2. The 201-op and witness-item limits apply to the worst satisfaction, not the best one

Where: `policy_shape.rs:439-441` and `to_miniscript.rs:127-129`.

- `sat_op_count()` is `static_ops + max_exec_op_count` (`extra_props.rs:1022-1023`).
- `MaxWitnessItems` likewise uses the maximum.
- So a script with one path over 201 ops and another usable path is refused whole. The text "at most 201 executed non-push ops" reads as a property of the script.
- This is safe: it only removes a summary. `md encode`'s `sanity_check` refuses the same, so only hand-built or legacy cards reach it.
- A port that counts the minimum, or per path, would disagree. Add "over every satisfaction (the maximum)" to the doc.

## Nit

- **n-1.** `CHANGELOG.md` does not mention that the existing case `thresh_neg_ori_two_zero` flipped from `typed: true` to `false` in the parity file. A Go port that vendored the earlier file will see an unexplained flip.
- **n-2.** The comment "`older_zero`/`after_above_i32` … not a type rule" in `tests/common/mod.rs` sits under the r3 m-1 heading. The doc's rule list makes this fine. The test name `an_untyped_tree_for_every_combinator_is_incomplete` now also covers timelock and limit rules, so the name is stale.

## The brief's questions

**(a) Does `within_resource_limits` refuse any valid, spendable, md-mintable policy?**

No, for anything `md encode` mints.

- `md-cli/src/parse/template.rs:3050` already runs `sanity_check()` on minting, and the experimental branch runs `ext_check` per script. Both include `within_resource_limits` with the same contexts.
- So the fold cannot newly refuse an encode-mintable policy.
- Every real-world template I probed (list above) returned `shape-key` rc 0, except three Liana and BIP-388 shapes that return the walk's existing `IncompleteShape` ("cannot break this policy into spend paths"). That is not NotMiniscript and was not changed by the fold.
- The encoder itself refuses `thresh` of more than about 24 `pkh` children in `wsh` ("exceeds the resource limits"). That predates the fold.
- Only hand-built or legacy cards can newly lose a summary, and only for a real limit breach, or for the over-strict cases in m-1 and m-2.

**(b) Is "a `thresh` with a never-satisfiable child is unsatisfiable" really rust-miniscript's semantics?**

No. See m-1: it is an off-by-one, k+1 children. It does make some satisfiable trees incomplete. All of them are hand-built and malleable or unmintable, and the error is conservative.

**(c) Are the kept SkeletonKeys and decode, ids, addresses and re-encode unchanged?**

Yes.
- The `minted.tsv` tests (`coordinator_thresh`, `policy_shape_thresh`, `skeleton_key`) pass.
- `the_review_cards_decode_but_get_no_summary_and_no_key` passes, covering re-encode and ids for all three cards.
- `policy_type_checks` is consulted only by `policy_shape` and `skeleton`.

**(d) Does the verdict table change?**

No diff after the regeneration.

**(e) Do the new untyped cases fail for the rule they name?**

Yes. I checked the conversion error or the failing check for each:

| case | refused by |
|---|---|
| `tap_leaf_v/a/s/and_v_v_v`, `tr_slot_leaf_v`, the bad leaf of `tap_two_leaves_one_v` | `top_level_type_check` ("non-T miniscript"), `local` ok |
| `tr_liana_two_leaves_one_v` | the B check (same tree with an `n:pk` leaf is typed) |
| `older_zero` | "minimum value of 1" |
| `after_above_i32` | "maximum value of 0x7fffffff" |
| `multi_in_tap_leaf` | "Invalid use of Multi node in taproot context" |
| `nested_sortedmulti` | "must be the sole child of wsh/sh" |
| `nested_pkh_wrapper` | "top-level wrapper Pkh cannot appear inside" |
| `script_over_3600_bytes` | "cannot be larger than 3600 bytes, but got 4861" |
| `ops_over_201` | conversion ok, then `MaxOpCountExceeded { actual: 203, limit: 201 }` |

My extra probes:
- A bad leaf at depth 3 under NUMS and under a slot key: NotMiniscript.
- The same tree with a good leaf: keyed.
- `or_c` root, `and_v(v:pk, v:older)` root: NotMiniscript.
- `c:pk` root, 16-of-31 `multi_a`: keyed.
