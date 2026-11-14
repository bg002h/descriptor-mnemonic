# Review: md FU1 companion (md-csid-bundle-stamped), head 7792f187

Reviewer: Claude Sonnet 5.5, model id claude-sonnet-5-5. Diff `542806a6..7792f187`. This is a first review, so no previous fix commit to count.

**Verdict: no Critical, no Important.** The diff makes `md descriptor --from-mk1` and `md address` silent on a bundle-stamped card. Every other declared != derived card still warns.

## Mutation gate
`mutation-gate 7792f187 vs 542806a6: caught 18, MISSED 2, timeout 0, unviable 0`

Both survivors are `|` -> `^` at `input.rs:565:38` and `565:61`, graded with `survivor.sh`.
- Grade: **equivalent** for both. The three terms `s0<<12`, `s1<<4` and `s2>>4` occupy disjoint bit ranges (19..12, 11..4, 3..0), so `|` and `^` give the same value.

## Lens 1: tests can fail (each plant run with `cargo nextest run --locked -p md-cli -j 8`, tree restored with `git checkout -- <file>`, clean at the end)
| Plant | Result |
|---|---|
| Drop `&& !is_bundle_stamped` from `seat_notes` (the fix reverted) | Both integration tests FAIL. The cards really are declared != derived, so the tests are not vacuous. |
| `is_bundle_stamped` body -> `true` | Unit test FAILS. `csid_warning_fires_on_a_pinned_mismatch...` also FAILS, so "other mismatches still warn" is pinned. |
| `< 32` -> `<= 32` | Unit test FAILS. |
| Limit 32 -> 16 | Unit test FAILS. |
| No-stubs branch returns `true` | Unit test FAILS. |
| `.first()` -> `.last()` | **SURVIVES** (see M1). |

## Lens 2: slot window
- Window `declared ^ top20(stubs[0]) < 32`. 32 is excluded and 31 included, and the unit test asserts both. No stubs gives a warning.
- Toolkit `synthesize.rs:81-95` is `derive_mk1_chunk_set_id(stub) ^ slot`, with base = top 20 bits MSB-first. This matches `is_bundle_stamped` line for line.
- Toolkit `synthesize.rs:713-730` sets `stubs = vec![stub; n]`, so stubs[0] is always the base. The "first stub only" rule is correct.

## Findings
**M1 (Minor)** `input.rs` unit test `is_bundle_stamped_window_is_xor_below_32` never uses more than one stub. The "only the FIRST stub counts" rule is untested, and the `.last()` plant survives. Cargo-mutants cannot make this mutant, so the mutation gate does not see it.
Fix (run: green, and the `.last()` plant then FAILS). In that test, replace the two lines `card.policy_id_stubs.clear();` and `assert!(!is_bundle_stamped(base, &card));` with:
```
card.policy_id_stubs = vec![[0xda, 0x7b, 0xa5, 0x11], [0, 0, 0, 0]];
assert!(is_bundle_stamped(base, &card));
card.policy_id_stubs = vec![[0, 0, 0, 0], [0xda, 0x7b, 0xa5, 0x11]];
assert!(!is_bundle_stamped(base, &card));
card.policy_id_stubs.clear();
assert!(!is_bundle_stamped(base, &card));
```

**M2 (Minor)** The const doc and the CHANGELOG say slots are 0-31 because md1 caps keys at 32. The toolkit actually caps cosigners at 16 (`synthesize.rs:88`, `618`, `873`), so a bundle stamps slots 0-15 only. The window 0..=31 is a superset: 16 extra ids out of 2^20 are silenced that no bundle makes. This is harmless and was ruled in the grade. Optional wording: add "(the toolkit currently uses 0..=15)" to the const doc.

**N1 (Nit)** The FOLLOWUPS Status `fixed in this PR` is not one of the file's common status forms (`resolved <sha>`, `deferred-minor`, ...). Consider `resolved <sha>` once merged.

## Lens 3: CHANGELOG and FOLLOWUPS
- The CHANGELOG `### Fixed` entry under `[Unreleased]` follows the 0.20.6 form. It is true to the code.
- Both FOLLOWUPS entries are true to the code. `bip/` lines :437, :608 and :756 do attribute the 32-key cap to BIP 388, as the entry says.
- Not run: Windows, macOS, musl and aarch64 legs (the brief states this).

C / I / M / N: 0 / 0 / 2 / 1
