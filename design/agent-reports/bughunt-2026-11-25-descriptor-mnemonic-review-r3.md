# descriptor-mnemonic fold review r3 (2026-11-25)

Branch `claude/project-thread-utm1aa`, fold range `d05655ac..a7680cf2`:
- `ca48a05e`: the I-1 fix.
- `b27d1f7d`: the seat_network temp-file race.
- `a7680cf2`: docs.

The review was read-only. I added two temporary probe test files, `crates/md-codec/tests/zz_r3_probe.rs` and `zz_r3_probe2.rs`, and deleted both. `git status` is clean.

**Counts: 0 Critical, 1 Important, 3 Minor, 3 Nit.**

## What I ran

All runs used `CARGO_INCREMENTAL=0` and `-j 2`, never `--release`.

- **md-codec, full suite:** `cargo test --locked -p md-codec --no-fail-fast`. 726 passed, 0 failed, 3 ignored, 58 binaries. Log: `scratchpad/r3/codec-full.log`.
  - This includes `policy_shape_typecheck` (new), `policy_shape_cases` (freshness test and verdict test), `coordinator_thresh` and `policy_shape_thresh` (both read `minted.tsv`), and `skeleton_key`.
- **md-cli, full suite:** 1180 passed, 0 failed, 3 ignored, 92 binaries. Log: `scratchpad/r3/cli-full.log`.
  - This includes `seat_network` (31 passed, so the n-1 race is gone), `policy_shape_cases_tree_schema` (81 cases) and `bh1125_md_mint_refusals`.
- **Verdict table:** I rebuilt `xtask` against the fold and ran `target/debug/xtask verdicts`.
  - It wrote 548 rows: 246 stored cells, 206 refusals the rules explain, 2 unkeyable.
  - `git status` showed no diff afterwards.
- **Probe of `policy_type_checks`, `policy_shape`, `skeleton` and the real conversion.** I checked about 50 trees, comparing against `to_miniscript_descriptor_with_network` over real test xpubs to get the error text. The trees were:
  - all 15 `untyped_trees()` and the 2 `UNTYPED_CARDS`;
  - tap leaves, multi-leaf trees, and NUMS, Liana and slot internal keys;
  - `sh(wsh(..))` and `sh(..)`;
  - timelock edge values, duplicate keys, nested `Tag::Pkh`, `RawPkH`, nested `sortedmulti`;
  - 20-key `multi`, a 32-key `thresh`, and a 120-leaf `or_i` chain.
  - I also derived master keys from `Xpriv::new_master(Bitcoin, [i;32])` for every i in 0..=255. None fails, so the synthetic-key loop never returns `false` for a non-type reason.
- **CLI checks with the review card `md1f7xrgqq…`:**
  - `md decode` and `md inspect` return rc 0. The ids are unchanged: encoding id `f1868f93…`, template id `c52d14a5…`, policy id `c1eb41ff…`.
  - `md shape-key` refuses with the NotMiniscript sentence, rc 2.

## Important

### I-1. A taproot leaf whose root is not type B is still "typed", complete and keyed. The fold's tap-leaf claim is false.

Where:
- `crates/md-codec/src/to_miniscript.rs:123-141` (`tree_type_checks`);
- `:716-769` (`tree_to_taptree` builds `TapTree::leaf(Arc::new(ms))` with no top-level check);
- `crates/md-codec/src/policy_shape.rs:412-425` (the normative doc).

Why the check misses it:
- `tree_type_checks` relies on the conversion to check context. The doc says "top-level `wsh`/`sh`/`tr` leaf rules".
- For `wsh`/`sh` the top-level B check runs, because `Wsh::new` calls `Segwitv0::top_level_checks` (`vendor/miniscript/src/descriptor/segwitv0.rs:41-44`).
- For taproot it does not:
  - `tree_to_taptree` wraps each leaf's `Miniscript` without calling `Tap::top_level_checks`.
  - `Tr::new` checks only the internal key (`vendor/miniscript/src/descriptor/tr/mod.rs:95-98`).
  - `Miniscript::from_ast` checks the node's type and global validity, but not that the root is B (`vendor/miniscript/src/miniscript/mod.rs:322-338`).

Measured (probe; each row reads `typed / complete / skeleton`):

| tree | typed | complete | skeleton |
|---|---|---|---|
| `tr(NUMS, v:pk(@0))` | true | true | KEY |
| `tr(NUMS, a:pk(@0))` | true | true | KEY |
| `tr(NUMS, s:pk(@0))` | true | true | KEY |
| `tr(NUMS, and_v(v:pk(@0),v:pk(@1)))` | true | true | KEY |
| `tr(NUMS, {pk(@0), v:pk(@1)})` | true | true | KEY |
| `tr(@0, v:pk(@1))` | true | true | KEY |
| `tr(Liana-unspendable, {pk(@0), v:pk(@1)})` | true | true | KEY |

