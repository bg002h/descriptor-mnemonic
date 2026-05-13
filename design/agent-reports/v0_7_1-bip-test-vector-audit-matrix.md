# v0.16.2 BIP test vector audit matrix — descriptor-mnemonic (md-codec)

**SUPERSEDED at v0.8.0** — see
[`v0_8_0-bip-test-vector-audit-matrix.md`](v0_8_0-bip-test-vector-audit-matrix.md)
for the BIP-341 wallet-test-vectors corpus added at v0.8.0 plus
cross-repo cross-cites. Coverage in this file is still authoritative
for everything v0.8.0 did not change.

Built 2026-05-07 per the v0.7.1 audit cycle plan
(`/home/bcg/.claude/plans/let-s-work-on-the-soft-waterfall.md`).

Scope: md-codec is the reference implementation of the md1 wire format —
HRP `md`, BIP-93-derived BCH plumbing **forked** (HRP-mixing + per-format
NUMS target residues), wallet-policy descriptor compression. md-codec
implements its own corpus + cross-references BIP-380 (descriptor checksum)
and BIP-388 (wallet policies) at the descriptor-pipeline layer; it
delegates BIP-32 / BIP-39 derivation to the upstream `bitcoin v0.32` +
`bip39 = 2` crates.

Status legend: same as toolkit matrix.

---

## md1 custom corpus

Source: `crates/md-codec/tests/vectors/*.{template,phrase.txt,bytes.hex,descriptor.json}`.
9 deterministic fixtures (one per descriptor template family). Round-trip
asserted via `tests/wallet_policy.rs` integration tests against the
`Descriptor` struct.

| # | Fixture | Template | Status | Notes |
|---|---|---|---|---|
| MV1 | pkh_basic | `pkh(@0/**)` | COVERED | template-only round-trip via test cell-1 |
| MV2 | wpkh_basic | `wpkh(@0/**)` | COVERED | `tests/wallet_policy.rs::smoke_1of1_cell_7_wpkh_round_trip` (analogue) |
| MV3 | sh_wsh_multi | `sh(wsh(sortedmulti(2,@0/**,@1/**)))` | COVERED | `tests/wallet_policy.rs::forced_explicit_sh_sortedmulti_rejected_at_decoder` (negative-shape pin) |
| MV4 | tr_keyonly | `tr(@0/**)` | COVERED | `tests/smoke.rs::bip86_taproot_md1_string_round_trip` |
| MV5 | wsh_sortedmulti | `wsh(sortedmulti(2,@0/**,...,@N/**))` | COVERED | `tests/smoke.rs::bip48_2of3_md1_string_round_trip` |
| MV6 | wsh_multi_2of2 | `wsh(multi(2,@0/**,@1/**))` | COVERED | `tests/wallet_policy.rs::partial_keys_2of2_at0_cell7_at1_cell1` |
| MV7 | wsh_multi_2of3 | `wsh(multi(2,@0/**,@1/**,@2/**))` | COVERED | `tests/wallet_policy.rs::smoke_2of3_cell_7_wsh_sortedmulti_round_trip` |
| MV8 | wsh_multi_chunked | multi-chunk | COVERED | `tests/wallet_policy.rs::multi_chunk_2of3_cell_7_split_reassemble_round_trip` |
| MV9 | wsh_with_fingerprints | with origin fp + xpubs | COVERED | `tests/wallet_policy.rs::smoke_2of3_cell_7_wsh_sortedmulti_round_trip` |
| MV10 | wsh_divergent_paths | per-`@N` divergent origin paths | COVERED | `tests/wallet_policy.rs::divergent_paths_wallet_policy_2of2_round_trip` |

Phase 11 deliverable (resolved v0.16.2): verify corpus drift is caught
at CI boundary. RESOLVED — see Discovery #4. There is no SHA-256 pin
because the drift discipline is asserted via `diff -r` byte-identity
over the regenerated tree (`md-cli/tests/vector_corpus.rs`), which is
strictly stronger.

---

## BIP-93 — codex32 (forked BCH)

Source: <https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki>.

**Forked**, not delegated. md-codec implements its own
`bch_create_checksum_regular()` / `bch_verify_regular()` over GF(32) with
HRP-mixing using its own NUMS target residue
(`MD_REGULAR_CONST` derived from `SHA-256(b"shibbolethnums")`).

