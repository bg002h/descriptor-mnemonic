//! P1.1 — `md decode` / `md inspect` partial-decode of pathless/dead-card
//! shapes (`canonical_origin == None`, no explicit origin resolvable).
//!
//! Per `design/SPEC_pathless_partial_decode.md` / `design/
//! IMPLEMENTATION_PLAN_pathless_partial_decode.md` (mnemonic-toolkit repo),
//! Phase P1. Consumes P0's md-codec partial-allowing decode API
//! (`decode_md1_string_with_opts` / `reassemble_with_opts` /
//! `Descriptor::unresolved_origin_indices`).
//!
//! Contract:
//!   - A `canonical_origin == None` shape with no explicit per-`@N` origin
//!     now DECODES (instead of hard-rejecting `MissingExplicitOrigin`):
//!     the template renders as usual, PLUS a text line
//!     `origin: «unspecified — supply on restore»`, PLUS a stderr note,
//!     exit **4**.
//!   - `md inspect` additionally OMITS the `wallet-policy-id` +
//!     `wallet-policy-id-fingerprint` lines (JSON: `wallet_policy_id` key)
//!     on partial — `compute_wallet_policy_id` must not even be called
//!     (it would error under partial), while `md1-encoding-id` +
//!     `wallet-descriptor-template-id` (no origin dependency) stay present.
//!   - `--json` gains an additive `partial: {"reason":
//!     "missing_explicit_origin", "unresolved_indices":[...]}` object.
//!   - Canonical (`canonical_origin == Some`) shapes are COMPLETELY
//!     UNCHANGED: exit 0, byte-identical text/JSON (BOUNDARY, RED-proof —
//!     any drift here is a regression, not a feature).

#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;

/// Run `md encode <extra_args.. > <template>` and return the first
/// (only, unbroken) stdout line — the md1 phrase.
fn encode(template: &str, extra_args: &[&str]) -> String {
    let mut args = vec!["encode", "--group-size", "0"];
    args.extend_from_slice(extra_args);
    args.push(template);
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(&args)
        .output()
        .expect("invoke md encode");
    assert!(
        out.status.success(),
        "encode {template:?} {extra_args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
}

/// Run `md encode --force-chunked <extra_args..> <template>` and return the
/// ordered `md1...` chunk strings.
fn encode_chunked(template: &str, extra_args: &[&str]) -> Vec<String> {
    let mut args = vec!["encode", "--force-chunked", "--group-size", "0"];
    args.extend_from_slice(extra_args);
    args.push(template);
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(&args)
        .output()
        .expect("invoke md encode --force-chunked");
    assert!(
        out.status.success(),
        "encode --force-chunked {template:?} {extra_args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("md1"))
        .map(String::from)
        .collect()
}

/// The exact text marker P1.1 specifies (must appear verbatim, partial only).
const ORIGIN_MARKER: &str = "origin: \u{ab}unspecified \u{2014} supply on restore\u{bb}";

// ─────────────────────────────────────────────────────────────────────────
// Dead-shape fixtures (canonical_origin == None), no --path supplied.
// One entry per SPEC-required golden class:
//   - tr(@0, pk(@1))            — tr + script tree
//   - sh(sortedmulti(2,@0,@1))  — legacy P2SH multisig
//   - wsh(pk(@0))               — bare wsh (single-key, non-multi inner)
//   - wsh(or_d(...))            — raw miniscript body
// ─────────────────────────────────────────────────────────────────────────
const DEAD_SHAPES: &[(&str, &str)] = &[
    ("tr_with_taptree", "tr(@0/<0;1>/*,pk(@1/<0;1>/*))"),
    (
        "sh_sortedmulti_legacy",
        "sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))",
    ),
    ("bare_wsh_single_key", "wsh(pk(@0/<0;1>/*))"),
    (
        "raw_miniscript_body",
        "wsh(or_d(pk(@0/<0;1>/*),and_v(v:pk(@1/<0;1>/*),older(144))))",
    ),
];

// ─── decode: text ───────────────────────────────────────────────────────

#[test]
fn decode_text_partial_renders_and_exits_4_for_every_dead_shape() {
    for (name, template) in DEAD_SHAPES {
        let phrase = encode(template, &[]);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["decode", &phrase])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(4),
            "[{name}] dead shape {template:?} must partial-decode with exit 4"
        );
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(
            stdout.contains(*template),
            "[{name}] template line missing; got {stdout:?}"
        );
        assert!(
            stdout.contains(ORIGIN_MARKER),
            "[{name}] origin-unspecified marker missing; got {stdout:?}"
        );
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            !stderr.is_empty(),
            "[{name}] expected a stderr note on partial decode"
        );
    }
}

// ─── decode: json ───────────────────────────────────────────────────────