For comparison, `wsh(v:pk(@0))`, `sh(v:pk(@0))` and `sh(wsh(v:pk(@0)))` are correctly refused ("non-T miniscript").

Hand-built card (the encoder's parser refuses this text, so the reach is the same as r2's I-1):
`md1fajunqq9qpcu49cqps8xjsgqjcysy6hxsr3` = `tr(NUMS, v:pk(@0/<0;1>/*))`.

- `md decode` returns rc 0.
- `md shape-key` prints `tr(…,v:pk(@0/<0;1>/*))␟[[]][]␟Nums`, rc 0.
- With pubkeys attached (`md1f0aewqs9q… md1f0aewqsgy…`):
  - `md address` prints `bc1pahged8v92s4sxl357hs4v49c6sus7cvwj7338nwgpsde6zl34zaqt7w9vm`.
  - `md descriptor` prints coordinator verdicts.

Why the wallet is unspendable:
- The leaf script `<K> CHECKSIGVERIFY` leaves an empty stack. BIP-342 requires exactly one element, and it must be true.
- An `a:`/`s:` leaf always leaves two or more elements.
- So no non-B leaf can ever be satisfied. With a NUMS internal key, the wallet above cannot be spent at all.
- md calls it "1 key", keys it and gives it an address. In the multi-leaf case it shows a recovery path that does not exist.
- This is the same class r2's I-1 named: a complete, keyed summary that misstates what the script allows. Here it overstates spendability instead of understating the signatures.

The fold does not cover this case:
- The brief asks for tap leaves and multi-leaf trees to be covered.
- `cases.json` has no non-B leaf case (`untyped_tr_leaf_or_b_pk_pk` fails inside the leaf, at the `or_b` node).
- A Go port that follows the doc ("B/V/K/W … in its script context") would answer `typed: false`. Rust answers `true`, so the parity file cannot catch the disagreement.

Fix:
1. In `tree_type_checks`, after the conversion succeeds, walk `Descriptor::Tr` leaves and require `ms.ty.corr.base == Base::B`, for example with `Tap::top_level_type_check(ms)`.
2. Add `untyped_tap_leaf_v`, `untyped_tap_leaf_w` and a two-leaf case with one bad leaf to `untyped_trees()`, which also puts them in `cases.json`.
3. Separately, log a FOLLOWUP: `to_miniscript`/`derive_address` gives an address for such a tree. That predates this fold and is an address-behaviour change, so it is the owner's call.

## Minor

### m-1. The normative "type rule" for ports is narrower than what Rust enforces

`policy_shape.rs:412-425`, the `skeleton.rs:137-140` message and `CHANGELOG.md`.

`policy_type_checks` is documented as BIP-379's B/V/K/W and z/o/n/d/u rules. But `tree_type_checks` returns `false` for more than that. Each of the following is `typed=false`, measured with its conversion error:

- **Timelock range:**
  - `older(0)`: "relative locktimes … minimum value of 1";
  - `after(0)`;
  - `after(0xFFFFFFFF)`: "maximum value of 0x7fffffff";
  - `older(0x80000001)`: "not a valid BIP68 relative locktime".
- **Script-size limits:**
  - a typed 120-leaf `or_i` chain is refused with "cannot be larger than 3600 bytes, but got 3607". That is standardness (`MAX_STANDARD_P2WSH_SCRIPT_SIZE`), not consensus, whose limit is 10000.
  - by reading only: Legacy's 520-byte limit and rust-miniscript's recursion depth of 402.
- **Fragment placement:**
  - `multi` in a tap leaf: "Invalid use of Multi node in taproot context";
  - nested `sortedmulti`;
  - `RawPkH`: "not constructible";
  - nested `Tag::Pkh`: "top-level wrapper Pkh cannot appear inside a miniscript context".

Refusing these is safe. Each refusal only removes a summary, and every one of these scripts is invalid, non-standard or unspendable. The `to_miniscript.rs:116-118` doc lists "resource limits", so the refusal looks intended, and I do not count it as overreach. But:

- the user-facing sentence says "(it does not type-check)", which is wrong for these;
- a port that implements only the stated rule will disagree with Rust on all of them;
- `cases.json` holds only two of them (`thresh_neg_older_zero` and `thresh_neg_older_bit31`, both `typed: false`).

