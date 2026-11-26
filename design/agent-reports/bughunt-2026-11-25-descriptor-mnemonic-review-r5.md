# descriptor-mnemonic fold review r5 (2026-11-26)

Branch `claude/project-thread-utm1aa`, fold range `6ca7e05c..2d5c9935`:
- `5ad5a9b2`: the fix;
- `2d5c9935`: docs.

The review was read-only. To get cards that the fold now refuses, I used two temporary steps and undid both. `git status` is clean.
1. I added a probe test, `crates/md-codec/tests/zz_r5_probe.rs`, and deleted it.
2. I built a pre-fold `md` by checking out the `6ca7e05c` copy of `template.rs` (the only md-cli source file the fold changes), copied the binary to `scratchpad/r5/md-prefold`, restored the file, and rebuilt HEAD.

**Counts: 0 Critical, 1 Important, 1 Minor, 2 Nit.**

**Verdict:** the fold closes r4 I-1.
- No real wallet stops minting.
- Kept keys and the verdict table do not change.
- The two `thresh` vectors behave as their labels say.

The fold adds one read-side defect: the mint-only check runs inside `parse_template_ext(…, Refuse)`. That call is also the probe `md decode` and `md bytecode` use to decide whether to warn about a no-key spend path. An existing card that has both a dead leaf and a no-key leaf loses that warning. A wallet card then gets "watch-only, cannot spend" in its place.

## What I ran

All runs used `CARGO_INCREMENTAL=0` and `-j 2`, never `--release`.

**md-codec tests**, all passing:
- `policy_shape_cases` 6;
- `policy_shape_typecheck` 2;
- `skeleton_key` 7;
- `coordinator_thresh` 4;
- `policy_shape_thresh` 14;
- `policy_shape` 11;
- `policy_shape_thresh_props` 21;
- `coordinator` 2;
- `skeleton_key_conformance` 2.

**md-cli tests**, all passing:
- `bh1125_md_mint_refusals` 19;
- `policy_shape_cases_tree_schema` 1 (100 cases);
- `cli_output_class` 19;
- `bh1119_md_output_advisories` 23;
- `cli_encode_advisory` 5;
- `cli_compose_unspendable` 9;
- `cli_coordinator_verdict` 11;
- `cmd_decode` 8;
- `cmd_address` 20;
- `cmd_descriptor` 7;
- `--bins` unit tests: 340 passed, 3 ignored.

**(d) Kept keys and verdict table:**
- `cargo run -p xtask -- verdicts` wrote 548 rows (246 cells, 206 refusals, 2 unkeyable), and `git status` showed no diff.
- `minted.tsv` and `xtask/` are untouched in the fold range.

**(a) Real templates.** Script: `scratchpad/r5/probe.sh`. I ran each template through `md encode --path bip48`, then `md shape-key`, then `md address --template … --key …`. Every one returned rc 0 at every step and got a key:
- `tr(@0)`;
- `tr(@0,pk)`, and `tr(@0,{pk,pk})`;
- NUMS with `multi_a` 2-of-3, and with `sortedmulti_a`;
- `tr(@0,{multi_a 2-of-2, and_v(v:pk,older 52560)})`;
- a Liana-style decaying tree with key path, `{and_v(v:multi_a,older 4320), and_v(v:pk,older 52560)}`;
- `after` leaves, and `after`+`older` two-leaf trees;
- `and_v(v:pkh,older)`, `or_d(pk,and_v(v:pk,older))`, `or_i(…)`, `andor(pk,older,pk)`;
- `thresh(2,pk,s:pk,sln:older)`, and NUMS `thresh(3,…,sln:after)`;
- a `sha256` leaf;
- a four-leaf NUMS tree, a three-leaf tree with recovery keys at two timelocks, and `older(1)`;
- controls: `wsh` multi, `wsh` `or_d` decaying, `wpkh`.

