//! md thresh / seedhammer I-1: the shared shape-vector file
//! `fixtures/policy_shape/cases.json` (DESIGN_thresh_summary.md section 6,
//! review I-2), which the seedhammer Go port vendors and compares against.
//!
//! GENERATED, never edited by hand. This test regenerates it and fails when
//! the committed file differs; `MD_REGEN_POLICY_SHAPE_CASES=1` rewrites it.
//!
//! Each case: `name`; `template` (text for the reader, `null` for a
//! hand-built tree); `tree` in the node schema of the vendored
//! `.descriptor.json` files (`tag`, `body.kind`, `body.data`); `md1`, the
//! card as chunk strings with an explicit origin (`m/48'/0'/0'/2'` for the
//! hand-built trees), or `null` with `md1_absent` saying why no card can
//! carry the tree; `typed`, whether the tree is typed miniscript (BIP-379's
//! B/V/K/W and z/o/n/d/u rules, in its script context); and `shape`, Rust's
//! `policy_shape`. `key_path_entry` marks the entry Rust adds for a spendable
//! taproot internal key.
//!
//! `shape` is Rust's own output, so this is a parity file: a port compares
//! against Rust, not against the design table (md review r2 n-2). The
//! independent expectations are `policy_shape_thresh.rs` and the verdict test
//! below.
//!
//! md reviews r2 I-1 and r3: a tree that is not valid miniscript in its
//! context (`policy_type_checks`'s doc lists every rule: types, B script
//! roots including each tap leaf, timelock ranges, fragment placement, script
//! and satisfaction limits) has no summary (`complete` false) and no key,
//! whatever the allowlist says. The `untyped_*` cases are such trees, one or
//! more per rule, plus the r2 review's two cards and the r3 review's card; `typed: false` marks that a
//! port needs that check to get them right. Several `thresh_neg_*` cases are
//! also `typed: false`.

mod common;

use common::{UNTYPED_CARDS, keyarg, node2, thresh_node, timelock, untyped_trees, wrap};
use md_codec::chunk::{reassemble, split, split_unadmitted};
use md_codec::descriptor_route::descriptor_from_text;
use md_codec::encode::Descriptor;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::policy_shape::{KeyPathKind, PolicyShape, policy_shape, policy_type_checks};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, InternalKey, Node};
use md_codec::use_site_path::UseSitePath;
use serde_json::{Value, json};

const MINTED: &str = include_str!("fixtures/policy_shape/minted.tsv");
const CASES: &str = "tests/fixtures/policy_shape/cases.json";

fn hex(b: &[u8]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::new(), |mut out, x| {
        let _ = write!(out, "{x:02x}");
        out
    })
}

/// md-cli's `JsonNode` schema (`format/json.rs`); `md_cli_reads_the_same_tree`
/// in md-cli checks the two agree on every case that has a card.
fn node_json(n: &Node) -> Value {
    let body = match &n.body {
        Body::KeyArg { index } => json!({"kind": "KeyArg", "data": {"index": index}}),
        Body::Children(c) => {
            json!({"kind": "Children", "data": c.iter().map(node_json).collect::<Vec<_>>()})
        }
        Body::Variable { k, children } => json!({"kind": "Variable", "data": {
            "k": k, "children": children.iter().map(node_json).collect::<Vec<_>>()}}),
        Body::MultiKeys { k, indices } => {
            json!({"kind": "MultiKeys", "data": {"k": k, "indices": indices}})
        }
        Body::Tr { internal_key, tree } => {
            let (is_nums, key_index, kind) = match internal_key {
                InternalKey::Slot(i) => (false, *i, None),
                InternalKey::NumsPoint => (true, 0, None),
                InternalKey::LianaUnspendable => (true, 0, Some("liana_unspendable")),
            };
            let mut data = json!({"is_nums": is_nums, "key_index": key_index,
                "tree": tree.as_ref().map(|t| node_json(t))});
            if let Some(k) = kind {
                data["unspendable_kind"] = json!(k);
            }
            json!({"kind": "Tr", "data": data})
        }
        Body::Hash256Body(h) => json!({"kind": "Hash256Body", "data": hex(h)}),
        Body::Hash160Body(h) => json!({"kind": "Hash160Body", "data": hex(h)}),
        Body::Timelock(v) => json!({"kind": "Timelock", "data": v}),
        Body::Empty => json!({"kind": "Empty"}),
    };
    json!({"tag": format!("{:?}", n.tag), "body": body})
}

