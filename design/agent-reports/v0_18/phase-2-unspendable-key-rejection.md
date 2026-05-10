# v0.18 Phase 2 — Item G `--unspendable-key` xpub-form rejection (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

Pre-v0.18 behavior of `--unspendable-key`:

- `md compile --unspendable-key <xpub>`: rendered an output (the xpub propagated into the rendered template through miniscript's compile_tr).
- `md encode --from-policy --unspendable-key <xpub>`: failed with an opaque downstream error.

The compile.rs doc-comment claimed three accepted forms (xpub / NUMS hex / None) but only two work end-to-end. v0.18 narrows the public surface to NUMS-hex-or-omitted via a uniform CLI dispatch guard at both Compile and Encode --from-policy sites. v0.17 architect L1 finding from the whole-PR review identified this gap and was carried forward to v0.18 (after v0.17.1 was canceled).

## Artifacts

### Centralized guard helper (`main.rs`)

```rust
#[cfg(feature = "cli-compiler")]
fn validate_unspendable_key_nums_only(uk: Option<&str>) -> Result<(), CliError> {
    if let Some(v) = uk {
        if v != parse::template::NUMS_H_POINT_X_ONLY_HEX {
            return Err(CliError::BadArg(
                "--unspendable-key currently only accepts the BIP-341 NUMS H-point literal hex \
                 (50929b74...e803ac0) or omitted (auto-NUMS default). Other forms (xpub-style \
                 descriptor keys, arbitrary x-only hex) are not supported in this release; \
                 track v0.19+ for caller-supplied internal-key support.".into()
            ));
        }
    }
    Ok(())
}
```

Called from both dispatch sites (Encode --from-policy at main.rs:247, Compile at main.rs:316), positioned AFTER the existing empty-string and segwitv0-incompat guards so the more-specific errors fire first.

### Doc-comment update (`compile.rs`)

`compile_policy_to_template`'s doc-comment trimmed from "three accepted forms (per SPEC v0.17)" to "two accepted forms (v0.18 Item G CLI guard)". Library layer accepts any `Option<&str>` for the Tap branch — the CLI dispatch is the enforcement point, not the library function. The reviewer confirmed this posture is correct: shape-gating is a CLI release policy, not a library correctness invariant.

### Tests added

Three integration tests in `crates/md-cli/tests/cmd_compile.rs`:

1. `compile_unspendable_key_rejects_xpub_form` — xpub-form input, asserts BadArg with the v0.18 error wording AND "v0.19+" forward-pointer.
2. `compile_unspendable_key_rejects_non_nums_x_only_hex` — arbitrary 32-byte x-only hex (`0000...0001`), asserts BadArg. Pins that the guard is strict-equality, not a "looks like 64 hex chars" heuristic.
3. `encode_from_policy_unspendable_key_rejects_xpub_form` — same xpub-form rejection on the encode-side dispatch path. Confirms uniform behavior.

The positive-control case (literal NUMS hex through compile → success) is already covered by the v0.17 test `compile_pk_tap_with_explicit_nums_unspendable_key`; no duplicate test added.

## Verification

- `cargo build -p md-cli --features cli-compiler` → clean.
- `cargo test --workspace --all-features` → 401 pass (was 398 pre-Phase-2; +3 new = exact target).
- `cargo test -p md-cli --features cli-compiler --test cmd_compile` → 11 pass (8 prior + 3 new).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` → clean.
- Live probe: 5 scenarios — auto-NUMS (omitted) + literal NUMS hex pass; xpub-form (compile + encode), arbitrary x-only hex all reject with the expected message.

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0 / I: 0 / L: 0**
- Reviewer confirmed all six review-focus areas: guard placement (specificity-correct), helper signature (`Option<&str>` is the right choice for centralized None-passthrough), doc-comment accuracy (CLI-as-enforcement-layer is correct posture), test coverage (positive control already exists), error message ergonomics (inline literal hex is a usability plus; v0.19+ pointer signals release policy not permanence), and feature-gating (matches surrounding pattern; helper is unreachable when the feature is off).

Net: 0C/0I/0L — no findings.

## Exit gate

- ✅ Both dispatch sites (Compile + Encode --from-policy) reject xpub-form `--unspendable-key`.
- ✅ Strict NUMS-hex-or-omitted equality check.
- ✅ Three tests pin xpub rejection (compile), arbitrary-non-NUMS rejection (compile), and encode-side parity.
- ✅ compile.rs doc-comment updated.
- ✅ Workspace tests + clippy clean.
- ✅ Per-phase reviewer 0C/0I/0L.

Phase 2 closed; proceeding to Phase 3 (Item B — NUMS sentinel wire-format change). This is the load-bearing wire-format break that bumps md-codec to 0.18.0.
