---
title: Astra adversarial review of the Inspector 0.8.29 port
date: 2026-11-18
status: ACTIVE
desc: Bounded-inspection finding remediated and approved by independent Astra re-review
---

# Inspector 0.8.29 port — adversarial review

Astra-medium reviewed imported port `551625462` against unchanged Engine base
`6edc1155f`. Initial verdict: one concrete P2 requiring remediation; no other
findings were reported. Independent Astra-medium re-review approved the actual
remediation at `0262425fa`; the prior P2 is resolved with no new material findings.

## P2 — repeated queued-scope scans exhaust the request budget

`Reader::summary_fields` queried every queued JSON cursor array separately for
each canonical record. Catalogs and pages repeat that work on one connection,
whose real guard allows 100 million VM operations/30 seconds. Valid large queued
scopes can exhaust it before ordinary bounded inspection completes; the error
was misleadingly mapped to unknown disclosure classification.

The initial reviewer demonstrated the repeated lookup using system SQLite
3.45.1: a constructed 16-by-16,384-cursor query hit its operation cap after 76
unrelated lookups. That was a SQL microproof, **not a real Engine fixture or
runtime qualification**. The Engine's actual request/metadata caps were checked
against source; no fabricated queue is used as positive acceptance evidence.

A real Engine regression then created 8,192 ordinary canonical targets plus
200 unrelated records, accepted one exact source-cut queue using the existing
ExplicitMaintenance API, and reopened it normally. At RED `ec41ce56a`, the
catalog failed with `unknown_classification` (12.03 seconds for the whole test,
including fixture setup). The oracle requires a complete exact catalog, usable
200-record unrelated page, correct target refusal, and fresh content access
after a test-only public withdrawal. No Engine deletion or persisted policy is
changed by Inspector.

## Implemented remediation

- `ReadConnection` lazily builds one sorted exact cursor set per SQLite read
  transaction and reuses it for membership checks. It is dropped with that
  snapshot; neither session lifetime nor visibility generation reuses it across
  requests. The real withdrawal case verifies freshness.
- Consumed queued metadata is bounded by 16 requests, 8 MiB, at most 16,384
  cursors per scope and 262,144 total positive integer cursors (at most about
  2 MiB of vector storage). Interpreted version/array/logical-state shapes are
  checked before membership is trusted; invalid or oversized input fails closed.
- SQLite interruption is `budget_exhausted`, not `unknown_classification`.
  An actual SQLite progress-handler interruption failed at RED `4a843a1c2` and
  passed after GREEN `a50f7241b`. This uses the owned real connection, not a
  sleep/race or mocked database.
- Cache GREEN `a8390d60d` passes the unchanged real catalog/page/freshness
  regression (10.42 seconds including setup). These timings are test outcomes,
  not an isolated performance benchmark or cross-platform claim. Existing D-01
  flags, exact scope, independent inactive/telemetry restrictions and physical
  deletion semantics remain unchanged.

## Verification and remaining limits

Final focused lint and ordinary Inspector regression results are recorded in
[the port verification](../plans/0.1/compatibility-0.8.29-port.md). Required full
source verification was attempted and stopped because checkout-owned
`.venv/bin/python` is missing. The four prior explicit scale/allocation/native
GUI qualifications and installed-release qualification remain unperformed by
this remediation. No full-workspace green is claimed.

**Astra re-review: APPROVE at `0262425fa`.** The prior P2 is resolved; no new
material findings. The reviewer confirmed the bounded fail-closed cursor cache
belongs to each read transaction, fresh `ReadConnection` instances observe
changed scope, and SQLite interruption remains budget exhaustion.

The reviewer independently ran the existing compiled 37-test artifact's real
8,192-target catalog/unrelated-page/withdrawal test (PASS, 10.41 seconds) and
queued SQLite interruption test (PASS, 0.17 seconds). Its Cargo invocation could
not open the build-cache lock in the read-only sandbox, so these were direct
artifact executions, not a fresh reviewer compilation. The earlier writer's
all-target clippy and ordinary suite remain separately recorded evidence.

Scope recheck confirmed Engine/SDK/MCP/schema and all pre-existing lockfile
packages remain unchanged. This approval adds no browser, native GUI, scale,
packaging or full-workspace qualification claim. The requested adversarial
review, remediation and re-review cycle is complete.
