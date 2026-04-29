# v0.7.0 Design Spec — test rebaseline + defensive corpus + md-signer-compat + policy compiler

**Status:** Draft (2026-04-29)
**Companion documents:**
- v0.6 rationale: [`MD_SCOPE_DECISION_2026-04-28.md`](./MD_SCOPE_DECISION_2026-04-28.md)
- v0.6 spec: [`SPEC_v0_6_strip_layer_3.md`](./SPEC_v0_6_strip_layer_3.md)
- Implementation plan: [`IMPLEMENTATION_PLAN_v0_7_0.md`](./IMPLEMENTATION_PLAN_v0_7_0.md)
- FOLLOWUPS source-of-truth: [`FOLLOWUPS.md`](./FOLLOWUPS.md)

---

## §1. Scope and Goals

v0.7.0 is the first post-strip-Layer-3 release. It bundles four tracks:

### 1.1 Track A: test rebaseline (urgent)

v0.6.0 shipped with 38 unit tests pinning v0.5 byte literals (e.g., `vec![0x05, 0x16, 0x00, 0x01]` for `[Wsh, OrD, False, True]` where `OrD = 0x16` in v0.5 / `0x18` in v0.6). Tests are semantically correct; only literals are stale. v0.7.0 rebaselines all 38 tests, preferring symbolic `Tag::Foo.as_byte()` references where practical so future Tag changes don't re-break.

Closes FOLLOWUPS: `v06-test-byte-literal-rebaseline`.

### 1.2 Track B: defensive corpus growth

Three FOLLOWUPS entries deferred during v0.6 execution:
- `v06-corpus-or-c-coverage` — V-typing constraint forbids unwrapped `or_c` as top-level tap leaf.
- `v06-corpus-d-wrapper-coverage` — `d:` requires Vz-type child; `older` is B-type.
- `v06-corpus-j-n-wrapper-coverage` — `j:` and `n:` require specific child types.
- `v06-corpus-byte-order-defensive-test` — hand-pinned hash byte-order test (defensive against the round-trip-stable-but-format-changed regression class).

These all fall back to **hand-constructed AST tests** in `tests/taproot.rs` rather than corpus fixtures (the typing constraints make BIP 388 source-form policies awkward; hand-AST tests via `Miniscript::from_ast` lock the wire-byte form directly).

Closes FOLLOWUPS: `v06-corpus-or-c-coverage`, `v06-corpus-d-wrapper-coverage`, `v06-corpus-j-n-wrapper-coverage`, `v06-corpus-byte-order-defensive-test`, `v06-plan-targeted-decoder-arm-tests`.

### 1.3 Track C: md-signer-compat new workspace crate

Per FOLLOWUPS `md-signer-compat-checker-separate-library` (chosen scope **3a in-workspace** per user direction): new crate at `crates/md-signer-compat/` housing named signer subsets for opt-in caller-driven validation.

**Crate scope:**
- `pub struct SignerSubset { name: &'static str, allowed_operators: &'static [&'static str] }` — subset definition.
- `pub const COLDCARD_TAP: SignerSubset` — Coldcard's documented tap-leaf admit list (pk/pk_h/multi_a/or_d/and_v/older + c:/v:/sortedmulti_a per the firmware/edge `docs/taproot.md` source).
- `pub const LEDGER_TAP: SignerSubset` — Ledger's documented set (per `vanadium/apps/bitcoin/common/src/bip388/cleartext.rs` first-class compound shapes).
- `pub fn validate(subset: &SignerSubset, ms: &Miniscript<DescriptorPublicKey, Tap>, leaf_index: Option<usize>) -> Result<(), md_codec::Error>` — invokes md-codec's retained `validate_tap_leaf_subset` infrastructure with the named subset's operator allowlist.

**Vendor-citation discipline:** every named subset value carries an inline citation comment: source URL + source SHA + last-checked date. Updates to the subset (vendor doc revisions) bump the crate's patch version and refresh the citation.

Closes FOLLOWUPS: `md-signer-compat-checker-separate-library`.

### 1.4 Track D: policy compiler wrapper API

Per FOLLOWUPS `md-policy-compiler-feature` (chosen scope **4b wrapper API** per user direction): enable rust-miniscript's `compiler` feature and expose a one-shot wrapper.