The two templates that reuse one key across leaves (Liana's own 3-of-3 → 2-of-3) are refused by md's existing N1 rule, not by this fold. That refusal is unchanged.

**Dead-leaf refusals checked.** `encode`, `encode --experimental`, `address --template` and `descriptor --template` all refuse each of these with the new message:
- `tr(NUMS,and_v(v:pk,0))`;
- `tr(@0,0)`;
- `tr(@0,and_v(v:pk(@1),or_i(0,0)))`.

**(b) Read verbs on existing dead-leaf cards.** I used the three `untyped_tap_*` and `untyped_tr_slot_leaf_zero` cards from `cases.json`, plus the probe cards.
- `decode`, `inspect`, `bytecode` and `verify --template` all return rc 0 with unchanged output.
- `shape-key` gives NotMiniscript, as intended.
- The keyless warning is the one exception: I-1 below.

**(e) The `thresh` vectors.**
- `extra_props.rs:899-906` sorts unsatisfiable children last and gives sat to `i <= k`, so it needs `min(k+1, n)` satisfiable children.
- `thresh(2,pk,s:pk,X)`: n=3, needs 3, has 2. Fails, labelled false.
- `_2of4`, k=2: needs 3, has 3. Passes, labelled true.
- `_3of4`, k=3: needs 4, has 3. Fails, labelled false.
- The types are B/Wdu throughout, so `typed` is decided by the satisfaction analysis alone, as the labels claim.
- The test pins all three by name (`policy_shape_cases.rs:425-431`).

## Important

### I-1. The mint check silences the "spend with NO key" warning on existing cards that have a dead leaf and a no-key leaf. A wallet card is then labelled "watch-only, cannot spend".

**Where:**
- `crates/md-cli/src/parse/template.rs:3066-3089` (the new block, gated on `Disposition::Refuse`);
- it is reached from `crates/md-cli/src/keyless_advisory.rs:160-176` (`needs_no_key`);
- `needs_no_key` is called by `cmd/decode.rs:91` and by `emit_card_advisories` (`keyless_advisory.rs:184-207`), which `decode --json` and `bytecode` use (`cmd/bytecode.rs:69,81`).

**Mechanism:**
- `needs_no_key(t)` returns `parse(t, exp=false, Refuse).is_err() && parse(t, exp=true, Refuse).is_ok()`. It uses `Refuse` deliberately, to ask "would a minting verb refuse this?". It runs on the template of a card that already exists.
- The fold's dead-leaf check now makes the `exp=true` parse fail as well. So `needs_no_key` returns false for any card whose tree holds a never-satisfiable leaf, whatever its other leaves are.

**Measured.** Card minted by the pre-fold `md encode --experimental 'tr(NUMS,{and_v(v:pk(@0/<0;1>/*),0),older(10)})'`: rc 0, `md1yqfdsssj5qqcr32v6238qqqqqq587m7pw4u7l9p3`.

| verb | pre-fold stderr | post-fold stderr |
|---|---|---|
| `decode` | `warning: this card can be spent with NO key when: older(10) …` | (no warning) |
| `decode --json` / `bytecode` | same warning | (no warning) |

The same template with `--key @0=xpub6BmeGmRo… --path bip86` gives a three-chunk wallet card, `md1fm0cjpqpqpm66…`:
- pre-fold `bytecode` and `decode --json`: the no-key warning;
- post-fold: `note: stdout is watch-only — public keys only, cannot spend`, on a policy anyone can spend 10 blocks after each coin confirms.

**Reach:**
- The pre-fold `md encode --experimental` minted such cards, and it still decodes them.
- The defect is narrow, because it needs a dead leaf, which nobody writes on purpose.
- It is exactly the read-side leak the brief rules out: "the refusal must not reach decode/inspect/verify of existing cards".
- No test covers it. `cli_output_class` and `bh1119_md_output_advisories` pass.

**Fix**, either of:
- give the dead-leaf check its own switch, so that `needs_no_key`'s two probe parses skip it. It asks only about the signature rule.
- or have `needs_no_key` test the signature rule directly, per leaf, with `requires_sig`/`ext_check(sane)` on the parsed descriptor, instead of comparing two full mint parses.

Add a decode, bytecode and `decode --json` regression test with the card above. A wallet-card variant should assert that the watch-only note is absent.

## Minor

### m-1. The fold's description lists the refusal's reach, but not its side effect on `needs_no_key`

Where: `CHANGELOG.md:13-18` ("an existing card decodes and renders as before") and `FOLLOWUPS.md:3585` ("decode and rendering of existing cards are unchanged").

The I-1 case falsifies both lines. Correct them with the I-1 fix.

## Nit

### n-1. A doc comment was moved off its function

`crates/md-codec/tests/common/mod.rs:1663`: "/// One untyped tree per combinator md decodes, with the rule it breaks." now documents the new `fn zero()`, not `untyped_trees()` at `:1731`.

### n-2. The mint message's last clause is conditional, and so reads oddly when a key path exists

`template.rs:3080-3084` ends "a wallet with no other path cannot be spent". For `tr(@0,0)` and for a leaf beside good leaves, the wallet does have another path, and the operator is not told why it is refused anyway: md refuses any wallet with a dead leaf. Suggested wording: "…md refuses a taproot wallet with any such leaf; remove it".

Not graded (the brief excludes it):
- `md decode` still prints "md address --template <the line above> …" for a dead-leaf card.
- That command now refuses, which is the safe direction and falls under the pending owner decision on addresses for existing cards.

## Questions (a)-(e), answered

- **(a)** No real spendable template is refused. This covers multi-leaf Liana or decaying trees, `multi_a`, timelocked recovery leaves, NUMS trees, and key path plus leaves.
- **(b)** `decode`, `inspect` and `verify` (with `--template`) still accept existing cards. The no-key warning leaks, as I-1 describes.
- **(c)** The message is accurate. Its last clause is conditional (n-2).
- **(d)** `minted.tsv` and the verdict table are unchanged.
- **(e)** Both `thresh` vectors behave as labelled.
