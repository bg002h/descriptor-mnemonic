## Status: DONE

**Commit:** `1ccc1d4` (merged alongside CLI integration tests committed by a parallel agent)

**Files changed:**
- `crates/wdm-codec/src/bytecode/decode.rs` — added explicit arity check in four decoder branches
- `crates/wdm-codec/tests/conformance.rs` — un-ignored `rejects_invalid_bytecode_missing_children`

**Role:** implementer

---

### What was done

Added per-child error interception in the four variable-arity decoder branches:

- `Tag::SortedMulti` in `decode_wsh_inner`
- `Tag::Multi`, `Tag::MultiA`, `Tag::Thresh` in `decode_terminal`

In each loop, when `decode_placeholder` or `decode_miniscript` returns
`Error::InvalidBytecode { kind: UnexpectedEnd | Truncated, .. }`, the error is
replaced with `MissingChildren { expected: n, got: i }` before returning.
Other errors propagate unchanged.

Also updated `decode_multi_rejects_truncated_mid_keys` in the inline `#[cfg(test)]`
block of `decode.rs` to expect `MissingChildren { expected: 3, got: 2 }` instead
of `UnexpectedEnd`, since the new arity check now fires first.

### Gate results

- `cargo test -p wdm-codec --test conformance`: **34 passed, 0 failed, 0 ignored**
  (`rejects_invalid_bytecode_missing_children` now active and passing)
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --check` on modified files: clean
  (unrelated `tests/cli.rs` fmt issue exists from parallel agent — not in scope)
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc -p wdm-codec`: clean

### Closed short-ids

- `6e-missing-children-unreachable`
