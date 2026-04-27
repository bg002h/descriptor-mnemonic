# Example Miniscript Corpus — WDM Format Validation

**Status:** v1, populated 2026-04-26.

**Purpose:** A reference corpus of miniscript wallet policies used to
pressure-test the **Wallet Descriptor Mnemonic (WDM)** format. Every
encoding decision must be validated against this corpus; every change to
the bytecode spec must produce updated byte counts for these examples.

The corpus spans simple to complex, single-string to chunked, to ensure
the format works across the realistic complexity range.

**Encoding assumptions** (until tag values finalize):
- Slim header: version(1) + type(1) = 2 chars overhead
- WDM bytecode: 1-byte version/type header + shared-path
  declaration (2 bytes for v0) + descriptor-codec tags
- `@i` placeholder: 0x32 + 1-byte index (2 bytes per reference)
- Path code: 1-byte dictionary code for standard paths (BIP 44/49/84/86/48)
- No fingerprints (default v0 setting)
- `wsh(...)` top-level wrapper unless otherwise noted

**Bech32 conversion:** `payload_chars = ⌈binary_bytes × 8 / 5⌉`

**Codex32 framing:** HRP(3) + slim-header(2) + checksum(13 regular or 15 long)

---

## C1 — Single-key (smallest meaningful)

```
wsh(pk(@0/**))
```

| Component | Bytes |
|---|---|
| WDM header | 1 |
| Shared-path tag + dict code (BIP 84) | 2 |
| Wsh wrapper | 1 |
| PkK + @0 | 3 |
| **Total bytecode** | **7 B** |

- Payload chars: ⌈7×8/5⌉ = 12
- Single-string regular: 12 + 18 = **30 chars**
- Chunks: 1
- Steel surface: trivial single line

**Use:** sanity check; demonstrates the lower bound.

---

## C2 — 2-of-3 multisig (most common self-custody case)

```
wsh(sortedmulti(2,@0/**,@1/**,@2/**))
```

| Component | Bytes |
|---|---|
| WDM header | 1 |
| Shared-path (BIP 48) | 2 |
| Wsh | 1 |
| SortedMulti(0x09) + threshold(2) + count(3) | 3 |
| 3× @i references (2 B each) | 6 |
| **Total bytecode** | **13 B** |

- Payload chars: ⌈13×8/5⌉ = 21
- Single-string regular: 21 + 18 = **39 chars**
- Chunks: 1
- Steel surface: one small plate

**Use:** validates multisig opcodes, the most common real-world wallet.

---

## C3 — 2-of-3 with timelock fallback (simple inheritance)

```
wsh(or_d(multi(2,@0/**,@1/**),
         and_v(v:older(52560), pk(@2/**))))
```

(2-of-2 normal path; one key + 1-year timelock recovery path.)

| Component | Bytes |
|---|---|
| Header + shared-path | 3 |
| Wsh | 1 |
| OrD(0x16) | 1 |
| Multi(0x19) + threshold(2) + count(2) | 3 |
| 2× @i refs | 4 |
| AndV(0x11) | 1 |
| Verify(0x0E) + Older(0x1F) + LEB128(52560) | 5 |
| PkK + @2 | 3 |
| **Total bytecode** | **21 B** |

- Payload chars: ⌈21×8/5⌉ = 34
- Single-string regular: 34 + 18 = **52 chars**
- Chunks: 1

**Use:** validates timelock encoding, OR/AND combinations, recovery paths.

---

## C4 — User-supplied 6-key inheritance miniscript

```
wsh(andor(
      pk(@0/**),
      after(1200000),
      or_i(
        and_v(v:pkh(@1/**),
              and_v(v:pkh(@2/**),
                    and_v(v:pkh(@3/**), older(4032)))),
        and_v(v:pkh(@4/**),
              and_v(v:pkh(@5/**), older(32768)))
      )))
```

| Component | Bytes |
|---|---|
| Header + shared-path | 3 |
| Wsh | 1 |
| AndOr(0x13) | 1 |
| PkK + @0 | 3 |
| After(0x1E) + LEB128(1200000) | 4 |
| OrI(0x17) | 1 |
| LEFT branch: 3× AndV chain + 3× v:pkh(@i) + Older(4032) | 18 |
| RIGHT branch: 2× AndV chain + 2× v:pkh(@i) + Older(32768) | 14 |
| **Total bytecode** | **45 B** |

- Payload chars: ⌈45×8/5⌉ = 72
- Single-string regular: 72 + 18 = **90 chars**
- Chunks: 1
- Steel surface: ~24 4-letter-equivalent words; comparable to a BIP 39
  seed phrase

**Use:** real-world complex inheritance; pressure-tests the regular/long
boundary. Without slim header would push into long code; with slim header
fits regular comfortably.

