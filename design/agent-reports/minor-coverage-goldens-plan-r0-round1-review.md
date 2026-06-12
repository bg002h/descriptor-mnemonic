# R0 Review — minor coverage goldens (PLAN) — Round 1
Reviewer: Fable 5, 2026-06-12. Verified: descriptor-mnemonic origin/main 96aaab3, mnemonic-toolkit origin/master 6c27585 (binary built at 6c27585).

## Verdict: GREEN (0C/0I)

All five items verified buildable/true; both deferred 5b probes RESOLVED empirically. 5 Minors, none gating.

## 5a findings
- **Item 1** `wsh(multi(17,20 keys))` — probe-converts Ok (`to_miniscript_descriptor` Ok for 17-of-20 wsh); `descriptor_with_pubkeys` accepts 1..=32, `test_xpubs()` = 32 keys; precedent `self_test_bad_wsh_multi_21_keys` (proptest:629). One 17-of-20 cell sufficient; optional 17-of-17 cap+1 edge cell (M4). **Stale comment is at `common/mod.rs:947-949` (`wide_multi` :950), NOT :886 (recon's 422b049 snapshot; Cycle 1 shifted it) — M1.**
- **Item 2** `after(800000)` — mirror `self_test_wsh_and_v_pk_older_144` (proptest:136); probe-converts Ok; 800000 in-range.
- **Item 3** hash256/ripemd160/hash160 tap-leaf — mirror `self_test_tr_nums_and_v_sha256_pk` (:173); all three probe-convert Ok; `hash32` :73 / `hash20` :79 constructors exist. **`hash20` NOT yet in proptest's `use common::{…}` — must add (M2).** `derive` is a default feature → the `-- self_test_` run is non-vacuous.

## 5b findings (PROBES RESOLVED)
- **Item 4 hashlock round-trip — WORKS** (exit 0, result ok). Mirror Cell 3 (`cli_verify_bundle_multi_cosigner_mk1.rs:247`):
  ```sh
  H=1111111111111111111111111111111111111111111111111111111111111111
  PHRASE="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
  mnemonic bundle --descriptor "wsh(and_v(v:sha256($H),pk(@0)))" --network mainnet --account 0 --slot "@0.phrase=$PHRASE" --json > bundle.json   # exit 0
  mnemonic verify-bundle --descriptor "wsh(and_v(v:sha256($H),pk(@0)))" --network mainnet --account 0 --slot "@0.phrase=$PHRASE" --bundle-json bundle.json  # exit 0, "result: ok"
  ```
- **Item 5 BIP-388 refusal — REFUSES** exit **2**, stderr `error: descriptor mixes @N placeholders with inline keys; use one form` (source `wallet_import/pipeline.rs:137`, classify `(true,true)` → `DescriptorParse`; the policy JSON trips both @N + inline-key probes). `verify_bundle.rs:667` calls classify directly (no `is_bip388_policy_shape` probe — asymmetry vs `bundle.rs:311`). Classify error fires BEFORE card decode → use dummy `--mk1 mk1qqq --md1 md1qqq`, NO temp file, NO `--bundle-json` (a missing path fails earlier exit 1). M3: pin the exact mixed-form string, not "miniscript parse error".

## Assessment
- Split 5a (md-codec)/5b (toolkit) CORRECT — independent, toolkit pins md-codec `=0.35.2` unchanged, no lockstep. No clap surface → no manual/GUI/schema_mirror.
- Pin-the-refusal (item 5) CORRECT — keeps GAP-5 NO-BUMP; feature stays FOLLOWUP `verify-bundle-bip388-policy-intake` (filed, toolkit FOLLOWUPS:4082).
- No mis-specified shapes.

## Critical / Important
None / None.

## Minor
- M1: stale-comment citation `:886` → live `:947-949`.
- M2: add `hash20` to proptest's `use common::{…}`.
- M3: pin the exact mixed-form refusal string + exit 2 (not "miniscript parse error").
- M4: optional 17-of-17 cap+1 cell (~18 LOC) — recommended, not required.
- M5: fold the probe invocations into the plan so it ships concrete.

Process: temp probe test deleted; tree clean; no goldens derived (impl's derive-once preserved). GREEN — implementation may begin.
