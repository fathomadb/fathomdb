---
title: Inspector port to dev/0.8.29
date: 2026-11-18
status: ACTIVE
desc: Selective Inspector import and current Engine compatibility checks
---

# Inspector port to dev/0.8.29

Branch `llm/0.8.29-inspector-additional` starts at
`6edc1155f039f87cd0a37660cb27ae4373bbce71` on `dev/0.8.29`.
Inspector snapshot `7d3637657` from `dev/0.8.30` is imported selectively:
`src/inspector/`, `dev/inspector/`, `docs/inspector/`, historical design
redirects and minimal Cargo/docs/index/lifecycle wiring. No Engine, schema,
SDK, MCP, release state, database package version or existing dependency pin
is changed. Source Inspector is version 0.1.0 and remains unpublished.

## Current composition

- Engine workspace packages remain 0.8.28 as recorded on the selected 0.8.29
  branch; the compiled current-only format is 37. Old schema-34/35 files refuse;
  the imported historical comparison/review/status evidence is not new support.
- Inspector aligns with existing rusqlite 0.41 and its mutable progress-handler
  API. Cargo.lock adds only the Inspector package; every prior package stays pinned.
- Format 35 vector spans can represent multiple windows per canonical cursor.
  Current Slice 20 exposes projection declarations and actual scalar attribute
  associations, not stored vector rows or one-vector-per-record assertions.
  Slice 40 must qualify span/row population and exact origin cardinality.
- Format 36/37 preserves physical-erasure policy. Autonomous deletion remains
  atomic and absent content unavailable. Existing ExplicitMaintenance stores
  can have real queued exact cursor targets. Those targets obey D-01 erasure
  disclosure/export gates without being reclassified inactive or logically
  removed. Inspector never queues, withdraws, activates or delays maintenance.
- Format 37 adds clean visibility coalescing. Preflight and each request's
  transaction require `coalescing=0` alongside the existing identity/schema and
  visibility evidence; committed dirty state refuses without a false context.

## Verification

Executed against this checkout's unchanged 0.8.29 Engine and lockfile:

- `cargo clippy -p fathomdb-inspector --all-targets --locked --offline -- -D warnings`: passed.
- `cargo test -p fathomdb-inspector --locked --offline`: passed 35 unit/real-database,
  one CLI, one no-GUI and two worker tests. The four original explicit native-GUI,
  128 MiB allocation and F-06 scale cases remain ignored and were not requalified.
- Paired `cargo build -p fathomdb-inspector --bins --examples --locked --offline`:
  passed. Offline `--help`/`--build-description` report current schema 37 and
  default artifact without fixture controls. The packaging script passes `bash -n`;
  release archive/installed-artifact qualification was not repeated by this port.
- Default-build browser/API checks passed: `api.mjs`, `document.mjs`, `metadata.mjs`,
  `projection.mjs`, `operational-copy.mjs`, `association-choice.mjs`,
  `chunk-navigation.mjs` and `refusal.mjs`. After the matching explicit
  `--features fixture-control` paired build, unchanged `cache.mjs` and
  `response-order.mjs` passed. Default artifacts were restored afterward.
  The existing local Playwright module and Chromium build 1248 were used;
  no browser/model download or other checkout's Python environment was used.
- Guarded Markdown formatting, `./scripts/agent-lint-md.sh`,
  `./scripts/agent-lint-docs.sh`, `mkdocs build --strict` and `git diff --check`:
  passed. All pre-existing Cargo.lock package records compare equal to the base;
  only the new Inspector entry is added.
- Required `./scripts/agent-verify.sh` was attempted and stopped at preflight:
  `checkout-owned .venv/bin/python is missing; prepare this checkout before full verification.`
  No full-workspace green is claimed and no unrelated environment/release-state
  repair was folded into the port.

The old mean-recovery fixture now supplies format-37 vector lifecycle/validity
metadata while preserving its original refusal assertion. The browser refusal
fixture derives the compiled version from the real fixture header and tests its
older/newer neighbors instead of hardcoding schema 34. New disclosure and clean
visibility regressions were executed RED before their Inspector-only fixes.
The real queued case checks all eight flag combinations, exact payload export
and an unrelated source; no queued database state is fabricated.

Imported Slice 20 remains historically ACTIVE; this port does not close its
outstanding full release gates or implement planned Slices 30/40/50/60.

## Astra review follow-up

The [Astra port review](../../reviews/compatibility-0.8.29-port-review.md) found
one P2: repeated per-record queued-array scans exhausted the real SQLite work
budget. A lazy bounded cursor set now belongs to each read transaction, with
no session/generation cache; SQL interruption remains `budget_exhausted`. A real
8,192-target queued fixture verifies complete catalog/unrelated 200-record page
and freshness after a test-only public withdrawal. No Engine policy, deletion,
API or dependency pin changes. Independent Astra-medium re-review approved
`0262425fa`: prior P2 resolved, no new material findings. It independently ran
the existing compiled real-queue/freshness and SQLite-interruption tests; the
[review record](../../reviews/compatibility-0.8.29-port-review.md) states the
read-only build-cache lock limitation and exact evidence scope.

Focused remediation checks passed:

- All-target Inspector clippy with `--locked --offline -- -D warnings`.
- Ordinary Inspector suite: 37 unit/real-database tests, one CLI, one no-GUI and
  two worker tests passed; the original four explicit long/native cases remain
  ignored. The fresh real-queue test and SQLite interruption test both have
  executed RED/GREEN evidence in the linked review record.
- Scoped Markdown and diff checks passed; normal commit hooks remain enabled.
  Required full source gate again stopped at missing checkout-owned `.venv`.
  No browser/installed-release qualification or full-workspace green is added
  by this backend-only correction.
