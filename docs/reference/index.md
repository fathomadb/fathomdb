# Reference

Reference for the 0.8.27 release. Field spellings and type-level details are
authoritative in the interface specs
(`dev/interfaces/{python,typescript,rust-sdk,cli}.md`); this section is the
client-facing view.

- [Python API](python-api.md) — `Engine`, `admin`, governed reads,
  data-plane additions, data shapes, and instrumentation methods.
- [TypeScript API](typescript-api.md) — Promise-based `Engine`, `admin`,
  governed reads, data-plane additions, data shapes, and instrumentation.
- [Rust SDK](rust-sdk.md) — `fathomdb-sdk`, the Rust peer of the Python and
  TypeScript SDKs (0.8.27)
- [Rust API](rust-api.md) — facade model, Engine methods, public carrier
  families, feature boundary, and links to generated rustdoc.
- [CLI](cli.md) — `fathomdb doctor` + `fathomdb recover` verbs, flag
  spelling, exit-code classes, JSON output shape.
- [Errors](errors.md) — the shared typed error taxonomy, base class,
  triggers, structured payloads, and recovery hint codes.
- [Config](config.md) — `EngineConfig` knobs (Python snake_case + TS
  camelCase column).

Item-level Rust API documentation is published at `docs.rs/fathomdb`; the page
above explains how that generated surface maps to FathomDB's governed facade.

## Known limitations

These are documented gaps in the release surface:

- Published performance evidence is not a general latency SLA; see
  [compatibility § performance posture](../compatibility/index.md).
- The `SearchFilter.status` field is wired end-to-end but has no
  population source, so a `status=`-filtered query prunes every row.
- Custom Python / TypeScript embedder implementations are not exposed;
  the binding choice is the built-in default embedder or none.

The generated [SDK operation map](../generated/sdk-operation-map.md) indexes
matching names and owners across the three languages. Detailed signatures,
options, results and errors remain in the reference pages above.
