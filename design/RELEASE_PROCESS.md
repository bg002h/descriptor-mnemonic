# md-codec release process

## Release-checklist invariants

### Path-dictionary lockstep with mk1

Any change to the path dictionary in `crates/md-codec/src/bytecode/path.rs`
(or the BIP §"Path dictionary" table) requires an mk1 spec amendment in the
same release window. mk1 inherits md1's path dictionary byte-for-byte; silent
drift is a wire-format hazard.

**Before tagging a release that touches the path dictionary:**

- [ ] Open a coordinated PR in `bg002h/mnemonic-key` updating mk1's spec to
      mirror md1's table.
- [ ] Cross-link the two PRs in their bodies.
- [ ] Land both before tagging the md1 release; mk1 ships its corresponding
      version (or follow-on patch) in the same window.

### CLAUDE.md crosspointer maintenance

When a mk1-surfaced FOLLOWUPS entry resolves in this repo, update both:

- `design/FOLLOWUPS.md` — mark `Status: resolved <COMMIT>`.
- mk1's `design/FOLLOWUPS.md` companion entry — note the resolving md1
  commit.
- `CLAUDE.md` — drop the entry from the "Currently open mk1-surfaced items"
  list.

### Wire-format SHA pin

Every release that ships canonical-vector changes updates
`tests/vectors_schema.rs`'s SHA pin (`V0_2_SHA256` for the schema-2 file at
the time of writing) to the new corpus SHA, computed via:

```bash
sha256sum crates/md-codec/tests/vectors/v0.2.json
```

Releases that touch the schema-1 corpus (`v0.1.json`) follow the same
pattern; both files regenerate via:

```bash
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.1.json --schema 1
cargo run --features test-helpers --bin gen_vectors -- --output crates/md-codec/tests/vectors/v0.2.json --schema 2
```

If the schema-1 corpus changes from a release (e.g., due to error-variant
renames affecting `expected_error_variant` strings), call that out
explicitly in the release notes — schema-1's "byte-identical regen across
patch versions" invariant is load-bearing for downstream consumers that pin
SHAs externally.

### Family generator string

`vectors.rs::GENERATOR_FAMILY` embeds `MAJOR.MINOR` only, so vector files do
not churn SHA-256 on patch bumps. When bumping minor (e.g., 0.8 → 0.9),
regenerate vectors as part of the release and accept the SHA churn.

## Release-step checklist

For a normal release:

1. Update `Cargo.toml` (and `crates/md-signer-compat/Cargo.toml` if its
   surface broke).
2. Update `CHANGELOG.md`. Lead with a "Why" callout if the release renames
   identifiers or breaks public API.
3. Update `MIGRATION.md` if there are consumer-visible breaks. Append a new
   `## vX.Y → vA.B` section; do not edit prior sections (they are frozen
   historical record).
4. Regenerate vectors (both schemas).
5. Update SHA pin(s) in `tests/vectors_schema.rs`.
6. Update path-dictionary corpus count assertion if a vector was added.
7. `cargo build --workspace --all-features`.
8. `cargo test --workspace --all-features`.
9. `cargo clippy --workspace --all-features --all-targets -- -D warnings`.
10. `cargo doc --workspace --all-features --no-deps` (no new warnings).
11. Mark relevant `design/FOLLOWUPS.md` entries `resolved <COMMIT>`.
12. Update `CLAUDE.md` crosspointer if any entries were mk1-surfaced.
13. Open release PR; cite mk1 companion PR in body if applicable.
14. After CI green and merge: tag `md-codec-vX.Y.Z` on the merge commit;
    push tag.
15. Create GitHub Release; attach changelog excerpt.
16. Cross-update sibling repo's FOLLOWUPS companions and any forward-
    reference hedges (see `design/agent-reports/` for per-release hedge
    audits).