#[cfg(feature = "json")]
#[test]
fn decode_json_partial_has_reason_and_nonempty_indices_for_every_dead_shape() {
    for (name, template) in DEAD_SHAPES {
        let phrase = encode(template, &[]);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["decode", &phrase, "--json"])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(4),
            "[{name}] json partial-decode must exit 4"
        );
        let stdout = String::from_utf8(out.stdout).unwrap();
        let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(
            v["partial"]["reason"], "missing_explicit_origin",
            "[{name}] partial.reason mismatch; got {v}"
        );
        let idxs = v["partial"]["unresolved_indices"]
            .as_array()
            .unwrap_or_else(|| panic!("[{name}] unresolved_indices must be an array; got {v}"));
        assert!(
            !idxs.is_empty(),
            "[{name}] unresolved_indices must be non-empty; got {v}"
        );
        // The raw path_decl must NOT be double-represented — still "m".
        assert_eq!(
            v["descriptor"]["path_decl"]["data"], "m",
            "[{name}] raw path_decl must stay elided \"m\"; got {v}"
        );
        assert_eq!(v["schema"], "md-cli/1");
    }
}

// ─── inspect: text ──────────────────────────────────────────────────────

#[test]
fn inspect_text_partial_omits_policy_id_lines_for_every_dead_shape() {
    for (name, template) in DEAD_SHAPES {
        let phrase = encode(template, &[]);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["inspect", &phrase])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(4),
            "[{name}] inspect on dead shape must exit 4"
        );
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(
            stdout.contains(&format!("template: {template}")),
            "[{name}] template line missing; got {stdout:?}"
        );
        assert!(
            stdout.contains(ORIGIN_MARKER),
            "[{name}] origin marker missing; got {stdout:?}"
        );
        assert!(
            stdout.contains("md1-encoding-id:"),
            "[{name}] md1-encoding-id must stay present (no origin dep); got {stdout:?}"
        );
        assert!(
            stdout.contains("wallet-descriptor-template-id:"),
            "[{name}] wallet-descriptor-template-id must stay present; got {stdout:?}"
        );
        assert!(
            !stdout.contains("wallet-policy-id:"),
            "[{name}] wallet-policy-id line must be OMITTED under partial; got {stdout:?}"
        );
        assert!(
            !stdout.contains("wallet-policy-id-fingerprint:"),
            "[{name}] wallet-policy-id-fingerprint line must be OMITTED under partial; got {stdout:?}"
        );
    }
}

// ─── inspect: json ──────────────────────────────────────────────────────

#[cfg(feature = "json")]
#[test]
fn inspect_json_partial_omits_wallet_policy_id_key_for_every_dead_shape() {
    for (name, template) in DEAD_SHAPES {
        let phrase = encode(template, &[]);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["inspect", &phrase, "--json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(4), "[{name}] must exit 4");
        let stdout = String::from_utf8(out.stdout).unwrap();
        let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            v.get("wallet_policy_id").is_none(),
            "[{name}] wallet_policy_id key must be OMITTED under partial; got {v}"
        );
        assert!(
            v.get("md1_encoding_id").is_some(),
            "[{name}] md1_encoding_id must stay present; got {v}"
        );
        assert!(
            v.get("wallet_descriptor_template_id").is_some(),
            "[{name}] wallet_descriptor_template_id must stay present; got {v}"
        );
        assert_eq!(
            v["partial"]["reason"], "missing_explicit_origin",
            "[{name}] partial.reason mismatch; got {v}"
        );
        assert!(
            !v["partial"]["unresolved_indices"]
                .as_array()
                .unwrap()
                .is_empty(),
            "[{name}] unresolved_indices must be non-empty; got {v}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// BOUNDARY (RED-proof): canonical shapes stay byte-identical / exit 0.
// Any drift here (e.g. accidentally emitting the origin marker, or
// dropping a policy-id line) is a REGRESSION, not a feature — this is the
// funds-relevant guard: canonical cards must not start reading as partial.
// ─────────────────────────────────────────────────────────────────────────
const CANONICAL_SHAPES: &[(&str, &str)] = &[
    ("tr_keypath_only", "tr(@0/<0;1>/*)"),
    ("wpkh_single_key", "wpkh(@0/<0;1>/*)"),
    ("sh_wpkh_nested", "sh(wpkh(@0/<0;1>/*))"),
    ("wsh_multi", "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))"),
];

#[test]
fn decode_text_canonical_shapes_stay_exit_0_and_never_show_origin_marker() {
    for (name, template) in CANONICAL_SHAPES {
        let phrase = encode(template, &[]);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["decode", &phrase])
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "[{name}] canonical shape {template:?} must stay exit 0"
        );
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(
            stdout.trim_end(),
            *template,
            "[{name}] canonical decode text must be BYTE-IDENTICAL to the template line alone"
        );
        assert!(
            !stdout.contains(ORIGIN_MARKER),
            "[{name}] canonical shape must NEVER show the origin-unspecified marker"
        );
    }
}

#[cfg(feature = "json")]
#[test]
fn decode_json_canonical_shapes_never_carry_partial_key() {
    for (name, template) in CANONICAL_SHAPES {
        let phrase = encode(template, &[]);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["decode", &phrase, "--json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "[{name}] must stay exit 0");
        let stdout = String::from_utf8(out.stdout).unwrap();
        let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            v.get("partial").is_none(),
            "[{name}] canonical shape JSON must NOT carry a `partial` key; got {v}"
        );
    }
}

#[test]
fn inspect_text_canonical_shapes_keep_all_policy_id_lines_and_exit_0() {
    for (name, template) in CANONICAL_SHAPES {
        let phrase = encode(template, &[]);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["inspect", &phrase])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "[{name}] must stay exit 0");
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(
            stdout.contains("wallet-policy-id:"),
            "[{name}] canonical shape must KEEP wallet-policy-id; got {stdout:?}"
        );
        assert!(
            stdout.contains("wallet-policy-id-fingerprint:"),
            "[{name}] canonical shape must KEEP wallet-policy-id-fingerprint; got {stdout:?}"
        );
        assert!(
            !stdout.contains(ORIGIN_MARKER),
            "[{name}] canonical shape must never show the origin marker"
        );
    }
}

