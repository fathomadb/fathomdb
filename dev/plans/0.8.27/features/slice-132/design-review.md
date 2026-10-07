---
title: FathomDB 0.8.27 Slice 132 — design review
status: RESOLVED
target_release: 0.8.27
---

# Slice 132 design review

An independent Opus (high-effort) reviewer checked the plan and design against
the source at `d2800a158`. It edited nothing. The verdict was
**APPROVE-WITH-FIXES**.

## Confirmed

- The 44 operation rows and the core calls in the design match the calls that
  `fathomdb-py` and `fathomdb-napi` make.
- These Python/TypeScript differences are real and correctly classified:
  - the `search_frozen` `pool_n` default;
  - `drain` units;
  - the `graph.search_expand` filter shape;
  - `DependencyTraceError` missing from `errors.__all__`.
- `default = []` is right, because Cargo unifies features across workspace
  builds.
- Adding a `rust` endpoint to the operation map does not affect the Python or
  TypeScript oracles or the signed-allowlist pin.

## Findings and resolutions

| Finding | Severity | Resolution in the design |
| --- | --- | --- |
| The bindings reject an embedded NUL with `WriteValidation` (AC-068a); the core does not. | P1 | Translation rule 7 adds the SDK guard, the list of guarded strings, and the `source_id` exemption. |
| The re-export rule could not be met from the engine alone. `DeviceResolution` was wrongly excluded. The schema and embedder types were unreachable. | P1 | The design now gives a concrete three-source re-export list and makes the schema, embedder, and embedder-api crates non-optional dependencies. |
| `search` dropped the unified `Filter`. | P2 | Added `SearchFilterArg`, lowered with `Filter::to_search_filter()`. |
| `graph::search_expand` would forward `attributes`, which neither SDK does. | P2 | Non-empty attributes are refused with `InvalidArgument`. |
| `read.list` `limit` (100) and `graph.neighbors` `direction` (`Both`) lost their defaults. | P2 | Added `ListOptions` and `NeighborsOptions`. |
| Core request structs have no defaults. | P2 | Rule 4: Rust callers set every field. The interface document lists the binding defaults. |
| Host-language argument errors were misdescribed. | P2 | Rule 5 maps them to `InvalidArgument`. Python `rerank`'s missing `alpha` check goes to the ledger. |
| `Validation` could not carry post-call errors. | P2 | Renamed to `Sdk`, covering errors from the SDK and from components outside the engine. |
| `embed_batch_cls` with empty input and no feature contradicted the bindings. | P2 | Without the feature, every call returns `EmbedderNotConfigured`. |
| The source-scan observer was under-specified, and the checker fixture would break. | P2 | Added the scan scope, `RUST_NON_COMMAND`, the fixture update, and in-unittest observation. |
| `local-dry-run.sh` `LEAVES` was wrong; the hidden-surface probe needs a `ROWS` entry. | P2 | Removed the `LEAVES` entry; deferred the probe to Slice 150. |
| The unruled `slice-132-external-provider-disposition` decision was not addressed. | P2 | It stays open, does not block this slice, and is carried forward at close. |
| Nits: the variant count, the base name, the exhaustiveness claim, the `close` order, the `search_limit` spelling, the limit-check order, and the documentation cross-references. | P3 | All folded in. |
| Use `compile_fail` doctests rather than trybuild golden files. | P3 | Adopted. |
| Drop the separate verification pass. | P3 | Not adopted. The user requires an independent Sonnet verification. |

## Second review (Fable, one-shot)

The user requested a one-shot Fable 5.1 review of the revised design. It
verified the design at `83f461c2c` and returned **APPROVE-WITH-FIXES**.

| Finding | Severity | Resolution |
| --- | --- | --- |
| The rule that refuses trait impls on `Engine` was pinned by tests but not in the design. | P1 | Documented. The checker refuses every trait impl other than `Drop` and `Debug`. |
| The `SubscriberEvent` name and the order of `ErrorKind::ALL` were pinned by tests but not documented. | P1 | Documented. The implementation already matched the tests. |
| Type-level leaks were invisible to the function scan. | P2 | The checker now refuses glob or core-`Engine` re-exports, public `Engine` fields, and extra `pub mod`s. RED tests landed first. The remaining gap for re-exported types is recorded until Slice 150. |
| The rerank non-finite-score behavior hid a TypeScript divergence. | P2 | Documented as an explicit exception: `WriteValidation`, following Python and napi. |
| The facade list would import `encode_resolved_graph_evidence_v1` and `Subscription`. | P2 | Both excluded. Neither SDK exposes them. |
| The lifecycle re-exports were not closed over their field types. | P2 | Added `Phase`, `EventSource`, `EventCategory`, and `ProjectionStatus`. |
| The interface doc referred to a mapping table that did not exist. | P2 | Added the table, along with the forms an optional view can take. |
| Validation order, the redundant limit pre-check, empty-string divergences, scan regex hygiene, the unnameable open-error payload. | P3 | The SDK follows Python's order, so the limit pre-check stays. Empty strings are left to the core. The regex now covers `const`, `async`, and `unsafe`. `RuntimeEmbedderError` is re-exported. |
