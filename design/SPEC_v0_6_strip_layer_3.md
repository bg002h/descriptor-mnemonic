# v0.6 Design Spec: Strip Layer 3 (signer-compatibility curation)

**Status:** Draft (2026-04-28)
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

// Bare top-level (1)
0x32  Bare                        // bare(SCRIPT) — top-level only; rejected by encoder per scope (kept allocated for completeness)

// MD-specific framing (3)
0x33  Placeholder                 // @i — placeholder for key-info-vector index
0x34  SharedPath                  // shared-path declaration for placeholder framing
0x35  Fingerprints                // fingerprints block (Phase E v0.2)

// 0x36–0xFF: unallocated (return None from from_byte)
```

### 2.3 Byte-for-byte changes from v0.5

Every existing operator's byte changes. Worst case: every fixture in v0.1.json + v0.2.json regenerates with new bytecode. Selected before/after pairs:

| Operator | v0.5 Tag | v0.6 Tag |
|---|---|---|
| `False` | 0x00 | 0x00 (unchanged) |
| `True` | 0x01 | 0x01 (unchanged) |
| `Pkh` | 0x02 | 0x02 (unchanged) |
| `Sh` | 0x03 | 0x03 (unchanged) |
| `Wpkh` | 0x04 | 0x04 (unchanged) |
| `Wsh` | 0x05 | 0x05 (unchanged) |
| `Tr` | 0x06 | 0x06 (unchanged) |
| `Bare` | 0x07 | 0x32 (moved to high range) |
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
| `Reserved*` 0x24–0x31 | 14 variants | (dropped — `from_byte` returns `None`) |
| `Placeholder` | 0x32 | 0x33 |
| `SharedPath` | 0x33 | 0x34 |
| `Fingerprints` | 0x35 | 0x35 (unchanged) |

Note: `0x07 ↔ 0x08` swap (Bare ↔ TapTree) and `0x08 ↔ 0x19` rotation (Multi adjacent to SortedMulti, then MultiA, then SortedMultiA new) are the structural changes. Wrappers and logical operators shift by 2 bytes (0x0A→0x0C etc.) because the multisig family expanded into the 0x08-0x0B range. Constants/top-level/keys/timelocks/hashes/Fingerprints stay byte-identical.

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

`validate_tap_leaf_subset` and `validate_tap_leaf_terminal` remain `pub fn` in the same file. They can be called explicitly by consumer code that wants signer-aware validation. Their rustdoc updates to make this plain.

`encode_tap_terminal`'s catch-all is broadened to dispatch any in-Tag-set Terminal variant. The defensive arm that produced `TapLeafSubsetViolation` is removed (the unconditional Tag emission handles it; type errors surface via miniscript's own type system at parse time).

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

### 4.3 New tap-context Tag arms

The decoder gains explicit handling for tap-context Tags previously rejected:

| Tag | Terminal | Notes |
|---|---|---|
| `SortedMultiA` (NEW) | `Terminal::SortedMultiA(thresh)` | Read `[k][n][key_1]...[key_n]` like `MultiA`; construct `Threshold` with sorted-multisig discipline |
| `Sha256` | `Terminal::Sha256(hash)` | Read 32-byte payload |
| `Hash256` | `Terminal::Hash256(hash)` | Read 32-byte payload |
| `Ripemd160` | `Terminal::Ripemd160(hash)` | Read 20-byte payload |
| `Hash160` | `Terminal::Hash160(hash)` | Read 20-byte payload |
| `After` | `Terminal::After(lock)` | Read varint, construct AbsLockTime |
| `AndB` | `Terminal::AndB(X, Y)` | Two recursive children |
| `AndOr` | `Terminal::AndOr(X, Y, Z)` | Three recursive children |
| `OrB` | `Terminal::OrB(X, Z)` | Two recursive children |
| `OrC` | `Terminal::OrC(X, Z)` | Two recursive children |
| `OrI` | `Terminal::OrI(X, Z)` | Two recursive children |
| `Thresh` | `Terminal::Thresh(thresh)` | Read `[k][n][X_1]...[X_n]` |
| `Alt`/`Swap`/`DupIf`/`NonZero`/`ZeroNotEqual` | wrapper terminals | Single recursive child |

Most of these decoder arms ALREADY EXIST in `decode_tap_terminal` from Phase D (the encoder/decoder symmetry was implemented at Phase D time even though `validate_tap_leaf_subset` was the gate). Audit during implementation confirms which arms need adding versus only the catch-all needing removal.

---

## §5. Error type — `TapLeafSubsetViolation` retained

### 5.1 No structural change

`Error::TapLeafSubsetViolation { operator: String, leaf_index: Option<usize> }` (variant `#[non_exhaustive]`) stays as-is. It's no longer raised by the default encoder/decoder paths but is the canonical error variant for explicit `validate_tap_leaf_subset(..)` calls.