#[cfg(feature = "json")]
#[test]
fn inspect_json_canonical_shapes_keep_wallet_policy_id_and_no_partial_key() {
    for (name, template) in CANONICAL_SHAPES {
        let phrase = encode(template, &[]);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["inspect", &phrase, "--json"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "[{name}] must stay exit 0");
        let stdout = String::from_utf8(out.stdout).unwrap();
        let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            v.get("wallet_policy_id").is_some(),
            "[{name}] canonical shape must KEEP wallet_policy_id; got {v}"
        );
        assert!(
            v.get("partial").is_none(),
            "[{name}] canonical shape must NOT carry a partial key; got {v}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Chunked (multi-chunk) dead card also partial-renders + exit 4 — proves
// the partial variant threads through `reassemble_with_opts`, not just the
// single-string `decode_payload_with_opts` path.
// ─────────────────────────────────────────────────────────────────────────

/// A dead shape (`wsh(thresh(...))` — Thresh is not a canonical
/// wsh-inner-multi tag) large enough (17 placeholders) to force
/// `--force-chunked` into 2+ chunks. No `--path` supplied.
fn multi_chunk_dead_template() -> String {
    let n = 17;
    let mut parts = vec!["pk(@0/<0;1>/*)".to_string()];
    for i in 1..n {
        parts.push(format!("s:pk(@{i}/<0;1>/*)"));
    }
    format!("wsh(thresh({n},{}))", parts.join(","))
}

#[test]
fn decode_text_multi_chunk_dead_card_partial_renders_and_exits_4() {
    let template = multi_chunk_dead_template();
    let chunks = encode_chunked(&template, &[]);
    assert!(
        chunks.len() >= 2,
        "fixture must force 2+ chunks; got {}: {chunks:?}",
        chunks.len()
    );
    let mut args = vec!["decode".to_string()];
    args.extend(chunks.iter().cloned());
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(&args)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(4),
        "multi-chunk dead card must partial-decode with exit 4"
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains(&template),
        "template line missing; got {stdout:?}"
    );
    assert!(
        stdout.contains(ORIGIN_MARKER),
        "origin marker missing; got {stdout:?}"
    );
}

#[cfg(feature = "json")]
#[test]
fn decode_json_multi_chunk_dead_card_partial_has_reason() {
    let template = multi_chunk_dead_template();
    let chunks = encode_chunked(&template, &[]);
    assert!(chunks.len() >= 2, "fixture must force 2+ chunks");
    let mut args = vec!["decode".to_string()];
    args.extend(chunks.iter().cloned());
    args.push("--json".to_string());
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(&args)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["partial"]["reason"], "missing_explicit_origin");
    assert!(
        !v["partial"]["unresolved_indices"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn inspect_multi_chunk_dead_card_partial_renders_and_exits_4() {
    let template = multi_chunk_dead_template();
    let chunks = encode_chunked(&template, &[]);
    assert!(chunks.len() >= 2, "fixture must force 2+ chunks");
    let mut args = vec!["inspect".to_string()];
    args.extend(chunks.iter().cloned());
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(&args)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains(ORIGIN_MARKER));
    assert!(!stdout.contains("wallet-policy-id:"));
}

/// Chunked-of-one dead card (single `md1...` string bearing a chunk header,
/// e.g. from `md encode --force-chunked` on a small dead shape) also
/// partial-decodes via the `decode_md1_string`-with-opts auto-dispatch path
/// (chunked_flag branch), not just the multi-arg `reassemble` path.
#[test]
fn decode_chunked_of_one_dead_card_partial_renders_and_exits_4() {
    let chunks = encode_chunked("sh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))", &[]);
    assert_eq!(
        chunks.len(),
        1,
        "small dead shape --force-chunked should still be a single chunk; got {chunks:?}"
    );
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &chunks[0]])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains(ORIGIN_MARKER));
}