fn shape_json(s: &PolicyShape) -> Value {
    let branches: Vec<Value> = s
        .branches
        .iter()
        .enumerate()
        .map(|(i, b)| {
            json!({
                "key_path_entry": i == 0 && s.key_path == KeyPathKind::Xpub,
                "any_of": b.any_of.map(|a| json!({"k": a.k, "n": a.n})),
                "k": b.k,
                "n": b.n,
                "sorted": b.sorted,
                "slots": b.slots,
                "locks": b.locks.iter().map(|l| json!({"kind": format!("{:?}", l.kind), "value": l.value})).collect::<Vec<_>>(),
                "hashlocks": b.hashlocks.len(),
                "depth": b.depth,
                "min_keys": b.any_of_min_keys(),
            })
        })
        .collect();
    json!({
        "complete": s.complete,
        "key_path": format!("{:?}", s.key_path),
        "tap_depth": s.tap_depth,
        "branches": branches,
    })
}

// ─── hand-built trees (DESIGN section 6; the same as policy_shape_thresh.rs) ─

fn f() -> Node {
    Node {
        tag: Tag::False,
        body: Body::Empty,
    }
}
fn pk(i: u8) -> Node {
    keyarg(Tag::PkK, i)
}
fn pkh(i: u8) -> Node {
    keyarg(Tag::PkH, i)
}
fn c(x: Node) -> Node {
    wrap(Tag::Check, x)
}
fn s(x: Node) -> Node {
    wrap(Tag::Swap, x)
}
fn a(x: Node) -> Node {
    wrap(Tag::Alt, x)
}
fn v(x: Node) -> Node {
    wrap(Tag::Verify, x)
}
fn n(x: Node) -> Node {
    wrap(Tag::ZeroNotEqual, x)
}
fn j(x: Node) -> Node {
    wrap(Tag::NonZero, x)
}
fn d(x: Node) -> Node {
    wrap(Tag::DupIf, x)
}
fn l(x: Node) -> Node {
    node2(Tag::OrI, f(), x)
}
fn u(x: Node) -> Node {
    node2(Tag::OrI, x, f())
}
fn older(x: u32) -> Node {
    timelock(Tag::Older, x)
}
fn after(x: u32) -> Node {
    timelock(Tag::After, x)
}
fn wsh(x: Node) -> Node {
    wrap(Tag::Wsh, x)
}
fn th(k: u8, children: Vec<Node>) -> Node {
    thresh_node(k, children)
}
fn two_of(third: Node) -> Node {
    th(2, vec![pk(0), s(pk(1)), third])
}
fn three() -> Node {
    th(2, vec![pk(0), s(pk(1)), s(pk(2))])
}

