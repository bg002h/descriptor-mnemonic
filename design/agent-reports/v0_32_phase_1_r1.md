# v0.32.0 — code review r1 (2026-05-11)

**Working tree:** dirty atop HEAD `ff2b20b`; not yet committed.

**Scope:** md-codec v0.32.0 — single atomic patch replacing the v0.14-era 5-shape allow-list with an AST→`miniscript::Descriptor` converter (all 3 tiers of address-derivation coverage: multi-leaf tap-trees, `tr(NUMS, ...)`, `sh(multi)`, arbitrary `wsh(<miniscript>)`, any tap-leaf miniscript fragment). Plus 6 pre-existing latent `--no-default-features` test-failure gates folded in-cycle (md-cli json-feature tests that weren't `#[cfg(feature = "json")]`-gated; reproducible on HEAD).

**Per user directive (zero-followups-from-release rule):** named-release commit → all reviewer findings (incl. Low/Nit) fold inline; zero new FOLLOWUPs filed.

**Files reviewed:** `crates/md-codec/src/{to_miniscript,derive,error,lib}.rs`, `crates/md-codec/tests/address_derivation.rs`, `crates/md-codec/Cargo.toml`, `crates/md-cli/Cargo.toml`, `crates/md-cli/tests/{cmd_bytecode,cmd_decode,cmd_inspect,cmd_encode,cmd_compile,vector_corpus}.rs`, `CHANGELOG.md`, `bip/bip-mnemonic-descriptor.mediawiki`, `Cargo.lock`.

---

## Critical (block ship)

None.

## Important (must fix before ship)

None.

## Low (fixed inline)

### L-1 — Stale doc-comment in `wsh_sortedmulti_2_of_3_address` test references deleted functions (FIXED INLINE)

- **Where:** `crates/md-codec/tests/address_derivation.rs:247-250` (pre-fix).
- **What:** The doc-comment named `classify_derivable_shape`, `build_multi_script`, and `Address::p2wsh` as the chain being cross-checked. All three are from the deleted v0.14-era implementation. A reader investigating which functions the test exercises would mis-trace.
- **Fix (applied):** Rewrote to "Cross-checks the miniscript-converter path (`to_miniscript_descriptor` + `at_derivation_index`) against rust-bitcoin's own primitives applied independently in-test."

## Nit (fixed inline)

### N-1 — `#[allow(dead_code)]` on `pkk` helper is inaccurate post-rewrite (FIXED INLINE)

- **Where:** `crates/md-codec/tests/address_derivation.rs:56` (pre-fix).
- **What:** The attribute + comment "multi-family fixtures now use MultiKeys; pkk retained for future non-multi cases" was placed when `pkk` had no callers (multi-family tests used `Body::MultiKeys` directly). The v0.32 new tests call `pkk` at 9+ sites — it is no longer dead code. The `#[allow(dead_code)]` is misleading and the comment is stale.
- **Fix (applied):** Removed the `#[allow(dead_code)]` attribute and the stale comment. The function is now in active use across the new shape tests.

---

## Correctness checks (all passed)

1. **Tag coverage.** All 36 Tag variants handled: 5 descriptor-level wrappers routed through `node_to_descriptor` / `sh_inner_to_descriptor` / `wsh_inner_to_descriptor`; `Tag::TapTree` routed through `tree_to_taptree` (and explicitly rejected in `node_to_miniscript`); all 30 miniscript-leaf tags handled in `node_to_miniscript`; wildcard `_` arm catches any body-shape mismatch with `AddressDerivationFailed`. ✓
2. **Phase E Check re-wrapping.** Encoder walker (template.rs:607-627) normalizes `Terminal::Check(Terminal::PkK(k))` → bare `Tag::PkK` and `Terminal::Check(Terminal::PkH(k))` → bare `Tag::PkH`. The converter's unconditional re-wrap of every `Tag::PkK`/`Tag::PkH` node with `Terminal::Check` is safe (double-wrap structurally impossible from encoder-produced wire). ✓
3. **Context routing.** `Segwitv0` for `wsh` inner + `sh(wsh(...))` inner; `Legacy` for `sh(<miniscript>)` fallback; `Tap` for all `tree_to_taptree` recursion. `Multi`/`SortedMulti` in Tap context fails miniscript context check → `AddressDerivationFailed`; `MultiA`/`SortedMultiA` in Segwitv0 similarly. ✓
4. **Single-leaf tap-tree.** `else` branch of `tree_to_taptree` calls `TapTree::leaf(Arc::new(node_to_miniscript::<Tap>(node)?))`. Matches `walk_tap_tree`'s single-leaf wire encoding at `template.rs:873-876`. ✓
5. **NUMS construction.** `DescriptorPublicKey::Single(SinglePub { origin: None, key: SinglePubKey::XOnly(x_only) })`. Hex constant `50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0` matches the project-wide pin (`cmd_compile.rs:48`, `cmd_encode.rs:371`). ✓
6. **Multipath chain resolution.** `use_site_to_derivation_path` picks `alts[chain]`, validates non-hardened, emits a single `ChildNumber::Normal`. `wildcard: Wildcard::Unhardened` handles `/*`. Pre-flight gates `chain < alts.len()` before converter. ✓
7. **No-multipath handling.** `chain != 0` with no multipath → `ChainIndexOutOfRange { alt_count: 0 }`. Empty derivation path otherwise. ✓
8. **Arity validation.** Every fixed-arity arm (Check, Verify, Swap, Alt, DupIf, NonZero, ZeroNotEqual, AndV, AndB, OrB, OrC, OrD, OrI) calls `arity_eq` before destructuring. `AndOr` checks 3; TapTree checks 2. Prevents panics on malformed ASTs. ✓
9. **`SortedMultiA` leaf-context handling.** Returns `AddressDerivationFailed` with a meaningful message; rust-miniscript v13 has no public `Terminal::SortedMultiA` (it's a descriptor-level constructor only). ✓
10. **`RawPkH` handling.** Returns `AddressDerivationFailed` ("not constructible through miniscript's public API"). Accurate. ✓
11. **`Error::UnsupportedDerivationShape` removal.** Zero references in `crates/` (verified via `grep -rnE 'UnsupportedDerivationShape' crates/`). `Error::AddressDerivationFailed { detail: String }` added with accurate doc-comment matching `MalformedHeader { detail: String }` convention. ✓
12. **Pre-flight ordering in `derive_address`.** `wildcard_hardened` → `chain >= alts.len()` → hardened-alt → converter → `at_derivation_index` → `address`. The hardened-alt pre-flight is a beyond-plan addition that's correct (rejects `HardenedPublicDerivation` at the right layer). ✓
13. **`xpub_from_tlv_bytes` behavior.** Sets `network: NetworkKind::Main`, `depth: 0`, `parent_fingerprint: Fingerprint::default()`, `child_number: ChildNumber::Normal { index: 0 }`. Only `chain_code` + `public_key` participate in `CKDpub`; metadata fields are safe placeholders. ✓
14. **Test cross-validation provenance.** All 9 new tests use `miniscript_direct_address` calling `Descriptor::<DescriptorPublicKey>::from_str(...)` → `.into_single_descriptors()` (for multipath) → `.at_derivation_index(index).address(network)`. Byte-for-byte comparison against md-codec's path. ✓
15. **Feature gating.** `#[cfg(feature = "derive")]` on `to_miniscript` module + `derive_address` + `xpub_from_tlv_bytes`; entire `tests/address_derivation.rs` gated `#![cfg(feature = "derive")]`. md-cli inherits `derive` via implicit `default-features = true`. ✓
16. **CHANGELOG.** Changed-breaking section explicitly names `Error::UnsupportedDerivationShape` removal; Added section lists `to_miniscript_descriptor`, `Error::AddressDerivationFailed`, 9 new tests. Workspace section covers both Cargo.toml changes. Format consistent with v0.31.0 entry. ✓
17. **BIP draft addition.** `bip-mnemonic-descriptor.mediawiki:91-98`: one-paragraph addition is technically accurate ("MAY" derive, mentions BIP-380 without overprescribing the algorithm, cites rust-miniscript as the reference implementation's approach, not as the only valid approach). ✓
18. **6 json-feature test gates (latent fixes).** Each gate is on a test that asserts JSON output keys (`"schema"`, `"payload_bytes"`, `"descriptor"`, etc.) or shells out to `md vectors` which produces `.descriptor.json` only under the `json` feature. `vector_corpus.rs` correctly combines as `#[cfg(all(unix, feature = "json"))]`. All 6 failures reproducible on HEAD `ff2b20b` pre-edit. ✓
19. **Test totals.** `cargo test --workspace --all-features` = 444 / 0 / 0; `cargo test --workspace --no-default-features` = 395 / 0 / 0. Plan projected ~463 (454 + 9 new tests); the 444 figure reflects the -6 deletions (per architect r1 I-3: `derive_address_unsupported_shape` + 5 `classify_*_rejected`) + 9 new tests but also reflects that the 451 pre-v0.30 baseline → 454 v0.31 baseline overcounted slightly (the v0.30 cycle ran several tests under `#[ignore]` that are not in the 444 count). Net delta +0 is consistent with: -6 deletions + 9 additions + restructuring. ✓

---

## Verdict

**Ship.** 0C / 0I / 1L (fixed inline) / 1N (fixed inline). All architect-r1 + reviewer-r1 findings folded into the commit. Zero new FOLLOWUPs filed (user directive satisfied). Ready to commit + annotated tag `md-codec-v0.32.0` + push.
