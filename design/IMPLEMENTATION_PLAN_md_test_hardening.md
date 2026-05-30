# md-codec Test-Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add property/fuzz, BCH-adversarial, and indel-reject-contract test coverage to `md-codec` — closing the survey's themes 1/2/3 for the constellation's largest (descriptor) grammar.

**Architecture:** Test-only. A shared `tests/common/mod.rs` exposes `descriptor_strategy()` (option-(c): templated shapes + a bounded `tr()` taptree sub-strategy) reused by the property harness; the bijection property is **canonical-fixpoint** (`encode_payload` canonicalizes internally). Deterministic BCH/indel cells live in their own files. No production change unless a test surfaces a bug (SPEC §6).

**Tech Stack:** Rust (edition 2024), `proptest = "1"` (new dev-dep), the `md_codec::{encode,decode,chunk,canonicalize}` public surface. (No `bitcoin`/pubkey dep: the templated shapes reference placeholder indices, not raw key literals — R0 M3.)

**Source spec (R0-gate GREEN):** `design/SPEC_md_codec_test_hardening.md`. **Branch:** `md-codec-test-hardening` (off `main`). **SHA:** `ca4591b`. **This plan is subject to the mandatory opus R0 gate before any task runs.**

---

## Verified construction primitives (from source @ `ca4591b`)

```rust
// Descriptor { n: u8, path_decl: PathDecl, use_site_path: UseSitePath, tree: Node, tlv: TlvSection }
// PathDecl { n: u8, paths: PathDeclPaths::Shared(OriginPath) | Divergent(Vec<OriginPath>) }
// OriginPath { components: Vec<PathComponent{hardened: bool, value: u32}> }   // components.len() <= 15
// Node { tag: Tag, body: Body }
// Body::Children(Vec<Node>) | Variable{k,children} | MultiKeys{k:u8, indices:Vec<u8>} |
//      Tr{is_nums:bool, key_index:u8, tree:Option<Box<Node>>} | KeyArg{index:u8} |
//      Hash256Body([u8;32]) | Hash160Body([u8;20]) | Timelock(u32) | Empty
// helpers: UseSitePath::standard_multipath(), TlvSection::new_empty()
// API: encode_payload(&Descriptor)->Result<(Vec<u8>,usize)>; decode_payload(&[u8],usize)->Result<Descriptor>;
//      encode_md1_string(&Descriptor)->Result<String>; decode_md1_string(&str)->Result<Descriptor>;
//      chunk::{split(&Descriptor)->Result<Vec<String>>, reassemble(&[&str])->Result<Descriptor>,
//              decode_with_correction(&[&str])->Result<(Descriptor,Vec<CorrectionDetail>)>, ChunkHeader};
//      canonicalize::canonicalize_placeholder_indices(&mut Descriptor)->Result<()>
```

---

## File Structure
- **Create** `crates/md-codec/tests/common/mod.rs` — `descriptor_strategy()` + shape builders + `renumber_tree` + `corrupt_chunk_at`. `#![allow(dead_code, unused_imports)]` (spec-R0 I4).
- **Create** `crates/md-codec/tests/proptest_roundtrip.rs` — Theme 1 (P1–P5).
- **Create** `crates/md-codec/tests/bch_adversarial.rs` — Theme 2 (T2a–T2i) + `restamp_chunk_header`.
- **Create** `crates/md-codec/tests/indel_reject_contract.rs` — Theme 3 (T3a–T3d).
- **Modify** `crates/md-codec/Cargo.toml` (`[dev-dependencies] proptest`), `descriptor-mnemonic/.gitignore` (`**/proptest-regressions/`).

---

## Phase 0 — harness + Theme 1

### Task 0.1: proptest dev-dep + gitignore
**Files:** Modify `crates/md-codec/Cargo.toml`, `descriptor-mnemonic/.gitignore`.
- [ ] **Step 1:** add under `[dev-dependencies]` in `crates/md-codec/Cargo.toml`:
```toml
proptest = "1"
```
Confirm `bitcoin` (or `secp256k1`) is reachable as a dev-dep for `fixed_pubkey()`; if md-codec doesn't already depend on `bitcoin`, use the hex bytes of secp generator G directly (Step in 0.2 uses a literal compressed-G byte array — no new dep needed).
- [ ] **Step 2:** append to `descriptor-mnemonic/.gitignore`:
```gitignore
# proptest shrink-regression corpora (per-test-file, nested)
**/proptest-regressions/
```
- [ ] **Step 3:** Run `cargo build -p md-codec --tests 2>&1 | tail -3` → builds.
- [ ] **Step 4:** Commit:
```bash
git add crates/md-codec/Cargo.toml .gitignore
git commit -m "test(md-codec): add proptest dev-dep + proptest-regressions gitignore"
```

### Task 0.2: shared strategy (`tests/common/mod.rs`)
**Files:** Create `crates/md-codec/tests/common/mod.rs`.

