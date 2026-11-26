# descriptor-mnemonic bug hunt, review r2 (2026-11-25)

Branch `claude/project-thread-utm1aa`, head `6620a7dd`. Scope: r1 folds `6604c88c`, `9ecf5b0c`, `9cf3ab9a` and the thresh work `a940c035..6620a7dd`. Design checked against: `scratchpad/DESIGN_thresh_summary.md` (rev 2) and the owner's approved points. Read-only. The only changes to the tree were temporary and are reverted: a probe test file, two one-line mutations of `policy_shape.rs`, and a temporary checkout of the `9cf3ab9a` `crates/md-codec/src` to build the pre-change `md`. `git status` is clean.

**Counts: 0 Critical, 1 Important, 2 Minor, 2 Nit.**

## What I ran

- `cargo test --locked -j 2 --no-fail-fast -p md-codec`: 722 passed, 0 failed across 56 binaries, plus doctests (`scratchpad/r2md/codec-full.log`).
- md-cli: `bh1125_md_mint_refusals` (17), `v017_v1_encode_acceptance` (1) and `policy_shape_cases_tree_schema` (3) all pass. `cargo fmt --check` is clean.
- `target/debug/xtask verdicts`: it rewrote `table.rs` with 548 rows, and `git status` showed no diff afterwards. The verdict table is not hand-edited and is not stale. `table.rs` has 0 `thresh(` cells. The evidence file has no `thresh(` descriptor (its 73 "thresh" hits are all the `threshold` field).
- **SkeletonKey, compared with the pre-change `md`.** I built `md` from `9cf3ab9a`'s `crates/md-codec/src` (`scratchpad/r2md/md-old`) and ran `md shape-key --descriptor` on all 30 rows of `minted.tsv`.
  - The old output is byte-identical to the tsv's key column (`OLD==TSV`).
  - The new output is byte-identical to the old for all 20 kept shapes.
  - The 9 shapes that lose their key are exactly the `thresh_neg_*` minted rows. `thresh_neg_v_thresh_under_or_c` already had no key before.
- **Mutation checks (the property tests can fail).**
  - Allowing `s:pkh` fails `hand_built_trees_outside_the_list_are_incomplete`, `an_accepted_thresh_node_type_checks_on_its_own` (proptest finds it), and both `policy_shape_cases` tests.
  - Restoring the old `k == n` recursion in `collect` fails `hand_built_trees_outside_the_list_are_incomplete`, `minted_thresh_outside_the_list_refuses_the_summary_and_the_key` and the arbitrary-tree property.
- **CLI, new binary.**
  - Decaying 2-of-2: Liana reads "unproven" (it was the false "refuses (no unlocked path)").
  - `thresh_keyless_1of2`: "refuses (a path with no key)" for Liana, Nunchuk and Core 29+.
  - `thresh_neg_sn_pk`: the new "no verdict" sentence.
  - `thresh_2of3_keys`: unchanged.
  - All of these match design section 4.
- **Bare-@N refusal (`6604c88c`).** `md encode 'wpkh(@0)'` is refused at exit 1. The existing card `md1yqqqq2wy43c0uqcl29` still works with `md decode`, `decode --json` and `bytecode` (rc 0, renders `wpkh(@0/*)`).
  - `parse_template_ext`'s Refuse disposition is reached only from `encode`, `compose`, `build` (`address`/`descriptor --template`), `vectors`, and `keyless_advisory::needs_no_key`.
  - `needs_no_key` re-parses a template rendered from a card. A card's `UseSitePath` always carries a wildcard (`use_site_path.rs:63-68`), so a rendered template always has a tail. The refusal cannot reach existing-card decode. `verify` uses Warn.
- **Coordinator clause edits.**
  - Each edit only removes fires, except L2/N1/C1, which now fire exactly when `locks >= k`. That matches BIP-379's thresh "s" rule, and the refusal is true: Liana, libnunchuk and Core each require a signature on every path.
  - L3 still reads `KEY_IDENTITY`, the only bit used to skip a clause (`mod.rs:330`), so moving it from `K` to `K.with(S)` changes nothing for template-only cards. `fp_partition` is 1:1 with `branches` (`policy_shape.rs:1068-1083`), so the zip lines up.
  - No path can turn into "imports": that needs a measured Description, and no thresh evidence exists.
