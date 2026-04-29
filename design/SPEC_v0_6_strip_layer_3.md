# v0.6 Design Spec: Strip Layer 3 (signer-compatibility curation)

**Status:** Draft (2026-04-28; round-1 review folded in)
**Supersedes (in concept):** Phase D's `validate_tap_leaf_subset` enforcement (commit `6f6eae9`) and the BIP draft MUST clause at `bip/bip-mnemonic-descriptor.mediawiki:547`.
**Companion documents:**
- Rationale: [`MD_SCOPE_DECISION_2026-04-28.md`](./MD_SCOPE_DECISION_2026-04-28.md)
- Implementation plan: [`IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md`](./IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md)
- Master FOLLOWUPS entry: `md-scope-strip-layer-3-signer-curation`

---

## §1. Scope and Goals

### 1.1 What changes

v0.6 strips MD's signer-compatibility curation layer. After v0.6:

- The encoder admits any `Miniscript<DescriptorPublicKey, Tap>` AST that rust-miniscript produces from a valid BIP 388 wallet-policy parse — no per-operator subset gate by default.
- The decoder admits any wire input that parses to a valid `Tag` and structurally well-formed bytecode — no per-operator subset gate by default.
- `validate_tap_leaf_subset` (and its helper `validate_tap_leaf_terminal`) are retained as `pub fn` for explicit-call use by callers who want signer-aware validation.
- `Error::TapLeafSubsetViolation` variant is retained (used by the explicit-call validator path).

### 1.2 What does NOT change

- Wire format for the operator tags MD has historically admitted is preserved at the bit level *only after* the Tag-space rework's renumbering settles (§2). The bytecode of any individual operator changes byte-for-byte, but the format's structural rules (LEB128 LEN encoding, placeholder framing, BCH error correction) stay identical.
- Top-level descriptor admit set (`wsh`/`tr`/`wpkh`/`sh(wsh(...))` etc.) is untouched. The strip is a tap-leaf concern only.
- `Tag::TapTree` depth-128 ceiling stays. That's a BIP 341 consensus rule, not a signer-policy gate.
- The wallet-policy framing (`@i/<a;b>/*` placeholders, key-info-vector) is unchanged.

### 1.3 Scope boundaries (out of scope for v0.6)

The following are tracked in FOLLOWUPS but explicitly NOT in this spec:

- **Caller-supplied opt-in API design** (`md-signer-compat-checker-separate-library`): `EncodeOptions::with_signer_subset(subset)` and the `SignerSubset` struct. v0.6 retains the `pub fn` validators; the integrated API ships later.
- **Named signer subsets** (Coldcard, Ledger). Lives in a separate crate at the consumer's discretion.
- **Policy compiler** (`md-policy-compiler-feature`): rust-miniscript `compiler` feature stays disabled. v0.7+ candidate.

---

## §2. Wire Format — Tag-space rework

### 2.1 Goals

- Allocate `Tag::SortedMultiA` (currently no Tag exists; `Terminal::SortedMultiA` falls through to a literal string in `tap_terminal_name`).
- Reorganize the Tag enum from descriptor-codec-vendored layout into a coherent grouping.
- Drop the `Reserved*` range (0x24–0x31, 14 bytes) entirely. Justification: MD's BIP-388 wallet-policy framing forbids inline keys; descriptor-codec's inline-key vendoring is dead weight relative to MD's stated scope.

### 2.2 Final Tag layout

The post-rework Tag enum:

```
// Constants (2)
0x00  False                       // miniscript `0`
0x01  True                        // miniscript `1`

// Top-level descriptor wrappers (5)
0x02  Pkh                         // pkh(KEY) — top-level only (NOT inside tr)
0x03  Sh                          // sh(...)
0x04  Wpkh                        // wpkh(KEY)
0x05  Wsh                         // wsh(...)
0x06  Tr                          // tr(KEY) or tr(KEY, TREE)

// Tap-tree framing (1)
0x07  TapTree                     // inner-node framing inside tr(KEY, TREE)

// Multisig family (4) — grouped together post-rework
0x08  Multi                       // multi(k, ...)         (P2WSH-only by miniscript typing)
0x09  SortedMulti                 // sortedmulti(k, ...)   (P2WSH-only by miniscript typing)
0x0A  MultiA                      // multi_a(k, ...)       (Tapscript-only by miniscript typing)
0x0B  SortedMultiA                // sortedmulti_a(k, ...) (Tapscript-only — NEW Tag at v0.6)

// Wrappers (7)
0x0C  Alt                         // a:
0x0D  Swap                        // s:
0x0E  Check                       // c:
0x0F  DupIf                       // d:
0x10  Verify                      // v:
0x11  NonZero                     // j:
0x12  ZeroNotEqual                // n:

// Logical operators (8)
0x13  AndV                        // and_v(X, Y)
0x14  AndB                        // and_b(X, Y)
0x15  AndOr                       // andor(X, Y, Z)
0x16  OrB                         // or_b(X, Z)
0x17  OrC                         // or_c(X, Z)
0x18  OrD                         // or_d(X, Z)
0x19  OrI                         // or_i(X, Z)
0x1A  Thresh                      // thresh(k, X_1, ..., X_n)

// Keys (3)
0x1B  PkK                         // pk_k(KEY)
0x1C  PkH                         // pk_h(KEY)
0x1D  RawPkH                      // pk_h with inlined 20-byte hash (HTLC-style)

// Timelocks (2)
0x1E  After                       // after(n)
0x1F  Older                       // older(n)

// Hashes (4)
0x20  Sha256                      // sha256(h)
0x21  Hash256                     // hash256(h)
0x22  Ripemd160                   // ripemd160(h)
0x23  Hash160                     // hash160(h)

// (gap: 0x24–0x31 — formerly Reserved* descriptor-codec inline-key forms; now unallocated, return None from from_byte)
// (gap: 0x32 — formerly Placeholder in v0.5; now unallocated. Placeholder moves to 0x33 to avoid ambiguity with v0.5 strings under transcoding mistakes — see §2.4)

// MD-specific framing (3)
0x33  Placeholder                 // @i — placeholder for key-info-vector index
0x34  SharedPath                  // shared-path declaration for placeholder framing
0x35  Fingerprints                // fingerprints block (Phase E v0.2)

// 0x36–0xFF: unallocated (return None from from_byte)
```

