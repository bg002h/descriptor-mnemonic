# v0.18 Phase 3 — Item B NUMS sentinel wire-format change (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

The load-bearing wire-format break of the v0.18 cycle. Retire `Tag::TrUnspendable` (extension sub-code `0x05`, shipped in v0.17.0 only) in favor of a sentinel rule on `Tag::Tr`: `key_index == n` (one past the highest placeholder index) signals that the implicit internal key is the BIP-341 NUMS H-point `50929b74...e803ac0`. Saves 3-4 bits per usage (one bech32 char at common alignments).

## Wire-format consequences

- `key_index_width` formula moved from `⌈log₂(n)⌉` (with special-case 0 at n=1) to `⌈log₂(n+1)⌉` uniformly. Equivalent Rust expression: `(32 - (n as u32).leading_zeros()) as u8`.
- At n=1 width grows 0→1; at n=2 width grows 1→2; at n=3 unchanged; at n=4 width grows 2→3. The +1 reserves room for the sentinel `key_index = n` value.
- Strict break vs v0.17. v0.17 phrases (e.g. `md1qqpqqxqxkceprx7rap4t` for `wpkh(@0/<0;1>/*)`) no longer round-trip under v0.18; encountering a v0.17 `0x1F 0x05` tag now yields `Error::UnknownExtensionTag(0x05)`.

## Architect-round-1 critical findings addressed

- **C1 (lockstep formula):** the new `bit_length(n)` formula lives in BOTH `crates/md-codec/src/encode.rs::Descriptor::key_index_width` AND `crates/md-codec/src/decode.rs::decode_payload`, with comments in each pointing at the other. Phase 3 step 6's `tr_sentinel_n_1_bare_round_trip` boundary test would have caught silent desync if only one had been updated.
- **C2 (lockstep placeholder bound):** the `>= n` rejection check was loosened to `> n` in BOTH `crates/md-codec/src/validate.rs::walk_for_placeholders` AND `crates/md-codec/src/canonicalize.rs::check_placeholder_bounds`. Both sites now accept `key_index = n` (sentinel) and reject `key_index = n+1`.

## Architect-round-1 important findings addressed

