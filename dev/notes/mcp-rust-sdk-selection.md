---
title: Rust MCP SDK selection for FathomDB — rmcp versus rust-mcp-sdk
date: 2026-10-17
status: research note (non-canonical; see dev/notes/README.md)
purpose: Compare the two Rust MCP SDKs against FathomDB's goals and constraints and recommend one for a future MCP adapter. Input to an ADR; does not itself decide or commission work.
inputs:
  - https://github.com/modelcontextprotocol/rust-sdk and https://crates.io/api/v1/crates/rmcp
  - https://github.com/rust-mcp-stack/rust-mcp-sdk and https://crates.io/api/v1/crates/rust-mcp-sdk
  - rmcp release notes for v3.0.0 and the migration discussions #926 and #969
  - dev/adr/ADR-0.6.0-async-surface.md, ADR-0.6.0-cli-scope.md, ADR-0.6.0-json-schema-policy.md, ADR-0.6.0-crate-topology.md
  - dev/research/personal-agent-database-market-2026-07-02.md (MCP as the top integration gap)
  - dev/notes/agent-datastore-priorities-critical-review.md (MCP adapter as next step 1)
  - Cargo.toml, rust-toolchain.toml, deny.toml
---

# Rust MCP SDK selection for FathomDB

> Facts were gathered on 2026-10-17 from the crates.io API, the GitHub API and
> each repository's README and release notes. Version, license, MSRV and
> download figures were re-checked directly against crates.io. Items marked
> *unverified* were not confirmed from a primary source.

## 1. Recommendation

Use **`rmcp`**, the official SDK, pinned exactly, confined to a thin adapter
crate. The deciding factors are a minimal stdio-only dependency set, an
in-memory duplex transport that supports real-engine TDD, schemars 1.x output
schemas that match FathomDB's JSON Schema 2020-12 policy, and official
stewardship. Its main cost is major-version churn; §5 lists the containment.

## 2. FathomDB constraints that bear on the choice

- **Scope.** FathomDB is an embedded retrieval and graph store, positioned
  below the agent loop; Memex, Hermes and OpenClaw build on it. MCP is absent
  from every ADR and the roadmap, and the July market report names it the
  largest table-stakes integration gap.
- **Sync engine.** `ADR-0.6.0-async-surface` makes the engine surface sync
  everywhere except the TypeScript binding, with a single writer thread. Tokio
  is a dependency today only in `fathomdb-napi`.
- **No query verbs on the CLI.** `ADR-0.6.0-cli-scope` rejects application
  `search` / `get` / `list` verbs on the `fathomdb` CLI. An MCP server therefore
  needs its own crate and binary (for example `fathomdb-mcp`), built on
  `fathomdb-sdk`, with its own ADR and `dev/interfaces/` document because
  public surface is contract.
- **Schema dialect.** `ADR-0.6.0-json-schema-policy` uses JSON Schema draft
  2020-12.
- **Toolchain and policy.** Rust 1.95, edition 2021; MIT; `deny.toml` allows
  MIT, Apache-2.0 and Unicode-3.0 (cargo-deny is not currently wired into the
  scripts); exact version pins; TDD against a real database, never a mock.
- **Transport priority.** Local-first and embedded, so stdio launch is the
  primary transport; it is how Claude Code, OpenClaw and Hermes start MCP
  servers. Streamable HTTP is secondary.

## 3. The two crates

### `rmcp`

- **Release.** 3.5.1 on 2026-10-05. 1.0.0 on 2026-03-03, 2.0.0 on
  2026-06-29, 3.0.0 on 2026-07-28; about 20 releases in six months.
- **Breaking changes.** Most 0.x minors broke. After 1.0, a major per spec
  revision: 2.0 aligned to the 2025-11-25 spec; 3.0 added 2026-07-28
  (sessionless streamable HTTP, stateless lifecycle, Tasks moved to an
  extension, deprecated APIs removed). No LTS line.
- **Governance and adoption.** `modelcontextprotocol` organisation, about 210
  contributors, about 4,000 stars, active daily. 36.9M total and 20.5M recent
  downloads; 2,239 dependent crates.
- **License, MSRV, edition.** Apache-2.0; Rust 1.88; edition 2024 (does not
  constrain an edition-2021 dependent). GitHub's licence detector reports
  NOASSERTION; read the repository licence file before adoption.
- **Spec support.** Implements 2026-07-28 with features gated on the
  negotiated version, compatible with 2025-11-25 and earlier. Structured output
  with derived `outputSchema`, elicitation (form and URL), Tasks extension,
  progress, cancellation, cursor pagination with `list_all_*` helpers,
  subscriptions. Tool annotations and resource links are expected from the
  2025-11-25 model alignment but were *unverified* individually.
- **Transports.** stdio (`transport-io`), client-side child process, streamable
  HTTP server as a Tower service, streamable HTTP client, `transport-worker`.
  Legacy HTTP+SSE is a stated non-goal.
- **Runtime and features.** Tokio only. Defaults are `base64`, `macros`,
  `server`, with no axum, hyper or reqwest. A minimal stdio server needs
  `server`, `macros`, `transport-io`.
- **Tool ergonomics.** `#[tool]`, `#[tool_router]`, `#[tool_handler]`, prompt
  macros. Schemas from **schemars 1.x** (draft 2020-12); a tool returning
  `Json<T>` with `T: JsonSchema` gets an `outputSchema`.