**Note on `Tag::Bare` removal.** v0.5's `Tag::Bare` (0x07) is **dropped entirely** in v0.6 per the round-1 spec review. Rationale: `Descriptor::Bare` is permanently rejected by the encoder per BIP draft scope (encode.rs:176-179) and `Tag::Bare` was never used as an inner tag, so the variant is dead weight. Its byte 0x07 is reused for `TapTree` (the more useful adjacent-to-Tr placement). The implementation must also drop the `Tag::Bare => "bare"` arm in `tag_to_bip388_name` at decode.rs:822.

**Note on `Fingerprints = 0x35` retention.** The byte stays put rather than relocating into the contiguous framing block at 0x33–0x34 because the v0.2 fingerprints-block byte has shipped as part of the wire format; an external decoder that already inspects bytecode for the fingerprints flag pattern continues to work after the rework on this specific byte. (Other bytes change; this one preserves continuity.)

**Note on byte 0x32 unallocation.** v0.5 emitted `Placeholder = 0x32` in every encoded MD string. Reusing 0x32 in v0.6 for any other operator would silently misinterpret a v0.5 string fed into a v0.6 decoder. Leaving 0x32 unallocated (return `None` from `from_byte`) surfaces such transcoder mistakes as a clean error (`UnknownTag(0x32)`) rather than data corruption.

### 2.3 Byte-for-byte changes from v0.5

Every tap-leaf-bearing existing operator's byte may change. Worst case: every fixture in v0.1.json + v0.2.json regenerates with new bytecode. Full table:

| Operator | v0.5 Tag | v0.6 Tag |
|---|---|---|
| `False` | 0x00 | 0x00 (unchanged) |
| `True` | 0x01 | 0x01 (unchanged) |
| `Pkh` | 0x02 | 0x02 (unchanged) |
| `Sh` | 0x03 | 0x03 (unchanged) |
| `Wpkh` | 0x04 | 0x04 (unchanged) |
| `Wsh` | 0x05 | 0x05 (unchanged) |
| `Tr` | 0x06 | 0x06 (unchanged) |
| `Bare` | 0x07 | (DROPPED — variant removed; not allocated) |
| `TapTree` | 0x08 | 0x07 (moved adjacent to Tr) |
| `SortedMulti` | 0x09 | 0x09 (unchanged byte; now grouped with multisig family) |
| `Alt` | 0x0A | 0x0C |
| `Swap` | 0x0B | 0x0D |
| `Check` | 0x0C | 0x0E |
| `DupIf` | 0x0D | 0x0F |
| `Verify` | 0x0E | 0x10 |
| `NonZero` | 0x0F | 0x11 |
| `ZeroNotEqual` | 0x10 | 0x12 |
| `AndV` | 0x11 | 0x13 |
| `AndB` | 0x12 | 0x14 |
| `AndOr` | 0x13 | 0x15 |
| `OrB` | 0x14 | 0x16 |
| `OrC` | 0x15 | 0x17 |
| `OrD` | 0x16 | 0x18 |
| `OrI` | 0x17 | 0x19 |
| `Thresh` | 0x18 | 0x1A |
| `Multi` | 0x19 | 0x08 |
| `MultiA` | 0x1A | 0x0A |
| `SortedMultiA` | (none) | 0x0B (NEW) |
| `PkK` | 0x1B | 0x1B (unchanged) |
| `PkH` | 0x1C | 0x1C (unchanged) |
| `RawPkH` | 0x1D | 0x1D (unchanged) |
| `After` | 0x1E | 0x1E (unchanged) |
| `Older` | 0x1F | 0x1F (unchanged) |
| `Sha256` | 0x20 | 0x20 (unchanged) |
| `Hash256` | 0x21 | 0x21 (unchanged) |
| `Ripemd160` | 0x22 | 0x22 (unchanged) |
| `Hash160` | 0x23 | 0x23 (unchanged) |
| `Reserved*` 0x24–0x31 | 14 variants | (DROPPED — variants removed; `from_byte` returns `None`) |
| `Placeholder` | 0x32 | 0x33 |
| `SharedPath` | 0x33 | 0x34 |
| `Fingerprints` | 0x35 | 0x35 (unchanged) |

