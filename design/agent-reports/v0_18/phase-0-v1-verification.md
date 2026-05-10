# v0.18 Phase 0 — V1 verification (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

Pre-implementation V1 verification of three load-bearing assumptions before kicking off Phase 1.

## V1 checks

### V1.a — `--path` bug reproduces

Confirmed: three different `--path` values (`48'/0'/0'/2'`, `86'/0'/0'`, `44'/9999'/0'`) all produce byte-identical output.

```
$ md encode --from-policy 'thresh(2,pk(@0),pk(@1),pk(@2))' --context tap                            → md1qzq0j6qgjs54gk3aayrxh8yz7w
$ md encode --from-policy 'thresh(2,pk(@0),pk(@1),pk(@2))' --context tap --path "48'/0'/0'/2'"     → md1qzq0j6qgjs54gk3aayrxh8yz7w
$ md encode --from-policy 'thresh(2,pk(@0),pk(@1),pk(@2))' --context tap --path "86'/0'/0'"        → md1qzq0j6qgjs54gk3aayrxh8yz7w
$ md encode --from-policy 'thresh(2,pk(@0),pk(@1),pk(@2))' --context tap --path "44'/9999'/0'"     → md1qzq0j6qgjs54gk3aayrxh8yz7w
```

Bug location confirmed at `crates/md-cli/src/main.rs:218`: `path: _,` destructures and drops the value.

### V1.b — miniscript 13 Terminal enum variant names match plan

Read `~/.cargo/registry/src/index.crates.io-*/miniscript-13.0.0/src/miniscript/decode.rs:90–158`. All 17 walker-arm variants confirmed by exact name:

- Binary: `AndB`, `AndOr` (ternary), `OrB`, `OrC`, `OrD`, `OrI` ✓
- Threshold: `Thresh(Threshold<Arc<Miniscript<Pk, Ctx>>, 0>)` (const-generic 0 = no max bound, distinct from `Multi`/`MultiA`) ✓
- Timelock: `After(AbsLockTime)`, `Older(RelLockTime)` ✓ (Older was already shipped in v0.17)
- Hashes: `Sha256`, `Hash256`, `Ripemd160`, `Hash160` ✓
- Wrappers: `Swap`, `Alt`, `DupIf`, `NonZero`, `ZeroNotEqual` ✓ (Verify was already shipped in v0.17)
- Render-only: `True`, `False`, `RawPkH(hash160::Hash)` ✓

**Significant finding for Phase 4:** `RawPkH`'s doc-comment says "It is not possible to construct this variant from any of the Miniscript APIs." This validates the plan's design — `RawPkH` is decode-side-only (md-codec carries `Tag::RawPkH`; renderer needs the arm), and the negative walker test `walker_rejects_raw_pkh_in_top_level_context` is well-founded.

### V1.c — sentinel-bit-savings baselines (v0.17, pre-Phase-3)

Captured for Phase 3 verification:

| Policy | n | v0.17 phrase | length |
|--------|---|--------------|--------|
| `thresh(2,pk(@0),pk(@1),pk(@2))` | 3 | `md1qzq0j6qgjs54gk3aayrxh8yz7w` | 29 |
| `and(pk(@0),pk(@1))` | 2 | `md1qpq0jum232vapr6wrkmtp5p` | 26 |
| `pk(@0)` (control: no NUMS path) | 1 | `md1qqqqsd8ufd6gm049t0` | 21 |

Predicted v0.18 deltas (per plan §"Wire-bit accounting"):

- n=3: 11 bits TrUnspendable header → 8 bits Tr+sentinel header. Saves 3 bits ≈ 0–1 bech32 chars (alignment-dependent).
- n=2: 11 → 8 bits. Saves 3 bits.
- n=1 NUMS case (none of these are NUMS-keyed): 11 → 7 bits. Saves 4 bits.

Phase 3 will re-encode these and compare lengths.

### V1.d — feature branch created

`git checkout -b feat/v0.18-full-tap-and-nums-engraving` off main at `c5a5ddd` (the v0.17 merge commit). Working tree clean.

## Per-phase code-reviewer round

Skipped: V1 verification is read-only investigation, no implementation surface to review.

## Exit gate

- ✅ V1.a: `--path` bug reproduces (3 paths → identical output).
- ✅ V1.b: All 17 miniscript Terminal variant names match plan.
- ✅ V1.c: v0.17 baselines captured for Phase 3 sentinel-savings comparison.
- ✅ V1.d: Branch created off main at c5a5ddd.

Phase 0 closed; proceeding to Phase 1 (Item J `--path` fix).