fn hand_built() -> Vec<(&'static str, Node)> {
    let sha = Node {
        tag: Tag::Sha256,
        body: Body::Hash256Body([7u8; 32]),
    };
    let nums_leaf = |x: Node| Node {
        tag: Tag::Tr,
        body: Body::Tr {
            internal_key: InternalKey::NumsPoint,
            tree: Some(Box::new(x)),
        },
    };
    vec![
        (
            "thresh_locks_only_1of2",
            wsh(th(1, vec![l(n(older(10))), s(u(n(older(20))))])),
        ),
        (
            "thresh_mixed_after_kinds",
            wsh(th(
                2,
                vec![pk(0), s(l(n(after(800000)))), s(l(n(after(1700000000))))],
            )),
        ),
        (
            "thresh_zero_delay_units_lock",
            wsh(two_of(s(l(n(older(0x0040_0000)))))),
        ),
        (
            "thresh_check_wrapped_keys",
            wsh(th(2, vec![c(pk(0)), s(c(pk(1))), a(c(pkh(2)))])),
        ),
        (
            "thresh_kn_under_and",
            wsh(node2(Tag::AndV, v(pk(2)), th(2, vec![pk(0), s(pk(1))]))),
        ),
        ("thresh_neg_av_pk", wsh(two_of(a(v(pk(2)))))),
        ("thresh_neg_sv_pk", wsh(two_of(s(v(pk(2)))))),
        (
            "thresh_neg_sl_older",
            wsh(th(3, vec![pk(0), s(pk(1)), s(pk(2)), s(l(older(2)))])),
        ),
        ("thresh_neg_su_older", wsh(two_of(s(u(older(2)))))),
        ("thresh_neg_a_older", wsh(two_of(a(older(2))))),
        ("thresh_neg_an_older", wsh(two_of(a(n(older(2)))))),
        ("thresh_neg_slj_older", wsh(two_of(s(l(j(older(144))))))),
        ("thresh_neg_sjl_older", wsh(two_of(s(j(l(older(144))))))),
        ("thresh_neg_aj_pk", wsh(two_of(a(j(pk(2)))))),
        ("thresh_neg_an_pk", wsh(two_of(a(n(pk(2)))))),
        ("thresh_neg_al_pk", wsh(two_of(a(l(pk(2)))))),
        ("thresh_neg_s_pkh", wsh(two_of(s(pkh(2))))),
        (
            "thresh_neg_s_first",
            wsh(th(2, vec![s(pk(0)), s(pk(1)), s(pk(2))])),
        ),
        (
            "thresh_neg_bare_second",
            wsh(th(2, vec![pk(0), pk(1), s(pk(2))])),
        ),
        ("thresh_neg_adv_older_wsh", wsh(two_of(a(d(v(older(2))))))),
        (
            "thresh_neg_adv_older_tr",
            nums_leaf(two_of(a(d(v(older(2)))))),
        ),
        ("thresh_neg_ss_pk", wsh(two_of(s(s(pk(2)))))),
        ("thresh_neg_sc_c_pk", wsh(two_of(s(c(c(pk(2))))))),
        ("thresh_neg_older_zero", wsh(two_of(s(l(n(older(0))))))),
        (
            "thresh_neg_older_bit31",
            wsh(two_of(s(l(n(older(1 << 31)))))),
        ),
        (
            "thresh_neg_ori_two_zero",
            wsh(two_of(s(node2(Tag::OrI, f(), f())))),
        ),
        ("thresh_neg_hash_child_bare", wsh(two_of(a(sha)))),
        ("thresh_neg_root_alt", wsh(a(three()))),
        ("thresh_neg_root_verify", wsh(v(three()))),
        (
            "thresh_neg_or_b_first_alt",
            wsh(node2(Tag::OrB, a(three()), a(pk(3)))),
        ),
        (
            "thresh_neg_or_d_alt",
            wsh(node2(Tag::OrD, pk(3), a(three()))),
        ),
        (
            "thresh_neg_kn_av_pk",
            wsh(th(3, vec![pk(0), s(pk(1)), a(v(pk(2)))])),
        ),
        (
            "thresh_neg_kn_sl_older",
            wsh(th(3, vec![pk(0), s(pk(1)), s(l(older(2)))])),
        ),
        (
            "thresh_neg_kn_under_and_av_pk",
            wsh(node2(
                Tag::AndV,
                v(pk(3)),
                th(3, vec![pk(0), s(pk(1)), a(v(pk(2)))]),
            )),
        ),
        (
            "thresh_neg_dup_slot",
            wsh(th(2, vec![pk(0), s(pk(0)), s(pk(1))])),
        ),
        ("thresh_neg_k_zero", wsh(th(0, vec![pk(0), s(pk(1))]))),
        ("thresh_neg_k_above_n", wsh(th(3, vec![pk(0), s(pk(1))]))),
    ]
}