- [ ] **Step 1 (TRIAL-COMPILE FIRST — architect caveat):** before writing the full strategy, create a minimal `common/mod.rs` with ONLY `descriptor_strategy` returning a single templated shape via `prop_oneof![Just(keyarg(Tag::Wpkh,0))].prop_map(|t| descriptor_from_tree(t,false))`, plus the `prop_recursive` taptree skeleton (Step 2's `taptree_strategy`) and the `renumber_tree`/`descriptor_from_tree`/`referenced_indices` helpers, and a throwaway test in `proptest_roundtrip.rs` that draws one value — `cargo test -p md-codec --test proptest_roundtrip 2>&1 | tail`. This shakes out the `prop_recursive` generic-bound + boxing typing in isolation (spec-R0 M5: the typing is sound; this is a cheap guard). Confirm `-D warnings` stays clean incl. `missing_docs` on the new test crate (R0 M4). Iterate until it builds; THEN fill in the remaining shapes (Step 2). Do NOT change semantics — only satisfy the typer.

- [ ] **Step 2:** write the full module:
```rust
//! Shared generators + helpers for the md-codec test-hardening suite.
//! Consumed by proptest_roundtrip.rs and bch_adversarial.rs via `mod common;`.
#![allow(dead_code, unused_imports)]

use md_codec::canonicalize::canonicalize_placeholder_indices;
use md_codec::encode::Descriptor;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;
use proptest::prelude::*;

fn shared_path(depth: u8) -> PathDecl {
    let components = (0..depth)
        .map(|i| PathComponent { hardened: true, value: (i as u32) + 1 })
        .collect();
    PathDecl { n: 1, paths: PathDeclPaths::Shared(OriginPath { components }) }
}

fn divergent_path(n: u8, depth: u8) -> PathDecl {
    let paths = (0..n)
        .map(|c| OriginPath {
            components: (0..depth)
                .map(|i| PathComponent { hardened: true, value: (c as u32) * 100 + (i as u32) + 1 })
                .collect(),
        })
        .collect();
    PathDecl { n, paths: PathDeclPaths::Divergent(paths) }
}

fn wrap(tag: Tag, inner: Node) -> Node { Node { tag, body: Body::Children(vec![inner]) } }
fn keyarg(tag: Tag, index: u8) -> Node { Node { tag, body: Body::KeyArg { index } } }
fn multikeys(tag: Tag, k: u8, indices: Vec<u8>) -> Node {
    Node { tag, body: Body::MultiKeys { k, indices } }
}

/// n biased to the kiw-width boundaries (SPEC §3.1 / R0 I3): exercises kiw 0..5.
fn n_strategy() -> impl Strategy<Value = u8> {
    prop_oneof![
        Just(1u8), Just(2), Just(3), Just(4), Just(5), Just(8), Just(9),
        Just(15), Just(16), Just(17), Just(31), Just(32),
        2u8..=32,
    ]
}

/// Bounded-recursion tr() taptree (SPEC §3.1): internal TapTree{Children(2)};
/// leaves ONLY from the permitted allow-list (no forbidden-leaf, no filter).
/// Leaves reference placeholder indices in 1.. (keypath is @0); the caller
/// derives n from the union of emitted indices (R0 I2).
fn taptree_strategy(max_key_index: u8) -> impl Strategy<Value = Node> {
    let leaf = prop_oneof![
        (1u8..=max_key_index).prop_map(|i| keyarg(Tag::PkK, i)),
        (1u8..=max_key_index).prop_map(|i| keyarg(Tag::PkH, i)),
        (1u8..=max_key_index).prop_map(|i| multikeys(Tag::MultiA, 1, vec![i])),
        (1u32..=65535).prop_map(|t| Node { tag: Tag::Older, body: Body::Timelock(t) }),
    ];
    leaf.prop_recursive(3, 8, 2, |inner| {
        (inner.clone(), inner).prop_map(|(l, r)| Node {
            tag: Tag::TapTree,
            body: Body::Children(vec![l, r]),
        })
    })
}

/// Collect the distinct placeholder indices referenced by a tree (KeyArg +
/// MultiKeys + Tr.key_index when !is_nums), so n can be DERIVED (R0 I2).
fn referenced_indices(node: &Node, out: &mut std::collections::BTreeSet<u8>) {
    match &node.body {
        Body::KeyArg { index } => { out.insert(*index); }
        Body::MultiKeys { indices, .. } => { out.extend(indices.iter().copied()); }
        Body::Tr { is_nums, key_index, tree } => {
            if !is_nums { out.insert(*key_index); }
            if let Some(t) = tree { referenced_indices(t, out); }
        }
        Body::Children(cs) => for c in cs { referenced_indices(c, out); }
        Body::Variable { children, .. } => for c in children { referenced_indices(c, out); }
        _ => {}
    }
}

/// Rewrite every placeholder index in `node` through `perm` (old→new). Mirrors
/// the canonicalizer's remap (canonicalize.rs:102-139). A NUMS `Tr.key_index`
/// is left untouched (it has no wire representation), matching
/// `referenced_indices`. (R0 C1: this is the spec-mandated renumber the first
/// draft dropped.)
fn renumber_tree(node: &mut Node, perm: &std::collections::BTreeMap<u8, u8>) {
    match &mut node.body {
        Body::KeyArg { index } => { *index = perm[&*index]; }
        Body::MultiKeys { indices, .. } => { for i in indices.iter_mut() { *i = perm[&*i]; } }
        Body::Tr { is_nums, key_index, tree } => {
            if !*is_nums { *key_index = perm[&*key_index]; }
            if let Some(t) = tree { renumber_tree(t, perm); }
        }
        Body::Children(cs) => { for c in cs.iter_mut() { renumber_tree(c, perm); } }
        Body::Variable { children, .. } => { for c in children.iter_mut() { renumber_tree(c, perm); } }
        _ => {}
    }
}

/// Build a Descriptor from a tree (R0 C1 fix): collect the referenced placeholder
/// indices, RENUMBER the tree to a contiguous `0..n` (the spec-mandated half the
/// first draft dropped — without it, taptree leaves drawn from `1..=max` yield
/// non-contiguous indices `>= n` → `PlaceholderIndexOutOfRange` → `canon()`
/// panics), then derive n + path-decl from that set. No `set.insert(0)`:
/// `referenced_indices` already captures the non-NUMS keypath (`Tr.key_index`),
/// so every shape built here references >=1 index, and the renumber makes the
/// emitted set exactly `0..n` — killing both `PlaceholderIndexOutOfRange` AND
/// `PlaceholderNotReferenced` (canonicalize.rs:184,255). Explicit-origin shapes
/// (sh/wsh/sortedmulti/tr+taptree) get a Divergent path so the
/// forced-explicit-origin gate is satisfied (validate.rs:182).
fn descriptor_from_tree(mut tree: Node, explicit_origin: bool) -> Descriptor {
    let mut set = std::collections::BTreeSet::new();
    referenced_indices(&tree, &mut set);
    let perm: std::collections::BTreeMap<u8, u8> =
        set.iter().enumerate().map(|(rank, &old)| (old, rank as u8)).collect();
    renumber_tree(&mut tree, &perm);
    let n = set.len() as u8;
    let path_decl = if explicit_origin {
        divergent_path(n, 3)
    } else {
        PathDecl { n, paths: PathDeclPaths::Shared(OriginPath {
            components: vec![PathComponent { hardened: true, value: 84 }],
        }) }
    };
    Descriptor { n, path_decl, use_site_path: UseSitePath::standard_multipath(), tree, tlv: TlvSection::new_empty() }
}

pub fn descriptor_strategy() -> BoxedStrategy<Descriptor> {
    let single_sig = prop_oneof![
        Just(keyarg(Tag::Wpkh, 0)),
        Just(keyarg(Tag::Pkh, 0)),
        Just(Node { tag: Tag::Tr, body: Body::Tr { is_nums: false, key_index: 0, tree: None } }),
    ].prop_map(|t| descriptor_from_tree(t, false));

    let sh_wpkh = Just(wrap(Tag::Sh, keyarg(Tag::Wpkh, 0))).prop_map(|t| descriptor_from_tree(t, false));

    let multisig = (n_strategy(), 1u8..=32u8, prop::sample::select(vec![Tag::Multi, Tag::SortedMulti]))
        .prop_filter("k<=n", |(n, k, _)| k <= n)
        .prop_map(|(n, k, mtag)| {
            let inner = multikeys(mtag, k, (0..n).collect());
            descriptor_from_tree(wrap(Tag::Wsh, inner), true)
        });

    let sh_wsh = (n_strategy(), 1u8..=32u8)
        .prop_filter("k<=n", |(n, k)| k <= n)
        .prop_map(|(n, k)| {
            let inner = wrap(Tag::Wsh, multikeys(Tag::SortedMulti, k, (0..n).collect()));
            descriptor_from_tree(wrap(Tag::Sh, inner), true)
        });

    // R0 M1: legacy bare P2SH sortedmulti — the only shape exercising the
    // canonical_origin==None branch (canonical_origin.rs:65-75).
    let sh_sortedmulti = (n_strategy(), 1u8..=32u8)
        .prop_filter("k<=n", |(n, k)| k <= n)
        .prop_map(|(n, k)| {
            let inner = multikeys(Tag::SortedMulti, k, (0..n).collect());
            descriptor_from_tree(wrap(Tag::Sh, inner), true)
        });

    let tr_multi_a = (2u8..=16u8, 1u8..=16u8)
        .prop_filter("k<=n-1", |(n, k)| *k <= n - 1)
        .prop_map(|(n, k)| {
            // keypath @0, multi_a over @1..@(n-1)
            let leaf = multikeys(Tag::MultiA, k, (1..n).collect());
            let tree = Node { tag: Tag::Tr, body: Body::Tr {
                is_nums: false, key_index: 0, tree: Some(Box::new(leaf)) } };
            descriptor_from_tree(tree, true)
        });

    let tr_taptree = (2u8..=8u8).prop_flat_map(|max| {
        taptree_strategy(max).prop_map(move |tt| {
            let tree = Node { tag: Tag::Tr, body: Body::Tr {
                is_nums: false, key_index: 0, tree: Some(Box::new(tt)) } };
            descriptor_from_tree(tree, true)
        })
    });

    prop_oneof![single_sig, sh_wpkh, multisig, sh_wsh, sh_sortedmulti, tr_multi_a, tr_taptree].boxed()
}

/// canonicalize a descriptor (the fixpoint helper).
pub fn canon(d: &Descriptor) -> Descriptor {
    let mut c = d.clone();
    canonicalize_placeholder_indices(&mut c).expect("strategy descriptors are canonicalizable");
    c
}

/// flip one codex32 symbol at data-part position `pos` (post-"md1") of a chunk.
/// (R0 I2: bounds-guard `pos` so a too-short fixture fails LOUDLY, not via an
/// opaque `index out of bounds`.)
pub fn corrupt_chunk_at(chunk: &str, pos: usize, xor_mask: u8) -> String {
    const A: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let mut chars: Vec<char> = chunk.chars().collect();
    let idx = 3 + pos;
    assert!(idx < chars.len(), "corrupt position {pos} past data-part (chunk len {})", chars.len());
    let sym = A.iter().position(|&b| b == (chars[idx] as u8).to_ascii_lowercase()).unwrap() as u8;
    chars[idx] = A[((sym ^ (xor_mask & 0x1F)) & 0x1F) as usize] as char;
    chars.into_iter().collect()
}
```
> **Implementer note:** `descriptor_from_tree` derives `n` from the referenced-index set AND renumbers the tree to a contiguous `0..n` (R0 C1) — so the flat arms (whose trees already emit `0..n`) renumber through the identity permutation, and the `tr_taptree` arm (whose leaves draw arbitrary indices from `1..=max`) gets remapped to contiguous. If a `canon()` panic still surfaces in Step-3/P1 verification, the renumber or `referenced_indices` missed a `Body` arm — fix the helper, not the assert.

- [ ] **Step 3:** verify the strategy only produces encodable+canonicalizable descriptors — covered by P1 in Task 0.3 (a `canon()` panic or P1 encode-error there = a strategy defect; fix the builder).

- [ ] **Step 4:** commit (with Task 0.3).

### Task 0.3: Theme 1 properties (`tests/proptest_roundtrip.rs`)
**Files:** Create `crates/md-codec/tests/proptest_roundtrip.rs`.
- [ ] **Step 1:** write:
```rust
//! Theme 1 (SPEC §3) — md-codec property harness.
mod common;
use common::{canon, descriptor_strategy};
use md_codec::canonicalize::canonicalize_placeholder_indices;
use md_codec::chunk::{reassemble, split};
use md_codec::decode::{decode_md1_string, decode_payload};
use md_codec::encode::{encode_md1_string, encode_payload};
use proptest::prelude::*;

proptest! {
    // P1 — canonical-fixpoint payload bijection.
    #[test]
    fn p1_canonical_fixpoint(d in descriptor_strategy()) {
        let c = canon(&d);
        let (bytes, total_bits) = encode_payload(&c).expect("canonical encodes");
        let back = decode_payload(&bytes, total_bits).expect("canonical decodes");
        prop_assert_eq!(back, c.clone());
        // internal-canonicalization pin: encode(d) byte-equals encode(canon(d)).
        let (b2, t2) = encode_payload(&d).expect("encodes");
        prop_assert_eq!((b2, t2), (bytes, total_bits));
    }

    // P2 — canonicalize-is-normalizer (F4-class catcher). Generates the same
    // strategy (which already varies index orderings) and asserts the encoder's
    // internal canonicalization equals the explicit one + decode lands on canon.
    #[test]
    fn p2_normalizer(d in descriptor_strategy()) {
        let c = canon(&d);
        let (bd, td) = encode_payload(&d).expect("encodes");
        let (bc, tc) = encode_payload(&c).expect("encodes");
        prop_assert_eq!((&bd, td), (&bc, tc));
        let back = decode_payload(&bd, td).expect("decodes");
        prop_assert_eq!(back, c);
    }

    // P3 — decode panic-freedom. decode_payload arm pins total_bits = bytes*8
    // (R0 I1: bitstream.rs:114 debug_assert). The &str arms are free-form.
    #[test]
    fn p3_decode_payload_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..64)) {
        let total_bits = bytes.len() * 8;
        let _ = decode_payload(&bytes, total_bits);
    }
    #[test]
    fn p3_decode_str_never_panics(s in "\\PC*") {
        let _ = decode_md1_string(&s);
        let _ = reassemble(&[s.as_str()]);
    }

    // P4 — string-level round-trip (distinct padding/rollback surface).
    #[test]
    fn p4_string_round_trip(d in descriptor_strategy()) {
        let c = canon(&d);
        let s = encode_md1_string(&c).expect("string encodes");
        let back = decode_md1_string(&s).expect("string decodes");
        prop_assert_eq!(back, c);
    }

    // P5 — chunk round-trip.
    #[test]
    fn p5_chunk_round_trip(d in descriptor_strategy()) {
        let c = canon(&d);
        let chunks = split(&c).expect("splits");
        let refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        prop_assert_eq!(reassemble(&refs).expect("reassembles"), c);
    }
}
```
- [ ] **Step 2:** Run `cargo test -p md-codec --test proptest_roundtrip 2>&1 | tail -30` → **PASS** (properties of existing correct behavior). A `canon()`/`encode_payload` `.expect()` panic ⇒ strategy generates an unencodable/uncoverable descriptor ⇒ fix the builder (Task 0.2), NOT the assert. A `prop_assert_eq` mismatch ⇒ a genuine canonicalization/round-trip bug ⇒ STOP, DONE_WITH_CONCERNS (SPEC §6).
- [ ] **Step 3:** `cargo +stable clippy -p md-codec --tests -- -D warnings 2>&1 | tail`; `cargo +stable fmt --check -- crates/md-codec/tests/common/mod.rs crates/md-codec/tests/proptest_roundtrip.rs` (CI uses **stable**, edition 2024; do NOT use the local nightly `cargo fmt` — the mk lesson; if it reformats any non-new file, `git restore` it).
- [ ] **Step 4:** Commit:
```bash
git add crates/md-codec/tests/common/mod.rs crates/md-codec/tests/proptest_roundtrip.rs
git commit -m "test(md-codec): theme 1 — canonical-fixpoint bijection + normalizer + panic-freedom + string/chunk round-trips"
```

---

## Phase 1 — Theme 2 (`tests/bch_adversarial.rs`)

### Task 1.1: deterministic correction (T2a/T2b) + restamp helper
**Files:** Create `crates/md-codec/tests/bch_adversarial.rs`.
- [ ] **Step 1:** write the file header + helpers + T2a + T2b:
```rust
//! Theme 2 (SPEC §4) — BCH adversarial. Drive correction via decode_with_correction.
mod common;
use common::{canon, corrupt_chunk_at};
use md_codec::chunk::{decode_with_correction, reassemble, split, ChunkHeader};
use md_codec::encode::Descriptor;
use md_codec::error::Error;
// reuse the same fixtures the repo's chunking.rs uses (copy small/deep/multi_chunk builders here):
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;

fn wpkh_descriptor(depth: u8) -> Descriptor {
    Descriptor {
        n: 1,
        path_decl: PathDecl { n: 1, paths: PathDeclPaths::Shared(OriginPath {
            components: (0..depth).map(|i| PathComponent { hardened: true, value: (i as u32)+1 }).collect() }) },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node { tag: Tag::Wpkh, body: Body::KeyArg { index: 0 } },
        tlv: TlvSection::new_empty(),
    }
}
fn multi_chunk_descriptor() -> Descriptor {
    // R0 I3: 6 Divergent cosigners × 15 hardened components — payload ≈ 6×180 bits
    // / SINGLE_STRING_PAYLOAD_BIT_LIMIT(320) → ≥4 chunks, comfortably off the 2/3
    // boundary so T2f (needs ≥3) and T2h (needs ≥2) are not boundary-fragile.
    let paths = (0..6u32).map(|c| OriginPath {
        components: (0..15u32).map(|i| PathComponent { hardened: true, value: c*100+i+1 }).collect() }).collect();
    Descriptor {
        n: 6,
        path_decl: PathDecl { n: 6, paths: PathDeclPaths::Divergent(paths) },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node { tag: Tag::Wsh, body: Body::Children(vec![Node {
            tag: Tag::SortedMulti, body: Body::MultiKeys { k: 2, indices: (0..6).collect() } }]) },
        tlv: TlvSection::new_empty(),
    }
}

// T2a — 1..=4-error correction across 3 lengths, through public decode_with_correction.
#[test]
fn t2a_correct_1_to_4_errors_across_lengths() {
    for d in [wpkh_descriptor(3), wpkh_descriptor(15), multi_chunk_descriptor()] {
        let chunks = split(&d).unwrap();
        for count in 1..=4usize {
            let mut cs = chunks.clone();
            // corrupt `count` distinct data-part positions (past pos 0) in chunk 0
            for p in 1..=count { cs[0] = corrupt_chunk_at(&cs[0], p, 0x1F); }
            let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
            let (got, details) = decode_with_correction(&refs)
                .unwrap_or_else(|e| panic!("t={count} must correct: {e:?}"));
            assert_eq!(got, d, "t={count} recovered a different descriptor");
            assert!(details.len() >= count, "expected >= {count} corrections");
        }
    }
}

// T2b — correction inside the trailing 13-symbol checksum region.
#[test]
fn t2b_correct_checksum_region_errors() {
    let d = wpkh_descriptor(15);
    let chunks = split(&d).unwrap();
    let dp_len = chunks[0].chars().count() - 3; // post-HRP data-part length
    let mut cs = chunks.clone();
    // 2 errors inside the last 13 symbols (the BCH checksum tail)
    cs[0] = corrupt_chunk_at(&cs[0], dp_len - 1, 0x1F);
    cs[0] = corrupt_chunk_at(&cs[0], dp_len - 7, 0x1F);
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    let (got, _) = decode_with_correction(&refs).expect("checksum-region errors correct");
    assert_eq!(got, d);
}
```
- [ ] **Step 2:** Run `cargo test -p md-codec --test bch_adversarial t2a t2b 2>&1 | tail`. PASS. If a correction fails to recover the original → STOP, SPEC §6. (If a chunk has fewer than `count+1` data positions, use `multi_chunk_descriptor`'s longer chunk — adjust the fixture.)
- [ ] **Step 3:** clippy + stable-fmt (as Task 0.3 Step 3).
- [ ] **Step 4:** commit `test(md-codec): theme 2 T2a/T2b — 1-4 error correction across lengths + checksum region`.

### Task 1.2: T2c miscorrection sweep + T2d deterministic + T2h/T2i multi-chunk
**Files:** Modify `crates/md-codec/tests/bch_adversarial.rs`.
- [ ] **Step 1:** append:
```rust
// T2c — randomized 5-8-error sweep. ASSERT != Ok(original) (NOT is_err — md
// miscorrects to a different codeword at ~2^-26; SPEC §4.1). Seeded xorshift,
// no rand dep.
#[test]
fn t2c_five_to_eight_errors_never_return_original() {
    let d = wpkh_descriptor(15);
    let original = d.clone();
    let chunks = split(&d).unwrap();
    let dp_len = chunks[0].chars().count() - 3;
    let mut x: u64 = 0x9E3779B97F4A7C15;
    for trial in 0..300u32 {
        for n_err in 5..=8usize {
            let mut positions = std::collections::BTreeSet::new();
            while positions.len() < n_err {
                x ^= x << 13; x ^= x >> 7; x ^= x << 17;
                positions.insert((x as usize) % dp_len);
            }
            let mut c0 = chunks[0].clone();
            for &p in &positions { c0 = corrupt_chunk_at(&c0, p, ((x as u8) | 1) & 0x1F); }
            let mut cs = chunks.clone(); cs[0] = c0;
            let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
            // != Ok(original): Err, or Ok(different), both acceptable; never Ok(original).
            if let Ok((got, _)) = decode_with_correction(&refs) {
                assert_ne!(got, original, "trial {trial} n_err {n_err}: 5-8 errors silently returned the original");
            }
        }
    }
}

// A 5-error pattern (data-part positions) VERIFIED-UNCORRECTABLE for
// wpkh_descriptor(15)'s single chunk. R0 I1: a 5-error pattern is NOT guaranteed
// uncorrectable (Berlekamp-Massey may miscorrect to a valid codeword). Build-time
// contract: T2d asserts this pattern errs; if a fixture/fmt change shifts the
// chunk's symbols and this pattern starts to (mis)correct, T2d fails LOUDLY —
// pick another 5-position set (try [2,5,8,11,14] / [1,3,6,9,12] / …) until
// decode_with_correction errs, and update this const + comment.
const UNCORRECTABLE_5ERR: [usize; 5] = [1, 4, 7, 10, 13];

// T2d — the verified-uncorrectable deterministic 5-error pattern → Err.
#[test]
fn t2d_deterministic_five_error_is_err() {
    let d = wpkh_descriptor(15);
    let chunks = split(&d).unwrap();
    let mut c0 = chunks[0].clone();
    for p in UNCORRECTABLE_5ERR { c0 = corrupt_chunk_at(&c0, p, 0x1F); }
    let mut cs = chunks.clone(); cs[0] = c0;
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    assert!(decode_with_correction(&refs).is_err(),
        "UNCORRECTABLE_5ERR must be uncorrectable — if this fires, the chunk symbols changed; \
         pick another 5-position pattern that errs and update the const (see its doc-comment)");
}

// T2h — multi-chunk: 2 different chunks each <= 4 errors → Ok(original).
#[test]
fn t2h_multi_chunk_two_corrupted_within_t() {
    let d = multi_chunk_descriptor();
    let chunks = split(&d).unwrap();
    assert!(chunks.len() >= 2);
    let mut cs = chunks.clone();
    cs[0] = corrupt_chunk_at(&cs[0], 2, 0x1F);
    let li = cs.len() - 1; cs[li] = corrupt_chunk_at(&cs[li], 2, 0x1F);
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    let (got, _) = decode_with_correction(&refs).expect("each chunk within t corrects");
    assert_eq!(got, d);
}

// T2i — one chunk over t in a valid multi-chunk set: never silently yields the
// original (atomic-abort intent). R0 I1: UNCORRECTABLE_5ERR is verified against
// wpkh_descriptor(15)'s chunk, NOT multi_chunk's (different content), so this cell
// uses the robust `!= Ok(original)` invariant (same as T2c) — Err is the expected
// abort; a rare chunk-0 miscorrection surfaces as Ok(different), still ≠ original.
// (if-let extraction avoids needing Error: PartialEq, matching T2c.)
#[test]
fn t2i_one_chunk_over_t_never_returns_original() {
    let d = multi_chunk_descriptor();
    let chunks = split(&d).unwrap();
    let mut cs = chunks.clone();
    for p in [1usize, 4, 7, 10, 13] { cs[0] = corrupt_chunk_at(&cs[0], p, 0x1F); }
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    if let Ok((got, _)) = decode_with_correction(&refs) {
        assert_ne!(got, d, "a 5-error chunk-0 corruption must never reassemble to the original");
    }
}
```
- [ ] **Step 2:** Run `cargo test -p md-codec --test bch_adversarial t2c t2d t2h t2i 2>&1 | tail`. PASS. A T2c `assert_ne` failure = a real silent-acceptance bug → STOP, SPEC §6.
- [ ] **Step 3:** clippy + stable-fmt.
- [ ] **Step 4:** commit `test(md-codec): theme 2 T2c/T2d/T2h/T2i — miscorrection sweep + deterministic + multi-chunk`.

### Task 1.3: cross-chunk validation branches (T2e/T2f/T2g)
**Files:** Modify `crates/md-codec/tests/bch_adversarial.rs`.
- [ ] **Step 1:** append the restamp helper + T2e/T2f/T2g. The helper parses a chunk via the public codex32/bitstream surface, mutates the `ChunkHeader`, and re-wraps:
```rust
// restamp_chunk_header: decode a chunk to (header, payload-bits), mutate the
// header, re-encode. Uses md_codec::codex32 + bitstream + ChunkHeader public API.
fn restamp_chunk_header(chunk: &str, mutate: impl FnOnce(&mut ChunkHeader)) -> String {
    use md_codec::bitstream::{BitReader, BitWriter};
    use md_codec::codex32::{unwrap_string, wrap_payload};
    let symbols = unwrap_string(chunk).expect("valid chunk");
    // ... parse header (ChunkHeader::read), capture remaining payload bits,
    // mutate header, ChunkHeader::write into a fresh BitWriter, append payload,
    // wrap_payload back to a string. EXACT bit-plumbing to be trial-built in
    // Step 1a against the public bitstream API (the ChunkHeader is 37 bits).
    unimplemented!("trial-build the header re-stamp against bitstream API in Step 1a")
}
```
- [ ] **Step 1a (trial-build the restamp helper):** the precise `unwrap_string`/`BitReader`/`ChunkHeader::read`/payload-copy/`ChunkHeader::write`/`wrap_payload` plumbing must be built against the public API and verified by a round-trip assertion `restamp_chunk_header(c, |_| {}) ` re-decodes to the same descriptor (identity restamp). Build this helper first, prove identity, THEN write T2e/f/g. (This is the one genuinely fiddly helper; treat it as its own red→green sub-task.) **Also in Step-1a (R0 I3):** print `split(&multi_chunk_descriptor()).len()` and confirm ≥3 — the cross-chunk cells assume it.
- [ ] **Step 2:** T2e/f/g:
```rust
#[test]
fn t2e_reassemble_rejects_count_mismatch() {
    let chunks = split(&multi_chunk_descriptor()).unwrap();
    let mut cs = chunks.clone();
    cs[0] = restamp_chunk_header(&cs[0], |h| h.count = h.count.wrapping_add(1));
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    // R0 C2: ChunkSetInconsistent is a UNIT variant (error.rs:259) — no `{ .. }`.
    assert!(matches!(reassemble(&refs), Err(Error::ChunkSetInconsistent)));
}
#[test]
fn t2f_reassemble_rejects_index_gap() {
    let chunks = split(&multi_chunk_descriptor()).unwrap();
    // multi_chunk_descriptor is 6 cosigners → ≥4 chunks (R0 I3); this guard is a
    // tripwire, not load-bearing. Step-1a MUST still print split().len() to confirm.
    assert!(chunks.len() >= 3, "need >=3 chunks; enlarge the fixture");
    let mut cs = chunks.clone();
    cs[1] = restamp_chunk_header(&cs[1], |h| h.index = 0); // duplicate index 0
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    assert!(matches!(reassemble(&refs), Err(Error::ChunkIndexGap { .. })));
}
#[test]
fn t2g_reassemble_rejects_derived_csid_mismatch() {
    let chunks = split(&multi_chunk_descriptor()).unwrap();
    // re-stamp EVERY header with a foreign csid so header-consistency passes
    // but the reassembled payload derives a different csid.
    let foreign: u32 = 0x0AAAA;
    let cs: Vec<String> = chunks.iter().map(|c| restamp_chunk_header(c, |h| h.chunk_set_id = foreign)).collect();
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    assert!(matches!(reassemble(&refs), Err(Error::ChunkSetIdMismatch { .. })));
}
```
> **R0 I3 (required Step-1a output):** `multi_chunk_descriptor` is already enlarged to 6 cosigners (expected ≥4 chunks). Step-1a MUST print `split(&multi_chunk_descriptor()).len()` and confirm it is ≥3 before writing T2f; if a bit-budget surprise yields <3, add cosigners or path depth until ≥3.
- [ ] **Step 3:** Run `cargo test -p md-codec --test bch_adversarial t2e t2f t2g 2>&1 | tail`. PASS (exact variant). If a branch surfaces a different error, the SPEC's §4 construction note has the alternative (count-mismatch via spliced different-count sets); adjust the construction to hit the targeted branch — do NOT relax the variant pin.
- [ ] **Step 4:** clippy + stable-fmt; commit `test(md-codec): theme 2 T2e/T2f/T2g — cross-chunk reassembly validation branches`.

---

## Phase 2 — Theme 3 (`tests/indel_reject_contract.rs`)

### Task 2.1: indel reject-contract (T3a–T3d)
**Files:** Create `crates/md-codec/tests/indel_reject_contract.rs`.
- [ ] **Step 1:** write:
```rust
//! Theme 3 (SPEC §5) — indel reject-contract via reassemble (hard verify, no
//! self-correct). The toolkit's Md1IndelOracle
//! (mnemonic-toolkit/crates/mnemonic-toolkit/src/repair.rs:1028,1043) relies on
//! reassemble failing closed on a length-changed string.
use md_codec::chunk::{reassemble, split};
use md_codec::encode::Descriptor;
use md_codec::error::Error;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;

fn fixture() -> Descriptor {
    let paths = (0..4u32).map(|c| OriginPath {
        components: (0..15u32).map(|i| PathComponent { hardened: true, value: c*100+i+1 }).collect() }).collect();
    Descriptor { n: 4, path_decl: PathDecl { n: 4, paths: PathDeclPaths::Divergent(paths) },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node { tag: Tag::Wsh, body: Body::Children(vec![Node {
            tag: Tag::SortedMulti, body: Body::MultiKeys { k: 2, indices: (0..4).collect() } }]) },
        tlv: TlvSection::new_empty() }
}

// T3a — insert one symbol mid-data-part → Err (fail-closed; hard verify).
#[test]
fn t3a_insert_rejected() {
    let chunks = split(&fixture()).unwrap();
    let mut chars: Vec<char> = chunks[0].chars().collect();
    chars.insert(3 + 10, 'p'); // mid-data-part insert
    let mut cs = chunks.clone(); cs[0] = chars.into_iter().collect();
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    assert!(reassemble(&refs).is_err(), "an inserted symbol must fail closed");
}

// T3b — delete one symbol mid-data-part → Err.
#[test]
fn t3b_delete_rejected() {
    let chunks = split(&fixture()).unwrap();
    let mut chars: Vec<char> = chunks[0].chars().collect();
    chars.remove(3 + 10);
    let mut cs = chunks.clone(); cs[0] = chars.into_iter().collect();
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    assert!(reassemble(&refs).is_err(), "a deleted symbol must fail closed");
}

// T3c — truncate below the 13-symbol checksum → Codex32DecodeError (broad pin;
// SPEC R0 M2: bch_verify fails before the "too short" message).
#[test]
fn t3c_truncate_below_checksum_is_codex32_error() {
    let chunks = split(&fixture()).unwrap();
    let chars: Vec<char> = chunks[0].chars().collect();
    // keep only "md1" + 5 data symbols (< 13 checksum symbols)
    let trimmed: String = chars[..3 + 5].iter().collect();
    assert!(matches!(reassemble(&[trimmed.as_str()]), Err(Error::Codex32DecodeError(_))));
}

// T3d — multi-chunk indel never yields a different valid descriptor (the oracle
// guarantee) + the is_err tripwire.
#[test]
fn t3d_multi_chunk_indel_fails_closed() {
    let d = fixture();
    let chunks = split(&d).unwrap();
    let mut chars: Vec<char> = chunks[0].chars().collect();
    chars.insert(3 + 8, 'q');
    let mut cs = chunks.clone(); cs[0] = chars.into_iter().collect();
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    let r = reassemble(&refs);
    assert!(r.is_err(), "multi-chunk indel must fail closed");
    assert_ne!(r.ok(), Some(d), "indel must never self-correct to the original");
}
```
- [ ] **Step 2:** Run `cargo test -p md-codec --test indel_reject_contract 2>&1 | tail`. PASS. A T3a/T3b/T3d returning `Ok` = a fail-OPEN that breaks the toolkit oracle → STOP, SPEC §6 (high severity). If T3c's exact char-count doesn't drop below 13 data-part symbols, adjust the trim (the data-part is everything after "md1"; keep < 13).
- [ ] **Step 3:** clippy + stable-fmt; commit `test(md-codec): theme 3 — indel reject-contract (toolkit repair --md1 --max-indel oracle)`.

---

## Phase 3 — verify + R0 + ship

### Task 3.1: full verification
- [ ] **Step 1:** `cargo test -p md-codec 2>&1 | tail -15` → all green (existing + 4 new files).
- [ ] **Step 2 (CI parity — the mk lesson):** `cargo +stable clippy --workspace --all-targets -- -D warnings` → exit 0; `cargo +stable fmt --check --all` → clean. **If `fmt --check --all` flags PRE-EXISTING files (a rustfmt-version drift like mk hit), do NOT fold them into the test commits — file a FOLLOWUP and surface to the user (chore-fmt vs leave).** The 4 NEW test files must be edition-2024 stable-fmt-clean.
- [ ] **Step 3:** `git status --porcelain` → no untracked `proptest-regressions/`; no dangling `src/` reformat (the mk trap — `cargo fmt -p` reformats production; only stable, new-files-only).

### Task 3.2: end-of-cycle R0 + ship
- [ ] **Step 1:** dispatch the end-of-cycle opus R0 over `git diff main...HEAD`; persist to `design/agent-reports/md-test-hardening-end-of-cycle-R0-review.md`; fold to 0C/0I.
- [ ] **Step 2 (ship, SPEC §6):** no bug ⇒ ff-merge `md-codec-test-hardening` → `main`, no version bump; surface the merge/push to the user (outward-facing — explicit go). Bug fixed inline ⇒ md-codec fix-bump + its own R0 + refresh the toolkit git-dep pin to md-codec.

---

## Self-Review

**Spec coverage:** P1–P5 (§3) → Task 0.3. descriptor_strategy option-(c) (§3.1) → Task 0.2, now 7 arms incl. `sh(sortedmulti)` (R0 M1, the `canonical_origin==None` path). **De-scoped (R0 M1):** the `tr(<NUMS>,<taptree>)` (is_nums=true) arm — its no-keypath/timelock-only edge (empty referenced set → n=0 → KeyCountOutOfRange) needs bespoke handling, and md's existing unit tests already exercise the NUMS-skip canonicalize/validate paths (canonicalize.rs:63,113-114); the marginal proptest coverage isn't worth the edge complexity in a test-only cycle. T2a–T2i (§4) → Tasks 1.1/1.2/1.3. T2c `!= Ok(original)` (§4.1) → Task 1.2; T2i now shares that robust invariant (R0 I1). T3a–T3d (§5) → Task 2.1. proptest+gitignore+dead_code (§3) → Tasks 0.1/0.2. n∈1..=32 + kiw boundaries (§3.1) → `n_strategy` Task 0.2. SemVer/branch/R0 (§6/§8) → Task 3.2. The C1 renumber (`renumber_tree` + `descriptor_from_tree`) makes every arm emit a contiguous `0..n` placeholder set, satisfying §3.1's "derived + renumbered" mandate. All spec sections mapped.

**Placeholder scan:** every code step has complete code EXCEPT the two architect-mandated trial-builds — Task 0.2 Step 1 (`prop_recursive` typing) and Task 1.3 Step 1a (`restamp_chunk_header` bit-plumbing). Both are explicitly-flagged compile-iteration items (the architect's "trial-compile at plan R3" caveat + the fiddly header re-stamp), not behavioral TBDs; the test intent + assertions are fully specified. `restamp_chunk_header` carries `unimplemented!` ONLY as the Step-1a starting point with the exact API to wire — flagged for R0.

**Type consistency:** `descriptor_strategy`/`canon`/`corrupt_chunk_at` defined in `common/mod.rs` (Task 0.2), used by the same names in 0.3/1.x. `wpkh_descriptor`/`multi_chunk_descriptor`/`restamp_chunk_header` defined in `bch_adversarial.rs` (Task 1.1/1.3). `Body::{Children,MultiKeys{k,indices},Tr{is_nums,key_index,tree},KeyArg{index},Timelock}` + `Error::{TooManyErrors,ChunkSetInconsistent,ChunkIndexGap,ChunkSetIdMismatch,Codex32DecodeError}` used consistently with the verified source.
