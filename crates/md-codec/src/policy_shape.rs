//! Structural policy summary — a branch-decomposition walk over an
//! already-decoded [`crate::tree::Node`].
//!
//! Ported from the fork's `md/policy_shape.go` (438 lines), which is
//! fork-native with no prior Rust counterpart — this module makes Rust
//! primary for it. `classifyPolicy` (elsewhere in the fork) returns
//! `PolicyComplex` for anything outside an enumerated shape list, and the
//! consent screen then degrades to an honest-minimal form: script family,
//! key-slot count, template-id, "cannot fully display on-device". That copy
//! is correct but tells an operator almost nothing about the policy they are
//! about to commit to steel. This walk is the cheaper, higher-value tier
//! between "nothing" and a full miniscript text render: it needs no
//! renderer, and emits no descriptor text at all, so it cannot emit a
//! malformed one.
//!
//! # The honesty contract
//!
//! [`PolicyShape::complete`] is `false` when the walk meets a node it does
//! not understand — and when it is `false`, no part of the summary may be
//! presented or keyed on. A summary that silently skipped a branch would be
//! worse than the honest-minimal screen it replaces: the operator would
//! believe they had seen the whole policy. This walk deliberately does not
//! reconstruct descriptor text — naming a fragment is a rendering, and a
//! rendering that cannot be re-parsed is the defect the wider cycle's
//! invariant exists to prevent.
//!
//! # Two extensions over the Go original
//!
//! 1. [`Branch::slots`] retains *which* placeholder indices a branch
//!    references. The Go original (`branchOf`, `md/policy_shape.go:239-245`)
//!    builds exactly this set and then writes `br.Keys = len(keys)`,
//!    discarding it. A verbatim port would leave a later task with no
//!    source for its data (the fingerprint partition needs the slot set,
//!    not just its size).
//! 2. [`KeyPathKind`] renames the Go's third value from `Spendable` to
//!    [`KeyPathKind::Xpub`]. This walk verifies nothing about spendability —
//!    the wire's only internal-key discriminant is `Body::Tr::is_nums`
//!    (`crate::tree::Body::Tr`), and an unspendable-xpub internal key (the
//!    Nunchuk shape F-449 records) is, on the wire, an ordinary `key_index`
//!    structurally identical to a spendable one. Telling the two apart needs
//!    re-deriving a specific coordinator's `unspendable_internal_key`
//!    function over the whole descriptor — Liana's does exactly that — which
//!    makes it a coordinator RULE, not a codec-observable property. `Xpub`
//!    says only "a real extended key, not NUMS"; a coordinator that needs
//!    the finer distinction computes it itself, one layer above this type.
//!
//! # Type reuse: hashlocks come from `compose`, locks do not
//!
//! [`Branch::hashlocks`] is `Vec<`[`crate::compose::HashLock`]`>` — this
//! module does NOT define its own hashlock type. `compose::HashLock` keeps
//! its `digest: [u8; 32]` field private specifically because a 20-byte kind
//! (`Ripemd160`/`Hash160`) carries twelve bytes of alloc-gate padding, and
//! `compose::HashLock`'s hand-written `PartialEq`/`Eq`/`Hash`/`Ord` compare
//! only the *visible* digest (`digest()`, sliced to `HashKind::digest_len()`)
//! — deriving those traits over the raw array would make the padding
//! observable, which is exactly the documented trap at
//! `crate::compose::HashLock`'s own doc comment. An earlier draft of this
//! module defined its own `HashLock { kind, digest: [u8; 32], len: u8 }`
//! with *derived* `PartialEq`/`Eq`, reopening that trap and duplicating
//! `HashKind::digest_len()` (documented there as "the only place a digest
//! length is written") in a second `len` field. Reusing `compose`'s types
//! closes both at once.
//!
//! [`Branch::locks`] is `Vec<Lock>`, a decode-side type distinct from
//! `compose::Lock` — deliberately, not by oversight; see [`Lock`]'s doc.

use crate::compose::{HashKind, HashLock};
use crate::tag::Tag;
use crate::tree::{Body, Node};
use std::collections::BTreeSet;