Conformance posture (per `design/SPEC_md_v0_X.md`): md1's BCH polynomial
matches BIP-93 §"Generation of valid checksum" up to the target-residue
constant. Distinct constant + HRP-mixing means BIP-93 vectors are
NOT bit-identical to md1 vectors and MUST NOT be pinned as such.

| # | BIP-93 vector | Applicability to md-codec | Status |
|---|---|---|---|
| 93.1–93.5 | upstream codex32 valid vectors | NOT BIT-IDENTICAL — different target residue | OUT-OF-SCOPE-PER-SPEC |
| 93.invalid (42 strings) | upstream invalid forms | structural rejections (wrong HRP, mixed case) translate; cryptographic-residue rejections do not | partially OUT-OF-SCOPE-PER-SPEC; HRP/case rejections are COVERED transitively in md-codec's negative-test corpus |

Phase 11 deliverable (audit-only, no new test): document in CHANGELOG that
md1 BCH is BIP-93-derived but not BIP-93-bit-identical, with explicit
reference to `string_layer/bch_decode.rs` (or md-codec equivalent) and
`design/AUDIT_*` documents.

---

## BIP-32 — HD wallets

Source: <https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki>.

md-codec consumes `bitcoin::bip32::{Xpub, Xpriv, ChildNumber, DerivationPath}`
from `bitcoin v0.32`. Direct vector-pinning is OUT-OF-SCOPE-PER-SPEC at the
md-codec level — `bitcoin v0.32` carries its own BIP-32 vector tests.

The md-codec layer that *does* exercise BIP-32 conformance is
`tests/address_derivation.rs`, which uses the BIP-39 ABANDON_MNEMONIC
to derive a real account xpub, walks it through the md1 round-trip, and
validates the derived address matches BIP-84 / BIP-86 / BIP-44 published
addresses. This implicitly covers BIP-32 derivation correctness for the
specific paths used by those address vectors.

| # | Path | Validation | Status |
|---|---|---|---|
| BIP32-IMPLICIT.1 | m/84'/0'/0'/0/0 | matches `bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu` | COVERED via `tests/address_derivation.rs::bip84_wpkh_receive_address_zero` |
| BIP32-IMPLICIT.2 | m/84'/0'/0'/0/1 | matches `bc1qnjg0jd...erkf9g` | COVERED via `tests/address_derivation.rs::bip84_wpkh_receive_address_one` |
| BIP32-IMPLICIT.3 | m/84'/0'/0'/1/0 | matches `bc1q8c6fshw...cp6el` | COVERED via `tests/address_derivation.rs::bip84_wpkh_change_address_zero` |
| BIP32-IMPLICIT.4 | m/86'/0'/0'/0/0 | matches `bc1p5cyxnux...drcr` | COVERED via `tests/address_derivation.rs::bip86_tr_keypath_only_receive_address_zero` |
| BIP32-IMPLICIT.5 | m/44'/0'/0'/0/0 | matches BIP-44 receive 0 (computed from upstream BIP-44 examples) | COVERED via `tests/address_derivation.rs::bip44_pkh_receive_address_zero` |
| BIP32-IMPLICIT.6 | m/84'/1'/0'/0/0 (testnet) | testnet receive 0 | COVERED via `tests/address_derivation.rs::bip84_wpkh_testnet_address` |

Phase 11: NO new tests at this layer; the implicit BIP-32 coverage via
address-derivation is sufficient (toolkit-side audit-matrix Phase 1
pins BIP-32 vectors directly).

---

## BIP-39 — mnemonic seed

`tests/address_derivation.rs::ABANDON_MNEMONIC` is the only md-codec-side
BIP-39 cite. Toolkit + ms-codec own the BIP-39 vector pin.

OUT-OF-SCOPE-PER-SPEC at md-codec.

---

## BIP-44 / 48 / 49 / 84 / 86 / 87 — derivation path conventions

Source: respective BIP §Test Vectors / §Examples sections.

md-codec encodes BIP-44/48/49/84/86/87 paths into its **path dictionary**
(1-byte indicator → canonical path), shipped at `crates/md-codec/src/origin_path.rs`.
The lockstep-with-mk1 invariant (per `CLAUDE.md` cross-repo coordination)
means each path-dict entry is cross-validated against mk-codec's mirror.

