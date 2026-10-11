# Review: fuzz oracle re-encode via unadmitted APIs

- **Reviewer role:** independent adversarial reviewer (single question: false PASS / wrong oracle)
- **Commit reviewed:** `bb1d63c1` "fuzz: re-encode decoded cards unadmitted, not through the mint path" (branch `fix/fuzz-oracle-unadmitted`)
- **Counts:** Critical 0 · Important 0 · Minor 0 · Nit 2
- **Verdict:** The change does not weaken any oracle. The new oracles are a strict superset of the old ones, minus only the false-positive class (decode accepts a card that mint refuses), which is by design. No blocking findings.

## Scope and settled facts (not re-derived)

Decode skips mint admission on purpose. Seeds were machine-verified (each panics the old target and passes the new one), the targets were fuzzed 300 s each, and fmt is clean. This review covers only the three targets in the commit.

## Analysis

### 1. Do the unadmitted APIs skip anything structural?

No. `encode_payload` and `encode_payload_unadmitted` both call `encode_payload_inner(d, _, None)` (crates/md-codec/src/encode.rs:137-139, 207-209). The only thing that differs is the `if admission == Admission::Enforce { ... }` block (encode.rs:251-262), which runs these four checks:

- `validate_origin_key_consistency` (F-217): policy.
- `validate_no_duplicate_key_slots` (F-218): policy.
- `validate_unspendable_shape` (validate.rs:585-610): kind-1 shape policy. This is the check the CI crash hit (`reject_nested_unspendable`).
- `validate_minimal_wire_version(d, version)` (validate.rs:709-720). With `forced_version = None`, `version = d.wire_version()` (encode.rs:240), so `minimal == version` and the check can never fail on either public path.

The structural checks sit outside the gate and run on both paths. They are `canonicalize_placeholder_indices`, `validate_placeholder_usage`, `validate_multipath_consistency` and `validate_tap_script_tree` (encode.rs:228-239), plus every writer error (`path_decl.write`, `use_site_path.write`, `write_node`, `tlv.write`). Nothing structural is lost.

`split_unadmitted` and `split` share `split_with` (chunk.rs:246-248, 275-277, 283+). The only branch is which encode function is called. The chunk-set id comes from `compute_md1_encoding_id`, which already does not admit, and the `ChunkCountExceedsMax` cap, the header and `wrap_payload` are shared. `wrap_payload` (codex32.rs:82-109) takes no admission parameter, and its `PayloadTooLongForSingleString` cap still applies in the string target.

### 2. Does the string target rebuild `encode_md1_string` minus admission?

Yes, exactly. `encode_md1_string` is `encode_payload(d)?` followed by `crate::codex32::wrap_payload(&bytes, bit_len)` (encode.rs:336-339). The target does `encode_payload_unadmitted(&d).and_then(|(bytes, bits)| wrap_payload(&bytes, bits))` (md1_decode_string.rs:30-32). It is the same function composition with the same error propagation, and the only change is the encode variant.

### 3. Can a real defect now slip through?

Old oracle:
- If decode accepts a card and admission refuses it, it panics. This is the false positive.
- Otherwise it round-trips the card.

New oracle:
- It round-trips every card decode accepts.
- If the card is admitted, it also checks that the minted output equals the unadmitted output.

Every input the old oracle round-tripped is still round-tripped, with the same `expect` on re-decode and the same fixed-point `assert_eq!(d, d2)`. On top of that, inputs the old oracle panicked on early now get a full round-trip (seed + fixed-point), so coverage of the decode/encode-asymmetry class has grown.

`encode_payload(&d)`, `encode_md1_string(&d)` and `split(&d)` are still called on every input decode accepts (md1_decode_payload.rs:49, md1_decode_string.rs:33, md1_reassemble.rs:39). A panic inside an admission validator would therefore still be caught as a crash, so the validators keep their never-panic coverage. Only their `Err` result is ignored, which is correct.

### 4. Is comparing reassemble's `split` output against `split_unadmitted` output sound?

Yes. Both produce `Vec<String>` from the same `split_with` body. Only the encode call differs, and when the admitted call returns `Ok` its bytes come from the same writer sequence. The comparison is ordered and exact, so it is sound.

### 5. Doc comments

All three module docs describe the new behaviour accurately. "Where mint does admit, its bytes must equal the unadmitted ones" matches the code. The seed path cited in md1_decode_payload.rs:27-28 exists in the commit (`fuzz/corpus/md1_decode_payload/unadmitted_nested_unspendable.payload`), and its path is relative to `fuzz/`, which is the convention elsewhere in these docs.

## Findings

### Nit 1: today the "admitted == unadmitted" assertions can only fire after a codec change

md1_decode_payload.rs:49-55 (and the twins at md1_decode_string.rs:33-38, md1_reassemble.rs:39-44):

```rust
if let Ok(minted) = encode_payload(&d) {
    assert_eq!(
        minted,
        (bytes2.clone(), tb2),
        "FINDING: admitted and unadmitted encodings differ"
    );
}
```

Both sides run the same `encode_payload_inner` with `forced_version = None`. `Admission` only gates the pure-refusal block, which runs before `BitWriter::new()` (encode.rs:251-264). With the current code the assertion therefore cannot fire on any input. It is a legitimate tripwire that would fire if a future change let `Admission` (or a `split_with` branch) affect output. That makes it neither vacuous nor a false PASS.

The commit message says it "additionally assert[s]" equality. That should not be read as new coverage of today's codec, only as protection against later changes. Each accepted input also pays for one extra encode, and in the reassemble target one extra encode plus an extra `compute_md1_encoding_id`, at a small cost to execs/s. No change is required. A one-line comment saying it is a regression guard would make the intent clear.

### Nit 2: unnecessary clone in the payload target

md1_decode_payload.rs:52: `(bytes2.clone(), tb2)` allocates on every admitted input only to build a tuple for comparison. `assert_eq!((&minted.0, minted.1), (&bytes2, tb2), ...)` avoids the allocation. This is cosmetic: it does not change the oracle's behaviour and costs very little.

## Conclusion

The change does not weaken any oracle. The unadmitted paths skip only the four mint-policy checks, and the minimum-version check among them cannot fail on public paths anyway. Every structural check and writer error still surfaces. The string reconstruction is exactly `encode_md1_string` without admission, and the split comparison is sound. 0 Critical / 0 Important. The gate may close.