- **I1 (v017_v1_c canary):** `v017_v1_c_nums_internal_key_encodes_via_tr_unspendable` renamed to `v018_v1_c_nums_internal_key_encodes_via_sentinel`; doc-comment now references the new mechanism with v0.17 history preserved.
- **I2 (render_node n threading):** `n: u8` threaded through `render_node`, `render_wrapper`, `render_tap_node` from `descriptor_to_template`. `render_multi` left unchanged (it doesn't recurse via render_node, only via render_key). String-compare-against-NUMS-hex anti-pattern rejected; sentinel detection is the structurally clean `key_index == n` check.

## Artifacts

### md-codec changes

- `tag.rs`: drop `Tag::TrUnspendable` enum variant + write/read arms. Update module header (5 ext ops, was 6). Update `tag_unknown_extension_rejected` test to assert `UnknownExtensionTag(0x05)` (next-free again).
- `tree.rs`: drop `Body::TrUnspendable` variant + write/read arms; expand `Body::Tr` doc-comment with the sentinel rule. Rewrite three v0.17 round-trip tests (`tr_unspendable_*`) as `tr_sentinel_n_{1,2,3}_*`. Add `tr_sentinel_n_4_bare_round_trip` boundary test. Update `tr_bip86_no_tree` comment to clarify it's a synthetic width-0 unit test, not a live-n=1 case (reviewer I1 mitigation).
- `encode.rs`: formula updated; doc-comment names lockstep-with-decode requirement.
- `decode.rs`: independent copy of formula updated; comment names silent-desync risk if formulas drift.
- `validate.rs`: `walk_for_placeholders` `>= n` → `> n`; sentinel passes through (don't register, don't reject). Drop `Tag::TrUnspendable` from `is_forbidden_leaf_tag`. Drop `tap_tree_leaf_rejects_tr_unspendable` test (variant gone). Update `placeholder_usage_rejects_out_of_range_in_tr_key_index` (now uses `key_index = 4` with n=3, since 3 is valid sentinel). Add `placeholder_usage_accepts_nums_sentinel_in_tr_key_index` for positive coverage.
- `canonicalize.rs`: three sites updated — `walk_collect_first` doc-comment (sentinel handled by `seen.get_mut` no-op-on-out-of-range); `remap_indices` adds `(*key_index as usize) < perm.len()` guard before remap (prevents panic on sentinel); `check_placeholder_bounds` `>= n` → `> n`.
- `tests/smoke.rs::bip84_single_sig_payload_bit_count`: 57 → 58 bits (n=1 width changed 0 → 1).

### md-cli changes

- `parse/template.rs::walk_tr`: emit `Tag::Tr { key_index = km.len() as u8, .. }` (sentinel) when internal key is NUMS hex; doc-comment of `NUMS_H_POINT_X_ONLY_HEX` const updated to v0.18 mechanism. Two existing NUMS walker tests rewritten to assert sentinel form.
- `format/text.rs::render_node`: signature gains `n: u8` parameter; `Tag::Tr` arm renders NUMS hex when `key_index == n`. `render_wrapper` and `render_tap_node` cascade. `Tag::TrUnspendable` arm deleted.
- `format/json.rs`: drop `JsonBody::TrUnspendable` variant; expand `JsonBody::Tr` doc-comment with sentinel rule.
- `main.rs`: `--help` EXAMPLES blocks updated for both encode and decode (v0.17 phrase no longer round-trips).
- `tests/smoke.rs::encode_wpkh_default_phrase`: pinned phrase updated `md1qqpqqxqxkceprx7rap4t` → `md1qqpqqxqq0zkd22pw8dmd3`, with comment naming the wire-format break.
- `tests/v017_v1_encode_acceptance.rs`: rename `v017_v1_c_*` → `v018_v1_c_*`; doc-comment updated.

### Snapshot + corpus regeneration (expected wire-format-break consequences)

- 4 inspect-* JSON snapshots: only `md1_encoding_id` / `wallet_descriptor_template_id` / `wallet_policy_id` hash drift. Auto-updated via `INSTA_UPDATE=always`.
- 7 vector corpus files (`crates/md-codec/tests/vectors/*.phrase.txt` / `.bytes.hex`): regenerated via `md vectors --out crates/md-codec/tests/vectors/`. Pinned smoke test (`encode_wpkh_default_phrase`) and bit-count test (`bip84_single_sig_payload_bit_count`) cross-validate the regen.

## Verification

- `cargo build -p md-codec` clean.
- `cargo build -p md-cli --features cli-compiler` clean.
- `cargo test --workspace --all-features` → 401 pass (same as Phase 2 baseline; net 0 because dropped `tap_tree_leaf_rejects_tr_unspendable` + `tag_tr_unspendable_extension` cancelled with new sentinel-boundary tests + `placeholder_usage_accepts_nums_sentinel_in_tr_key_index`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0 / I: 1 / L: 0**
- **I1** — `tr_bip86_no_tree` test comment said "n=1" but post-v0.18 width=1 at n=1; the test calls `write_node(&mut w, &n, 0)` directly which is now a synthetic width-0 unit test of the tree layer, not the live n=1 case. **Fixed inline** — comment clarified to say "synthetic width=0 unit test ... v0.18 Descriptor formula gives width=1 at n=1".
- All 8 review-focus areas passed: C1 formula equivalence (verified at n=0,1,2,3,4,7,8,15,16,32); C2 lockstep (`> n` in both sites with identical thresholds); `remap_indices` sentinel guard prevents panic; `walk_collect_first` correctly skips sentinel via `seen.get_mut` no-op; cross-crate `n` consistency (`km.len() == descriptor.n`); render-side n threading (no missed sites); vector corpus cross-validated by pinned smoke tests.

Net: 0C/0I after I1 fix.

## Exit gate

- ✅ `Tag::TrUnspendable` and `Body::TrUnspendable` removed from md-codec.
- ✅ `key_index_width` formula updated in BOTH encode.rs and decode.rs (architect C1 resolved).
- ✅ Placeholder-bounds `> n` in BOTH validate.rs and canonicalize.rs (architect C2 resolved).
- ✅ `remap_indices` and `walk_collect_first` correctly handle sentinel.
- ✅ md-cli walk_tr emits sentinel; render_node detects sentinel via threaded `n`; JSON variant dropped.
- ✅ v0.17 V1.c canary renamed (architect I1 resolved).
- ✅ 4 boundary tests pin sentinel at n=1, 2, 3, 4 (n=1 would have caught the C1 desync risk).
- ✅ Workspace tests + clippy clean (401 tests).
- ✅ Per-phase reviewer 0C/0I after inline I1 fix.

Phase 3 closed; proceeding to Phase 4 (Item A — full miniscript walker coverage; 17 walker arms + 19 render arms).
