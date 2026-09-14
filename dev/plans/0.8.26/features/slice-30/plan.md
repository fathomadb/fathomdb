---
title: FathomDB 0.8.26 Slice 30 — distributable operator integrity route
status: APPROVED
---

# Slice 30 plan — distributable operator integrity route

## Reconciliation since the draft

The initial plan was written at `183fe45a` and narrowed through `7ae1905f`.
This execution reviewed those revisions, the Slice 3/5/8 allocations, the
shipped 0.8.25 integrity implementation, and the completed Slice 20 graph
evidence work. The material deltas are:

1. Slice 20 completed at `d335ae9a` and left the already-published on-disk
   schema at 33. Slice 30 therefore must reject every non-33 database without
   migrating it.
2. The four bounded integrity checks, V1 report, check ordering, finding
   locators, request bounds, and CLI command already shipped in 0.8.25. They
   need qualification and hardening, not a second checker or report model.
3. The CLI currently invokes the check through ordinary `Engine::open`.
   Although the check uses one read transaction, ordinary open can migrate,
   reconcile projection state, start a projection runtime, and rewrite the
   product lock file. It cannot satisfy R26-30B's whole-process no-mutation
   claim.
4. The current CLI reference already documents the verb and the Python and
   TypeScript interfaces already exclude doctor/recovery methods. The missing
   documentation is the install/version/quiescence operator procedure and the
   incomplete doctor inventories in `dev/interfaces/cli.md` and the CLI crate
   README; the Rust install guide also needs the exact-pinning rule.
5. D26-02 (`seq-280`) authorizes the existing crates.io CLI route first and
   requires a loud HITL stop before any prebuilt fallback. No prebuilt artifact
   or new SDK authority is authorized.
6. The release workspace still reports package version 0.8.25 until the release
   cut. Slice 30 can prove source-candidate behavior and clean-root
   `cargo install --path`/discovery; Slice 50 owns the staged integrated
   candidate, and only the post-publication smoke owns crates.io 0.8.26.
7. No repository decision names Memex's final deployment target. This slice
   can qualify the source candidate on the available Linux x86_64 host; the
   final target and five-target matrix remain Slice 50 evidence.

## Plan verdict

Approve with adjustments. Retain the existing command and V1 report, add a
strict read-only inspection entry point behind the Rust `operator` feature,
route only this CLI verb through it, and add the missing operator procedure.
Reject a new package kind, prebuilt matrix, SDK method, new integrity checks,
report V2, raw-SQL workflow, automatic migration, and broad platform rehearsal
as overbuild.

## Need and requirements

- **N26-03:** An operator needs an installable out-of-process integrity check
  that cannot modify or silently upgrade the inspected store and does not
  require Memex to issue raw SQL or gain repair authority.
- **R26-30A — distribution and identity:** Qualify the exact-version
  `fathomdb-cli` crate/binary topology and source-candidate installation on the
  available Linux x86_64 host, while retaining the historically proven
  crates.io route. Slice 50 owns the integrated final candidate; only the
  post-publication smoke can prove the 0.8.26 registry artifact. If the route
  is unusable, stop for HITL before proposing a prebuilt artifact.
- **R26-30B — immutable inspection:** Inspect only a database whose
  `user_version` exactly equals the binary's compiled schema (33 at this slice),
  under the existing product lock and with no non-empty WAL or non-empty
  rollback journal pending. A normal persistent `-shm` file is permitted but
  remains unchanged.
  Open SQLite through a percent-encoded `file:` URI with `immutable=1`,
  `READ_ONLY|URI`, and query-only enforcement; never create a database or lock
  file, migrate, repair, rebuild, advance a projection, or start a worker.
- **R26-30C — stable bounded result:** Preserve the V1 check/report model,
  bounds, privacy-safe locators, canonical check order, and exits 0/65/70/71.
  All post-parse semantic and runtime failures for this verb use the V1 error
  envelope with a stable reason and RFC 6901 field path; return no partial
  report. Clap syntax/type failures retain the CLI-wide exit-2/stderr contract.
- **R26-30D — authority and guidance:** Document exact-version installation,
  identity, quiescence, invocation, and interpretation. Keep doctor/recovery
  absent from governed Python and TypeScript APIs and from operator-off Rust.

## Acceptance criteria

- **AC26-30A:** A clean Linux x86_64 operator root installs/discovers the source
  candidate and `fathomdb --version` identifies its package version. Historical
  registry-route, final-candidate, and post-publication evidence are
  distinguished explicitly.
- **AC26-30B:** A clean current database returns 0; a findings database returns
  65; post-parse invalid input, unavailable/corrupt state, or schema mismatch
  returns 70; an active engine, non-empty WAL, or non-empty rollback journal
  returns 71. Each response parses as the documented V1 envelope.
- **AC26-30C:** A before/after byte-and-path witness over the database and its
  product file set (`.lock`, `-wal`, `-shm`, and optional `-journal`) is
  identical after clean and refused invocations. A missing path creates no
  database or lock, and a down-level database remains down-level.
- **AC26-30D:** Work and findings remain bounded by 1..=10,000 and 1..=100;
  output contains no raw body/source text and never returns a partial report.
- **AC26-30E:** CLI/interface/operator guidance agrees, and existing
  governed-surface checks prove there is no Python, TypeScript, or operator-off
  Rust doctor/data-plane-integrity entry point.

## TDD implementation

1. **RED:** Add CLI process tests for byte-identical clean inspection,
   down-level schema refusal without migration, active-engine/non-empty
   WAL/non-empty-journal refusal, persistent-`-shm` success, URI-reserved path
   bytes, missing-path no-creation, and the uniform post-parse V1 error/exit
   contract. Retain existing 0.8.25 engine tests as the oracle for check
   semantics and bounds.
2. **GREEN:** Add the smallest operator-gated static inspection path: validate
   the existing lock without creating or rewriting it, reject non-empty
   WAL/rollback-journal recovery, initialize the SQLite runtime in canonical order, open
   exact-compiled-schema SQLite through a percent-encoded `file:` URI with
   `immutable=1`, `READ_ONLY|URI`, and query-only enforcement, and call the
   unchanged integrity executor in one read transaction. Route only
   `data-plane-integrity` through it and serialize typed failures without raw
   detail.
3. **REFACTOR:** Centralize this verb's error envelope and keep ordinary doctor
   verbs unchanged. Update CLI/interface/operator docs and the release-local
   trace; do not add package or binding code.
4. Obtain independent code review of the actual diff and resolve P0-P2.
5. Have an independent verification subagent run focused tests and challenge
   the no-mutation/version/authority evidence. Run canonical `agent-verify`
   because this is meaningful Rust and public-contract work; retain any known
   worktree-environment exception exactly rather than weakening a gate.

## Stop gates

Stop for HITL if the existing CLI cannot satisfy the named deployment, a
prebuilt artifact or SDK doctor method is required, exact schema pairing is
ambiguous at release cut, or read-only inspection requires modifying the
database or its sidecars.

## Closeout

Write `status.md` with exact commits, review and verification verdicts,
artifact limitations, and Slice 35 as the next dependency. This work uses the
existing durable release worktree and branch; create no temporary worktree or
branch.
