# md-cli JSON schema v1

Every JSON output carries `"schema"`. Schema version bumps with breaking changes.

## Version history

| Value | Since | What changed |
| --- | --- | --- |
| `md-cli/1` | md-cli v0.4.3 (md-codec v0.31.0) | Initial `--json` surface. |
| `md-cli/2` | stage 1b task 6 (F-449, SPEC §4a) | `JsonBody::Tr` gained `unspendable_kind` (`Option<&'static str>`, `Some("liana_unspendable")` or absent). **Additive at the wire-shape level** — the field is `#[serde(skip_serializing_if = "Option::is_none")]`, so every object this schema ever emitted before still deserializes unchanged, and no field was removed or renamed. **NOT additive at the semantic level, which is why the version moved**: every `Tr` object `md-cli/1` had EVER emitted satisfied the invariant "`is_nums == true` implies the internal key is the BIP-341 NUMS H-point" (see the `JsonBody` entry below, pre-v2 wording). This task is what makes that invariant stop holding through a normal `md decompose`/`md encode` flow — a second, DIFFERENT unspendable taproot internal key (Liana's own derived key, SPEC §2) can now also set `is_nums == true`, distinguishable only by checking `unspendable_kind`. A consumer written against `md-cli/1` that (until now, safely) treated `is_nums` as synonymous with "the provably-unspendable, no-known-discrete-log NUMS point" would silently misclassify a kind-1 wallet's internal key. This filename stays `docs/json-schema-v1.md` — schema versions are documented in this one file rather than split across per-version files, since every past `--json` shape a caller might still be reading is still described on this page. |

The wire-shape addition itself (making `JsonBody::Tr` carry the field at all)
landed one task earlier than the version bump above, in the SAME `Body::Tr`
match that produces `is_nums`/`key_index` — additive changes are recorded
here even when they predate the version they are folded into, so the two
entries in `format/json.rs`'s own history (the shape, then the version) do
not have to be reverse-engineered from git blame.

## Hex encoding
- `[u8; N]` and `Vec<u8>` → lowercase hex, no `0x` prefix.
- Identity-hash fingerprints → `"0x" + 8 hex chars`.

## Top-level wrappers per subcommand

### `encode --json`
| Field | Type | Always present? |
|---|---|---|
| `schema` | string | yes |
| `network` | string — `"mainnet"`/`"testnet"`/`"signet"`/`"regtest"` | yes (always; defaults to `"mainnet"`) |
| `phrase` | string | iff *not* `--force-chunked` |
| `chunk_set_id` | string `0xXXXXX` | iff `--force-chunked` |
| `chunks` | array of string | iff `--force-chunked` |
| `policy_id_fingerprint` | string `0xXXXXXXXX` | iff `--policy-id-fingerprint` |

### `decode --json`
| Field | Type |
|---|---|
| `schema` | string |
| `descriptor` | `JsonDescriptor` (see below) |

### `inspect --json`
| Field | Type |
|---|---|
| `schema` | string |
| `descriptor` | `JsonDescriptor` |
| `md1_encoding_id` | `JsonHash` |
| `wallet_descriptor_template_id` | `JsonHash` |
| `wallet_policy_id` | `JsonHash` (with `fingerprint`) |

### `bytecode --json`
| Field | Type |
|---|---|
| `schema` | string |
| `payload_bits` | u32 |
| `payload_bytes` | u32 |
| `hex` | string |

### `compile --json`
| Field | Type |
|---|---|
| `schema` | string |
| `template` | string |
| `context` | `"tap"` or `"segwitv0"` |

### `compose --json`
| Field | Type |
|---|---|
| `schema` | string |
| `template` | string — the origin-less template |
| `template_with_origins` | string — the inline-origin form `md encode` reads back |
| `wrapper` | string — `"tr"`/`"wsh"`/`"sh-wsh"`/`"sh"` |
| `slots` | array of `{ "index": u32, "path": usize, "ordinal": u32 }` — `path` is **0-based** |
| `internal_key_path` | usize or `null` — taproot only |
| `experimental` | array of string — **PROSE for humans**, byte-identical to the `warning: EXPERIMENTAL:` lines on stderr. Its path numbers count from **1**. |
| `experimental_paths` | array of `{ "kind": "keyless_path"\|"unsorted_keys", "path": usize }` — `path` is **0-based** and joins `slots[].path` |
| `preset` | `{ "name": string, "params": object }` or `null` — present with `--preset` |

**Join on `experimental_paths[].path`, never on the numbers inside
`experimental[]` (F-603).** The two count differently on purpose: the prose is
the human-facing stderr sentence, where "path 1" means the first path. Reading
its number as a `slots[].path` value yields a false statement with no parse
error to warn you — measured on a three-path wallet whose object says "path 2
has no key" while `slots[].path == 2` carries key slots @3 and @4.

`experimental_paths` was added alongside `experimental` rather than replacing
it, so no consumer of the prose array breaks and the schema version does not
move.

### `address --json`
| Field | Type |
|---|---|
| `schema` | string |
| `network` | string — `"mainnet"`/`"testnet"`/`"signet"`/`"regtest"` |
| `addresses` | array of `{ "chain": u32, "index": u32, "address": string }` |

Example:

```json
{
  "schema": "md-cli/2",
  "network": "mainnet",
  "addresses": [
    { "chain": 0, "index": 0, "address": "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu" }
  ]
}
```

## Shadow types

### `JsonDescriptor`
| Field | Type |
|---|---|
| `n` | u8 |
| `path_decl` | `JsonPathDecl` |
| `use_site_path` | `JsonUseSitePath` |
| `tree` | `JsonNode` |
| `tlv` | `JsonTlv` |

### `JsonPathDecl` (adjacent-tagged)
- `{"tag": "Shared", "data": "m/48'/0'/0'/2'"}`
- `{"tag": "Divergent", "data": ["m/...", "m/..."]}`

### `JsonUseSitePath`
| Field | Type |
|---|---|
| `multipath` | `[{"hardened": bool, "value": u32}, ...]` or `null` |
| `wildcard_hardened` | bool |

### `JsonNode`
| Field | Type |
|---|---|
| `tag` | string (Tag variant name, e.g. `"Wsh"`, `"Multi"`, `"PkK"`, `"Tr"`, `"TapTree"`). Stable since md-cli v0.4.3 / md-codec v0.31.0: emitted via a `JsonTag` serde mirror with `#[serde(rename_all = "PascalCase")]`. Byte-identical to the pre-v0.31 `format!("{:?}", tag)` output. |
| `body` | `JsonBody` |

### `JsonBody` (adjacent-tagged on `kind`)
Mirrors `tree::Body` variants under the v0.30 wire format:
- `{"kind": "KeyArg", "data": {"index": u8}}` — single key arg (Pkh, Wpkh, PkK, PkH at non-multi sites)
- `{"kind": "Children", "data": [JsonNode, ...]}` — wrapper nodes (Wsh, Sh, Check, Verify, AndV, AndOr, TapTree branches, …)
- `{"kind": "MultiKeys", "data": {"k": u8, "indices": [u8, ...]}}` — Multi / SortedMulti / MultiA / SortedMultiA (v0.30+ packs key indices at `kiw = ⌈log₂(n)⌉` bits)
- `{"kind": "Variable", "data": {"k": u8, "children": [JsonNode, ...]}}` — Thresh (mixed key + sub-policy children)
- `{"kind": "Tr", "data": {"is_nums": bool, "key_index": u8, "unspendable_kind": "liana_unspendable" | ABSENT, "tree": JsonNode | null}}` — Taproot root. The `is_nums` flag (v0.30+) replaces the pre-v0.30 `key_index = n` sentinel. Field order matches struct declaration (serde-serializes `is_nums`, `key_index`, `unspendable_kind`, `tree`, in that order). The inner `tree`, when present, is a plain `JsonNode` whose tag is either a leaf miniscript tag or `TapTree` for a branch.
  **`unspendable_kind` (since `md-cli/2`, stage 1b task 6, SPEC §4a):** ABSENT — not `null` — for `is_nums: false` (a real key slot) and for the literal BIP-341 NUMS H-point (`is_nums: true`, wire kind 0). `"liana_unspendable"` for Liana's derived unspendable internal key (`is_nums: true` ALSO, wire kind 1, SPEC §2) — the one case `is_nums` alone cannot distinguish. **A consumer that only ever checked `is_nums` before `md-cli/2` shipped was reading a schema where that check was sufficient; it is no longer sufficient** — check `unspendable_kind` whenever `is_nums` is `true` and the two internal-key kinds must be told apart (e.g. deriving an address: the NUMS point and Liana's derived xpub are different keys, and using the wrong one derives the wrong address). See the version-history table above.
- `{"kind": "Hash256Body", "data": "<hex64>"}` — 32-byte hash literal
- `{"kind": "Hash160Body", "data": "<hex40>"}` — 20-byte hash literal
- `{"kind": "Timelock", "data": u32}` — After/Older
- `{"kind": "Empty"}` — False/True

### `JsonTlv`
| Field | Type |
|---|---|
| `use_site_path_overrides` | `[(u8, JsonUseSitePath), ...]` or `null` |
| `fingerprints` | `[(u8, hex8), ...]` or `null` |
| `pubkeys` | `[(u8, hex130), ...]` or `null` |
| `origin_path_overrides` | `[(u8, "m/..."), ...]` or `null` |
| `unknown` | `[(u8, hex, u32), ...]` — `(tag, payload-hex, bit-length)` tuples for forward-compat round-trip |

### `JsonHash`
| Field | Type |
|---|---|
| `hex` | string |
| `fingerprint` | string `0xXXXXXXXX`, only on `WalletPolicyId` |