---

## C5 — 5-of-9 multisig with timelock recovery (corporate treasury)

```
wsh(or_d(thresh(5,
                pk(@0/**),s:pk(@1/**),s:pk(@2/**),
                s:pk(@3/**),s:pk(@4/**),s:pk(@5/**),
                s:pk(@6/**),s:pk(@7/**),s:pk(@8/**)),
         and_v(v:older(105120),
               multi(2,@9/**,@10/**))))
```

(5-of-9 primary; 2-of-2 recovery after 2 years.)

| Component | Bytes |
|---|---|
| Header + shared-path | 3 |
| Wsh | 1 |
| OrD(0x16) | 1 |
| Thresh(0x18) + threshold(5) + count(9) | 3 |
| 9× pk references (with `s:` Swap=0x0B wrapper on 8 of them): 8×(Swap+PkK+@i) + 1×(PkK+@i) = 8×4 + 3 | 35 |
| AndV(0x11) | 1 |
| Verify + Older + LEB128(105120) | 5 |
| Multi + threshold + count | 3 |
| 2× @i refs | 4 |
| **Total bytecode** | **56 B** |

- Payload chars: ⌈56×8/5⌉ = 90
- Single-string overhead with slim header: 18 chars
- Total: 90 + 18 = **108 chars** → **at long-code maximum (108)**
- Chunks: 1 (long code, on the edge)

**Use:** validates `thresh()`, `s:` swap wrapper, near-boundary single-string
case. If we add fingerprints (44 bytes) → exceeds long-code → forces
chunking. This is the natural boundary case.

---

## C6 — Pathological deeply-nested miniscript (chunking forced)

```
wsh(or_d(pk(@0/**),
         or_d(pk(@1/**),
              or_d(pk(@2/**),
                   or_d(pk(@3/**),
                        or_d(pk(@4/**),
                             or_d(pk(@5/**),
                                  or_d(pk(@6/**),
                                       or_d(pk(@7/**),
                                            and_v(v:older(1000),
                                                  pk(@8/**))))))))))
```

(Nested OR of 8 keys with timelock fallback.)

| Component | Bytes |
|---|---|
| Header + shared-path | 3 |
| Wsh | 1 |
| 8× OrD + 8× (PkK + @i) | 32 |
| AndV + Verify + Older + LEB128(1000) + PkK + @8 | 9 |
| **Total bytecode** | **45 B** |

- Wait: actually fits single string. Let me revise to genuinely force chunking.
- For a chunking test, scale up: a 20-key threshold or a deeply nested
  miniscript with ~150 byte bytecode → 240 payload chars → forces chunking.

**Use:** placeholder. The "8 nested or_d's" form actually fits single-string
(45 B). To genuinely force chunking we need ~150 B. Realistic candidates:
20-of-25 multisig, deeply nested HTLC trees with multiple sha256 preimages.

Will define the actual chunking-forcing
example once the spec is closer to fixed and we can construct a realistic
miniscript that hits the boundary.

**Open:** identify a realistic miniscript that genuinely needs chunking.
Threshold operators with 16+ keys may be the natural fit.

---

## Real-world examples (E-series)

