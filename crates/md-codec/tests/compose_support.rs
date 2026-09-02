//! Helpers shared by the compose integration tests. Included with
//! `#[path = "compose_support.rs"] mod support;` from `compose_crosscheck.rs`
//! and `compose_vectors.rs`; cargo also compiles this file as a test binary of
//! its own (with no tests), hence the allows: the workspace lints `pub` items
//! for docs, and an unused helper in one includer is dead code in that binary.
#![allow(dead_code, missing_docs)]

use std::str::FromStr;

use md_codec::compose::{Composed, PathList, SlotOrigin, compose, compose_with};
use md_codec::encode::Descriptor;
use md_codec::origin_path::{OriginPath, PathComponent};
use md_codec::tag::Tag;
use miniscript::bitcoin::bip32::{ChildNumber, Xpub};
use miniscript::bitcoin::secp256k1::Secp256k1;

/// The wallet-policy journey's four cosigners: one master (73c5da0a) at
/// m/48'/0'/{0..3}'/2'. Real public keys; nothing here is a secret.
pub const XPUB: [&str; 4] = [
    "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf",
    "xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk",
    "xpub6EGx8sPr9FxPPE1rbZazhqWwpMXA3Hf5DYKtZbL7c4BSddzmQktp96UaTvecEkoCZysuaj79GMCFZYT1KKk7Ph2M3Kf5g8B82KZ8TZ9SKQR",
    "xpub6E6Z3Ss5TXJYNJp4U1q3NZ3pCn82i7KXQAKUtNnzLJ3cCdchQeSdFvXemizaHUF7wNwRQAB8mPdoZhGHLiv49cWPtCnoJY3Az3E8JKxH9Mq",
];
pub const FP: [u8; 4] = [0x73, 0xc5, 0xda, 0x0a];
pub const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
pub const H: [u8; 32] = [0xa8; 32];
pub const HH: &str = "a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8";

pub fn hardened(values: &[u32]) -> OriginPath {
    OriginPath {
        components: values
            .iter()
            .map(|v| PathComponent {
                hardened: true,
                value: *v,
            })
            .collect(),
    }
}

/// `n` DISTINCT xpubs: the journey's four for the first four slots, then
/// unhardened children of the first one. BIP-32 makes a (fingerprint, path)
/// pair name exactly ONE key, so binding one xpub at two declared origins would
/// describe an impossible wallet (descriptor-mnemonic F-217,
/// `corpus_origin_consistency.rs`); a 32-slot policy therefore needs 32 keys.
pub fn slot_xpubs(n: usize) -> Vec<Xpub> {
    let secp = Secp256k1::verification_only();
    let base = Xpub::from_str(XPUB[0]).expect("fixture xpub parses");
    (0..n)
        .map(|i| {
            if i < XPUB.len() {
                Xpub::from_str(XPUB[i]).expect("fixture xpub parses")
            } else {
                let child = ChildNumber::from_normal_idx(i as u32).expect("small index");
                base.derive_pub(&secp, &[child])
                    .expect("unhardened derivation")
            }
        })
        .collect()
}

/// 65 wire bytes (chain code ‖ compressed point).
pub fn xpub_bytes(x: &Xpub) -> [u8; 65] {
    let mut out = [0u8; 65];
    out[..32].copy_from_slice(&x.chain_code[..]);
    out[32..].copy_from_slice(&x.public_key.serialize());
    out
}

/// Seat every slot at `m/48'/0'/<slot>'/T'` under one master fingerprint and
/// bind distinct xpub bytes: a KEYED descriptor the converter can derive.
pub fn keyed(list: &PathList) -> Descriptor {
    let unseated = compose(list).expect("list is composable");
    let n = unseated.slots.len();
    let declared: Vec<Option<SlotOrigin>> = (0..n)
        .map(|i| {
            Some(SlotOrigin {
                origin: hardened(&[48, 0, i as u32, list.wrapper.script_type()]),
                fingerprint: Some(FP),
            })
        })
        .collect();
    let mut c = compose_with(list, &declared).expect("declared origins compose");
    let xs = slot_xpubs(n);
    c.descriptor.tlv.pubkeys = Some(
        xs.iter()
            .enumerate()
            .map(|(i, x)| (i as u8, xpub_bytes(x)))
            .collect(),
    );
    c.descriptor
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// The rust-miniscript CONCRETE policy with the same spend conditions as
/// `list`, over `keys` (one key string per emitted slot, in slot order). This is
/// the compiler's input for the §5b lift-equality leg; it is built from the path
/// list, not from the lowered tree, so it cannot inherit a lowering defect.
pub fn concrete_policy(list: &PathList, c: &Composed, keys: &[String]) -> String {
    let mut paths: Vec<String> = Vec::new();
    for (pi, p) in list.paths.iter().enumerate() {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ks) = p.keys {
            let pks: Vec<String> = c
                .slots
                .iter()
                .filter(|slot| slot.path == pi)
                .map(|slot| format!("pk({})", keys[usize::from(slot.index)]))
                .collect();
            parts.push(if ks.n == 1 {
                pks[0].clone()
            } else {
                format!("thresh({},{})", ks.k, pks.join(","))
            });
        }
        if let Some(h) = p.hash {
            parts.push(format!("sha256({})", hex(&h)));
        }
        if let Some(lock) = p.lock {
            let (tag, v) = lock.operand().expect("validated");
            let name = if matches!(tag, Tag::Older) {
                "older"
            } else {
                "after"
            };
            parts.push(format!("{name}({v})"));
        }
        let mut acc = parts.pop().expect("a path has a part");
        while let Some(x) = parts.pop() {
            acc = format!("and({x},{acc})");
        }
        paths.push(acc);
    }
    let mut acc = paths.pop().expect("a list has a path");
    while let Some(x) = paths.pop() {
        acc = format!("or({x},{acc})");
    }
    acc
}