/// The top-level wrapper. Six-valued, mirroring the fork's `ScriptKind`
/// (`md/md.go:1165-1179`: `Wpkh, Pkh, Sh, Wsh, Tr, ShWpkh`, in that order).
///
/// NOT [`crate::compose::Wrapper`]: that type is four-valued, is the
/// compose-side *input* model the design forbids keying decode-side summary
/// from, and cannot express a decoded singlesig `wpkh`/`pkh`/`sh(wpkh)` — all
/// of which md1 encodes. `sh(wsh)` is not a variant here either; a later
/// task's `Skeleton` carries an `inner_wsh: bool` beside a `RootKind`
/// (design §1A), the same way the fork's `ScriptShape.InnerWsh` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    /// `wpkh` — P2WPKH singlesig.
    Wpkh,
    /// `pkh` — P2PKH singlesig.
    Pkh,
    /// `sh` — P2SH (bare `sh(...)`, or the outer wrapper of `sh(wsh(...))`).
    Sh,
    /// `wsh` — P2WSH.
    Wsh,
    /// `tr` — Taproot.
    Tr,
    /// `sh(wpkh(...))` — BIP 49 nested-segwit singlesig. Kept distinct from
    /// `Sh` because the decoder must tell a bare `sh(...)` script apart from
    /// the one shape `sh` wraps around a `wpkh` rather than a script.
    ShWpkh,
}

/// A taproot internal key. Three-valued, matching the Go original
/// (`KeyPathNone` / `KeyPathNUMS` / `KeyPathSpendable`,
/// `md/policy_shape.go:32-40`) in cardinality but NOT in the third name —
/// see [`KeyPathKind::Xpub`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPathKind {
    /// Not a taproot policy.
    NotTaproot,
    /// Provably unspendable internal key: script paths only (BIP-341 NUMS
    /// H-point). The wire's only internal-key discriminant
    /// (`crate::tree::Body::Tr::is_nums`) says exactly this and nothing
    /// more.
    Nums,
    /// A real extended key sits in the internal-key slot — `is_nums = false`
    /// on the wire. Named `Xpub`, NOT the Go's `Spendable`: this walk
    /// verifies nothing about whether the key can actually spend. An
    /// internal key can be a real xpub and still be *treated* as
    /// unspendable by convention (Nunchuk's key-path-disabled shape, F-449
    /// records it) — recognising that needs re-deriving a specific
    /// coordinator's `unspendable_internal_key`-style function over the
    /// whole descriptor (Liana's does exactly this), which makes it a
    /// coordinator RULE, not a codec-observable property. Do not "restore
    /// fidelity" with the Go name here: a coordinator that needs the finer
    /// distinction computes it itself, one layer above this type.
    Xpub,
}

/// One timelock, in wire units. Mirrors the fork's `LockKind`/`Lock`
/// (`md/compose.go:60-76`), read back off the decoded operand by
/// [`lock_from_wire`].
///
/// Deliberately NOT [`crate::compose::Lock`]: that type's `OlderBlocks(u16)`/
/// `OlderUnits(u16)`/`AfterHeight(u32)`/`AfterTime(u32)` payloads are the
/// *operator's* input range for composing a policy, capped where the spec
/// caps them (16 bits for the two `older` bands); a value read back off the
/// wire by this decode-side walk is not bound by that composer-input cap and
/// must fit whatever 32-bit operand a decoded `after`/`older` node actually
/// carries. Do not "fix" this duplication by merging the two — the narrower
/// type cannot hold every value the wider one must represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lock {
    /// Which of the four `after`/`older` bands this value denotes.
    pub kind: LockKind,
    /// The operand in the operator's unit for `kind` — blocks, 512-second
    /// units, a block height, or a Unix time.
    pub value: u32,
}

/// The operator's lock unit. The bare wire operand cannot say which of
/// these a value denotes on its own — `older`'s "units" flag lives in a bit
/// of the operand, and `after`'s height/time bands overlap on the wire
/// (`md/compose.go` §4c) — so a `Lock` always carries its kind explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKind {
    /// `after(h)` — a block height, 1..=499,999,999.
    AfterHeight,
    /// `after(t)` — a Unix time, 500,000,000..=2,147,483,647.
    AfterTime,
    /// `older(n)` — n blocks, 1..=65535.
    OlderBlocks,
    /// `older(0x400000 + u)` — u units of 512 seconds, 1..=65535.
    OlderUnits,
}

