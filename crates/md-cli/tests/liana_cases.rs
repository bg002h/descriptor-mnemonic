//! `md-cli`'s OWN reader for md-codec's vendored Liana-evidence fixture
//! (`crates/md-codec/tests/fixtures/liana/cases.json`). `case`/`Case` in
//! md-codec's own test root are `include!`d via `env!("CARGO_MANIFEST_DIR")`,
//! which resolves to md-cli's own directory from a file living HERE and
//! finds nothing there — the third time this exact class has bitten (r5/I-1
//! on `kind1_from_vector`, r4/I-2 on the same, now `case`) — so md-cli gets
//! its OWN loader over the SAME committed JSON: one fixture, two readers.
//!
//! Pulled into `liana_input_side.rs` (and, per the plan's own task-order
//! ruling, Task 8's later additions to that SAME file) as a genuine nested
//! MODULE — `#[path = "liana_cases.rs"] mod liana_cases;` — not `include!`:
//! this file is also auto-discovered by cargo as its own (test-less)
//! integration-test binary, like every top-level `tests/*.rs` file, and an
//! `include!`-spliced copy cannot carry this very module doc comment (inner
//! attributes/doc comments are only legal at the true start of the file
//! containing them; `common/facts.rs`, this crate's existing precedent for
//! the identical problem, uses the same `#[path]` module form for the same
//! reason). `pub` throughout, matching `common/facts.rs`'s own convention:
//! a `#[path]` module's items are private to it by default.
#![allow(missing_docs)]

/// One vendored Liana-evidence case — md-cli's slice of the fields its tests
/// shell out to `md` and check: `crates/md-codec/tests/common/liana.rs`'s own
/// (private, differently-shaped) `Case` also carries `leaf_tlv_hex`/
/// `leaf_pubkeys_hex`/`source_commit`, none of which any md-cli test touches
/// (serde ignores unknown JSON fields by default, so this narrower struct
/// still deserializes the same committed file).
///
/// `#[allow(dead_code)]`: not every consumer of `case()` reads every field
/// (this task's own tests read only `descriptor_with_checksum` and
/// `expected_xpub`) — the same reasoning md-codec's `common/liana.rs` states
/// on its own `Case`.
#[allow(dead_code)]
#[derive(serde::Deserialize, Clone)]
pub struct Case {
    pub name: String,
    pub accepted: bool,
    pub descriptor_with_checksum: String,
    pub expected_xpub: String,
    pub liana_receive: Vec<String>,
    pub liana_change: Vec<String>,
}

pub fn case(name: &str) -> Case {
    let raw = include_str!("../../md-codec/tests/fixtures/liana/cases.json");
    serde_json::from_str::<Vec<Case>>(raw)
        .expect("cases.json")
        .into_iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no vendored case {name}"))
}

/// A real `tr` whose internal key is a SPENDABLE xpub with NO recorded
/// origin — G-8: not Liana's recipe (it does not byte-match SPEC §2's
/// recomputation), not NUMS, and `md decompose` must keep TODAY'S
/// annotated-slot behaviour for it. The phantom property G-8 names is NO
/// ORIGIN, not "not Liana's" — refusing it would also lock out libnunchuk's
/// PR-1746 form and real origin-less spendable keys.
///
/// Built from `cases.json`'s own `preset-kofn-recovery-tr` key material: its
/// recorded internal-key xpub (origin stripped — the shape under test) as
/// THIS tr's internal key, and one of its own leaf keys (kept WITH its
/// origin) as the sole script-path leaf. Not the brief's own worked example:
/// that constant is truncated and does not parse (its own text says so) —
/// this one is real, vendored key material, and parses.
pub const ORIGINLESS_SPENDABLE_TR: &str = concat!(
    "tr(xpub6DXuQW1Q2JpZyweiMewTZuMPvjG8hKhV2qoF6wL9VFxsMBExtbfqAAoR4oMG4GyxFzVdfas1v2eAdfLxyjc4Ceo5B6w6zTpf7F2BuXCJ52i/<0;1>/*,",
    "and_v(v:pk([73c5da0a/48'/0'/1'/3']xpub6DXuQW1Q2JpZzLV9igdwdnmCSoaPVd4ZNZnvfgUsGvQ8AbNAhEmfBEMCMHctwZBuxWK8HkjqUW5F72MCSJCFfisVwRY62Kb1FuDZ66nNQe1/<0;1>/*),older(26280)))"
);
