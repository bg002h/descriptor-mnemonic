# Implementation Review — Cycle C phase 1 md-codec (round 1)

Reviewer: Fable 5 architect agent (a17684a03403bd9b3), 2026-06-11.
Target: uncommitted md-phase fuzz infra @ descriptor-mnemonic (HEAD 6c85ad3).
Persisted verbatim per CLAUDE.md convention. (The single Minor — rustfmt the
fuzz dir — was folded immediately after this review: `rustfmt +1.85.0
--edition 2024` over all 5 fuzz sources, re-`cargo +nightly fuzz build` clean.)

## Verdict: GREEN

All four targets compile clean (zero warnings), every oracle is correct against the GREEN spec, the clamp is present and provably abort-free, the corpus gate runs the same split-then-call the targets use and regenerates deterministically with no churn, workspace isolation holds, the lock is aligned to miniscript 13.0.0, CI is correctly scoped with no path collision, and I independently reproduced the bring-up panic-detection. The one defect found (fuzz sources not rustfmt-clean) is un-gated by any CI and cosmetic — Minor, not Important. 0C/0I.

## Critical
- none

## Important
- none

## Minor
- **[fmt — FOLDED] The fuzz sources were not rustfmt-clean and their import ordering contradicted the repo's own style.** `rustfmt +1.85.0 --edition 2024 --check` flagged all 4 targets + `gen_corpus.rs`. Most benign line-wrapping, but `gen_corpus.rs:163` and `md1_decode_with_correction.rs:22` wrote lowercase-before-uppercase imports, the opposite of the committed source convention (lib.rs:46 `encode::{Descriptor, encode_md1_string, encode_payload}`). NOT a gate: root `cargo fmt --all --check` and ci.yml's fmt job both ignore the nested `fuzz/` workspace, and fuzz-smoke.yml has no fmt step. (Nightly lacks the rustfmt component, which is why it wasn't caught locally.) Folded with a stable rustfmt pass.
- **[count, cosmetic — already flagged in R3] The "18 ms_codec::Error variants" miscount** lives in the spec (ms is phase 2), not this phase's deliverables. The R3 fold corrected it to 16; no md-phase action.
- **[gen_corpus naming]** Seed files use `.md1` / `.payload` / `.parts` suffixes. cargo-fuzz is content-agnostic about names; deliberate, fine, aids readability.

## Deliverable-conformance table
| Item | Conforms? | Notes |
|---|---|---|
| BUILD: all 4 targets compile | YES | Forced clean rebuild: `Finished release in 47.19s`, zero warnings, exit 0. |
| md1_decode_string oracle | YES | utf8-lossy → `decode_md1_string`; on Ok, `encode_md1_string(&d).expect("FINDING…")` → re-decode `.expect(…)` → `assert_eq!(d, d2)`. Re-encode Err PANICS; `if let Ok` wraps only the outer decode. |
| md1_reassemble oracle | YES | `data.split(\n).take(8).map(from_utf8_lossy)`; on Ok, `split(&d).expect(…)` → `reassemble()` → `assert_eq!(d, d2)`. Value compare. |
| md1_decode_with_correction oracle (subtle) | YES | Coordinate post-HRP: `apply_correction` lowercases, requires `starts_with("md1")`, strips 3 bytes, walks `char_indices()` skipping whitespace/`'-'`, lands on `symbol_idx == detail.position` — mirrors `parse_chunk_symbols` (chunk.rs:429-454) which `position:pos` (chunk.rs:552) indexes. `chunk_index` guarded `assert! < corrected.len()` (decoder always `< strings.len()`). Out-of-range/edge → `None` → return (NO false crash); char-boundary slicing safe. Asserts unchanged Descriptor + empty details. |
| md1_decode_payload clamp | YES | `data.len() < 2 → return`; `total_bits = candidate.min(remainder.len()*8)` guarantees `bit_limit ≤ bytes.len()*8` → debug_assert (bitstream.rs:114) unreachable; `remainder.len()*8` can't overflow usize. Fixed-point via `encode_payload` → re-decode → `assert_eq!`. |
| Clamp non-regression (no abort) | YES | Arithmetic by construction + empirically (full fuzz run, debug-assertions profile, no spurious aborts; R2/R3 reproduced clamped sweep panic-free). |
| Corpus validity gate (same-split-then-call) | YES | Each seed gated via the TARGET's path: string→decode Ok; payload→clamp-then-decode Ok; reassemble→split→reassemble Ok; correction→split→decode_with_correction Ok + empty details. Not "raw decode." |
| Multi-chunk \n-join, between-only, no trailing | YES | `wsh_multi_wallet_policy_chunked.parts`: 323B, 4 parts, 3 newlines between, last byte `0x71` `q` (not `0x0a`), no trailing empty part; each part starts `md1`. |
| Deterministic regen, no churn | YES | `cargo test --test gen_corpus` ran twice; corpus sha256 byte-identical (diff empty). Fixed literals, no RNG. |
| Workspace isolation | YES | Root `cargo +1.85.0 fmt --all --check` exit 0; `build --workspace` unperturbed; root `[workspace]` = 2 crates; `fuzz/Cargo.toml` own empty `[workspace]`. |
| Lock alignment (miniscript 13.0.0) | YES | fuzz & root both miniscript `13.0.0` crates.io, identical checksum `867b1f11…b650`. No `[patch]` in any manifest. |
| CI fuzz-smoke.yml | YES | Compile gate on `fuzz/**`+`crates/md-codec/src/**`+self; smoke `if schedule\|\|workflow_dispatch` (cron `13 7 * * *`, NOT push); `upload-artifact@v5`; cargo-fuzz via `taiki-e/install-action@v2`; nightly via `dtolnay/rust-toolchain@master` pinned `nightly-2026-04-27`. actionlint clean. |
| No ci.yml path collision | YES | ci.yml has no `paths:` filter but all cargo invocations are `--workspace` from root → never descend into nested `fuzz/`. No stable-1.85 fuzz build; fmt job won't see fuzz files. |
| .gitignore | YES | Adds `fuzz/artifacts/` + `fuzz/coverage/`; `fuzz/target` covered by `**/target/`. `git add -n fuzz/` lists corpus + `fuzz/Cargo.lock` trackable; `fuzz/target/` ignored. |
| Bring-up re-proof (independent) | YES | Planted `assert!(!s.contains("md1"))` in md1_decode_string, `-runs=200000 -max_total_time=25`: found crash (`panicked …:24:5 PLANTED-CRASH`, deadly signal, artifact written, exit 77). Reverted (md5 back to `a1aedc8…`), artifacts removed. |
| Findings-swallow / wrong-compare / nondeterminism scan | YES | No `dbg!`/`println!` in targets (only `eprintln!` in gen_corpus diagnostic). All oracles `assert_eq!` on Descriptor VALUE. Re-encode/decode `.expect("FINDING…")`. No committed crash artifacts. Deterministic gen. |

## Evidence log
```
repo HEAD = 6c85ad3 (2 docs(followups) commits past spec's cdd8501; no md-codec src Δ)
git status (orig & post-review identical): " M .gitignore", "?? .github/workflows/fuzz-smoke.yml", "?? fuzz/"
BUILD: cargo +nightly-2026-04-27 fuzz build → Finished release 47.19s, 0 warnings, exit 0
gen_corpus: 2 runs, ok 1 passed; corpus sha256 byte-identical run1==run2; 8 seeds × 4 targets
multi-chunk seed: 323B, parts=4, \n=3, last byte 0x71 (not 0x0a), no trailing empty part
fmt: cargo +1.85.0 fmt --all --check exit 0 (ignores fuzz/); rustfmt +1.85.0 --edition 2024 --check: 4 targets+gen_corpus MISFORMATTED (un-gated Minor; folded post-review)
build: cargo +1.85.0 build --workspace → Finished, unperturbed
lock: fuzz & root both miniscript 13.0.0 crates.io checksum 867b1f11…b650 (identical); no [patch]
CI: actionlint clean; build trigger fuzz/**+crates/md-codec/src/**+self; smoke if schedule||workflow_dispatch (cron 13 7 * * *); upload-artifact@v5; taiki-e/install-action@v2; dtolnay@master nightly-2026-04-27
ci.yml: no paths filter, all --workspace from root → excludes nested fuzz/
coordinate proof: chunk.rs:405-415 CorrectionDetail{chunk_index,position,was,now}; :410 "post-HRP-and-separator"; parse_chunk_symbols :429-454 strips "md1"+skips ws/'-'; position:pos :552; chunk_index from enumerate :510 < len. apply_correction mirrors exactly; out-of-range→None→return
clamp: md1_decode_payload.rs:29 len<2 return; :35 candidate.min(remainder.len()*8) ⇒ bit_limit ≤ bytes.len()*8, debug_assert unreachable
oracles: all 4 assert_eq!(d,d2) Descriptor PartialEq; re-encode/decode .expect("FINDING…"); if let Ok only on outer decode
BRING-UP: planted assert in md1_decode_string → -runs=200000 → panicked PLANTED-CRASH, deadly signal, artifact, exit 77; reverted (md5 a1aedc8…), artifacts removed
scan: no dbg!/println! in targets; eprintln! only gen_corpus:311; no committed crash artifacts
final tree: clean — matches original
```

GREEN — 0 Critical / 0 Important. Cleared past the gate.