**Structural summary.** `TapTree` moves down to 0x07 (adjacent to Tr=0x06); `Bare` is dropped entirely (its v0.5 byte 0x07 is reused for TapTree); the multisig family expands into 0x08–0x0B (Multi, SortedMulti, MultiA, SortedMultiA-NEW); wrappers (a:/s:/c:/d:/v:/j:/n:) and logical operators (and_v..thresh) all shift by 2 from their v0.5 positions because the multisig family grew. Constants (False/True), top-level descriptor wrappers (Pkh/Sh/Wpkh/Wsh/Tr), keys (PkK/PkH/RawPkH), timelocks (After/Older), hashes (Sha256/Hash256/Ripemd160/Hash160), and `Fingerprints` are byte-identical from v0.5 to v0.6. Placeholder/SharedPath shift by 1 (each moves up one byte) so byte 0x32 (formerly Placeholder) is unallocated post-rework — see §2.2 "Note on byte 0x32 unallocation".

### 2.4 Impact on family-stable SHAs

The v0.5.x family-stable promise (`"md-codec 0.5"` → byte-stable v0.5.x SHAs) does NOT carry to v0.6. v0.6.0 starts a new family-stable line:

- `GENERATOR_FAMILY` rolls `"md-codec 0.5"` → `"md-codec 0.6"`.
- v0.1.json and v0.2.json fully regenerate; SHA pins in `tests/vectors_schema.rs` update once at the v0.5.x → v0.6.0 boundary.
- v0.6.x patch line stable thereafter.

### 2.5 Pre-existing vector files

v0.1.json and v0.2.json file *names* are preserved (schema versions are independent of the family generator). Files are regenerated; bytecode_hex changes per §2.3; SHAs change once.

---

## §3. Encoder Design — default validator removal

### 3.1 Current behaviour (v0.5)

`crates/md-codec/src/bytecode/encode.rs` `Miniscript<DescriptorPublicKey, Tap>` `EncodeTemplate` impl calls:

```rust
fn encode_template(...) -> Result<(), Error> {
    validate_tap_leaf_subset(self, leaf_index)?;  // <-- gate
    encode_tap_terminal(&self.node, ..., leaf_index)?;
    Ok(())
}
```

`encode_tap_terminal` itself has a catch-all that produces `TapLeafSubsetViolation` for out-of-subset operators (defensive double-gate; redundant once `validate_tap_leaf_subset` is the first call).

### 3.2 v0.6 behaviour

Remove the default-path call to `validate_tap_leaf_subset`. The encoder admits any AST shape rust-miniscript provides:

```rust
fn encode_template(...) -> Result<(), Error> {
    encode_tap_terminal(&self.node, ..., leaf_index)?;
    Ok(())
}
```

`validate_tap_leaf_subset` and `validate_tap_leaf_terminal` remain `pub fn` in the same file. They can be called explicitly by consumer code that wants signer-aware validation. Their rustdoc updates to make this plain. Note that since `validate_tap_leaf_subset` becomes the only remaining caller of `tap_terminal_name`, the latter's rustdoc should clarify it is no longer the universal naming hook for tap-context errors — only the explicit-call validator path.

`encode_tap_terminal`'s catch-all is replaced by an **exhaustive match** (option (a) per the round-1 review). This is achievable because `Terminal` is NOT `#[non_exhaustive]` in the pinned miniscript revision (`apoelstra/rust-miniscript` rev `f7f1689b...` per `Cargo.toml:38`). The exhaustive match emits the Tag byte unconditionally for every `Terminal<DescriptorPublicKey, Tap>` variant — **including tap-illegal variants `Terminal::Multi` and `Terminal::SortedMulti`**. Rationale: the new "format is neutral" framing places the upstream gate in miniscript's parser (which refuses to construct `Terminal::Multi` in a `Tap` context). Emitting a wire byte for hand-built tap-illegal ASTs is harmless because no decoder will produce that AST shape from a tap-context decode (the decoder has its own context-aware dispatcher).