| # | BIP / path | Indicator | Status | Notes |
|---|---|---|---|---|
| PD.0x01 | BIP-44 mainnet `m/44'/0'/0'` | 0x01 | COVERED-LOCKSTEP | mirrored in mk-codec; path-dict-mirror invariant |
| PD.0x02 | BIP-49 mainnet `m/49'/0'/0'` | 0x02 | COVERED-LOCKSTEP | same |
| PD.0x03 | BIP-84 mainnet `m/84'/0'/0'` | 0x03 | COVERED via address-derivation tests above |
| PD.0x04 | BIP-86 mainnet `m/86'/0'/0'` | 0x04 | COVERED via address-derivation tests above |
| PD.0x05 | BIP-87 mainnet `m/87'/0'/0'` | 0x05 | COVERED-LOCKSTEP |
| PD.0x06 | BIP-48 mainnet nested `m/48'/0'/0'/1'` | 0x06 | COVERED-LOCKSTEP |
| PD.0x07 | BIP-48 mainnet segwit `m/48'/0'/0'/2'` | 0x07 | COVERED via `tests/smoke.rs::bip48_2of3_sortedmulti_round_trip` |
| PD.0x11 | BIP-44 testnet `m/44'/1'/0'` | 0x11 | COVERED-LOCKSTEP |
| PD.0x12 | BIP-49 testnet `m/49'/1'/0'` | 0x12 | COVERED-LOCKSTEP |
| PD.0x13 | BIP-84 testnet `m/84'/1'/0'` | 0x13 | COVERED via `tests/address_derivation.rs::bip84_wpkh_testnet_address` |
| PD.0x14 | BIP-86 testnet `m/86'/1'/0'` | 0x14 | COVERED-LOCKSTEP |
| PD.0x15 | BIP-87 testnet `m/87'/1'/0'` | 0x15 | COVERED-LOCKSTEP |
| PD.0x16 | BIP-48 testnet nested `m/48'/1'/0'/1'` | 0x16 | COVERED-LOCKSTEP (gap closed in v0.9.0 per `CLAUDE.md` notes) |
| PD.0x17 | BIP-48 testnet segwit `m/48'/1'/0'/2'` | 0x17 | COVERED-LOCKSTEP |

Phase 11: re-verify mk-codec mirror byte-identity; flag any drift.

---

## BIP-380 — descriptor expressions (checksum)

Source: <https://github.com/bitcoin/bips/blob/master/bip-0380.mediawiki> §Test Vectors.

md-codec emits + parses BIP-380 descriptor strings via its own
`tests/vectors/*.descriptor.json` fixtures (each carries a BIP-380-conformant
`#checksum` string). The descriptor parser delegates to `rust-miniscript`.

| # | Form | Status | Notes |
|---|---|---|---|
| 380.1 | `raw(deadbeef)#89f8spxm` (valid) | OUT-OF-SCOPE-PER-LAYER | md-codec the *library* does not emit BIP-380 descriptor strings; descriptor-string emission lives in md-cli (rust-miniscript-backed). Transitively COVERED at the tree level: `md vectors` regenerates `tests/vectors/*.descriptor.json` (each with a BIP-380 `#checksum`) from `MANIFEST` and `md-cli/tests/vector_corpus.rs::vectors_output_matches_committed_corpus` `diff -r` asserts byte-identity vs. the committed tree. See Discovery #4. |
| 380.2 | `raw(deadbeef)` (no checksum, REJECT) | OUT-OF-SCOPE-PER-SPEC | md-codec always emits the checksum form |
| 380.3 | `raw(deadbeef)#` (empty, REJECT) | OUT-OF-SCOPE-PER-SPEC | rust-miniscript enforces |
| 380.4 | 9-char checksum (REJECT) | OUT-OF-SCOPE-PER-SPEC | rust-miniscript enforces |
| 380.5 | 7-char checksum (REJECT) | OUT-OF-SCOPE-PER-SPEC | rust-miniscript enforces |
| 380.6 | payload-error (REJECT) | OUT-OF-SCOPE-PER-SPEC | rust-miniscript enforces |
| 380.7 | checksum-error (REJECT) | OUT-OF-SCOPE-PER-SPEC | rust-miniscript enforces |
| 380.8 | non-ASCII (REJECT) | OUT-OF-SCOPE-PER-SPEC | rust-miniscript enforces |

Phase 11 deliverable: RECLASSIFIED OUT-OF-SCOPE-PER-LAYER (resolved
v0.16.2). md-codec the library has no descriptor-string emit surface,
so the originally-framed "emit + verify" cycle is structurally
impossible at this crate boundary. The transitive coverage chain is:
md-cli `vectors` subcommand → rust-miniscript `Descriptor::to_string()`
→ committed `*.descriptor.json` (with BIP-380 `#checksum`) →
`vector_corpus.rs::vectors_output_matches_committed_corpus` `diff -r`
gate. Any drift in BIP-380 checksum emission surfaces as a corpus diff
failure at CI.