/// One independently satisfiable spend path: a tapscript leaf, or the whole
/// script for `wsh`/`sh`. Mirrors the fork's `Branch` (`md/policy_shape.go`
/// `Branch`), with `slots` in place of the discarded key-count-only field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// The threshold of the ONE `multi`/`sortedmulti`/`multi_a`/
    /// `sortedmulti_a` node this branch contains, at any depth, and only
    /// when that multi accounts for EVERY key the branch references. Zero
    /// means: no threshold node, more than one, or one that does not
    /// account for the branch's keys — it does NOT mean "1-of-1", which
    /// `slots.len() == 1` already reports.
    pub k: u8,
    /// The `n` half of `k`, under the same condition as `k`.
    pub n: u8,
    /// Which placeholder indices this branch references. Ascending,
    /// deduplicated. EXTENSION over the Go original, which keeps only
    /// `len()` of this (there as `Branch.Keys`).
    pub slots: Vec<u8>,
    /// Whether the threshold reported in `k`/`n` was the sorted spelling
    /// (`sortedmulti`/`sortedmulti_a`) rather than `multi`/`multi_a`.
    /// Meaningless when `k == 0`.
    pub sorted: bool,
    /// Every timelock this branch requires, in wire order. Empty means no
    /// timelock anywhere in the branch.
    pub locks: Vec<Lock>,
    /// Every hashlock this branch requires, in wire order, whatever its
    /// kind. Empty means no hashlock anywhere in the branch.
    pub hashlocks: Vec<HashLock>,
    /// Taptree depth of this leaf; 0 for wsh/sh.
    pub depth: u8,
}

/// Structural summary of one decoded policy. Mirrors the fork's
/// `PolicyShape` (`md/policy_shape.go:94-104`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyShape {
    /// The honesty contract, ported verbatim in meaning from the Go: FALSE
    /// means the walk met a node it could not classify, and NO part of this
    /// summary may be presented or keyed on.
    pub complete: bool,
    /// The taproot internal key's classification; [`KeyPathKind::NotTaproot`]
    /// for every non-`tr` root.
    pub key_path: KeyPathKind,
    /// Every independently satisfiable spend path, in the order the walk
    /// visited them.
    pub branches: Vec<Branch>,
    /// The deepest leaf's taptree depth; 0 for a non-taproot policy or a
    /// key-path-only `tr`.
    pub tap_depth: u8,
}

/// Walk a decoded descriptor's tree and summarize its structure. Never
/// panics on an unrecognized shape: that is reported as `complete = false`,
/// because "I do not understand this" is a result the caller must act on,
/// not an exception to swallow.
///
/// NOTE: `crate::`, never `md_codec::` — this crate does not self-alias
/// (`lib.rs` has no `extern crate self as md_codec`). Integration tests
/// under `tests/` are a separate crate and correctly say `md_codec::`.
pub fn policy_shape(d: &crate::encode::Descriptor) -> PolicyShape {
    let tree = &d.tree;
    let mut s = PolicyShape {
        complete: true,
        key_path: KeyPathKind::NotTaproot,
        branches: Vec::new(),
        tap_depth: 0,
    };
    match tree.tag {
        Tag::Tr => {
            let Body::Tr {
                is_nums,
                tree: inner_tree,
                ..
            } = &tree.body
            else {
                return incomplete();
            };
            s.key_path = if *is_nums {
                KeyPathKind::Nums
            } else {
                KeyPathKind::Xpub
            };
            if let Some(t) = inner_tree {
                walk_tap_tree(t, 1, &mut s);
            }
        }
        Tag::Wsh | Tag::Sh => {
            let Body::Children(children) = &tree.body else {
                return incomplete();
            };
            if children.len() != 1 {
                return incomplete();
            }
            let mut inner = &children[0];
            // sh(wsh(X)) — unwrap one more level so the branch describes the
            // script that actually runs, not the wrapper around it.
            if tree.tag == Tag::Sh && inner.tag == Tag::Wsh {
                let Body::Children(inner_children) = &inner.body else {
                    return incomplete();
                };
                if inner_children.len() != 1 {
                    return incomplete();
                }
                inner = &inner_children[0];
            }
            match split_branches(inner, 0) {
                Some(mut brs) => s.branches.append(&mut brs),
                None => return incomplete(),
            }
        }
        _ => {
            // wpkh/pkh and anything else: a single-key or unclassifiable root.
            match split_branches(tree, 0) {
                Some(mut brs) => s.branches.append(&mut brs),
                None => return incomplete(),
            }
        }
    }
    s
}