If miniscript is ever upgraded to make `Terminal` `#[non_exhaustive]`, the compiler will force a re-evaluation at that boundary — a wildcard arm returning `Error::SubsetViolation { operator: format!("{:?}", term), leaf_index: None }` is the correct fallback (the explicit-call validator path's error variant; see §5).

### 3.3 `Tag::SortedMultiA` encoding

Add a `Terminal::SortedMultiA` arm to `encode_tap_terminal`'s match emitting `Tag::SortedMultiA` followed by `[k][n][key_1]...[key_n]` per the existing multi/multi_a pattern. Symmetric with `Terminal::MultiA`.

### 3.4 Tap-tree encoder unchanged

The `encode_tap_subtree` recursive helper (multi-leaf TapTree v0.5 work) is unchanged in structure. The leaves it iterates over may now include any Terminal admitted by miniscript's type system, but the helper's depth-128 enforcement and recursive `Tag::TapTree` framing are independent of the per-leaf operator subset.

---

## §4. Decoder Design — default rejection removal

### 4.1 Current behaviour (v0.5)

Two gates:

- `decode_tap_terminal` (decode.rs ~648–731) has a catch-all match arm that returns `TapLeafSubsetViolation` for any Tag outside the Coldcard subset.
- `decode_tap_miniscript` callers (decode.rs:295 single-leaf path, decode.rs:802 multi-leaf path) explicitly call `validate_tap_leaf_subset(&leaf, Some(index))` after the AST is reconstructed.

### 4.2 v0.6 behaviour

- Remove the catch-all rejection arm from `decode_tap_terminal`. Replace with full coverage of every Tag that a tap-context Terminal can carry. Add a `Tag::SortedMultiA` arm symmetric with `Tag::MultiA`.
- Remove the explicit `validate_tap_leaf_subset` calls at decode.rs:295 and decode.rs:802.
- The `Tag::TapTree` depth-128 ceiling enforcement in `decode_tap_subtree` remains untouched — it's BIP 341 consensus, not signer policy.

### 4.3 New tap-context Tag arms (substantive new code)

**Audit (per round-1 review).** The current `decode_tap_terminal` (decode.rs:626-730) covers only the Phase D Coldcard subset (`PkK`/`PkH`/`MultiA`/`Older`/`AndV`/`OrD`/`Check`/`Verify`) plus a defensive `Tag::TapTree` rejection arm and a catch-all returning `TapLeafSubsetViolation`. **Every other Tag listed below is absent from `decode_tap_terminal` and must be added — ~20 new arms.** This is real new code, not "remove the catch-all". The Segwitv0 dispatcher `decode_terminal` (decode.rs:324-583) has all of these arms; the implementations can be largely copy-adapted (read the same payload format, recurse via `decode_tap_miniscript` instead of `decode_miniscript`, return `Terminal<_, Tap>` instead of `Terminal<_, Segwitv0>`).

**Add/Keep checklist** (use as a flat to-do during implementation):

| Tag | Status | Terminal | Notes |
|---|---|---|---|
| `False` | **ADD** | `Terminal::False` | No payload |
| `True` | **ADD** | `Terminal::True` | No payload |
| `PkK` | KEEP | `Terminal::PkK(key)` | Existing arm |
| `PkH` | KEEP | `Terminal::PkH(key)` | Existing arm |
| `RawPkH` | **ADD** | `Terminal::RawPkH(hash160)` | Read 20-byte payload |
| `Multi` | **ADD** | `Terminal::Multi(thresh)` | Read `[k][n][key_1]...[key_n]`; tap-illegal by miniscript typing — adding the arm completes the exhaustive match symmetric with the encoder, but in practice rust-miniscript refuses to construct this Terminal in a Tap context, so the arm is unreachable at runtime via parsed inputs. Surfacing the wire byte if encountered is consistent with §3.2 option (a). |
| `SortedMulti` | **ADD** | `Terminal::SortedMulti(thresh)` | Same shape as `Multi`; same tap-illegal note |
| `MultiA` | KEEP | `Terminal::MultiA(thresh)` | Existing arm |
| `SortedMultiA` (NEW) | **ADD** | `Terminal::SortedMultiA(thresh)` | Read `[k][n][key_1]...[key_n]` like `MultiA`; construct `Threshold` with sorted-multisig discipline |
| `Alt` | **ADD** | `Terminal::Alt(X)` | Single recursive child via `decode_tap_miniscript` |
| `Swap` | **ADD** | `Terminal::Swap(X)` | Single recursive child |
| `Check` | KEEP | `Terminal::Check(X)` | Existing arm |
| `DupIf` | **ADD** | `Terminal::DupIf(X)` | Single recursive child |
| `Verify` | KEEP | `Terminal::Verify(X)` | Existing arm |
| `NonZero` | **ADD** | `Terminal::NonZero(X)` | Single recursive child |
| `ZeroNotEqual` | **ADD** | `Terminal::ZeroNotEqual(X)` | Single recursive child |
| `AndV` | KEEP | `Terminal::AndV(X, Y)` | Existing arm |
| `AndB` | **ADD** | `Terminal::AndB(X, Y)` | Two recursive children |
| `AndOr` | **ADD** | `Terminal::AndOr(X, Y, Z)` | Three recursive children |
| `OrB` | **ADD** | `Terminal::OrB(X, Z)` | Two recursive children |
| `OrC` | **ADD** | `Terminal::OrC(X, Z)` | Two recursive children |
| `OrD` | KEEP | `Terminal::OrD(X, Z)` | Existing arm |
| `OrI` | **ADD** | `Terminal::OrI(X, Z)` | Two recursive children |
| `Thresh` | **ADD** | `Terminal::Thresh(thresh)` | Read `[k][n][X_1]...[X_n]` |
| `After` | **ADD** | `Terminal::After(lock)` | Read varint, construct AbsLockTime per the existing `decode_terminal` pattern at decode.rs:481-490 |
| `Older` | KEEP | `Terminal::Older(lock)` | Existing arm |
| `Sha256` | **ADD** | `Terminal::Sha256(hash)` | Read 32-byte payload; `bitcoin::hashes::sha256::Hash::from_byte_array` (internal byte order — matches encoder; see §6.3 byte-order note) |
| `Hash256` | **ADD** | `Terminal::Hash256(hash)` | Read 32-byte payload; `miniscript::hash256::Hash::from_byte_array` (internal byte order, NOT reversed-display-order — matches encoder.rs:316-319) |
| `Ripemd160` | **ADD** | `Terminal::Ripemd160(hash)` | Read 20-byte payload; `bitcoin::hashes::ripemd160::Hash::from_byte_array` |
| `Hash160` | **ADD** | `Terminal::Hash160(hash)` | Read 20-byte payload; `bitcoin::hashes::hash160::Hash::from_byte_array` |

**Total: 8 KEEP + 20 ADD = 28 arms** in the post-rework `decode_tap_terminal`. The catch-all becomes the standard "Tag valid in some context but not here" rejection (e.g., a top-level descriptor tag like `Tag::Wsh` showing up where a tap leaf is expected); per §3.2 option (a), in-Tag-set tap-illegal Terminals (Multi/SortedMulti) get explicit arms rather than catch-all rejection.

---

## §5. Error type — `TapLeafSubsetViolation` renamed to `SubsetViolation`

### 5.1 Variant rename

Per the round-1 review (IMP-6 / nice-to-have folded inline), `Error::TapLeafSubsetViolation { operator: String, leaf_index: Option<usize> }` is **renamed** to `Error::SubsetViolation { operator: String, leaf_index: Option<usize> }`. Field shape unchanged.

Rationale: the variant name `TapLeafSubsetViolation` presumes Tap-context, but the explicit-call validator infrastructure (`validate_tap_leaf_subset` retained as `pub fn`) could plausibly be extended to Segwitv0 subsets later. Pre-1.0 + breaking-release boundary makes this rename cheap.

### 5.2 Rustdoc update

The renamed variant's rustdoc:

> Raised by explicit `validate_tap_leaf_subset` invocations (and the future opt-in `EncodeOptions::with_signer_subset(...)` API) when a wallet-policy AST contains an operator outside the caller's signer subset. v0.5 produced this error during default encode/decode paths; v0.6 retains the variant for opt-in validator paths only. The variant name was renamed from `TapLeafSubsetViolation` to `SubsetViolation` in v0.6 to allow future Segwitv0 subset use without further rename.

### 5.3 Use sites

In v0.5, `TapLeafSubsetViolation` was raised at three sites:
- `encode.rs` — encoder default-path call to `validate_tap_leaf_subset` (REMOVED per §3.2)
- `encode.rs` — `encode_tap_terminal` catch-all (REMOVED per §3.2)
- `decode.rs:295` and `decode.rs:802` — explicit calls after AST reconstruction (REMOVED per §4.2)

In v0.6, `SubsetViolation` is raised only by:
- The retained `pub fn validate_tap_leaf_subset` and `pub fn validate_tap_leaf_terminal` when explicit-called.
- (Future) the `EncodeOptions::with_signer_subset(...)` opt-in API path; deferred to `md-signer-compat-checker-separate-library`.

---

## §6. Test Corpus

### 6.1 Positive vectors — newly admitted shapes

Add to `crates/md-codec/src/vectors.rs` (`CORPUS_FIXTURES` or its v0.6 equivalent). The corpus aims for at least one round-trip fixture per newly-admitted Terminal variant so the v0.6 byte form is locked for every operator the strip newly admits.

**Centerpiece + Ledger/Coldcard documented shapes (10):**

| Fixture id | Policy | Purpose |
|---|---|---|
| `tr_sortedmulti_a_2of3_md_v0_6` | `tr(@0/**, sortedmulti_a(2, @1/**, @2/**, @3/**))` | New SortedMultiA Tag round-trip; tap-leaf bare form |
| `tr_thresh_in_tap_leaf_md_v0_6` | `tr(@0/**, thresh(2, pk(@1/**), s:pk(@2/**), s:pk(@3/**)))` | `thresh` + `s:` wrapper in tap leaf — signer-permissive shape |
| `tr_or_b_in_tap_leaf_md_v0_6` | `tr(@0/**, or_b(pk(@1/**), s:pk(@2/**)))` | `or_b` + `s:` wrapper |
| `tr_sha256_htlc_md_v0_6` | `tr(@0/**, and_v(v:sha256(0xdead...beef), pk(@1/**)))` | Hash terminal in tap leaf |
| `tr_after_absolute_height_md_v0_6` | `tr(@0/**, and_v(v:multi_a(2, @1/**, @2/**), after(700000)))` | Absolute-height locked multisig (Ledger compound shape) |
| `tr_after_absolute_time_md_v0_6` | `tr(@0/**, and_v(v:multi_a(2, @1/**, @2/**), after(1734567890)))` | Absolute-time locked multisig (Ledger compound shape) |
| `tr_older_relative_time_md_v0_6` | `tr(@0/**, and_v(v:multi_a(2, @1/**, @2/**), older(4194305)))` | Relative-time locked multisig (Ledger compound shape; `older` was admitted but not exercised in this compound) |
| `tr_pkh_in_tap_leaf_md_v0_6` | `tr(@0/**, and_v(v:pkh(@1/**), older(144)))` | `pkh()` desugars to `c:pk_h()` and round-trips today; locks the round-trip |
| `tr_multi_leaf_with_sortedmulti_a_md_v0_6` | `tr(@0/**, {sortedmulti_a(2, @1/**, @2/**), pk(@3/**)})` | `sortedmulti_a` inside multi-leaf TapTree (Coldcard's documented shape) |
| `tr_complex_recovery_path_md_v0_6` | `tr(@0/**, {and_v(v:pkh(@1/**), older(1000)), pk(@2/**)})` | Coldcard's documented recovery-path shape |

**Per-Terminal coverage (8 — folded in from round-1 review IMP-5):**

| Fixture id | Policy | Locks byte form for |
|---|---|---|
| `tr_andor_in_tap_leaf_md_v0_6` | `tr(@0/**, andor(pk(@1/**), pk(@2/**), pk(@3/**)))` | `andor` (3 children) |
| `tr_or_c_in_tap_leaf_md_v0_6` | `tr(@0/**, or_c(pk(@1/**), v:pk(@2/**)))` | `or_c` |
| `tr_or_i_in_tap_leaf_md_v0_6` | `tr(@0/**, or_i(pk(@1/**), pk(@2/**)))` | `or_i` |
| `tr_hash256_htlc_md_v0_6` | `tr(@0/**, and_v(v:hash256(0xdead...beef), pk(@1/**)))` | `hash256` (locks internal-byte-order encoding per §6.3) |
| `tr_ripemd160_htlc_md_v0_6` | `tr(@0/**, and_v(v:ripemd160(0xdeadbeef...), pk(@1/**)))` | `ripemd160` |
| `tr_hash160_htlc_md_v0_6` | `tr(@0/**, and_v(v:hash160(0xdeadbeef...), pk(@1/**)))` | `hash160` |
| `tr_a_wrapper_in_tap_leaf_md_v0_6` | `tr(@0/**, and_b(pk(@1/**), a:pk(@2/**)))` | `a:` wrapper (Tag::Alt) |
| `tr_d_wrapper_in_tap_leaf_md_v0_6` | `tr(@0/**, andor(pk(@1/**), pk(@2/**), d:older(144)))` | `d:` wrapper (Tag::DupIf); also locks `andor` further |
| `tr_j_wrapper_in_tap_leaf_md_v0_6` | (TBD — `j:` requires a `B`-type child returning nonzero; pattern TBD during implementation) | `j:` wrapper (Tag::NonZero) |
| `tr_n_wrapper_in_tap_leaf_md_v0_6` | (TBD — similar typing constraint on `n:`) | `n:` wrapper (Tag::ZeroNotEqual) |

If `j:` and `n:` round-trip patterns are awkward to construct via `Descriptor::from_str` (BIP 388 source form often doesn't naturally produce these wrappers — the parser typically handles them via canonical-form expansion), consider hand-constructing the AST in a unit test rather than via a corpus fixture, or accept these as round-trip-tested elsewhere via the encoder's exhaustiveness check at the type level.

**Total:** 18 corpus fixtures + 2 hand-AST tests if `j:`/`n:` corpus is awkward. Every newly-admitted Terminal in §4.3's "ADD" list has at least one round-trip exercise.

### 6.3 Hash terminal byte order (per round-1 review Q3)

All four hash terminals encode their **internal** byte order (NOT reversed-display-order). For `Sha256`/`Ripemd160`/`Hash160`, internal byte order coincides with network/wire order. For `Hash256`, internal byte order is the SHA256d internal order, NOT the conventional reversed-display-order — this is documented in `crates/md-codec/src/bytecode/encode.rs:316-319` and is invariant from v0.5 to v0.6.

The new corpus vectors pin specific byte sequences for each hash terminal. Implementers regenerating fixtures must use `Hash::from_byte_array(...)` from rust-bitcoin (which interprets bytes as internal order) rather than `Hash::from_str("...")` (which interprets the canonical reversed-display-order hex string).

### 6.2 Negative vectors — flips and removals

Existing negative vectors that asserted rejection of out-of-subset operators in tap leaves are reviewed:

- **N3-N7** (`n_taptree_inner_*` family in `v0.2.json`) — these were structural rejections (e.g., `Tag::Wpkh` as a tap-leaf inner). They REMAIN — those rejections are about top-level descriptor scope, not subset enforcement.
- Any negative vector whose `expected_error_variant` is `TapLeafSubsetViolation` AND whose intent was "out-of-subset operator rejected" — these FLIP to positive vectors (the operator is now admitted).
- Any negative vector for shapes that are still genuinely invalid (BIP 341 consensus violations, malformed bytecode) — these REMAIN.

Audit table to be filled during implementation (Phase 5 of plan).

### 6.3 Removed vectors

v0.5 vector files (`v0.1.json`, `v0.2.json`) are regenerated with v0.6 bytecode. The *files* survive; their *contents* fully regenerate. SHA pins in `tests/vectors_schema.rs` update once at the v0.5.x → v0.6.0 boundary.

---

## §7. BIP draft changes

### 7.1 Section §"Taproot tree" — MUST → MAY-informational

**Replace** the existing paragraph at `bip/bip-mnemonic-descriptor.mediawiki:547`:

> Implementations supporting taproot MUST enforce the per-leaf miniscript subset constraints required by deployed hardware signers (notably Coldcard, which restricts to <code>pk</code>, <code>pk_h</code>, <code>multi_a</code>, <code>or_d</code>, <code>and_v</code>, <code>older</code> — plus the <code>c:</code> and <code>v:</code> wrappers required to spell those operators in canonical BIP 388 form — as of edge firmware). Producing a tap-miniscript wallet policy beyond this subset risks the wallet being unspendable on hardware signers.

**With:**

> Implementations MAY enforce a per-leaf miniscript subset matching their target hardware signer's documented admit list. The MD encoding format itself does not require this — see §"Signer compatibility (informational)" below for the layered-responsibility framing. Implementations SHOULD clearly document any such limitations per BIP 388 §"Implementation guidelines".

### 7.2 New section §"Signer compatibility (informational)"

Inserted after §"Taproot tree" or in a §"Implementation considerations" block:

```mediawiki
====Signer compatibility (informational)====

The MD encoding format is neutral on hardware-signer compatibility.
An MD-encoded backup of a BIP 388 wallet policy is structurally well-
formed if and only if the policy parses under BIP 388 + BIP 379;
whether the policy is signable on a particular hardware signer is a
separate concern handled by tools above and below MD.

The responsibility chain is:

# Wallet software (e.g., Sparrow, Liana, Specter, Bitcoin Core)
   constructs a BIP 388 wallet policy. The wallet should know which
   signer the policy targets and produce shapes the signer admits.
# MD encodes the policy losslessly into engravable steel-backup form.
# The user, recovering the backup, decodes the MD string back into a
   BIP 388 policy.
# The recovery wallet pairs the policy with the matching signer and
   constructs spending transactions.

If an MD-encoded backup contains operators a recovery-time signer
will not sign, recovery may require either a different signer or
manual signing infrastructure (Bitcoin Core, custom signing pipelines).

Implementations MAY provide opt-in signer-subset validators as a
layered concern. Such validators are tracked separately from this
specification — see for example the named subsets maintained for
Coldcard (firmware/edge <code>docs/taproot.md</code>) and Ledger
(<code>LedgerHQ/vanadium</code> <code>apps/bitcoin/common/src/bip388/cleartext.rs</code>).
A reference Rust implementation of the layered checker pattern is
the proposed <code>md-signer-compat</code> crate.
```

### 7.3 Tag table updates

The BIP draft Tag table at `bip/bip-mnemonic-descriptor.mediawiki:371-453` updates to the v0.6 layout per §2.2. Specifically:

- **Drop** the `Tag::Bare` row (was 0x07).
- **Add** the `Tag::SortedMultiA` row at 0x0B with description "<code>sortedmulti_a(k, ...)</code> — Tapscript sorted multisig (BIP 388 / rust-miniscript extension; not in BIP 379)".
- **Renumber** every operator from 0x07 onward per §2.3's table.
- **Drop** the 14 `Reserved*` rows for 0x24–0x31.

The prose paragraph at `bip/bip-mnemonic-descriptor.mediawiki:455` (currently "Tags 0x24–0x31 are reserved by descriptor-codec for inline-key forms; v1+ may expose them for foreign-xpub support if/when WDM extends beyond pure BIP-388") is **rewritten** to:

> Tags 0x24–0x31 are unallocated. (In MD v0.5 and earlier, these bytes were reserved for descriptor-codec inline-key compatibility; MD v0.6 dropped them since MD's BIP-388 wallet-policy framing forbids inline keys. See the project's `MD_SCOPE_DECISION_2026-04-28.md` design document for rationale.)

Tag 0x32 (formerly Placeholder; now unallocated) and Tag 0x34 (reserved-invalid) get a one-line note each:

> Tag 0x32 is unallocated in v0.6. (In v0.5 it was the Placeholder framing tag; v0.6 moves Placeholder to 0x33 and intentionally leaves 0x32 vacant so any v0.5→v0.6 transcoding mistake surfaces as a clean `from_byte=None` error rather than data corruption.)

> Tag 0x34 is reserved-invalid (no allocation; preserved across v0.5→v0.6).

Tag 0x36+ continues to read "reserved" (no change).

### 7.4 Status header

The BIP draft `Status:` field stays unchanged (`Pre-Draft, AI + reference implementation, awaiting human review`) unless the implementation reveals reasons to revise.

---

## §8. README + CLI changes

### 8.1 Top-level README (`README.md`)

Add to the "What is MD?" section a brief paragraph:

> MD is a wire format for engravable backups of BIP 388 wallet policies. It is *neutral* on hardware-signer compatibility — whether a given encoded policy is signable on a given signer is a separate concern handled by your wallet software and your signer's firmware. See the BIP draft §"Signer compatibility (informational)" for the responsibility chain.

### 8.2 Crate README (`crates/md-codec/README.md`)

Identical paragraph plus (if a "Limitations" or "Caveats" section exists) a sentence reaffirming that callers wanting signer-aware validation should invoke `validate_tap_leaf_subset` explicitly.

### 8.3 CLI help (`crates/md-codec/src/bin/md/main.rs`)

`md encode --help` long-form gains a one-line warning:

> WARNING: This tool encodes any BIP 388 wallet policy. It does not check whether the policy is signable on any particular hardware wallet — that is your responsibility. See the project README for details.

---

## §9. Migration considerations

### 9.1 Breaking changes summary (v0.5.x → v0.6.0)

For inclusion in `MIGRATION.md`:

1. **Tag-space rework**: every tap-leaf-bearing bytecode regenerates. Consumers depending on specific Tag byte values (e.g., parsing MD bytecode in non-Rust tooling) need to update their Tag tables to the v0.6 layout per §2.2/§2.3.
2. **Validator default flip**: encoder/decoder no longer raise `SubsetViolation` (formerly `TapLeafSubsetViolation`) by default. Callers depending on this rejection for safety must invoke `validate_tap_leaf_subset` explicitly.
3. **`DecodedString.data` field removed** (already shipped in `d79125d`; reaffirm in MIGRATION).
4. **`Reserved*` Tag variants removed**: any code matching on the 14 `ReservedOrigin`/`ReservedNoOrigin`/etc. variants will not compile under v0.6. These bytes (0x24–0x31) now return `None` from `Tag::from_byte`. Code that defensively matched these to error out can simply rely on the `None` arm; the compile error catches the safety concern.
5. **`Tag::Bare` variant removed**: any code matching on `Tag::Bare` will not compile under v0.6. The byte 0x07 is reused for `TapTree`. `Descriptor::Bare` continues to be rejected at the encoder via `PolicyScopeViolation` (unchanged behaviour).
6. **`Error::TapLeafSubsetViolation` renamed to `Error::SubsetViolation`**: any code matching on the old variant name will not compile under v0.6. Field shape is unchanged (`{ operator: String, leaf_index: Option<usize> }`); only the variant name changes. Sed `s/TapLeafSubsetViolation/SubsetViolation/g` over consumer code is sufficient.
7. **Tag enum is `#[non_exhaustive]`**: new variants in v0.6 (`SortedMultiA`) are additive in spirit, but the Tag *byte values* of existing variants change. Any code matching by byte value rather than enum variant must be updated.
8. **v0.5.x → v0.6.0 wire-format break**: v0.5.x-encoded MD strings are NOT decodable under v0.6 (different Tag bytes for almost every tap-leaf operator). v0.6 is a clean break. No deprecation cycle, no v0.5-compat decoder shim — pre-1.0 + no users yet.

### 9.2 Wire-format compatibility

v0.5.x-encoded MD strings are NOT decodable under v0.6 (different Tag bytes for almost every operator). v0.6 is a clean break.

### 9.3 No deprecation cycle

Pre-1.0 + no users: clean break is appropriate. No `#[deprecated]` shims, no v0.5-compat decoder.

---

## §10. Family-stable SHA reset

`GENERATOR_FAMILY` rolls `"md-codec 0.5"` → `"md-codec 0.6"` in `crates/md-codec/src/vectors.rs`. The mechanism is identical to v0.4→v0.5 transition.

After regeneration:
- `v0.1.json` SHA changes once.
- `v0.2.json` SHA changes once.
- v0.6.x patch line stable thereafter (per the family-stable promise mechanism inherited from `vectors-generator-string-patch-version-churn` resolution).

---

## §11. Acceptance criteria

The v0.6.0 release is acceptance-ready when:

1. **Compile + lint**: `cargo check`, `cargo clippy --all-targets`, and `cargo fmt --check` are clean.
2. **Tests**: `cargo test -p md-codec` passes (both unit + integration). Doctest passes.
3. **Rustdoc CI**: `RUSTDOCFLAGS="-D warnings" cargo doc -p md-codec --no-deps` clean.
4. **Vector verification**: `cargo run --bin gen_vectors -- --verify crates/md-codec/tests/vectors/v0.1.json` and the v0.2.json equivalent pass (both regenerated and SHA-pinned to v0.6 values).
5. **Round-trip suite**: every fixture in `CORPUS_FIXTURES` round-trips byte-identically (encode → decode → re-encode equals original).
6. **BIP draft**: §"Taproot tree" and §"Signer compatibility (informational)" rendered correctly; Tag table at lines 371-453 matches §2.2 final layout; the `Reserved*` paragraph at line 455 is rewritten per §7.3; the `Tag::Bare` row is dropped; the `Tag::SortedMultiA` row is added.
7. **CHANGELOG + MIGRATION**: `[0.6.0]` section in CHANGELOG; `v0.5.x → v0.6.0` section in MIGRATION covering all 8 breaking-changes items in §9.1.
8. **Phase D agent report forward-pointer**: already added (commit `93ac9ae`).
9. **Reconciliation**: every implementation-phase agent report in `design/agent-reports/` accounted for; nits/nice-to-haves filed in FOLLOWUPS, criticals/importants addressed inline, blockers escalated.
10. **Error coverage**: `tests/error_coverage.rs` (the exhaustiveness CI gate) passes with the renamed `Error::SubsetViolation`. The strum::EnumIter mirror table in that file must be updated to match.
11. **No regression on the v0.6 wire-format invariants**: per-operator round-trip tests (one per newly-admitted Terminal in §6.1) all pass. The bare-form `tr_sortedmulti_a_2of3_md_v0_6` fixture serves as the canonical SortedMultiA Tag byte-form lock.

---

## §12. Open questions — resolved by round-1 review

All five questions resolved by the spec reviewer (agent report at `design/agent-reports/v0-6-spec-review-1.md`). Resolutions folded inline into §§2-11; left here as a quick-reference closure log.

1. **Tag layout finalization.** RESOLVED — §2.2 layout endorsed: `TapTree` at 0x07 (adjacent to `Tr=0x06`); multisig family contiguous at 0x08–0x0B; wrappers at 0x0C–0x12; logical operators at 0x13–0x1A; `Fingerprints=0x35` retained for v0.2-shipped wire-byte continuity. Rationale notes added inline in §2.2.

2. **`encode_tap_terminal` catch-all behaviour.** RESOLVED — option (a): exhaustive match emitting wire bytes for all in-Tag-set Terminals (including tap-illegal `Multi`/`SortedMulti`). Compiler-checked exhaustiveness; future miniscript upgrades that add Terminal variants will force re-evaluation. Detail in §3.2.

3. **Hash terminal byte order.** RESOLVED — all four hash terminals encode internal byte order (NOT reversed-display-order); invariant from v0.5. Detail in §6.3 with citation to `encode.rs:316-319`.

4. **`Tag::Bare` retention.** RESOLVED — DROP `Tag::Bare` entirely. The variant is dead weight (encoder rejects `Descriptor::Bare` via a different path). Byte 0x07 is reused for `TapTree`. Implementation removes the `tag_to_bip388_name` `"bare"` arm. Detail in §2.2/§2.3.

5. **`p2-inline-key-tags` BIP draft cleanup.** RESOLVED — rewrite, don't remove entirely. The BIP draft's `Reserved*` paragraph at line 455 gets a one-line historical-orientation note pointing readers at `MD_SCOPE_DECISION_2026-04-28.md` for full rationale. Exact text pinned in §7.3.

---

(End of v0.6 strip-Layer-3 spec. Implementation plan: [`IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md`](./IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md).)
