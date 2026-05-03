# Phase 0 audit — md-cli extraction

Date: 2026-05-03
Branch: feat/md-cli-extraction
Spec: design/SPEC_md_codec_v0_16_library_only.md (commits 87f2cf7 + 479e4b0)

## API audit

### Imports detected in `crates/md-codec/src/bin/md/`

Source: `grep -rEn "^use md_codec::" crates/md-codec/src/bin/md/ | sort -u` (38 hits, 19 distinct import lines after de-duping by path).

```
crates/md-codec/src/bin/md/cmd/address.rs:4:    use md_codec::decode::decode_md1_string;
crates/md-codec/src/bin/md/cmd/address.rs:5:    use md_codec::chunk::reassemble;
crates/md-codec/src/bin/md/cmd/address.rs:6:    use md_codec::encode::Descriptor;
crates/md-codec/src/bin/md/cmd/bytecode.rs:2:   use md_codec::decode::decode_md1_string;
crates/md-codec/src/bin/md/cmd/bytecode.rs:3:   use md_codec::chunk::reassemble;
crates/md-codec/src/bin/md/cmd/bytecode.rs:4:   use md_codec::encode::encode_payload;
crates/md-codec/src/bin/md/cmd/decode.rs:3:     use md_codec::decode::decode_md1_string;
crates/md-codec/src/bin/md/cmd/decode.rs:4:     use md_codec::chunk::reassemble;
crates/md-codec/src/bin/md/cmd/encode.rs:6:     use md_codec::encode::encode_md1_string;
crates/md-codec/src/bin/md/cmd/encode.rs:7:     use md_codec::chunk::{derive_chunk_set_id, split};
crates/md-codec/src/bin/md/cmd/encode.rs:8:     use md_codec::identity::{compute_md1_encoding_id, compute_wallet_policy_id};
crates/md-codec/src/bin/md/cmd/inspect.rs:3:    use md_codec::decode::decode_md1_string;
crates/md-codec/src/bin/md/cmd/inspect.rs:4:    use md_codec::chunk::reassemble;
crates/md-codec/src/bin/md/cmd/inspect.rs:5:    use md_codec::identity::{compute_md1_encoding_id, compute_wallet_descriptor_template_id, compute_wallet_policy_id};
crates/md-codec/src/bin/md/cmd/verify.rs:4:     use md_codec::decode::decode_md1_string;
crates/md-codec/src/bin/md/cmd/verify.rs:5:     use md_codec::chunk::reassemble;
crates/md-codec/src/bin/md/cmd/verify.rs:6:     use md_codec::encode::encode_payload;
crates/md-codec/src/bin/md/format/json.rs:2:    use md_codec::header::Header;
crates/md-codec/src/bin/md/format/json.rs:3:    use md_codec::chunk::ChunkHeader;
crates/md-codec/src/bin/md/format/json.rs:4:    use md_codec::identity::{Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId};
crates/md-codec/src/bin/md/format/json.rs:95:   use md_codec::encode::Descriptor;
crates/md-codec/src/bin/md/format/json.rs:96:   use md_codec::tree::{Body, Node};
crates/md-codec/src/bin/md/format/json.rs:97:   use md_codec::tlv::TlvSection;
crates/md-codec/src/bin/md/format/json.rs:98:   use md_codec::origin_path::{OriginPath, PathDecl, PathDeclPaths};
crates/md-codec/src/bin/md/format/json.rs:99:   use md_codec::use_site_path::UseSitePath;
crates/md-codec/src/bin/md/format/text.rs:2:    use md_codec::encode::Descriptor;
crates/md-codec/src/bin/md/format/text.rs:3:    use md_codec::tag::Tag;
crates/md-codec/src/bin/md/format/text.rs:4:    use md_codec::tree::{Body, Node};
crates/md-codec/src/bin/md/format/text.rs:5:    use md_codec::use_site_path::UseSitePath;
crates/md-codec/src/bin/md/format/text.rs:171:  use md_codec::identity::{Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId};
crates/md-codec/src/bin/md/format/text.rs:172:  use md_codec::chunk::ChunkHeader;
crates/md-codec/src/bin/md/parse/template.rs:113: use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
crates/md-codec/src/bin/md/parse/template.rs:114: use md_codec::use_site_path::{Alternative, UseSitePath};
crates/md-codec/src/bin/md/parse/template.rs:330: use md_codec::tag::Tag;
crates/md-codec/src/bin/md/parse/template.rs:331: use md_codec::tree::{Body, Node};
crates/md-codec/src/bin/md/parse/template.rs:620: use md_codec::encode::Descriptor;
crates/md-codec/src/bin/md/parse/template.rs:621: use md_codec::tlv::TlvSection;
```