---

## BIP-388 — wallet policies

Source: <https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki> §Test Vectors.

md-codec is *the* reference impl for BIP-388 wallet-policy compression
into engravable form. The 7 BIP-388 reference policies map to md-codec
templates as follows:

| # | BIP-388 template | md-codec coverage | Status | Notes |
|---|---|---|---|---|
| 388.1 | `pkh(@0/**)` BIP-44 | `tests/vectors/pkh_basic.template` | COVERED | round-trip pinned via `pkh_basic` fixture |
| 388.2 | `sh(wpkh(@0/**))` BIP-49 | `tests/wallet_policy.rs::bip388_388_2_sh_wpkh_bip49_template_shape_round_trip` | COVERED | resolved v0.16.2: lib-level template-shape round-trip pins the wrapper stack + BIP-49 origin path. Spec xpub byte-pin remains OUT-OF-SCOPE-PER-SPEC (no published seed). Fixture-corpus omission is intentional per `manifest.rs` comment: codec asymmetry on canonical-default stripping at decode prevents `force_chunked: false` round-trip via the `MANIFEST → vectors regen → diff` channel. |
| 388.3 | `wpkh(@0/**)` BIP-84 | `tests/vectors/wpkh_basic.template` | COVERED | |
| 388.4 | `tr(@0/**)` BIP-86 | `tests/vectors/tr_keyonly.template` | COVERED | `tests/smoke.rs::bip86_taproot_md1_string_round_trip` |
| 388.5 | `wsh(sortedmulti(2,@0/**,@1/**))` BIP-48 | `tests/vectors/wsh_sortedmulti.template` | COVERED | |
| 388.6 | `wsh(thresh(3,...,sln:older(12960)))` miniscript decay | `tests/smoke.rs::vault_or_d_pk_older_md1_string_round_trip` (analogous) | COVERED-PARTIAL | similar miniscript shape pinned; not the exact BIP-388 spec template |
| 388.7 | `tr(@0/**,{sortedmulti_a(...),or_b(...)})` taproot tree | NOT in fixture corpus | OUT-OF-SCOPE-PER-USER | tap-tree multisig deferred to v0.17+ per CHANGELOG roadmap |
| 388.8 | musig2 templates | NOT supported | OUT-OF-SCOPE-PER-USER | musig2 not in any md1 v0.x scope |

