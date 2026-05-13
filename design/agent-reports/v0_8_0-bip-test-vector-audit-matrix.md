# v0.8.0 BIP test vector audit matrix — descriptor-mnemonic (md-codec)

Built 2026-05-13 per the v0.8.0 cross-repo audit cycle.
**Predecessor (still authoritative for v0.7.1 coverage that
carries forward unchanged):**
[`v0_7_1-bip-test-vector-audit-matrix.md`](v0_7_1-bip-test-vector-audit-matrix.md)
(marked SUPERSEDED at v0.8.0 in lockstep with this file).

**Cycle SPEC:** `mnemonic-toolkit/design/SPEC_test_vector_audit_v0_8_0.md`.
**Cycle plan:** `/home/bcg/.claude/plans/v0_8_0-bip-vector-adoption.md`.
**Phase 1 R1:** [`v0_8_0-phase-1-bip341-wallet-r1.md`](v0_8_0-phase-1-bip341-wallet-r1.md).

## §0 Cycle disposition

**md-codec Phase 1 of the v0.8.0 cycle: BIP-341 wallet-test-vectors
pin.** Adds upstream BIP-341 `scriptPubKey` corpus coverage that
v0.7.1 transitively delegated to `bitcoin v0.32`'s taproot path
without local pinning.

## §1 BIP-341 — taproot wallet test vectors

Source: <https://github.com/bitcoin/bips/blob/master/bip-0341/wallet-test-vectors.json>
(`scriptPubKey` array; `keyPathSpending` array OUT-OF-SCOPE-PER-LAYER
— no Schnorr signing surface in this constellation).

Fixture: `crates/md-codec/tests/vectors/bip341-wallet-test-vectors.json`,
sha256 `403e19fb81dd1f31e745699216308f61fb403774b2aafa87b631b8f7c042d37f`.

| # | Tree shape | Status | Test fn |
|---|---|---|---|
| BIP-341.SPK.0 | key-spend only (`scriptTree: null`) | COVERED | `vector_0_key_spend_only` |
| BIP-341.SPK.1 | single leaf, leafVersion 192 | COVERED | `vector_1_single_leaf` |
| BIP-341.SPK.2 | single leaf, leafVersion 192 | COVERED | `vector_2_single_leaf` |
| BIP-341.SPK.3 | balanced `[leaf, leaf]`, mixed leafVersions (192, 250) | COVERED | `vector_3_balanced_two_leaves` |
| BIP-341.SPK.4 | balanced `[leaf, leaf]` | COVERED | `vector_4_balanced_two_leaves` |
| BIP-341.SPK.5 | balanced `[leaf, leaf]` | COVERED | `vector_5_balanced_two_leaves` |
| BIP-341.SPK.6 | asymmetric `[leaf, [leaf, leaf]]` | COVERED | `vector_6_unbalanced_left_leaf_right_subtree` |
| BIP-341.KPS.0 | signing flow (sighash + sign + verify) | OUT-OF-SCOPE-PER-LAYER | filed FOLLOWUP `bip341-keypath-signing-vector-coverage` |

Plus 2 invariant cells:

- `fixture_sha256_pin` — guards against silent upstream fixture drift.
- `scriptpubkey_array_length_is_seven` — guards against upstream
  adding an 8th vector that this file would silently miss.

**Test file:** `crates/md-codec/tests/bip341_wallet_vectors.rs`.
**Phase 1 commit:** `7334f22` (impl), `b464f3f` (close fold).

## §2 BIP coverage unchanged from v0.7.1

All other BIP coverage in md-codec is unchanged from the v0.7.1
matrix and carries forward:

- md1 custom corpus (9 fixtures, MV1..MV9): COVERED via
  `wallet_policy.rs`.
- BIP-380 §380.1 descriptor checksum: COVERED via toolkit-side
  `cli_export_wallet.rs::bip380_valid_checksum_round_trip_via_miniscript`;
  remaining 45 key-expr vectors OUT-OF-SCOPE-PER-LAYER
  (`rust-miniscript` surface).
- BIP-388 reference policies: 4 SHAPE-covered, 4 deferred-per-scope
  (388.1 / .6 / .7 / .8).
- BIP-86 / BIP-49 / BIP-84 derivation: COVERED via toolkit-side
  `cli_convert_address.rs` (md-codec consumes but doesn't pin).

## §3 Sibling-repo cross-coverage (cycle context)

Cross-cite per-repo v0.8.0 matrices:

- `mnemonic-secret/design/agent-reports/v0_8_0-bip-test-vector-audit-matrix.md` —
  Phase 2: BIP-93 full inline corpus (+69 cells).
- `mnemonic-toolkit/design/agent-reports/v0_8_0-bip-test-vector-audit-matrix.md` —
  Phase 3: BIP-85 v85.3 (+1 cell); §0 cross-repo coverage table.
- `mnemonic-key/design/agent-reports/v0_8_0-bip-test-vector-audit-matrix.md` —
  no scope this cycle; cross-repo audit symmetry only.