Modules touched: `decode`, `chunk`, `encode`, `header`, `identity`, `tag`, `tree`, `tlv`, `origin_path`, `use_site_path` (10 modules). No reach into `bch`, `bitstream`, `varint`, `validate`, `canonical_origin`, `canonicalize`, `codex32`, `derive`, `error`, `phrase` from the binary.

### Public-API resolution

All resolutions traced through `crates/md-codec/src/lib.rs`. The `pub mod` declarations (lib.rs:15–33) make every cited module importable via its module path; the `pub use` block (lib.rs:35–48) provides a flat re-export mirror. Items not flat-re-exported (e.g. `tree::{Body, Node}`, `use_site_path::{Alternative, UseSitePath}`, `chunk::ChunkHeader` in module-path form, `identity::*` in module-path form) are still public via the `pub mod` declaration plus a `pub struct`/`pub enum`/`pub fn` at the item site.

| Imported item | Resolution | Action |
|---|---|---|
| `md_codec::decode::decode_md1_string` | `pub mod decode` (lib.rs:20); also flat re-export `pub use decode::{decode_md1_string, decode_payload}` (lib.rs:37) | None |
| `md_codec::chunk::reassemble` | `pub mod chunk` (lib.rs:18); flat re-export (lib.rs:36) | None |
| `md_codec::chunk::{derive_chunk_set_id, split}` | `pub mod chunk` (lib.rs:18); flat re-export (lib.rs:36) | None |
| `md_codec::chunk::ChunkHeader` | `pub mod chunk` (lib.rs:18); flat re-export (lib.rs:36) | None |
| `md_codec::encode::Descriptor` | `pub mod encode` (lib.rs:22); flat re-export `pub use encode::{Descriptor, encode_md1_string, encode_payload}` (lib.rs:38) | None |
| `md_codec::encode::encode_payload` | `pub mod encode` (lib.rs:22); flat re-export (lib.rs:38) | None |
| `md_codec::encode::encode_md1_string` | `pub mod encode` (lib.rs:22); flat re-export (lib.rs:38) | None |
| `md_codec::header::Header` | `pub mod header` (lib.rs:24); flat re-export `pub use header::Header` (lib.rs:40) | None |
| `md_codec::identity::{compute_md1_encoding_id, compute_wallet_policy_id}` | `pub mod identity` (lib.rs:25); flat re-export block (lib.rs:41–44) covers both | None |
| `md_codec::identity::{compute_md1_encoding_id, compute_wallet_descriptor_template_id, compute_wallet_policy_id}` | `pub mod identity` (lib.rs:25); flat re-export block (lib.rs:41–44) covers all three | None |
| `md_codec::identity::{Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId}` | `pub mod identity` (lib.rs:25); flat re-export block (lib.rs:41–44) covers all three IDs | None |
| `md_codec::tag::Tag` | `pub mod tag` (lib.rs:28); flat re-export `pub use tag::Tag` (lib.rs:47) | None |
| `md_codec::tree::{Body, Node}` | `pub mod tree` (lib.rs:30); items declared `pub struct Node` (tree.rs:9) and `pub enum Body` (tree.rs:18). No flat re-export — module-path access only. | None |
| `md_codec::tlv::TlvSection` | `pub mod tlv` (lib.rs:29); flat re-export `pub use tlv::TlvSection` (lib.rs:48) | None |
| `md_codec::origin_path::{OriginPath, PathDecl, PathDeclPaths}` | `pub mod origin_path` (lib.rs:26); flat re-export `pub use origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths}` (lib.rs:45) | None |
| `md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths}` | `pub mod origin_path` (lib.rs:26); flat re-export (lib.rs:45) | None |
| `md_codec::use_site_path::UseSitePath` | `pub mod use_site_path` (lib.rs:31); item is `pub` at the module. No flat re-export — module-path access only. | None |
| `md_codec::use_site_path::{Alternative, UseSitePath}` | `pub mod use_site_path` (lib.rs:31); `Alternative` declared `pub struct Alternative` (use_site_path.rs:19). | None |