**API surface (in md-codec):**

```rust
/// Parse a high-level Concrete Policy expression, compile it to optimal
/// miniscript, and encode the result as MD bytecode.
///
/// This is a convenience wrapper around the rust-miniscript policy
/// compiler. Available only when the `compiler` cargo feature is enabled
/// (default-off; opt-in to avoid the ~30-feature-gate code surface in
/// rust-miniscript that this requires).
///
/// # Errors
///
/// - `Error::PolicyParse` if the policy string fails BIP 380 / Concrete
///   Policy parsing.
/// - `Error::Miniscript` if the compiler can't satisfy the policy under
///   the chosen script context.
/// - Any encode-side error from `to_bytecode`.
#[cfg(feature = "compiler")]
pub fn policy_to_bytecode(
    policy: &str,
    options: &EncodeOptions,
    script_context: ScriptContext,
) -> Result<Vec<u8>, Error>
```

`ScriptContext` is a thin enum (`Segwitv0` | `Tap`) that selects which miniscript context the compiler targets. CLI gains `md encode --from-policy <policy> --context tap` mode.

Closes FOLLOWUPS: `md-policy-compiler-feature`.

### 1.5 What does NOT change

- Wire format: NO byte-level changes from v0.6.0. Tag layout unchanged. v0.6.x-encoded MD strings round-trip byte-identically through v0.7.0.
- Top-level descriptor admit set: unchanged.
- `validate_tap_leaf_subset` retained as `pub fn` in md-codec (its operator allowlist is now caller-supplied via `SignerSubset` from md-signer-compat; the v0.6 hardcoded Coldcard subset stays as the `validate_tap_leaf_subset` default for backwards-compat, but new caller-driven validation uses the named subsets).
- Family-stable promise: v0.7.0.x patches preserve byte-stable SHAs. `GENERATOR_FAMILY` rolls `"md-codec 0.6"` → `"md-codec 0.7"` — corpus regen at the v0.6.x → v0.7.0 boundary expected to produce SHA changes only from the family-token roll, not from any wire-format changes (assuming no corpus content additions in this release; defensive corpus is hand-AST tests, not corpus fixtures).

---

## §2. Track A — Test rebaseline

### 2.1 Inventory

The 38 failing tests fall into ~5 categories:

| Category | Count | Pattern |
|---|---|---|
| `bytecode::decode::tests::*_known_vector` | ~10 | `let bytes = vec![0x05, 0x16, ...]` — pin v0.5 OrD=0x16, AndOr=0x13, etc. |
| `bytecode::decode::tests::*_rejects_*` | ~8 | Same byte literals in negative-test bytecode buffers. |
| `bytecode::encode::tests::encode_terminal_*` | ~10 | `assert_eq!(out, vec![0x1B, 0x32, 0x00])` — v0.5 PkK=0x1B (unchanged) but Placeholder=0x32 (now 0x33). |
| `bytecode::path::tests::*` | ~6 | SharedPath byte (0x33→0x34) in path declaration tests. |
| `policy::tests::*` + `vectors::tests::*` | ~5 | Placeholder byte references in higher-level tests. |

Some tests are ALREADY symbolic (`Tag::Foo.as_byte()`) for SOME assertions and literal-pinned for others; rebaseline preserves the symbolic style and converts literals.

### 2.2 Rebaseline approach

For each failing test:

1. **Identify the byte literals**: typically `vec![...]` initializers for input/expected bytecode buffers.
2. **Look up v0.5→v0.6 mapping** per spec §2.3 of the v0.6 strip-layer-3 spec (already in tree).
3. **Replace literals with symbolic refs**: e.g., `0x16` → `Tag::OrD.as_byte()`. Where the byte is "the byte after the Tag" (e.g., a placeholder index of 0), keep it literal.
4. **Update inline comments** that named v0.5 byte values: e.g., `// = [0x05, 0x16, 0x00, 0x01]` → drop the explicit-byte annotation (the symbolic tags are self-documenting) OR update to v0.6 bytes.
5. **Re-run the specific test** to confirm it now passes.

The mapping table for the affected operators:

