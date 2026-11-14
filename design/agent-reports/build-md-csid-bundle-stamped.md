# Build report: md-cli bundle-stamped chunk-set id stays silent (FU1 md companion)

Model: Claude Sonnet 5.5, claude-sonnet-5-5.
Base: origin/main 542806a6799e4171d24fe9c06349d806060907e5 (had not moved at fetch).
Branch: fix/md-csid-bundle-stamped. RED commit 1aafcf30. GREEN commit d027e4ef. Head: see final reply (report commit follows). PR: see final reply.

## RED
Command: `cargo nextest run --locked --all-features -j 8 -p md-cli --test csid_bundle_stamped` on unmodified src.
Both rows FAIL for the stated reason: rc 0 but stderr carries `warning: this key card's stamped chunk-set id (da7ba) ... (da7bb)`.
Rows: `descriptor_bundle_stamped_card_is_silent`, `address_bundle_stamped_card_is_silent` (`seat::run` serves both verbs through `seat_notes`).
The unit test `is_bundle_stamped_window_is_xor_below_32` (in src/seat/input.rs) does not compile before the fn exists; it was added in the GREEN commit, and the RED commit holds the CLI rows alone. Log: /scratch/cache/md-csid-logs/red.log.

## GREEN
`seat_notes`: `if declared != derived && !is_bundle_stamped(declared, &c.card)`; private `is_bundle_stamped` and `BUNDLE_SLOT_LIMIT: u32 = 32` beside `derived_chunk_set_id`, body mirrors the mk fix-optionA.diff.
`cargo nextest run --locked --all-features -j 8 -p md-cli`: 1000 tests run: 1000 passed, 3 skipped (/scratch/cache/md-csid-logs/green-mdcli.log).
Unit test: declared = base, base^31 -> true; base^32 -> false; no stubs -> false.

## Guard (non-bundle mismatch still warns)
Existing test `seat::input::tests::csid_warning_fires_on_a_pinned_mismatch_and_is_silent_on_the_clean_twin` (declared 99999 vs first stub, outside the window) still passes. No new guard needed.

## Tests that change outcome
None.

## Texts placed
- design/SPEC*.md: grep "was not derived", "chunk_set_id", "chunk-set id", "Contract 6" found no sentence describing this warning (the hits are the encode JSON field, the derive function, and the auto-partition and wallet-form refusals, which are other texts). No SPEC edit.
- CHANGELOG.md md-cli [Unreleased]: new `### Fixed` bullet, text per brief.
- design/FOLLOWUPS.md: entry `md-chunk-set-id-bundle-stamped` (`fixed in this PR`, Tier cross-repo) and `md-bip-32-key-cap-attributed-to-bip388` (`deferred-minor`, Nit). Both put at the top of "Open items".
- BIP lines: the phrase at bip/bip-mnemonic-descriptor.mediawiki :437 is "(BIP 388 placeholder cap)", :608 says "the BIP 388 wallet-policy cap of <= 32 keys"; :756 did not match my search for a BIP 388 cap claim (not checked further). Text only, nothing edited.

## STOPs
None. Open point for coordinator, not a decision: the FOLLOWUPS Tier and Owner for the two new entries are my reading of the file's form (cross-repo for the companion; v1+ and "next md BIP change" for the Nit); the grade names none for the Nit.

## Gate
(appended below)

Logs in /scratch/cache/md-csid-logs/. RUSTFLAGS="-D warnings" as ci.yml sets it; commands as the workflows give them.
ci.yml:
- clippy job guards (docs-only.test.sh, shared-ci sha256, code-docs-check test and check): rc 0 (ci-guards.log)
- fmt job: conformance-vector checksum pin rc 0; `cargo fmt --all --check` rc 0
- test job (ubuntu equivalent): `cargo nextest run --workspace --all-targets --all-features --no-fail-fast`: 1677 tests run: 1677 passed, 6 skipped; `cargo test --workspace --doc --all-features` rc 0
- clippy: `cargo clippy --workspace --all-targets --all-features -- -D warnings` rc 0; `cargo build --locked -p md-cli --no-default-features` rc 0
- doc: `cargo doc --workspace --no-deps --document-private-items --all-features` rc 0
- freebsd: `cargo check --target x86_64-unknown-freebsd -p md-cli` rc 0
release.yml (runs on pull_request):
- release test: `cargo nextest run --locked --workspace --no-fail-fast`: 1629 tests run: 1629 passed, 6 skipped; `cargo test --locked --workspace --doc` rc 0
- build md linux-x86_64: `cargo build --release --locked --target x86_64-unknown-linux-gnu -p md-cli --features cli-compiler` rc 0 (workflow's own --release)
Other workflows: fuzz-smoke, bitcoind-differential, vendor-freshness, man-pages: path filters not touched by this diff (md-codec src, Cargo files, tags): not triggered.

NOT RUN: Windows and macOS legs of the test job and release build (no such host); musl x86_64 and aarch64 jobs (no musl-gcc here; aarch64 needs an arm64 runner); linux-aarch64 cross build (cross not installed); the release smoke-test step; release.yml `assemble + release` (tag only). The mutation gate (coordinator's).

Counts: Critical 0, Important 0, Minor 0, Nit 0.