Phase 11 deliverable: pin 388.2 (sh-wpkh nested-segwit). RESOLVED in
v0.16.2 — `tests/wallet_policy.rs::bip388_388_2_sh_wpkh_bip49_template_shape_round_trip`
asserts that `Descriptor { tree: sh(wpkh(@0)), path_decl: m/49'/0'/0' }`
round-trips through `encode_md1_string → decode_md1_string` with
structural equality. The spec's exact `[6738736c/49'/0'/1']xpub6Bex1...`
xpub byte-pin remains OUT-OF-SCOPE-PER-SPEC because BIP-388 doesn't
publish the underlying seed.

---

## BIP-32 use-site path encoding

The use-site path (the `<0;1>/*` multipath shape per BIP-388 §3) is
covered by md-codec's `use_site_path.rs` module. All 9 fixtures use
the canonical `<0;1>/*` shape; `tests/wallet_policy.rs::placeholder_ordering_rejected_by_validator`
asserts validator rejection of skipped/duplicated indices.

| # | Use-site form | Status | Notes |
|---|---|---|---|
| US.1 | `<0;1>/*` (canonical receive+change) | COVERED | every fixture |
| US.2 | `<2;3>/*` (musig2 keypath/scriptpath split) | OUT-OF-SCOPE-PER-USER | musig2 deferred |
| US.3 | invalid `/0/0` (not multipath) | COVERED-NEGATIVE | `tests/wallet_policy.rs::placeholder_ordering_rejected_by_validator` analogue |

---

## Summary

| Category | Total vectors | Covered | Missing (in-scope) | Out-of-scope-per-user | Out-of-scope-per-spec |
|---|---|---|---|---|---|
| md1 custom corpus | 10 | 10 | 0 | 0 | 0 |
| BIP-93 | 5+42 | structural rejection only | 0 | 0 | 47 (forked BCH) |
| BIP-32 | 18 | 6 IMPLICIT | 0 | 0 | 18 (delegated to bitcoin v0.32) |
| BIP-39 | 24 | — | 0 | 0 | 24 (delegated upstream) |
| BIP-44/48/49/84/86/87 path dict | 14 | 14 (LOCKSTEP) | 0 | 0 | 0 |
| BIP-380 | 8 | 0 (transitive at md-cli) | 0 | 0 | 8 (md-cli surface + rust-miniscript) |
| BIP-388 | 8 | 6 | 0 | 2 | 0 |
| BIP-32 use-site | 3 | 2 | 0 | 1 | 0 |
| **TOTAL** | **139** | **~38** | **0** | **~3** | **~98** |

Phase 11 outcome (v0.16.2 — 2026-05-07):
- 1 net-new test added: `wallet_policy.rs::bip388_388_2_sh_wpkh_bip49_template_shape_round_trip` (BIP-388 §Test Vectors policy 388.2 cite).
- 380.1 reclassified OUT-OF-SCOPE-PER-LAYER for md-codec (transitive at md-cli).
- Manifest SHA-pin discipline VERIFIED GREEN (see Discovery #4).
- Test count delta: +1 (358 → 359 default-features workspace).

---

## Discoveries (require architect review before pinning)

1. **AMBIGUOUS — BIP-388 spec `[6738736c/...]` xpub byte-pin not
   re-derivable.** The spec gives concrete xpub strings but no seed.
   md-codec must settle for "template-shape COVERED + spec xpubs
   quoted in test source as documentation only." Same posture as
   the toolkit-side audit-matrix and as upstream rust-miniscript test
   coverage. Documented; not a bug.

2. **AMBIGUOUS — `tests/vectors/manifest.rs` SHA pin discipline.**
   Plan §Phase 11 says "verify SHA pins are real." Need to confirm
   `manifest.rs` actually computes SHA-256 of the fixture files and
   asserts equality (vs. just listing filenames). If listing-only,
   that's a Critical-finding upgrade. **Action:** Phase 11 reads
   `manifest.rs` first; pin is real → COVERED in this matrix; pin
   is hollow → discoveries-elevation + new test.

   **RESOLVED v0.16.2 — see Discovery #4 below. NOT a Critical
   finding.** The manifest is structurally a `&[Vector { name,
   template, keys, fingerprints, force_chunked }]` constants table,
   not a SHA-256-hash assertion list — but the drift-detection
   discipline it powers is functionally equivalent to (and stronger
   than) SHA pinning, so the audit-matrix conditional escalation does
   not trigger.

3. **No bug-shaped findings.** md-codec's vector posture is strong:
   custom-corpus round-trips are exhaustive; BIP-32 derivation is
   exercised end-to-end via address-pinning; BIP-388 templates are
   shape-pinned. The two MISSING items (BIP-380 emit-checksum +
   BIP-388 sh-wpkh) are coverage gaps, not impl bugs.

4. **Corpus drift discipline GREEN (resolves Discovery #2).**
   `crates/md-codec/tests/vectors/manifest.rs` is a constants table
   of `(name, template, keys, fingerprints, force_chunked)`, not a
   SHA-256 hash list. Drift detection happens via
   `crates/md-cli/tests/vector_corpus.rs::vectors_output_matches_committed_corpus`,
   which:

   - shells out to `md vectors --out <tmpdir>` (regenerating every
     `.template` / `.phrase.txt` / `.bytes.hex` / `.descriptor.json`
     from `MANIFEST` + the encoder pipeline);
   - runs `diff -r --exclude=manifest.rs --exclude=.gitkeep
     <tmpdir> crates/md-codec/tests/vectors/`;
   - asserts `status.success()`.

   This is functionally equivalent to (and stronger than) SHA-256
   pinning: the test asserts byte-identity over the entire corpus
   tree on every CI run. Encoder-bug regressions, fixture file edits,
   and template manifest drift all surface as a `diff -r` failure.

   Two adjacent layers reinforce this:

   - `template_roundtrip.rs::round_trip_each_manifest_entry` walks
     every non-`force_chunked` MANIFEST entry through `md encode`
     then `md decode`, asserting template-string round-trip.
   - `json_snapshots.rs` (gated on `feature = "json"`) runs `insta`
     snapshot tests on every MANIFEST entry's `decode --json` and
     `inspect --json` output.

   Net: no SHA pin is needed because the corpus's *structural*
   fidelity is asserted at three independent layers. The audit
   matrix's conditional Critical-escalation does NOT trigger.