/// A keyless template with the explicit shared origin `m/48'/0'/0'/2'`.
fn template_card(tree: Node) -> Descriptor {
    let (tree, n) = common::renumbered(tree);
    Descriptor {
        n,
        path_decl: PathDecl {
            n,
            paths: PathDeclPaths::Shared(OriginPath {
                components: [48, 0, 0, 2]
                    .into_iter()
                    .map(|value| PathComponent {
                        hardened: true,
                        value,
                    })
                    .collect(),
            }),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree,
        tlv: TlvSection::new_empty(),
    }
}

/// When a card exists, `tree` and `shape` are read back from it: the encoder
/// writes placeholders in first-occurrence order, so the card's tree can
/// differ from the one built (`thresh_kn_under_and`), and the Go reader sees
/// the card's.
fn case(
    name: &str,
    template: Option<&str>,
    d: &Descriptor,
    md1: Result<Vec<String>, String>,
) -> Value {
    let (d, md1, absent) = match md1 {
        Ok(chunks) => {
            let refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
            let back = reassemble(&refs).unwrap_or_else(|e| panic!("{name}: {e}"));
            (back, json!(chunks), Value::Null)
        }
        Err(why) => (d.clone(), Value::Null, json!(why)),
    };
    json!({
        "name": name,
        "template": template,
        "tree": node_json(&d.tree),
        "md1": md1,
        "md1_absent": absent,
        "typed": policy_type_checks(&d),
        "shape": shape_json(&policy_shape(&d)),
    })
}

fn generate() -> String {
    let mut cases = Vec::new();
    for line in MINTED.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        let d = descriptor_from_text(fields[2]).unwrap_or_else(|e| panic!("{}: {e}", fields[0]));
        let md1 = split(&d).map_err(|e| e.to_string());
        cases.push(case(fields[0], Some(fields[1]), &d, md1));
    }
    for (name, card) in UNTYPED_CARDS {
        let d = md_codec::decode::decode_md1_string(card).unwrap_or_else(|e| panic!("{name}: {e}"));
        cases.push(case(name, None, &d, Ok(vec![card.to_string()])));
    }
    let untyped = untyped_trees()
        .into_iter()
        .map(|(name, tree)| (format!("untyped_{name}"), tree));
    let built = hand_built()
        .into_iter()
        .map(|(name, tree)| (name.to_string(), tree));
    for (name, tree) in built.chain(untyped) {
        let name = name.as_str();
        let d = template_card(tree);
        let md1 = if d.n == 0 {
            Err("no placeholder: an md1 card holds at least one".to_string())
        } else {
            split_unadmitted(&d).map_err(|e| e.to_string())
        };
        cases.push(case(name, None, &d, md1));
    }
    let mut out = serde_json::to_string_pretty(&json!({"format": 1, "cases": cases})).unwrap();
    out.push('\n');
    out
}

#[test]
fn policy_shape_cases_json_is_fresh() {
    let want = generate();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CASES);
    if std::env::var_os("MD_REGEN_POLICY_SHAPE_CASES").is_some() {
        std::fs::write(&path, &want).unwrap();
    }
    let got = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        got == want,
        "{CASES} is stale: run MD_REGEN_POLICY_SHAPE_CASES=1 cargo test -p md-codec --test policy_shape_cases"
    );
}

/// Every case of DESIGN section 6 is in the file, and its shape is the one
/// `policy_shape_thresh.rs` asserts: any-of entries for the in-scope names,
/// incomplete for every `thresh_neg_*`; every `untyped_*` is untyped and
/// incomplete, and every complete case is typed.
#[test]
fn every_design_case_is_present_with_its_verdict() {
    let v: Value = serde_json::from_str(&generate()).unwrap();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 30 + 37 + 3 + 29);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let complete = c["shape"]["complete"].as_bool().unwrap();
        let has_any_of = c["shape"]["branches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| !b["any_of"].is_null());
        let typed = c["typed"].as_bool().unwrap();
        assert!(typed || !complete, "{name}: complete but untyped");
        if name.starts_with("untyped_") {
            assert!(!typed && !complete, "{name}");
        } else if name.starts_with("thresh_neg_") {
            assert!(!complete, "{name}");
        } else if name.starts_with("thresh_kn_") {
            assert!(complete && !has_any_of, "{name}");
        } else {
            assert!(complete && has_any_of, "{name}");
        }
    }
}
