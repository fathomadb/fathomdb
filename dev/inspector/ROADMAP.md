---
title: Inspector ROADMAP
date: 2026-11-23
product: FathomDB Inspector
target_release: 0.3.0
desc: Adopted release commitment and explicitly deferred product work
status: ACTIVE
---

# Inspector ROADMAP

Inspector `0.3.0` (`R31-08`, [Slice 60](../plans/0.8.31/features/slice-60/plan.md))
owns the current release line, upgrading the completed `0.2.1`
([scope](plans/0.2.1/scope.md), [plan](plans/0.2.1/README.md)) and
[0.2.0 ladder](plans/0.2.0/README.md) onto FathomDB `0.8.31` (Schema `38`)
while retaining `D4 = B` native Ubuntu 24.04 (`glibc 2.39`) Chromium + bundled
SQLite qualification (zero Docker, `openbox`, `xdotool`, or `sudo`) and adding
automated GitHub Release artifact delivery via `.github/workflows/release.yml`
(`build-inspector`). The five-person UX study remains removed by owner decision
([INS-020-LOCAL-RELEASE](plans/0.2.0/release-decisions.md)).

## 0.3.0 — FathomDB 0.8.31 / Schema 38 and GitHub Release delivery

- **Schema `38` admission & reserved `scope`:** Strict Schema `38` admission
  (`compiled_schema = 38`), rejecting Schema `37` (`0.8.30`) cleanly, validating
  Schema `38` `properties`/`scope` columns, `_fathomdb_projection_registry.{entity_class,vector_kinds}`,
  `canonical_property_values`, `_fathomdb_commit_timestamps`, and
  `_fathomdb_vector_kinds`, and exposing reserved `scope` read-only (empty in
  `0.8.31`; multi-tenant scope enforcement belongs to `0.8.32`).
- **Outside-`body` `properties` UI & strict safety:** Expandable typed node/edge
  `properties` inspection and exact copy (preserving 64-bit integers outside
  `2^53`, `-0`, empty strings, and `NULL`), `.textContent`-only XSS safety,
  64 MiB admission parity, and complete 8-flag `D-01` erasure non-disclosure.
- **Historical `as_of` graph exploration:** Mutually exclusive
  `as_of_write_cursor` / `as_of_epoch_ms` in stored-adjacency (`graph`) and
  native (`graph_serving`) traversals (`P4-13` `INDEXED BY` lift via O(log N)
  two-branch `UNION ALL`), preserved across comparison pins and strictly
  subordinate to all 8 `D-01` disclosure flags.
- **Engineering loop & reliability (`P2-13`, `P7-01`, `P7-02`, `P7-04`):**
  `local-ci.sh` defaults to `--scoped` (~5 min Inspector-scoped check with
  fail-closed diff guard and `--base <ref>`) while retaining `--full-workspace`
  and `--installed-only`; bounded timeouts and `/proc` descendant reaping apply
  across shell, Node/Playwright, and Rust `launch.rs` harnesses; `local-release.py`
  stays locked to `D4 = B` Ubuntu 24.04 / `glibc 2.39`.
- **GitHub Release delivery (`P7-06`):** `.github/workflows/release.yml`
  `build-inspector` builds, verifies ELF `GLIBC <= 2.39`, seals, verifies, and
  stages non-colliding `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and
  `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` assets for attachment
  by `github-release` (gated by `all-builds-passed`).

## 0.2.0 — adopted connected exploration scope

A local browser GUI for FathomDB's canonical records, operational collections,
FTS/BM25, vectors, and interactive graphs. It supports coordinated identity-
preserving navigation, actual retrieval observations, readiness/lifecycle
inspection, and bounded provenance/revision investigation.

The [historical port](plans/0.1/compatibility-0.8.29-port.md) supplied the record
foundation; text/vector/interactive graph and person/comparison are implemented. The [Inspector backlog](BACKLOG.md) owns actionable examples and
follow-ups; this roadmap retains release commitments and promotion authority.

All seven Need actions and four Should actions originally assessed in the
[Astra assessment](reviews/fable-action-assessment.md) carry into 0.2.0 scope.
Historical Slices 15/16 established the earlier requirements/verification and
architectural baselines. The current proposed ladder allocates prework to 0–8,
human disposition to 9 and only individually approved prework implementation
to conditional 10. Features occupy 20/30/40/50, comparison and coordinated
interactions 60, and final installed/integrated qualification 70. The
[legacy allocation](plans/plan-0.1.md) remains historical input; the current
master maps every M01–M40 owner, including comparison INS-05.7–05.10/M40. Initial support is current compiled schema,
local attachment with external writers stopped, Engine startup allowed,
inspection-only controls and cached query models.
The owner adopted D-01 defaults and launch overrides on 2026-10-30; see
[the policy](requirements.md#adopted-operator-disclosure-policy--d-01-2026-10-30).
Narrow local export of still-retained erasure-pending content with
`--show-erased` remains in scope, subject to inactive/telemetry gates.

Owner S9-010 narrows 0.2.0 vector qualification to supported single head windows;
the within-source multiple-window positive is deferred.

## Distribution after0.2.0

`0.2.0` and `0.2.1` shipped as local-only tar/extracted bundles under
gitignored `dist/inspector/`. Starting with `0.3.0` (`R31-08` / `P7-06`),
`.github/workflows/release.yml` adds the `build-inspector` job on
`ubuntu-24.04`, gating `all-builds-passed` and attaching
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` to the GitHub Release
alongside core FathomDB release artifacts while preserving `local-release.py`
for local candidate sealing, verification, and atomic promotion. Any future
Inspector-only crates.io source package, OS package channel, or additional
host platform beyond `D4 = B` (Ubuntu 24.04 / `glibc 2.39` Linux x86_64)
still requires a separate commission, qualification matrix, and signing policy.