- **Testing.** `transport-async-rw` accepts any `AsyncRead`/`AsyncWrite`; the
  crate's own tests use `tokio::io::duplex`. CI runs the official conformance
  scenarios.
- **Security.** Five fixed high-severity advisories (GHSA-9pj6-vhgr-3mwh,
  GHSA-89vp-x53w-74fx, GHSA-c9xm-49cp-xcr9, GHSA-9g45-5xwm-f3wc,
  GHSA-33f5-2c5q-wgwj), all in HTTP or OAuth paths; none in stdio.
- **Open issues of note.** #1374 (`outputSchema` derived from the Deserialize
  contract while results are serialized, so they can disagree); #1092 (`#[tool]`
  compile failure with some closure patterns); #1067 (feature-flag cleanup).
- **Users.** goose, apollo-mcp-server, rustfs-mcp, hyper-mcp, mise, rig.

### `rust-mcp-sdk`

- **Release.** 2.0.0 on 2026-08-27; 1.0.0 on 2026-07-26; about five releases
  in six months, none for seven weeks.
- **Breaking changes.** Frequent in 0.x; 2.0 a month after 1.0. 2.x targets
  2026-07-28; a **1.x LTS line for 2025-11-25** is kept on `release-1.x`.
- **Governance and adoption.** `rust-mcp-stack` organisation, effectively a
  single maintainer (301 of the human commits; every other contributor at four
  or fewer). 13 contributors, about 200 stars; only dependency bumps since
  2026-09-19. 315K total and 117K recent downloads; 52 dependent crates.
- **License, MSRV, edition.** MIT; Rust 1.80; edition 2021.
- **Spec support.** `rust-mcp-schema` 2.0 carries feature flags for
  2024-11-05 through 2026-07-28 plus a draft. README claims full server and
  client conformance passes. Elicitation, auto-pagination and macro-generated
  `output_schema` are documented; Tasks, progress, cancellation and resource
  links are *unverified*.
- **Transports.** stdio, streamable HTTP (POST-only in 2.x), legacy SSE,
  client-side child-process launch; HTTP servers via `rust-mcp-axum` and
  `rust-mcp-actix`.
- **Runtime and features.** Tokio. Defaults enable client, server, auth,
  macros, stdio, SSE and streamable HTTP, pulling in reqwest and jsonwebtoken. A
  trimmed stdio server still requires hmac, sha2, base64, time, uuid, http and
  bytes.
- **Tool ergonomics.** `#[mcp_tool]`, `tool_box!`, resource, prompt and
  elicitation macros. Schemas from the crate's **own derive**, not schemars;
  only named-field structs and enums, and it panics on other shapes.
- **Testing.** A conformance-server crate and wiremock-based HTTP tests; an
  in-memory duplex transport was not found (*unverified*).
- **Security.** No published advisories; much smaller user base and audit
  surface.
- **Users.** rust-mcp-filesystem, mistral.rs, moon, mcp-discovery, scim-server.

## 4. Fit against the constraints

| Criterion | `rmcp` | `rust-mcp-sdk` |
| --- | --- | --- |
| Stewardship | Official organisation, broad contributor base | Single maintainer, quiet since mid-September |
| Adoption | 36.9M downloads, 2,239 dependents | 315K downloads, 52 dependents |
| Lean stdio build | No HTTP stack by default | Heavy defaults; residual crypto and HTTP deps when trimmed |
| Schema dialect | schemars 1.x, draft 2020-12, matches policy | Custom derive with panic paths |
| Real-engine TDD | In-memory duplex transport, used upstream | None found |
| Spec coverage | Current spec plus older, feature-gated | Comparable claims, partly unverified |
| Toolchain and licence | 1.88 MSRV, Apache-2.0, both allowed | 1.80 MSRV, MIT, both allowed |
| Security posture | Five fixed advisories, all outside stdio | None published, little scrutiny |
| API stability | Major per spec revision, no LTS | Also churns; has a 1.x LTS line |

`rust-mcp-sdk`'s one structural advantage is the LTS line. The single
maintainer and the seven-week release gap undercut it for a component FathomDB
would maintain for years.

## 5. Risks of `rmcp` and their containment

1. **Major-version churn.** Pin with `=` as FathomDB already does for its own
   crates. Confine `rmcp` types to one adapter module; governed verbs and their
   types never depend on it. Budget a migration per spec revision.
2. **`outputSchema` and result disagreement (#1374).** Use result types with
   symmetric serde attributes, and add a test that validates every
   `structuredContent` against its declared `outputSchema`.
3. **Async SDK over a sync engine.** The adapter owns a tokio runtime and calls
   the engine through `spawn_blocking`; one `Engine` per server process
   preserves the single-writer invariant.
4. **HTTP attack surface.** Ship stdio only at first; enable streamable HTTP
   only behind its own ADR, with DNS-rebinding and origin checks reviewed
   against the advisories above.

## 6. Follow-on, if adopted

1. An ADR for a `fathomdb-mcp` crate and binary: stdio first, read-first,
   built on `fathomdb-sdk`, `rmcp` pinned exactly, with the CLI-scope boundary
   restated.
2. `dev/interfaces/mcp.md`: tool set and response envelope (`max_tokens`,
   snippets, continuation handles), per
   `dev/notes/agent-datastore-priorities-critical-review.md` §5.
3. A spike: one `search` tool, written test-first over the duplex transport
   against a real database file.