/// The zero-value `PolicyShape`: `complete = false` and nothing else filled
/// in, matching the Go original's bare `PolicyShape{}` return on refusal.
fn incomplete() -> PolicyShape {
    PolicyShape {
        complete: false,
        key_path: KeyPathKind::NotTaproot,
        branches: Vec::new(),
        tap_depth: 0,
    }
}

/// Descend a binary taptree, appending one [`Branch`] per LEAF. Every leaf is
/// visited: a summary that stopped early would hide a spend path.
fn walk_tap_tree(n: &Node, depth: u8, s: &mut PolicyShape) {
    if n.tag == Tag::TapTree {
        let Body::Children(children) = &n.body else {
            s.complete = false;
            return;
        };
        if children.len() != 2 {
            s.complete = false;
            return;
        }
        walk_tap_tree(&children[0], depth + 1, s);
        walk_tap_tree(&children[1], depth + 1, s);
        return;
    }
    if depth - 1 > s.tap_depth {
        s.tap_depth = depth - 1;
    }
    match split_branches(n, depth - 1) {
        Some(mut brs) => s.branches.append(&mut brs),
        None => s.complete = false,
    }
}

/// Turn a node into one [`Branch`] per ALTERNATIVE: `or_b`/`or_c`/`or_d`/
/// `or_i` are two alternatives each (recursively), `andor(X,Y,Z)` is `(X and
/// Y) or Z`, and anything else is one branch. `thresh(k,...)` with `k < n` is
/// also a set of alternatives but a combinatorial one; it stays one branch,
/// honestly described by [`collect`]. `None` propagates an unknown tag
/// exactly as the Go `branchOf` does.
fn split_branches(n: &Node, depth: u8) -> Option<Vec<Branch>> {
    match n.tag {
        Tag::OrB | Tag::OrC | Tag::OrD | Tag::OrI => {
            let Body::Children(children) = &n.body else {
                return None;
            };
            if children.len() != 2 {
                return None;
            }
            let mut left = split_branches(&children[0], depth)?;
            let mut right = split_branches(&children[1], depth)?;
            left.append(&mut right);
            Some(left)
        }
        Tag::AndOr => {
            let Body::Children(children) = &n.body else {
                return None;
            };
            if children.len() != 3 {
                return None;
            }
            // The X-and-Y half is summarized as a conjunction; the synthetic
            // node is never emitted or encoded, only walked.
            let xy = Node {
                tag: Tag::AndV,
                body: Body::Children(vec![children[0].clone(), children[1].clone()]),
            };
            let mut left = split_branches(&xy, depth)?;
            let mut right = split_branches(&children[2], depth)?;
            left.append(&mut right);
            Some(left)
        }
        _ => {
            let br = branch_of(n, depth)?;
            Some(vec![br])
        }
    }
}