## Beyond v0.1 — candidates, no version or delivery date assigned

This historical heading keeps existing RM links stable. These candidates are
now outside adopted 0.2.0 as well; none has an assigned successor release.
The unpublished 0.1.0 source package is the record-browser baseline, not a
completed full-view release. The [backlog](BACKLOG.md) owns actionable detail.

| ID | Group | Candidate | Promotion condition |
| --- | --- | --- | --- |
| RM-01 | Deeper vector inspection | Side-by-side binary-candidate versus float-neighborhood comparison. | Basic operation attribution is qualified and users need deeper diagnostics. |
| RM-02 | Deeper lexical inspection | Inactive/historical lexical structures beyond supported active routes. | Their semantics and retention/disclosure policy can be supported. |
| RM-03 | Data summaries | Role-distribution dashboards and dedicated engine-derived aggregate counters. | Observed usage justifies summaries beyond clear labels/scoped counts. |
| RM-04 | Compatibility | Older-schema read adapters and a historical-file compatibility corpus. | Explicit supported versions and non-mutating reads are designed. |
| RM-05 | Availability | Attachment with initially nonempty WAL/journal; distinct from WAL created by an admitted Engine. | Separate admission, snapshot, and preservation validation exists. |
| RM-06 | Availability | Live attachment to an actively writing application. | Supported integration and consistency model; no product-lock bypass. |
| RM-07 | Consolidation | Agent-access snippet integration or cross-service session consolidation beyond the carried reuse assessment. | Source-grounded benefit; no forced consolidation or assumed sibling-branch dependency. |
| RM-08 | Exploration depth | Exhaustive field/schema inference, statistical distributions, and broader predicates. | Concrete user tasks justify bounded execution and semantics. |
| RM-09 | Investigation workflow | Persistent investigation workspaces, reproducible saved sessions, and general export/share workflows beyond the narrow erasure-pending export. | Privacy, state lifetime, stale-reference, and portability contracts are defined. |
| RM-10 | Quality analysis | Comprehensive anomaly/duplicate/broken-link analysis beyond factual inspection. | Independent oracles and bounded work exist; no automatic repair implied. |
| RM-11 | Presentation | Optional dimension plots or sampled 2D/3D vector maps beyond the required metric-faithful neighborhood. | Transformations and sampling are disclosed; geometry is not treated as semantic truth. |
| RM-12 | Vector source windows | Multiple emitted vector windows from one source revision. | A separately scoped producer/admission contract and ordinary-write positive are supported, exact spans/origins and bounded retrieval/inspection are independently qualified; no assigned release date. |

RM-01 through RM-05 and RM-07 preserve the assessed Nice-to-have groups.
The remaining rows preserve earlier expressly deferred Inspector needs.
The carried reuse assessment is required even though broader consolidation is not.

## Scope discipline

This roadmap does not schedule database-engine releases or change accepted
ADRs. A candidate enters a release through an explicit scope update and a
plan with acceptance signals. No dates or successor versions are promised.
Forensic immutable, Engine-free inspection is also a future candidate; it is
not required now that Engine startup is allowed.

Incorrect review assumptions are not backlog: migration-on-open, destructive
recovery as a routine inspection fix, and replacement of required graph/data-
type views with a generic grid remain rejected approaches.

The [public roadmap](../../docs/inspector/ROADMAP.md) summarizes user-facing
outcomes; this document owns internal scope and promotion conditions.

The current plan is Inspector-only. Engine/MCP/schema/SDK/agent-access changes
are not 0.2.0 slice work, even where an optional future consolidation candidate
might require them. Such changes need their own explicit scope decision.

The owner clarified on 2026-10-31 that retained-erasure display/export applies
**only if legitimate readable content exists**. Physical deletion remains
correct and unchanged. Atomic deletion leaves no retained target; legitimate
existing ExplicitMaintenance queues can expose exact retained targets under
the [current port contract](plans/0.1/compatibility-0.8.29-port.md). Absent
content remains unavailable/not-applicable. The Inspector does not retain
content, delay erasure, add queues or reconstruct erased bytes.
Positive Engine end-to-end export is required only for a qualified supported
Engine that legitimately exposes this state; other native-view positives remain
required. This does not expand general export scope.
