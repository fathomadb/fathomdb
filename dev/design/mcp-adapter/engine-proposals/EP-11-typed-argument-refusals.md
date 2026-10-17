---
title: EP-11 — Typed reasons for argument refusals
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Replace the free-text payloads of InvalidArgument and InvalidFilter, and the bare WriteValidation used for empty search queries, with closed reason codes and a field path, so callers can give a specific remedy without parsing messages that may quote input.
---

# EP-11 — Typed reasons for argument refusals

Status: Draft. Priority P2. Not an accepted decision.

## Problem

- `EngineError::InvalidArgument { msg: String }` is "intentionally
  non-exhaustive so the binding layer can forward it"
  (`fathomdb-engine/src/errors.rs:222-228`). `InvalidFilter { reason: String }`
  is the same (`errors.rs:219-221`). A grep finds about 22 construction sites of
  `InvalidArgument` in engine files that are not named as tests.
- An empty search query returns `WriteValidation`, a write-side class, on both
  paths (`search_api.rs:501-503`, `search_api.rs:1035-1037`).
- Messages can echo caller input. For example, the limit message includes the
  given value (`search_types.rs:364-370`), and the existence-relaxation refusal
  names flags (`temporal.rs:146-160`). The adapter never forwards SDK text,
  because MR-012.2 forbids record content in errors.

Adapter workaround removed: `InvalidArgument`, `WriteValidation`,
`SchemaValidation`, `InvalidFilter`, `Page` and `FrozenRead` all collapse to one
template, "the engine rejected the request arguments (empty query, bad filter or
bounds)" (`src/rust/crates/fathomdb-mcp/src/error.rs:230-237`). The agent gets
no field and no specific remedy (MR-054).

## Proposed surface

```rust
#[non_exhaustive]
pub struct ArgumentRefusal {
    pub reason: ArgumentRefusalReason,   // closed, #[non_exhaustive]
    pub field_path: &'static str,        // e.g. "/limit", "/view/includeSuperseded"
    pub msg: String,                     // unchanged human text
}
// EngineError::InvalidArgument(ArgumentRefusal)
// EngineError::InvalidFilter(ArgumentRefusal)
```

- Reasons start from the sites that exist. Examples: `OutOfRange`, `EmptyQuery`,
  `UnsupportedViewRelaxation`, `UndeclaredFilterAttribute`, `UnknownQueryId`,
  `TelemetryDisabled`, `MutuallyExclusive`.
- An empty query moves from `WriteValidation` to
  `InvalidArgument{EmptyQuery, "/query"}`.
- This follows the pattern of the typed families that already exist:
  `FrozenReadError { reason, field_path }` (`frozen_read.rs:120-125`) and the
  graph expansion reasons with paths such as `/maxWorkUnits`.

## Invariants and consistency

- `reason` and `field_path` never contain caller input. `msg` may, and stays for
  logs.
- `ErrorKind` mapping is unchanged (`InvalidArgument` stays
  `InvalidArgumentError`), so binding exception classes do not change. Bindings
  gain `.reason` and `.field_path` attributes.

## Interaction with ADRs

- `ADR-0.6.0-error-taxonomy.md`: this changes a variant's payload, so binding
  error tables change. It needs an amendment, not a successor.
- `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md` permits the
  in-place change.
- `ADR-0.8.27-rust-sdk-parity.md`: Python and TypeScript expose the same
  attributes.

## Cost and performance

None at run time.

## Test plan sketch

- For each reason, a real call that triggers it and asserts the reason and path
  in Rust, Python and TypeScript.
- A lint or test asserting that no `field_path` is built with `format!`.
- An empty query on `search` and on `search_text_only` returns `EmptyQuery`.

## Adapter change once it lands

`from_sdk` maps each reason to a specific code and remedy, for example
`bad_argument/limit_out_of_range` with "use 1..=100". The detail carries the
field path.

## Open questions for the owner

1. One shared `ArgumentRefusal` for both variants, or separate reason enums?
2. Is moving the empty-query refusal off `WriteValidation` acceptable as a
   breaking change, given that bindings raise a different exception class for
   it today?