/// Summarize ONE spend path. `None` means an unrecognised tag, which forces
/// `complete = false` upstream.
fn branch_of(n: &Node, depth: u8) -> Option<Branch> {
    let mut br = Branch {
        k: 0,
        n: 0,
        slots: Vec::new(),
        sorted: false,
        locks: Vec::new(),
        hashlocks: Vec::new(),
        depth,
    };
    let mut keys = BTreeSet::new();
    if !collect(n, &mut br, &mut keys) {
        return None;
    }
    // THE EXTENSION: keep the set, not just its size. Ascending and
    // deduplicated falls out of `BTreeSet`'s iteration order for free.
    br.slots = keys.into_iter().collect();

    // A bare threshold-over-keys, possibly wrapped: report k-of-n and
    // whether it was the sorted spelling.
    if let Some((k, nkeys, sorted)) = plain_multi(n) {
        br.k = k;
        br.n = nkeys;
        br.sorted = sorted;
    } else if let Some((k, nkeys, sorted)) = sole_multi(n) {
        // A THRESHOLD BEHIND A LOCK OR A HASH IS STILL A THRESHOLD. Two
        // conditions: the branch must contain EXACTLY ONE multi node (two
        // thresholds in one branch have no single k-of-n), AND that multi
        // must account for EVERY key the branch references (`nkeys ==
        // br.slots.len()`) — a branch holding a multi plus other key
        // material has no single k-of-n either, and reporting the multi's
        // would claim fewer signatures are needed than the branch actually
        // requires.
        if nkeys as usize == br.slots.len() {
            br.k = k;
            br.n = nkeys;
            br.sorted = sorted;
        }
    }
    Some(br)
}

/// Report the threshold of the ONE `multi`/`sortedmulti`/`multi_a`/
/// `sortedmulti_a` node a branch contains, at any depth. `None` when the
/// branch has none or more than one.
///
/// IT IS NOT SUFFICIENT ON ITS OWN: the caller also requires the multi's key
/// count to equal the branch's distinct-placeholder count, because a branch
/// can hold one multi and other key material beside it. "The one multi" and
/// "the branch's threshold" are different questions; this function answers
/// only the first.
fn sole_multi(n: &Node) -> Option<(u8, u8, bool)> {
    let mut found: u32 = 0;
    let mut result: Option<(u8, u8, bool)> = None;
    sole_multi_walk(n, &mut found, &mut result);
    if found != 1 {
        return None;
    }
    result
}

fn sole_multi_walk(n: &Node, found: &mut u32, result: &mut Option<(u8, u8, bool)>) {
    match n.tag {
        Tag::Multi | Tag::SortedMulti | Tag::MultiA | Tag::SortedMultiA => {
            *found += 1;
            if let Body::MultiKeys { k, indices } = &n.body {
                *result = Some((
                    *k,
                    indices.len() as u8,
                    matches!(n.tag, Tag::SortedMulti | Tag::SortedMultiA),
                ));
            } else {
                // A malformed body is not a threshold this can report; count
                // it again so `found != 1` still refuses.
                *found += 1;
            }
            return;
        }
        _ => {}
    }
    match &n.body {
        Body::Children(children) => {
            for c in children {
                sole_multi_walk(c, found, result);
            }
        }
        Body::Variable { children, .. } => {
            for c in children {
                sole_multi_walk(c, found, result);
            }
        }
        _ => {}
    }
}

/// Unwrap wrappers to find a bare `multi`/`sortedmulti`/`multi_a`/
/// `sortedmulti_a`. Anything else returns `None`, so k/n stay zero rather
/// than being invented for a shape that is not a plain threshold over keys.
fn plain_multi(n: &Node) -> Option<(u8, u8, bool)> {
    let mut cur = n;
    loop {
        match cur.tag {
            Tag::Multi | Tag::SortedMulti | Tag::MultiA | Tag::SortedMultiA => {
                let Body::MultiKeys { k, indices } = &cur.body else {
                    return None;
                };
                return Some((
                    *k,
                    indices.len() as u8,
                    matches!(cur.tag, Tag::SortedMulti | Tag::SortedMultiA),
                ));
            }
            Tag::Check
            | Tag::Verify
            | Tag::Swap
            | Tag::Alt
            | Tag::DupIf
            | Tag::NonZero
            | Tag::ZeroNotEqual => {
                let Body::Children(children) = &cur.body else {
                    return None;
                };
                if children.len() != 1 {
                    return None;
                }
                cur = &children[0];
            }
            _ => return None,
        }
    }
}

