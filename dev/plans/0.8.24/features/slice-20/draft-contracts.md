---
title: 0.8.24 Slice 20 — accepted slice-local contracts
status: ACCEPTED-LOCAL
target_release: 0.8.24
---

# Slice 20 — accepted slice-local contracts

These contracts refine the existing REQ-010/AC-076 and direct-text prefix
contract for this slice. They do not modify canonical needs, requirements,
acceptance criteria, or public API shape.

## Draft review and disposition

| Draft | Disposition | Accepted wording |
| --- | --- | --- |
| N20-DRAFT | **Adjust and accept locally.** | Direct FTS callers need the selected bounded rank-stream implementation without any weakening of exact ordered results. |
| R20-DRAFT-1 | **Accept locally.** | Eligible direct-text node collection completes the BM25 score group crossing the fixed 100-candidate boundary, restores stable `(score, write_cursor)` order, and truncates to 100. |
| R20-DRAFT-2 | **Adjust and accept locally.** | Filters, edge-bearing databases, hybrid/vector requests, old-schema paths, forced test controls, and any streamed prepare/step/conversion failure retain the current exact full-sort behavior. |
| AC20-DRAFT | **Adjust and accept locally.** | Strict, crossing-tie, all-equal, generated-group, failure-fallback, filter/edge-ineligible, query-plan, and public prefix checks must detect removal or unsafe broadening of the streamed path. |
| New general latency promise | **Reject.** | The retained result selects an algorithm; it does not revise REQ-010/AC-076 or create a new capacity guarantee. |
| General PRAGMA/default tuning | **Reject with one writer invariant correction.** | mmap, cache, temp-store, reader-pool, and runtime connections remain current-main behavior. The writer explicitly applies the already-accepted `WAL + synchronous=NORMAL` durability contract. |
| Public route telemetry | **Reject.** | Test oracle controls and route observations are private to `test-hooks`; no SDK/runtime contract is created. |

## Accepted requirements

- **R20-1 — bounded eligibility.** The streamed path applies only when the
  request is explicitly direct text-only, has no metadata filter, uses the
  fixed 100-node candidate window, and the database has no edge-body FTS rows.
- **R20-2 — rank-boundary completion.** The path reads FTS5 native rank order
  through the first row with a score different from the score at the
  100-candidate boundary. It retains the complete boundary group, restores
  ascending BM25 score then ascending `write_cursor`, and truncates to 100.
- **R20-3 — exact fallback.** A streamed statement preparation, iteration, or
  row-conversion error discards partial streamed output and executes the
  existing full stable-sort query. Ineligible paths execute that query
  directly.
- **R20-4 — contract preservation.** Node validity, active/superseded state,
  stable identity, provenance, edge fusion, body deduplication, RRF, public
  result limits, and direct-text prefix stability are unchanged.
- **R20-5 — accepted writer durability and no tuning.** The writer explicitly
  applies `WAL + synchronous=NORMAL` as required by
  ADR-0.6.0-durability-fsync-policy. The implementation changes no reader-pool
  or runtime connection, cache, mmap, temp-store, pool-size, or experimental
  setting.
- **R20-6 — private proof seam.** The full-sort oracle and route observation
  compile only with `test-hooks` and are absent from normal artifacts.
- **R20-7 — evidence discipline.** The existing SCALE-02 receipt remains the
  sole performance decision evidence; test runs prove correctness only.

## Acceptance criteria

1. A direct-text request over a strict boundary selects the streamed path and
   matches the forced full-sort result exactly.
2. A score group crossing row 100, including an all-equal corpus larger than
   100, returns exactly the stable full-sort first 100 node candidates and
   preserves caller-limit prefixes.
3. Generated score-group shapes produce the same stable prefix from the helper
   as a complete stable sort.
4. A row-conversion or statement-execution failure on the streamed attempt
   discards partial output and returns the existing serviceable full-sort
   result/error semantics.
5. Filters and edge-bearing databases do not select the streamed path, and
   existing filter/edge results remain unchanged.
6. The eligible statement pins `rank MATCH 'bm25()'`, remains exact even after
   a database changes its persistent rank mapping, and contains no temporary
   ORDER-BY B-tree on the bundled SQLite version.
7. A narrow private writer witness reports WAL and `synchronous=NORMAL` (`1`);
   reader-pool/runtime code is unchanged.
8. The existing `slice23_text_limit_prefix_stability` suite remains green;
   fixed-validity, hybrid fallback, and legacy-schema coverage stays green.
9. Targeted engine tests plus full workspace clippy/check and normal local
   verification pass without a benchmark or hosted workflow.
