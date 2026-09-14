# Slice 15 graph-evidence surface manifest

The current maintained graph family occurs in 39 files, found with:

```text
rg -l "GraphTargetV1|GraphExpandResultV1|GraphExpandRequestV1|graph_expand|graphExpand" \
  src/rust src/python src/ts docs dev/interfaces
```

The prototype-only test and its Cargo feature declaration are excluded from
the maintained count.

| Category | Files | Affected concern |
| --- | ---: | --- |
| Rust product source | 5 | engine carrier/codec, facade, Python and N-API bindings |
| Rust graph fixtures | 16 | wire, governed surface, runtime, projection, provenance, RSS |
| Python package source | 5 | exported types, graph conversion, engine method and stub |
| Python graph fixtures | 4 | public carrier and runtime coverage |
| TypeScript package source | 2 | binding and exported types |
| TypeScript graph fixtures | 4 | public carrier and runtime coverage |
| Public API documents | 3 | Rust, Python and TypeScript reference pages |
| Interface contracts | 4 | Rust, Python, TypeScript and wire contracts |

Option A and option B both require deliberate review across this complete
surface. B changes every `GraphTargetV1` serialization and fixture because the
fields are required. A changes the existing V1 request/result family to carry
an opt-in first-generation evidence sidecar and adds intrinsic resolution, but
leaves treatment-off target bytes and semantics unchanged. Neither option
requires a V2 family, schema, migration, evidence table, cache, or raw-ID
resolver.