- **Consumers.** No sibling repo builds `policy_shape::Branch`, so `#[non_exhaustive]` breaks nothing (grepped toolkit, refugium, refugium-wallet, engrave, gui).

## Important

### I-1. An ill-typed context around an accepted `thresh` gives a complete, keyed summary of a wallet anyone can spend

`policy_shape.rs:440-484` (`split_branches`), `:773-781` (`any_of_root`).

`any_of_root` accepts a bare `Thresh` in **any** position, and `split_branches` checks no child's type in `or_b`/`or_c`/`or_d`/`or_i`/`andor`. The design says the `thresh` "alone" type-checks. It is right that the allowlist cannot vouch for the context. But nothing else does either: neither the Rust decoder nor `md shape-key`, nor the Go device, which mirrors this walk.

A hand-built card:

```
md1f7xrgqq9qeuedejan9uuw2jemuklxheeuvn3jkxtsqrqjc5xsg3f8z5w9vqqw5d36huyrgmg
= wsh(or_b(pk(@0/<0;1>/*),thresh(2,pk(@1/<0;1>/*),s:pk(@2/<0;1>/*),s:pk(@3/<0;1>/*))))
```

- It decodes at rc 0 (`md decode`, `md inspect`).
- `md shape-key` prints a key.
- `policy_shape` returns `complete = true` with entries `[@0]` and `any 2 of {@1,@2,@3}`.
- On the device this reads "1: 1 key(s), custom / 2: any 2 of these 3 keys".
- `to_miniscript` refuses it: "or_b cannot accept children of types B and B".

The script is `<K0> CHECKSIG <K1> CHECKSIG SWAP <K2> CHECKSIG ADD SWAP <K3> CHECKSIG ADD 2 EQUAL BOOLOR`. With witness `[1, <>, <>, <>]`:

1. The first CHECKSIG pushes 0.
2. The thresh's first `CHECKSIG` takes that 0 as its signature.
3. The sum is 0, and `2 EQUAL` gives 0.
4. `BOOLOR` pops the 0 and the attacker's `1` and leaves `[1]`.

So the wallet is spendable with **no signature**, while the summary says one key or two of three.

The same trick with no thresh at all, `wsh(or_b(pk(@0),pk(@1)))` (`md1fa7e5qq9qfcu49nhed7d0ncqpsfv2z5qd7jmntp59fk69`), is also complete and keyed. This class predates the PR: pre-change, the `or_b(pk, thresh)` card was keyed too, as a 3-key conjunction. The encoder side is already logged as `md-encode-typecheck-gate` (FOLLOWUPS:3508). The shape walk's side is not logged.

Why it is Important here and not only a FOLLOWUP:

