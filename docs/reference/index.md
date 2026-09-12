# Reference

Reference for the published 0.8.25 surface. Field spellings and type-level details are authoritative
in the locked internal interface specs (`dev/interfaces/{python,typescript,cli}.md`);
this section is the client-facing view.

- [Python API](python-api.md) — `Engine`, `admin`, governed reads,
  data-plane additions, data shapes, and instrumentation methods.
- [TypeScript API](typescript-api.md) — Promise-based `Engine`, `admin`,
  governed reads, data-plane additions, data shapes, and instrumentation.
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

The reference reflects the shipped surface. These are documented gaps:

- Published performance evidence is not a general latency SLA; see
  [compatibility § performance posture](../compatibility/index.md).
- The `SearchFilter.status` field is wired end-to-end but has no
  population source, so a `status=`-filtered query prunes every row.
- Custom Python / TypeScript embedder implementations are not exposed;
  the binding choice is the built-in default embedder or none.
