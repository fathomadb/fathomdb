# TRACE-01 projection lifecycle contract

**Track:** `TRACE-01`

**Date:** 2026-08-16
**Scope:** fixed synthetic lifecycle fixture only; no corpus, extractor, model, GPU,
or external service execution.

## Purpose

`trace-projection.v1` is a safe, deterministic sidecar for the canary. It
checks that each derived retrieval projection remains attributable to one
canonical source through supersession, erasure, and re-open. It is not a
database lifecycle implementation and does not modify the experiment receipt,
index, or a harness configuration.

## Inputs

- A registered canonical source has `source_id` and `source_sha256`.
- A derived projection has one `projection_id`, one `source_id`, the matching
  `source_sha256`, and a projection kind. The canary inventory covers canonical
  text, vector child, summary, extracted fact, entity, and edge rows.
- ELPS supersession arrives only from the accepted result-envelope warning:
  `kind: "supersedes"` with `source_doc_id`, `prior_body`, and
  `supersedes_hint`. The sidecar hashes `prior_body` transiently to resolve one
  registered prior source; it never writes that body or hint.
- The competing extractor-input edge fields `supersedes_prior` and
  `prior_body` are not TRACE input representations. Their presence on an edge
  is a contract error, rather than an alternate mapping path.
- Lifecycle events are `erase` and `reopen` for a registered source. A
  `reopen` is valid only after that source was erased in the same trace.

## Required safety and lifecycle rules

1. Every source and projection identifier must match the safe identifier grammar
   `[A-Za-z0-9][A-Za-z0-9._:-]{0,127}`. Every projection must identify exactly
   one registered source and reproduce that source's hash. A repeated projection
   identifier with different sources, an unknown source, or a hash mismatch is
   rejected as ambiguous or unattributed attribution.
2. A `supersedes` warning must resolve exactly one prior source by the hash of
   its `prior_body`; otherwise the trace is rejected. The old source and all of
   its projections become `superseded` and non-searchable.
3. Erasure makes the named source and all of its projections `erased` and
   non-searchable. Re-open restores only an erased source and its projections to
   `active` and searchable. It does not revive a superseded source.
4. The produced sidecar may contain only schema/version identifiers, source and
   projection identifiers, SHA-256 values, projection kinds, lifecycle states,
   booleans, counts, and fixed diagnostic codes. It never contains source text,
   `prior_body`, `supersedes_hint`, corpus paths, model output, or credentials.
   `write_trace_projection` validates every sidecar field, ordering, lifecycle
   relation, count, and diagnostic before writing; it fails closed on an
   arbitrary mapping.

## `trace-projection.v1` output

The sidecar has these top-level fields:

| Field | Meaning |
| --- | --- |
| `schema_version` | Exactly `trace-projection.v1`. |
| `sources` | Safe source identifiers, hashes, and final lifecycle state. |
| `projections` | Safe derived-row identifiers, source identity/hash, kind, state, and searchable flag. |
| `supersessions` | New/prior source identifiers and the prior-body SHA-256; no raw warning fields. |
| `outcomes` | Source/projection lifecycle and attribution counts, including stale-searchable count. |
| `diagnostics` | Fixed non-payload lifecycle codes only. |

The canary succeeds only when the fixed fixture builds this complete sidecar,
has zero unattributed projections, and has zero stale searchable projections
after supersession or erasure.

## Downstream boundary

`PARENT-01` may consume this warning-only mapping and safe sidecar shape after
TRACE-01 is accepted. It must not infer a second ELPS representation or treat
the sidecar as live execution evidence.