Suggested fix: state the rule as "converts to valid miniscript in its context: types, timelock ranges, context fragments, script-size limits". Either rename `typed`/`NotMiniscript` text to "not valid miniscript" (the sentence's first half is already right), or add vectors for each class.

### m-2. The FOLLOWUP is "resolved", but the device side has no open item

The entry is `design/FOLLOWUPS.md:3573`.

- r2 reported that the seedhammer Go walk mirrors the Rust walk, so the device still shows "1: 1 key(s) / 2: any 2 of these 3 keys" for the review card.
- The entry names "the seedhammer Go port's policy-shape item" as companion. mnemonic-engrave's FOLLOWUPS has no item about a type check. The nearest is `sh-bh1125-thresh-k-lt-n-consent`, which is about k<n only.
- The repo's convention is a companion entry in each repo, each citing the other.
- Suggested fix: file the Go-port type-rule item (status `owed`, Rust-first done) and cite it. Otherwise the funds-relevant device display drops out of every open list.

### m-3. `tree_type_checks` does not run the satisfaction-time consensus checks

`from_ast` runs `check_global_validity` only (`vendor/miniscript/src/miniscript/context.rs:252-258`), not `check_local_validity` (`:262-270`, which checks ops ≤ 201 and impossible satisfaction).

- A typed tree whose every path needs more than 201 ops would be complete and keyed, although it cannot be spent.
- This is outside I-1, which was about typing. I did not construct such a card. Within md's 32-key limit it seems to need about 100 chained timelocks.
- If "valid miniscript" is meant to be the gate, `Segwitv0::check_local_consensus_validity` on the root closes it cheaply. Otherwise say in the doc that the gate does not cover it.

## Nit

- **n-1.** `skeleton.rs:385-388` still says `Tag::Pkh` and `Tag::PkH` are "two spellings of one miniscript fragment … sharing a key is correct". A nested `Tag::Pkh` now has no key at all (NotMiniscript), as it already had no address. The comment is stale.
- **n-2.** `skeleton()` calls `policy_type_checks` and then `policy_shape`, which calls it again (`skeleton.rs:222-228`). That is two rounds of n master-key derivations per call. It is cheap; noted only.
- **n-3.** The CHANGELOG says "the 17 `untyped_*` cases … one untyped tree per combinator … in `wsh` and a taproot leaf". Only one of the 17 is a taproot leaf (`tr_leaf_or_b_pk_pk`), and none is a non-B leaf (see I-1).

## The brief's questions

**(a) Can the synthetic-key conversion fail, or pass, for reasons unrelated to typing?**

- **False positives (refusals):** the timelock-range, script-size and placement refusals listed in m-1. All are safe, intended ("resource limits" is in the doc) and conservative, but worded as a type failure. They are not caused by:
  - key-expression rules: synthetic keys have no path, so multipath and hardened steps cannot reach the check;
  - duplicate keys: `wsh(or_d(pk(@0),pk(@0)))` is `typed=true`, complete and keyed, so they are not refused;
  - seed failures: none of the 256 seeds fails.
- **False negatives (acceptances):** non-B taproot leaf roots (I-1) and per-path op limits (m-3).
- **Contexts checked:**
  - `wsh`, `sh(wsh)`, `sh` and NUMS/slot/Liana `tr` with a combinator error inside a leaf are refused correctly.
  - `multi_a` and `sortedmulti_a` leaves are typed.
  - Liana unspendable converts with the synthetic xpubs.

**(b) Do decode, ids, addresses, re-encode and `md inspect` stay unchanged?**

Yes.

- In md-cli, only `verdict.rs` and `shape_key.rs` reach `skeleton`. Nothing in md-cli calls `policy_shape`, so `inspect`, `decode`, `bytecode` and `address` are unaffected.
- The review card's ids are unchanged.
- `the_review_cards_decode_but_get_no_summary_and_no_key` checks the byte round-trip and the id computation.
- `tree_type_checks` is a separate function, and the derive path is untouched.

**(c) Were any kept SkeletonKeys changed?**

No.

- `minted.tsv` is not in the diff, and the tests that read it pass.
- Every one of the 67 pre-existing `cases.json` cases is identical apart from the added `typed` field. No `complete` flipped.
- The golden change from `or_d` to `or_i`, and `sortedmulti` to `multi`, is a test-fixture correction. The old fixtures were hand-built trees that never type-checked:
  - `or_d` needs a Bdu first child, and `and_v(v:pkh,older)` is not d;
  - `sortedmulti` is not a miniscript fragment;
  - the fixtures used the descriptor-level `Tag::Pkh` nested, which no encoder emits: md-cli and the toolkit emit `Tag::PkH`, and `Tag::Pkh` only at the root.
- None of these was minted. The old key string `or_d(and_v(…` is not in `table.rs`, and no evidence cell holds it.
- This is not a hidden behaviour change: those trees never had an address either.

**(d) Is the verdict table unchanged?**

Yes. I regenerated it with a freshly built `xtask` and `git status` showed no diff.

**(e) Do the 17 `untyped_*` cases fail typing for the reason they claim?**

Yes. Each fails at the named rule, and none fails on a parse error:

- `or_b` B+B (×4, including both review cards and the tap leaf);
- `or_d` B+W;
- `or_c` B+B;
- `or_i` B+W;
- `and_v` B+B;
- `and_b` B+B;
- `andor` B,B,W;
- `thresh` "sub-fragment 1 has type B rather than W";
- `multi` under `and_v` B+B;
- `s:multi` "SWAP … does not take exactly one input";
- top-level `v:` and `a:` "non-T miniscript";
- `j:older` "might be satisfied by an input of size zero".