- The brief's I-1 class is "accepted as complete, overstates the signatures".
- This PR introduces a context rule (`or_b_second`: only `a:thresh` as `or_b`'s second child) but does not enforce its converse: a bare `thresh` as `or_b`'s second child is still accepted.
- The property `an_accepted_thresh_node_type_checks_on_its_own` checks the node alone by construction, so it cannot see this.

Fix options:

1. In Rust, have `skeleton()`/`policy_shape` require `to_miniscript` to type-check (Rust has the checker). Mirror a minimal context rule in Go: an `or_b`/`and_b` second child must be `a:`/`s:`-wrapped, `or_b`/`or_d`/`or_c`/`andor` first children must not be.
2. At minimum, refuse a bare `Thresh` (and any unwrapped B node) as `or_b`'s second child, and log the general context gap as a FOLLOWUP that cites the seedhammer companion.

## Minor

### m-1. `md verify` reports OK for a bare-@N template against a `/*` card, the pair A2 now calls different wallets

`crates/md-cli/src/cmd/verify.rs:53-59` (Warn disposition).

`md verify md1yqqqq2wy43c0uqcl29 --template 'wpkh(@0)'` prints `OK`, rc 0. `6604c88c`'s own refusal text says `@0` "names one fixed key, a different wallet from `@0/*`".

- An operator who checks a plate against a fixed-key wallet gets an OK for a card that restores the ranged chain, whose first address is not the fixed key's.
- Keeping verify readable for legacy plates is the owner's call, and the behaviour predates A2.
- But the md#3 note that explained the implied `/*` was deleted rather than moved to verify. Nothing on the verify path says so now.
- Suggested fix: verify prints the old note (stderr only, exit unchanged) when the template has a placeholder with no tail.

### m-2. A `1` alternative is an empty entry, the same as a `0` alternative, so a well-typed anyone-can-spend path is complete and keyed

`policy_shape.rs:447-459` and `coordinator/mod.rs` (`keyless_gated` doc: "A `1` branch is missed").

`wsh(or_i(1,thresh(2,pk(@0),s:pk(@1),s:pk(@2))))` type-checks (`to_miniscript` OK). It gives `complete = true` with branches `[(no keys, no locks), any 2 of 3]` and a SkeletonKey.

- No coordinator clause fires "a path with no key".
- The device entry for the first path cannot be told apart from the unsatisfiable `0` arm of `l:`/`u:`/`and_n`. The probe `wsh(l:thresh(…))` shows that arm as an identical empty entry.
- This predates the PR and is documented as "errs toward silence", and md mints it only with `--experimental`.
- Listed because the device's honesty depends on it, and because the new any-of root makes `or_i(1, thresh)` and `l:thresh` newly in scope.
- Suggested fix: carry an "always satisfiable" marker for a `True` arm, or refuse it (`complete = false`).

## Nit

- **n-1.** `crates/md-cli/tests/seat_network.rs:68` names its temp file `c8b-<pid>-<tag>.txt`. `address_from_mk1_file` and `descriptor_from_mk1_file` use the same tag and run in parallel threads of one process, so one removes the other's file. The r1-fold full run (`scratchpad/dm-r1fold-full.log`) shows 6 failures, all "No such file or directory". This flake predates the PR and is not in FOLLOWUPS. It would mask a real regression in that file at phase close. Add the verb to the name.
- **n-2.** `cases.json` `shape` is Rust's own output (`policy_shape_cases.rs:262-275`). The file is only as right as `policy_shape`. The independent expectations live in `policy_shape_thresh.rs` and in `every_design_case_is_present_with_its_verdict` (complete and any-of flags only, not slots, locks or min_keys). The Go port therefore compares against Rust, not against the design table. That is acceptable for a parity file, but say so in its header.

## Checked and found sound

- **The allowlist matches design section 1 exactly.**
  - KEY is bare or one `c:`. T is `1 ≤ v < 2^31`.
  - LOCK is the four `or_i`/`n:` trees, with exactly one `0` arm.
  - X_1 is KEY or LOCK. A later child is `s:` over `pk` only, `a:` over any KEY, or `s:`/`a:` over a LOCK.
  - The slots must be distinct, `1 ≤ k ≤ n` holds, and nothing outside the list is looked through.
- `k == n` uses the same list in `collect`.
- `andor` X/Y and `and_*` placements are refused for `k < n`. `v:`/`a:` roots outside `or_b`'s second child are refused.
- I probed `thresh_older_units_zero` and high-bit `older`: still counted as locks. `min_keys` assumes every lock can be met, so they cannot overstate.
- `any_of_min_keys`, `can_spend_without_lock`, `keyless_gated` and `mixed_any_of` are as designed.
- R1 returns "as built" for an all-keys any-of only when the measured threshold equals `(k, n)`. R2 (Nunchuk) is unchanged for k == 0.
- The md-codec 0.49.0 / md-cli 0.21.2 / `=0.49.0` pin is consistent, and `--locked` builds.
- The CHANGELOG `9ecf5b0c` (mainnet-only wording) is accurate.