### 5.2 Rustdoc update

The variant's rustdoc clarifies:

> Raised by explicit `validate_tap_leaf_subset` invocations when a tap-leaf miniscript contains an operator outside the caller's signer subset. v0.5 produced this error during default encode/decode paths; v0.6 retains the variant for opt-in validator paths only.

---

## §6. Test Corpus

### 6.1 Positive vectors — newly admitted shapes

Add to `crates/md-codec/src/vectors.rs` (`CORPUS_FIXTURES` or its v0.6 equivalent):

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

The BIP draft's Tag table (location TBD by audit during implementation; likely §"Bytecode" or similar) updates to the v0.6 layout per §2.2.

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

1. **Tag-space rework**: every existing tap-leaf bytecode regenerates. Consumers depending on specific Tag byte values (e.g., parsing MD bytecode in non-Rust tooling) need to update their Tag tables to v0.6.
2. **Validator default flip**: encoder no longer raises `TapLeafSubsetViolation` by default. Callers depending on this rejection for safety must invoke `validate_tap_leaf_subset` explicitly.
3. **`DecodedString.data` field removed** (already shipped in `d79125d`; reaffirm in MIGRATION).
4. **`Reserved*` Tag variants removed**: any code matching on the 14 `ReservedOrigin`/`ReservedNoOrigin`/etc. variants will not compile under v0.6. These bytes (0x24–0x31) now return `None` from `Tag::from_byte`.
5. **Tag enum is `#[non_exhaustive]`**: new variants in v0.6 are additive in spirit, but the Tag *byte values* of existing variants change. Any code matching by byte value rather than enum variant must be updated.

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
6. **BIP draft**: §"Taproot tree" and §"Signer compatibility (informational)" rendered correctly; Tag table matches §2.2 final layout.
7. **CHANGELOG + MIGRATION**: `[0.6.0]` section in CHANGELOG; `v0.5.x → v0.6.0` section in MIGRATION covering all 5 breaking-changes items in §9.1.
8. **Phase D agent report forward-pointer**: already added (commit `93ac9ae`).
9. **Reconciliation**: every implementation-phase agent report in `design/agent-reports/` accounted for; nits/nice-to-haves filed in FOLLOWUPS, criticals/importants addressed inline, blockers escalated.

---

## §12. Open questions for review

These are flagged for the spec-review agent to either confirm, reject, or refine:

1. **Tag layout finalization.** §2.2 proposes one specific reorganization. Is the chosen layout optimal (e.g., should `TapTree` stay at 0x08 for proximity to `Tr=0x06` rather than moving to 0x07)? Should `Bare` survive at all if it's still rejected by the encoder?

2. **`encode_tap_terminal` catch-all behaviour.** v0.5 returned `TapLeafSubsetViolation` for unmatched arms. v0.6 needs to either (a) be exhaustive over all in-Tag-set Terminals (preferred — exhaustiveness checked by the compiler), or (b) keep a catch-all that returns a different error variant (`UnsupportedOperator` or similar). §3.2 leans (a). Confirm or refine.

3. **Hash terminal byte order in `Sha256(h)` etc.** Is the existing 32-byte payload encoding little-endian, big-endian, or raw-network-order? Need to audit during implementation; spec leaves this to the implementation phase since the byte order is invariant from v0.5.

4. **`Tag::Bare` retention.** Currently allocated; encoder rejects `Descriptor::Bare` per scope. Should v0.6 drop `Tag::Bare` entirely (since it's never emitted)? Or keep allocated for future bare-descriptor support? §2.2 keeps it; revisit if review prefers dropping.

5. **`p2-inline-key-tags` Resolved status timing.** The entry was flipped to `wont-fix — out of scope per design` ahead of this spec landing. Should the BIP draft's existing `Reserved*` discussion be removed entirely, or kept with a "for historical context" note? §7 implicitly removes it; confirm.

---

(End of v0.6 strip-Layer-3 spec. Implementation plan: [`IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md`](./IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md).)