| Operator | v0.5 byte | v0.6 byte |
|---|---|---|
| `Tag::TapTree` | 0x08 | 0x07 |
| `Tag::Multi` | 0x19 | 0x08 |
| `Tag::SortedMulti` | 0x09 | 0x09 (unchanged) |
| `Tag::MultiA` | 0x1A | 0x0A |
| `Tag::SortedMultiA` | (NEW) | 0x0B |
| `Tag::Alt` | 0x0A | 0x0C |
| `Tag::Swap` | 0x0B | 0x0D |
| `Tag::Check` | 0x0C | 0x0E |
| `Tag::DupIf` | 0x0D | 0x0F |
| `Tag::Verify` | 0x0E | 0x10 |
| `Tag::NonZero` | 0x0F | 0x11 |
| `Tag::ZeroNotEqual` | 0x10 | 0x12 |
| `Tag::AndV` | 0x11 | 0x13 |
| `Tag::AndB` | 0x12 | 0x14 |
| `Tag::AndOr` | 0x13 | 0x15 |
| `Tag::OrB` | 0x14 | 0x16 |
| `Tag::OrC` | 0x15 | 0x17 |
| `Tag::OrD` | 0x16 | 0x18 |
| `Tag::OrI` | 0x17 | 0x19 |
| `Tag::Thresh` | 0x18 | 0x1A |
| `Tag::Placeholder` | 0x32 | 0x33 |
| `Tag::SharedPath` | 0x33 | 0x34 |

(Constants, top-level descriptors, keys, timelocks, hashes, Fingerprints byte-identical.)

### 2.3 Acceptance

`cargo test -p md-codec` passes 432/432 tests. The error_coverage gate stays green; the previously-failing 38 tests pass without any test deletions or `#[ignore]` markers.

---

## §3. Track B — Defensive corpus growth

### 3.1 Hand-AST tests for typing-awkward operators

Per spec §1.2 of v0.6 strip-Layer-3, BIP 388 source-form parsers reject some valid miniscript shapes due to top-level B-type requirement. Hand-AST tests bypass the parser:

```rust
// Example pattern — exact form refined during implementation
#[test]
fn or_c_tap_leaf_round_trip_via_hand_ast() {
    use std::sync::Arc;
    use miniscript::{Miniscript, Tap, Terminal};

    let key_a: DescriptorPublicKey = /* ... */;
    let key_b: DescriptorPublicKey = /* ... */;

    // Build or_c(pk_k(a), v:pk_k(b)) via from_ast (bypasses BIP 388 parser).
    let pk_a = Miniscript::from_ast(Terminal::PkK(key_a.clone())).unwrap();
    let pk_b = Miniscript::from_ast(Terminal::PkK(key_b.clone())).unwrap();
    let v_pk_b = Miniscript::from_ast(Terminal::Verify(Arc::new(pk_b))).unwrap();
    let or_c = Terminal::OrC(Arc::new(pk_a), Arc::new(v_pk_b));

    // Encode + assert byte form matches expected v0.6 byte sequence:
    // [Tag::OrC=0x17, Tag::PkK=0x1B, Placeholder, key_a_index,
    //  Tag::Verify=0x10, Tag::PkK=0x1B, Placeholder, key_b_index]
    let mut out = Vec::new();
    let map: HashMap<DescriptorPublicKey, u8> = [(key_a, 0), (key_b, 1)].into();
    or_c.encode_template(&mut out, &map).unwrap();
    assert_eq!(
        out,
        vec![
            Tag::OrC.as_byte(),
            Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 0,
            Tag::Verify.as_byte(), Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 1,
        ]
    );

    // Decode the bytecode back; assert structural round-trip via Miniscript::from_ast equality.
    // (Exact assertion shape refined during implementation; the goal is:
    //  encoded bytes round-trip back through the decoder to the same AST shape.)
}
```

