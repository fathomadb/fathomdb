---
title: 0.8.25 Slice 40 — projection generation and readiness
status: COMPLETE
depends_on: 35
design: design.md
design_status: PASS_CYCLE_4
---

# Slice 40 plan

## Outcome and boundary

Implement the retained core of R25/AC25-40, Memex need 13, N25-01/N25-04,
and A25-05 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Add database-local
identity for the one in-place physical serving projection set, truthful
readiness through an explicit boundary, and compact correlation from the
canonical write cursors already returned by Slice 25. Preserve the existing
projection scheduler, cursor/terminal authority, and ordinary search path.

This slice does not add side-by-side physical generations, a public work
manifest, application-managed cleanup, exactly-once model execution, profile
routing, or a general projection scheduler. Richer generation retention and
work administration remain after 0.8.25.

## Requirements and acceptance criteria

| Requirement | Acceptance criterion |
|---|---|
| S40-R1 Generation identity | S40-AC1 proves Engine-minted `pgen1:<32-lower-hex>` identity is database-local, immutable for one serving epoch, preserved across restart/backup copy, never reused, and changed exactly on configuration or in-place rebuild transition. |
| S40-R2 Boundary-qualified readiness | S40-AC2 reports `observed_boundary` and `ready_through`; no response says ready when an applicable earlier cursor is pending or failed, including after restart. |
| S40-R3 Compact mutation correlation | S40-AC3 resolves every Slice-25 `pending_projection_write_cursor` through the receipt's indexed `operation_id`, required expected generation, and physical-member state without a per-record work manifest or reinterpretation of the Slice-20 dependency generation. |
| S40-R4 Honest in-place transition | S40-AC4 configuration/rebuild retires the prior metadata epoch, installs one new serving epoch, and exposes processing/degraded state until existing physical stores catch up; no API claims an unavailable side-by-side store. |
| S40-R5 Frozen-read and lifecycle closure | S40-AC5 changes to generation/readiness authority invalidate Slice-35 frozen reads, and erasure/lifecycle races cannot publish an ineligible projection or leave source-bearing generation artifacts. |
| S40-R6 Additive parity and bounded cost | S40-AC6 proves Rust/Python/TypeScript/wire parity, conservative compatibility mapping, Windows source/build coverage, packaged CPU/CUDA status behavior, indexed bounds, and the preregistered latency/storage limits. |

## Design and review gate

Reconcile the current draft against the shipped Slice 20/25/30/35 code and the
accepted projection/readiness/rebuild designs. The corrected design must choose
the metadata-correlation model over fictional side-by-side physical stores and
close the cycle-1 findings recorded in
[`design-review-cycle1.md`](design-review-cycle1.md). Obtain independent review;
at most four FIX-n design cycles are allowed. No implementation begins with an
unresolved implementation-shaping P1/P2 finding.

## TDD RED/GREEN implementation

Commit RED separately. Use real SQLite databases and preserved fixtures:

- schema/open tests for ID grammar, uniqueness, fresh/upgrade state, corruption,
  backup copy, and no reuse;
- Engine property/fault tests for non-owner cursor gaps, three-part physical
  completion, the full incomplete/corrupt state table, terminal failure, state
  transitions, restart at each transition, duplicate/stale publication, and
  configuration/rebuild epochs;
- concurrency tests for write, worker publication, lifecycle closure, erasure,
  and rebuild races;
- Slice-35 frozen-read tests for pre-migration drift, generation/readiness
  changes, and unchanged v1 token codec;
- Rust/Python/TypeScript canonical wire and error-precedence fixtures;
- installed wheel and offline npm/native status smokes;
- CPU and RTX-3090 CUDA readiness/reopen tests; Windows execution when available,
  otherwise an explicit unavailable record;
- preregistered write/storage/status/reopen measurement with source-bound
  receipts.

The initial focused routes are:

```bash
cargo test -p fathomdb-schema --test step32_projection_generation
cargo test -p fathomdb-engine --features test-hooks \
  --test slice40_projection_generation \
  --test slice40_projection_completion \
  --test slice40_mutation_projection_status \
  --test slice40_projection_generation_races \
  --test slice35_frozen_read
cargo test -p fathomdb-py
cargo test -p fathomdb-napi
(cd src/ts && npm run build:native:debug && tsc -p tsconfig.json && \
  node --test dist/tests/slice40-projection-generation.test.js)
```

GREEN implements the smallest contract-complete behavior. Tests remain frozen
during each fix-to-spec cycle. Independent implementation review used the
owner-authorized eighth cycle to close the final temporal-cache and dispatcher
progress findings; its durable verdict is
[`implementation-review-cycle8.md`](implementation-review-cycle8.md).

## Verification routes

Run focused tests first, then `./scripts/agent-verify.sh --tier=fast`, applicable
heavy/combined operator routes, fresh wheel/npm artifact smokes, and the
preregistered measurement. CUDA is required because dense readiness changes;
ptrace-dependent checks run unchanged outside the sandbox. A monolithic
CUDA-plus-Metal `--all-features` command is invalid. Windows runtime absence is
reported, never simulated.

Stop on generation reuse, an unqualified ready result, status under the wrong
generation, public work-manifest expansion, application-owned cleanup, changed
Slice-35 token bytes, or any false side-by-side activation claim.

## Closeout

Independent verification and evidence review pass. The exact retained evidence
is indexed by [`evidence-manifest.json`](evidence-manifest.json), and the
closeout verdict is recorded in [`closeout-review.md`](closeout-review.md).
Every run and receipt remains retained. The generated views and status record
pass their final consistency gate, so release state advances to Slice 45.
