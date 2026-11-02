**⚠ Important: ruling R-4 contradicts itself on [2^29, 2^31) (STOP S-1); the build waits on it.**

# FIX-B doc report: md bug hunt, codec fixes

FIX doc: `design/FIX_bughunt_md_b.md`, commit 680e2f12 on `fix/bughunt-b`
(base b2bfdd63), together with the RED test code in
`crates/md-codec/tests/bughunt.rs` and `crates/md-codec/tests/policy_shape.rs`.
No production code is committed. Nothing is pushed.

## What was verified

- **RED on unmodified b2bfdd63:** 14 run, 2 passed, 12 failed.
  - The two passes are the inverted #9 row (as required) and the unchanged
    sibling `mixed_branch_without_a_lock_still_reports_zero_threshold`.
  - The 12 failures each fail for the stated reason; the doc's §3 records
    each observed output.
- **GREEN on a scratch application of the doc's §2:**
  - all 13 FIX-B rows and every md-codec test pass;
  - the only workspace failures are the FIX-A md-cli rows and the private
    rows 06 and 13.
  - The scratch application was then reverted.
- **Gate filter:** `cargo nextest list` matches exactly 13 tests.
- **Lint:** `cargo fmt --check` and clippy `-D warnings` on md-codec are clean.
- **Guard mutation:** with the scratch fix applied, replacing
  `nkeys as usize == br.slots.len()` with `true` makes the sibling
  policy_shape test fail. R-5 replaces the old test that covered this, so the
  sibling now catches the mutant.
- **CLI (unmodified code):**
  - `md decode` on the Ź alias decodes the card and exits 0;
  - the `'b'` control prints `md: codec error: codex32 decode error: character 'b' not in codex32 alphabet` and exits 1.

## Findings

- **Important: S-1.** R-4 sets the guard at > 2^29-1, citing BIP :459. It
  also says "[2^29, 2^31) unchanged: address and descriptor accept them".
  These conflict.
  - Measured on 536870912: the guard makes `md address` refuse, where
    unmodified code prints an address.
  - The doc writes option (i), the BIP limit, and leaves the choice as a STOP.
  - Rows 04b and 04c hold under both options.
- **STOP S-2 (question).** The ruling does not settle two points:
  - which value is reported when several overflow;
  - whether the single-path entries also check alternatives that are not
    selected.
  - The scratch fix reports origin values first, then alternatives, and
    checks every alternative.
- **Notes on the grade's hints. Neither is a finding.**
  - "origin_path_to_derivation becomes fallible": one guard in
    `assemble_origin_and_xkey` gives the same behaviour, because both
    callers pass through it.
  - "codec `to_descriptor` row": no function has that name. Rows 04b and 04c
    run `to_miniscript_descriptor`, `to_miniscript_descriptor_multipath` and
    `derive_address`.
- **Build order:** FIX-A's row 04, and any #4 address/descriptor row it adds,
  depend on FIX-B's VarintOverflow refusal. The doc's §1 lists them.

Counts: C 0, I 1 (S-1), M 0, N 0; STOPs 2 (S-1, S-2); RED rows 13 (12 fail, 1 inverted passes); matched gate count 13.