Same pattern for:
- `tr_d_wrapper_hand_ast`: `d:v:older(144)` (where `v:older` is V-typed and z-typed, satisfying d:'s Vz requirement).
- `tr_j_wrapper_hand_ast`: `j:` wrapper around a B-type child satisfying the typing.
- `tr_n_wrapper_hand_ast`: `n:` wrapper similarly.

### 3.2 Hash byte-order defensive pin test

Per spec §6.3 of v0.6:

```rust
#[test]
fn hash_terminals_encode_internal_byte_order_not_display_order() {
    // Pin all four hash terminals to internal byte order (NOT reversed-display-order).
    // This catches the round-trip-stable-but-format-changed regression class
    // that the corpus alone cannot detect.
    use bitcoin::hashes::{Hash, hash160, ripemd160, sha256};
    use miniscript::hash256;

    let known_32 = [0xAAu8; 32];
    let known_20 = [0xBBu8; 20];

    // Sha256
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Sha256(sha256::Hash::from_byte_array(known_32));
    let mut out = Vec::new();
    term.encode_template(&mut out, &HashMap::new()).unwrap();
    assert_eq!(out[0], Tag::Sha256.as_byte());
    assert_eq!(&out[1..33], &known_32[..]);

    // Repeat for Hash256, Ripemd160, Hash160.
}
```

### 3.3 Per-arm decoder unit tests

Per FOLLOWUPS `v06-plan-targeted-decoder-arm-tests`: 5–7 targeted decoder unit tests that synthesize a known bytecode (Tag byte + payload), feed to `decode_tap_terminal` directly, and assert the resulting Terminal matches the expected AST shape and consumed-byte-count. Catches decoder bugs the round-trip alone cannot.

### 3.4 Acceptance

All hand-AST tests + byte-order pin test + per-arm decoder tests pass under `cargo test -p md-codec`. No corpus content additions (defensive tests live in `tests/taproot.rs` or new `tests/hand_ast_coverage.rs`).

---

## §4. Track C — md-signer-compat workspace crate

### 4.1 Crate location and structure

```
crates/
├── md-codec/                       (existing)
└── md-signer-compat/               (NEW)
    ├── Cargo.toml
    ├── README.md
    └── src/
        ├── lib.rs
        ├── coldcard.rs             (COLDCARD_TAP subset)
        ├── ledger.rs               (LEDGER_TAP subset)
        └── tests.rs
```

### 4.2 `SignerSubset` API

```rust
/// A named subset of miniscript operators a hardware signer is documented
/// to admit. Operator names match BIP 388 / BIP 379 source-form spelling
/// (e.g., "pk_k", "multi_a", "and_v", "older", "c:", "v:").
///
/// The vendor-citation comment on each subset value carries the source
/// URL, the source repo's commit SHA at the time of last verification,
/// and the verification date. Vendor doc updates → subset bump →
/// crate patch release.
#[derive(Debug, Clone)]
pub struct SignerSubset {
    /// Human-readable name (e.g., "Coldcard tap-leaf").
    pub name: &'static str,
    /// Operator names (BIP 388 source-form spellings) the signer admits.
    pub allowed_operators: &'static [&'static str],
}

/// Validate a miniscript leaf against a named signer subset.
///
/// Returns `Ok(())` if every operator in the leaf AST appears in
/// `subset.allowed_operators`. Returns
/// [`md_codec::Error::SubsetViolation`] with the offending operator
/// name and `leaf_index` on the first out-of-subset operator.
pub fn validate<C: miniscript::ScriptContext>(
    subset: &SignerSubset,
    ms: &miniscript::Miniscript<miniscript::DescriptorPublicKey, C>,
    leaf_index: Option<usize>,
) -> Result<(), md_codec::Error>;
```

### 4.3 Named subsets

```rust
// crates/md-signer-compat/src/coldcard.rs

/// Coldcard tap-leaf miniscript subset.
///
/// **Source:** `Coldcard/firmware` repo, `edge` branch, `docs/taproot.md`
/// §"Allowed descriptors". Verified at commit `<SHA>` on 2026-04-28.
/// The documented allowed-descriptors list:
///   - `tr(key)` (single-sig keypath)
///   - `tr(internal_key, sortedmulti_a(2,@0,@1))`
///   - `tr(internal_key, pk(@0))`
///   - `tr(internal_key, {sortedmulti_a(...), pk(@2)})`
///   - `tr(internal_key, {or_d(pk(@0), and_v(v:pkh(@1), older(1000))), pk(@2)})`
///
/// Operators extracted: pk_k, pk_h (via pkh sugar → c:pk_h), multi_a,
/// sortedmulti_a, or_d, and_v, older, c:, v:.
pub const COLDCARD_TAP: SignerSubset = SignerSubset {
    name: "Coldcard tap-leaf (firmware/edge as of 2026-04-28)",
    allowed_operators: &[
        "pk_k", "pk_h", "multi_a", "sortedmulti_a",
        "or_d", "and_v", "older",
        "c:", "v:",
    ],
};
```

```rust
// crates/md-signer-compat/src/ledger.rs

/// Ledger tap-leaf miniscript subset.
///
/// **Source:** `LedgerHQ/vanadium`, `apps/bitcoin/common/src/bip388/cleartext.rs`.
/// Verified at commit `<SHA>` on 2026-04-28. The variants admitted (from
/// the `cleartext.rs` enum) are:
///   - `Singlesig` (key-only tr)
///   - `SortedMultisig` (sortedmulti_a)
///   - `Multisig` (multi_a)
///   - `RelativeHeightlockMultiSig` (and_v(v:multi_a, older(n<65536)))
///   - `RelativeTimelockMultiSig` (and_v(v:multi_a, older(n in time-encoding range)))
///   - `AbsoluteHeightlockMultiSig` (and_v(v:multi_a, after(n<500_000_000)))
///   - `AbsoluteTimelockMultiSig` (and_v(v:multi_a, after(n>=500_000_000)))
///
/// Operators extracted: pk_k (single-sig keypath), multi_a, sortedmulti_a,
/// and_v, older, after, c:, v:.
pub const LEDGER_TAP: SignerSubset = SignerSubset {
    name: "Ledger tap-leaf (LedgerHQ/vanadium as of 2026-04-28)",
    allowed_operators: &[
        "pk_k", "pk_h", "multi_a", "sortedmulti_a",
        "and_v", "older", "after",
        "c:", "v:",
    ],
};
```

### 4.4 Implementation strategy

The `validate` function delegates to md-codec's retained `validate_tap_leaf_subset` infrastructure but with a configurable allowlist. Two implementation paths:

**Option A (preferred):** add a new `pub fn validate_tap_leaf_subset_with_allowlist` to md-codec that takes `&[&str]` allowlist; have md-signer-compat call it with `subset.allowed_operators`.

**Option B (alternative):** md-signer-compat re-implements the AST-walking validator from scratch.

§7.1 of this spec selects Option A; the new md-codec function is a small refactor of the existing `validate_tap_leaf_terminal`.

### 4.5 Cargo dependencies

```toml
[package]
name = "md-signer-compat"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Named signer-subset validators for Mnemonic Descriptor (MD) wallet policies"

[dependencies]
md-codec = { path = "../md-codec", version = "0.7" }
miniscript = { workspace = true }
```

### 4.6 Acceptance

- `cargo build -p md-signer-compat` clean.
- 4–6 unit tests in `crates/md-signer-compat/src/tests.rs` exercising: a happy-path Coldcard validation; a Coldcard rejection with operator name + leaf_index; a happy-path Ledger validation; a Ledger rejection.
- Optional CLI integration deferred to v0.7.x patch (not blocking ship).

---

## §5. Track D — Policy compiler wrapper API

### 5.1 Cargo feature

Add to `crates/md-codec/Cargo.toml`:

```toml
[features]
default = ["cli"]
cli = ["dep:clap", "dep:anyhow"]
compiler = ["miniscript/compiler"]    # NEW

[dependencies]
miniscript = { ..., features = ["compiler"], optional = true }
# Wait — miniscript is a hard dep, not optional.
# Better:
miniscript = { ... }   # core (no extra features)
# Then compiler feature gates code paths inside md-codec.
```

Actually the cleaner shape: enable `miniscript/compiler` via a passthrough feature:

```toml
[features]
default = ["cli"]
cli = ["dep:clap", "dep:anyhow"]
compiler = ["miniscript/compiler"]

[dependencies]
miniscript = { git = "...", rev = "..." }   # unchanged
```

The cargo `compiler = ["miniscript/compiler"]` feature pulls in the rust-miniscript compiler module via cargo's feature passthrough. Default build does NOT pull the compiler (~30 cfg-gated files in rust-miniscript).

### 5.2 Public API

```rust
/// Script context for policy compilation.
#[cfg(feature = "compiler")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptContext {
    /// Segwit v0 (`wsh()` descriptors).
    Segwitv0,
    /// Tapscript (`tr()` descriptors).
    Tap,
}

/// Parse a high-level Concrete Policy expression, compile to optimal
/// miniscript, and encode as MD bytecode.
///
/// This is a convenience wrapper around the rust-miniscript policy
/// compiler. Available only when the `compiler` cargo feature is enabled.
///
/// # Example
///
/// ```rust,no_run
/// # #[cfg(feature = "compiler")]
/// # {
/// use md_codec::{policy_to_bytecode, EncodeOptions, ScriptContext};
/// let bytecode = policy_to_bytecode(
///     "or(99@pk(@0/**),1@and(older(1000),pk(@1/**)))",
///     &EncodeOptions::default(),
///     ScriptContext::Tap,
/// )?;
/// # Ok::<(), md_codec::Error>(())
/// # }
/// ```
///
/// # Errors
///
/// - [`Error::PolicyParse`] if the policy string fails Concrete-Policy parsing.
/// - [`Error::Miniscript`] if the compiler can't satisfy the policy under the chosen context.
/// - Any error from [`crate::encode_template`] applied to the compiled miniscript.
#[cfg(feature = "compiler")]
pub fn policy_to_bytecode(
    policy: &str,
    options: &EncodeOptions,
    script_context: ScriptContext,
) -> Result<Vec<u8>, Error>;
```

### 5.3 CLI integration

`md encode --from-policy <policy> --context tap` mode:

```
USAGE:
    md encode [OPTIONS] <POLICY>
    md encode --from-policy <CONCRETE_POLICY> --context <tap|segwitv0> [OPTIONS]

OPTIONS:
    --from-policy <CONCRETE_POLICY>
        Compile a high-level Concrete Policy expression to optimal miniscript
        before encoding. Requires --context. Available only if md-codec was
        built with the `compiler` feature (default-off). See
        https://bitcoin.sipa.be/miniscript/ for Concrete Policy syntax.

    --context <tap|segwitv0>
        Script context for compilation. Required with --from-policy.
```

The CLI's `Cargo.toml` `cli` feature gains an optional dependency on the `compiler` feature:

```toml
[features]
default = ["cli"]
cli = ["dep:clap", "dep:anyhow"]
cli-compiler = ["cli", "compiler"]    # NEW
```

Default `cli` build does NOT include `--from-policy`. Users opt in with `cargo install --features cli-compiler`.

### 5.4 Acceptance

- `cargo build -p md-codec --features compiler` clean.
- `cargo build -p md-codec --features cli-compiler` clean.
- 3-4 unit tests in `bytecode/encode.rs` (or new `policy_compiler.rs`) covering happy-path Tap compile + encode, Segwitv0 compile + encode, parse-error path, compile-error path.
- CLI integration test in `tests/cli.rs` exercising `md encode --from-policy ... --context tap`.

---

## §6. Migration considerations

### 6.1 v0.6.x → v0.7.0 breaking changes

None on the wire — v0.7.0 wire format is byte-identical to v0.6.x for any policy admitted by both versions.

API additions:

1. **NEW crate**: `md-signer-compat` (workspace member; depend if you want named-subset validation).
2. **NEW pub function**: `md_codec::validate_tap_leaf_subset_with_allowlist(...)` (used internally by md-signer-compat; safe to call directly).
3. **NEW cargo feature**: `md_codec/compiler` (default-off).
4. **NEW public types behind `compiler` feature**: `ScriptContext`, `policy_to_bytecode`.
5. **NEW CLI feature**: `cli-compiler` (`md encode --from-policy ... --context tap`).

API removals:
- None. v0.6.x callers continue to work unchanged.

Family-stable SHA reset:
- `GENERATOR_FAMILY` rolls `"md-codec 0.6"` → `"md-codec 0.7"`. v0.1.json + v0.2.json regenerate at the v0.6.x → v0.7.0 boundary; SHA pins update once. No corpus content additions in v0.7.0 (defensive tests are hand-AST in `tests/`, not corpus fixtures), so the only delta is the family token.

### 6.2 No `[Unreleased]` MIGRATION section needed

v0.7.0 is purely additive on the API + tooling fronts. MIGRATION.md `v0.6.x → v0.7.0` section is brief: "no breaking changes; new opt-in features."

---

## §7. Implementation order

Phase order in [`IMPLEMENTATION_PLAN_v0_7_0.md`](./IMPLEMENTATION_PLAN_v0_7_0.md):

1. **Phase 1**: Track A — test rebaseline. 38 tests fixed; full suite green.
2. **Phase 2**: Track B — defensive corpus growth. Hand-AST tests + byte-order pin + per-arm decoder tests added.
3. **Phase 3**: refactor `validate_tap_leaf_subset` to accept caller-supplied allowlist (per §4.4 Option A). Existing call paths preserved; new `_with_allowlist` variant added.
4. **Phase 4**: Track C — md-signer-compat new crate. SignerSubset + COLDCARD_TAP + LEDGER_TAP + tests.
5. **Phase 5**: Track D — policy compiler feature + wrapper API. ScriptContext enum + policy_to_bytecode + tests + CLI integration.
6. **Phase 6**: release plumbing. Cargo bump 0.6.0 → 0.7.0; vector regen with new family token; CHANGELOG; MIGRATION; tag.
7. **Phase 7**: final reconciliation. Agent reports vs FOLLOWUPS; PR + GitHub Release; memory update.

---

## §8. Acceptance criteria

The v0.7.0 release is acceptance-ready when:

1. `cargo check --workspace` clean.
2. `cargo clippy --workspace --all-targets` clean.
3. `cargo test --workspace` passes 100% — no `#[ignore]` markers introduced.
4. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` clean.
5. `cargo build --workspace --features md-codec/compiler` clean.
6. `cargo build --workspace --features md-codec/cli-compiler` clean.
7. `gen_vectors --verify` passes against regenerated v0.1.json + v0.2.json (SHA-pinned to v0.7 values).
8. md-signer-compat unit tests cover happy-path + rejection-path for both COLDCARD_TAP and LEDGER_TAP.
9. policy_to_bytecode wrapper has unit tests for happy-path Tap, happy-path Segwitv0, and at least one error-path.
10. CLI integration test exercises `md encode --from-policy ...` mode.
11. CHANGELOG `[0.7.0]` and MIGRATION `v0.6.x → v0.7.0` sections in place.
12. All 7 closing FOLLOWUPS entries (per §1.1-§1.4 above) flipped to `resolved <SHA>` in design/FOLLOWUPS.md.

---

## §9. Open questions for review

These are flagged for the spec-review agent:

1. **Refactor visibility (Phase 3)**: should the new `validate_tap_leaf_subset_with_allowlist` be `pub fn` or `pub(crate)` (with md-signer-compat having a separate impl)? §4.4 leans Option A (pub fn in md-codec). Confirm or refine.

2. **`SignerSubset.allowed_operators` shape**: `&'static [&'static str]` (operator names as BIP 388 source-form strings) vs a typed enum (variant per Tag). String-based is simpler for vendor tracking; typed enum is compiler-checked. §4.2 chose string-based; reconfirm.

3. **`ScriptContext` placement**: in md-codec (publicly exposed via `pub use ScriptContext`) vs in a new module (`pub mod compiler { pub enum ScriptContext { ... } }`). §5.2 leans top-level pub; reconfirm.

4. **`cli-compiler` feature naming**: alternative names: `cli-with-compiler`, `cli-policy`, etc. §5.3 chose `cli-compiler`; pick a final name.

5. **CLI surface for md-signer-compat validation**: `md validate --signer coldcard <bytecode>` was mentioned in earlier FOLLOWUPS. Defer to v0.7.x or include in v0.7.0? §4.6 defers; reconfirm.

6. **CHANGELOG `Unreleased` discipline**: v0.7.0 work happens on `feature/v0.7.0-development`; should the in-flight `[Unreleased]` section in CHANGELOG.md track per-phase entries (rolled into `[0.7.0]` at release time)? Or single consolidated entry at release time only? §6 picks the latter; confirm.

---

(End of v0.7.0 spec.)
