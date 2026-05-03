# md-cli JSON schema v1

Every JSON output carries `"schema": "md-cli/1"`. Schema version bumps with breaking changes.

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

### `address --json`
| Field | Type |
|---|---|
| `schema` | string |
| `network` | string — `"mainnet"`/`"testnet"`/`"signet"`/`"regtest"` |
| `addresses` | array of `{ "chain": u32, "index": u32, "address": string }` |

Example:

```json
{
  "schema": "md-cli/1",
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
| `tag` | string (Tag variant name, e.g. `"Wsh"`, `"Multi"`, `"PkK"`, `"Tr"`, `"TapTree"`) |
| `body` | `JsonBody` |

### `JsonBody` (adjacent-tagged on `kind`)
Mirrors v0.14's `tree::Body` variants exactly:
- `{"kind": "KeyArg", "data": {"index": u8}}` — single key arg (Pkh, Wpkh, PkK, PkH, multi children)
- `{"kind": "Children", "data": [JsonNode, ...]}` — wrapper nodes (Wsh, Sh, Check, Verify, AndV, AndOr, TapTree branches, …)
- `{"kind": "Variable", "data": {"k": u8, "children": [JsonNode, ...]}}` — Multi/SortedMulti/MultiA/SortedMultiA/Thresh
- `{"kind": "Tr", "data": {"key_index": u8, "tree": JsonNode | null}}` — Taproot root (the inner `tree`, when present, is a plain `JsonNode` whose tag is either a leaf miniscript tag or `TapTree` for a branch)
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