These are taken from canonical miniscript sources (Pieter Wuille's
miniscript site, Liana wallet templates, BitBox documentation, Unchained
articles, BOLT #3 Lightning) and converted to BIP 388 wallet policy form.
They are real shipping or commonly-cited patterns. Adding them to the
corpus validates the format against actual production wallets.

### E10 — Liana "Simple Inheritance" (single + 1-year recovery)

```
wsh(or_d(pk(@0/**),
         and_v(v:pk(@1/**), older(52560))))
```

| Component | Bytes |
|---|---|
| Header + shared-path | 3 |
| Wsh | 1 |
| OrD | 1 |
| PkK + @0 | 3 |
| AndV | 1 |
| Verify + PkK + @1 | 4 |
| Older + LEB128(52560) | 4 |
| **Total bytecode** | **17 B** |

- Payload chars: 28
- Total: 28 + 18 = **46 chars** → regular
- **Source:** Liana template (`wsh(or_d(pk(pubkey1),and_v(v:pk(pubkey2),older(52560))))` per BitBox blog)

**Use:** the most common production miniscript wallet template. Single-key
primary with single-key 1-year-timelock recovery. Most users who choose a
miniscript wallet are running this exact structure.

### E12 — Liana "Expanding Multisig" (2-of-2 + 1-year recovery key)

```
wsh(or_d(multi(2,@0/**,@1/**),
         and_v(v:older(52560), pk(@2/**))))
```

| Component | Bytes |
|---|---|
| Header + shared-path | 3 |
| Wsh | 1 |
| OrD | 1 |
| Multi + threshold(2) + count(2) + 2× @i | 7 |
| AndV | 1 |
| Verify + Older + LEB128(52560) | 5 |
| PkK + @2 | 3 |
| **Total bytecode** | **21 B** |

- Payload chars: 34
- Total: 34 + 18 = **52 chars** → regular
- **Source:** Liana production template, second-most-common after Simple Inheritance.

**Use:** validates `multi()` opcode in production context. 2-of-2 with a
recovery key; common for couples/business partners with an external
recovery agent.

### E13 — HTLC pattern with sha256 preimage

```
wsh(andor(pk(@0/**),
          sha256(H),
          and_v(v:pk(@1/**), older(144))))
```

(Where H is a 32-byte hash literal embedded in the script.)

| Component | Bytes |
|---|---|
| Header + shared-path | 3 |
| Wsh | 1 |
| AndOr | 1 |
| PkK + @0 | 3 |
| Sha256 + 32-byte hash | 33 |
| AndV | 1 |
| Verify + PkK + @1 | 4 |
| Older + LEB128(144) | 3 |
| **Total bytecode** | **49 B** |

- Payload chars: ⌈49×8/5⌉ = 79
- Total: 79 + 18 = 97 chars → **exceeds regular (96), needs long or
  chunked**
- Long-code total: 79 + 20 = 99 chars → fits long
- **Source:** standard HTLC pattern from BOLT #3 / atomic swaps / sipa
  miniscript page.

**Use:** boundary case for hash-using scripts. The 32-byte sha256 preimage
hash is the largest single payload element in any common miniscript.
Demonstrates that hash-using scripts move into long-code territory even
at modest structural complexity. **Format must support 32-byte hash
literals as inline data, not as references.**

### E14 — Decaying multisig with 6 distinct keys (3-of-3 → 2-of-3)

```
wsh(or_d(multi(3,@0/**,@1/**,@2/**),
         and_v(v:older(52560),
               multi(2,@3/**,@4/**,@5/**))))
```

| Component | Bytes |
|---|---|
| Header + shared-path | 3 |
| Wsh | 1 |
| OrD | 1 |
| Multi(3) + threshold + count + 3× @i | 9 |
| AndV | 1 |
| Verify + Older + LEB128(52560) | 5 |
| Multi(2) + threshold + count + 3× @i | 9 |
| **Total bytecode** | **29 B** |

- Payload chars: ⌈29×8/5⌉ = 47
- Total: 47 + 18 = **65 chars** → regular
- **Source:** standard "decaying multisig" pattern; BIP 388-clean variant
  using 6 distinct keys (rather than the textbook 3-key version which
  reuses keys and is therefore invalid).

**Use:** validates the BIP 388-compliant form of decaying multisig. Notes
that "decaying" with the same-but-fewer keys is forbidden by BIP 388;
must use distinct keys for the recovery quorum.

---

## Summary table

| ID | Description | Bytecode | Chars (regular) | Chars (long) | Chunks |
|---|---|---|---|---|---|
| C1 | `pk(@0)` | 7 B | 30 | — | 1 |
| C2 | 2-of-3 sortedmulti | 13 B | 39 | — | 1 |
| C3 | 2-of-3 + timelock recovery | 21 B | 52 | — | 1 |
| C4 | User's 6-key inheritance | 45 B | 90 | 92 | 1 |
| C5 | 5-of-9 + 2-of-2 recovery | 56 B | — | 108 | 1 (max) |
| C6 | Chunking-forced | TBD | — | — | 2+ |
| **E10** | **Liana Simple Inheritance** | **17 B** | **46** | — | 1 |
| **E12** | **Liana Expanding Multisig** | **21 B** | **52** | — | 1 |
| **E13** | **HTLC with sha256 preimage** | **49 B** | — | **99** | 1 (long) |
| **E14** | **Decaying multisig 3-of-3 → 2-of-3** | **29 B** | **65** | — | 1 |

---

## Implementation invariants

For each entry in the corpus, the reference implementation must:

1. Round-trip: BIP 388 wallet policy string → bytecode → string yields
   identical canonical form
2. Match the byte count above ±2 bytes (tolerance for tag-value changes)
3. ECC stress test: random single-char substitutions, multi-char errors,
   and consecutive erasures all decode correctly within BCH guarantees
4. Decoder rejects malformed inputs loudly with specific error messages

---

## Updates required as spec firms up

- Update byte counts when final tag values are chosen
- Add C6 once a realistic chunking-forcing miniscript is identified
- Add taproot variants (tap-miniscript) once tap encoding is specified
- Add fingerprints-on variants once fingerprint encoding is specified
- Add 1–2 v1+ examples (foreign xpubs, per-`@i` paths) for forward-compat
  testing once v1 spec is drafted