### Promotion candidates

None — every binary import resolves to an item already publicly accessible via `pub mod` (and most are also flat-re-exported). Spec assertion confirmed: zero md-codec public-API promotions are required for the extraction.

## Test classification

20 test files in `crates/md-codec/tests/`. The plan/spec wording "21 test files" is off-by-one; the enumerated 14 CLI + 6 lib totals 20.

| File | Classification | Reason |
|---|---|---|
| `address_derivation.rs` | stay-in-md-codec | only `md_codec::*` imports; no `assert_cmd`/`cargo_bin` |
| `chunking.rs` | stay-in-md-codec | only `md_codec::*` imports; no `assert_cmd`/`cargo_bin` |
| `cmd_address.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `cmd_address_json.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` (also uses `insta::assert_snapshot!`) |
| `cmd_bytecode.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` (and `assert_cmd::cargo::cargo_bin`) |
| `cmd_compile.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `cmd_decode.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `cmd_encode.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `cmd_inspect.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `cmd_verify.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `compile.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `exit_codes.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `forward_compat.rs` | stay-in-md-codec | only `md_codec::*` imports; no `assert_cmd`/`cargo_bin` |
| `help_examples.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `json_snapshots.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` (also uses `insta::assert_snapshot!`) |
| `scaffold.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` |
| `smoke.rs` | stay-in-md-codec | only `md_codec::*` imports; no `assert_cmd`/`cargo_bin` (architect spot-check confirmed) |
| `template_roundtrip.rs` | move-to-md-cli | `use assert_cmd::Command` + `Command::cargo_bin("md")` (architect spot-check confirmed line 10) |
| `vector_corpus.rs` | move-to-md-cli | `use assert_cmd::Command` (line 3) + `Command::cargo_bin("md")` (line 10) — **DEVIATES from spec, which lists this as lib-only** |
| `wallet_policy.rs` | stay-in-md-codec | only `md_codec::*` imports; no `assert_cmd`/`cargo_bin` |

Tally: 15 move-to-md-cli, 5 stay-in-md-codec.

### Deviation from spec

- `vector_corpus.rs` is a CLI test (spawns `md` via `assert_cmd::Command::cargo_bin("md")` to regenerate the committed vector corpus and `diff -r` against it). The spec lists it as lib-only. **Action:** Phase 3 must move `vector_corpus.rs` along with the other CLI tests; `tests/vectors/` (committed corpus + manifest) must move too, or the test must be reworked to read the corpus from the md-codec package rather than its own `CARGO_MANIFEST_DIR`. The spec's CLI-test count rises from 14 to 15 and its lib-test count drops from 6 to 5.

This adjusts the totals: 15 CLI / 5 lib (total 20).

## `insta` dev-dep verdict

**Drop from md-codec.** Survey: `grep -rE "use insta|insta::" crates/md-codec/tests/` lists hits only in `json_snapshots.rs` (CLI) and `cmd_address_json.rs` (CLI). The five lib-only tests (`address_derivation`, `chunking`, `forward_compat`, `smoke`, `wallet_policy`) contain no `insta` references. After Phase 3 moves the CLI tests to md-cli, md-codec will have no `insta` consumers; Phase 2's manifest swap should drop `insta` from md-codec's `[dev-dependencies]`.