/// Walk one branch, recording key references and the presence of time/hash
/// locks. Returns `false` on ANY tag it does not know — the whole point of
/// [`PolicyShape::complete`].
fn collect(n: &Node, br: &mut Branch, keys: &mut BTreeSet<u8>) -> bool {
    match n.tag {
        Tag::PkK | Tag::PkH | Tag::Pkh | Tag::RawPkH | Tag::Wpkh => {
            if let Body::KeyArg { index } = &n.body {
                keys.insert(*index);
            }
            // A raw key hash (`Tag::RawPkH`) carries no placeholder; it is
            // still a known shape.
            true
        }
        Tag::Multi | Tag::SortedMulti | Tag::MultiA | Tag::SortedMultiA => {
            let Body::MultiKeys { indices, .. } = &n.body else {
                return false;
            };
            keys.extend(indices.iter().copied());
            true
        }
        Tag::After | Tag::Older => {
            let Body::Timelock(v) = &n.body else {
                return false;
            };
            br.locks.push(lock_from_wire(n.tag, *v));
            true
        }
        Tag::Sha256 | Tag::Hash256 => {
            let Body::Hash256Body(h) = &n.body else {
                return false;
            };
            let kind = if n.tag == Tag::Sha256 {
                HashKind::Sha256
            } else {
                HashKind::Hash256
            };
            br.hashlocks.push(HashLock::new(kind, *h));
            true
        }
        Tag::Ripemd160 | Tag::Hash160 => {
            let Body::Hash160Body(h) = &n.body else {
                return false;
            };
            let kind = if n.tag == Tag::Ripemd160 {
                HashKind::Ripemd160
            } else {
                HashKind::Hash160
            };
            // `HashLock::new` takes the full 32-byte alloc-gate slot; the
            // trailing 12 bytes are padding that `HashLock`'s own `digest()`
            // never exposes and its `PartialEq`/`Hash`/`Ord` never compare,
            // so zero-filling here is a courtesy (predictable `Debug` output)
            // rather than a correctness requirement.
            let mut digest = [0u8; 32];
            digest[..20].copy_from_slice(h);
            br.hashlocks.push(HashLock::new(kind, digest));
            true
        }
        Tag::True | Tag::False => true,
        Tag::Thresh => {
            let Body::Variable { children, .. } = &n.body else {
                return false;
            };
            children.iter().all(|c| collect(c, br, keys))
        }
        Tag::AndB
        | Tag::AndV
        | Tag::AndOr
        | Tag::OrB
        | Tag::OrC
        | Tag::OrD
        | Tag::OrI
        | Tag::Check
        | Tag::Verify
        | Tag::Swap
        | Tag::Alt
        | Tag::DupIf
        | Tag::NonZero
        | Tag::ZeroNotEqual
        | Tag::Wsh
        | Tag::Sh => {
            let Body::Children(children) = &n.body else {
                return false;
            };
            children.iter().all(|c| collect(c, br, keys))
        }
        // tagTr and tagTapTree cannot appear inside a branch; anything else
        // is a tag this walk has not been taught. Refuse rather than guess.
        Tag::Tr | Tag::TapTree => false,
    }
}

/// `operand`'s inverse: the kind and operator-unit value a decoded
/// `older`/`after` node denotes. `older` carries bit 22 for 512-second
/// units; `after` is a time at or above 500,000,000 and a height below it
/// (BIP-68 / BIP-65, the same split §4c's bands are built on).
fn lock_from_wire(tag: Tag, operand: u32) -> Lock {
    const SEQUENCE_TYPE_FLAG: u32 = 1 << 22;
    const LOCKTIME_THRESHOLD: u32 = 500_000_000;

    if tag == Tag::Older {
        if operand & SEQUENCE_TYPE_FLAG != 0 {
            return Lock {
                kind: LockKind::OlderUnits,
                value: operand & !SEQUENCE_TYPE_FLAG,
            };
        }
        return Lock {
            kind: LockKind::OlderBlocks,
            value: operand,
        };
    }
    if operand >= LOCKTIME_THRESHOLD {
        return Lock {
            kind: LockKind::AfterTime,
            value: operand,
        };
    }
    Lock {
        kind: LockKind::AfterHeight,
        value: operand,
    }
}
