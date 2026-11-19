---
title: 0.8.30 prework Slice 2 repository and documentation disposition
status: COMPLETE
slice: 2
target_release: 0.8.30
date: 2026-11-18
---

# Slice 2 — Repository and documentation disposition

## Coverage

Read-only inventory at `release/0.8.30` commit `18427b7a8`, incorporating
`42629c2a1` from main. Git lists 14,565 tracked paths, including 11,199 under
`dev/` (9,862 in `dev/plans/`), 38 under `docs/`, 1,142 under `src/` and
1,101 under `scripts/`. Sampled actual index and authority text, source
and test registrations, linked history, notes and experiment outputs,
and public and contributor entry points. This inventories tracked material
and verifies representative candidates. It does not establish readership
outside the repository or cover every file line by line.

The [release plan](../plan.md) allocates approved setup and documentation
corrections to Slice 10. Feature-bearing work stays with its slice.
The [previous cleanup review](../../0.8.27/prework/slice-2.md) provides
candidate history. Its proposals confer no current deletion authority.

## Candidate groups

| Area and candidate | Verified evidence | Proposed disposition and placement |
| --- | --- | --- |
| Program schedule and navigation | The [engineering index](../../../README.md) points to the program schedule and live state selector, while its architecture summary still calls the integrated profile 0.8.28. The [roadmap](../../../ROADMAP.md#0830-backlog) identifies B30-01 and already links the proposed .30 plan. | Keep the roadmap and sequencing history. Update the active architecture description and pointer in 10 if selected. Use the selector for live work and explicit dated links for history. |
| Engineering doc index | [DOC-INDEX](../../../DOC-INDEX.md) still frames its top-level rule by the .8.0 review gate and duplicates historical context in linked detail maps. | Keep the useful map. Defer any narrower wording change until the owner and actual validators agree. Avoid mass freshness edits and per-file audit work. |
| Stable product records | [Needs](../../../needs.md) and [requirements](../../../requirements.md) retain locked 0.6.0 identifiers and an applicability paragraph pointing to .29. [Acceptance](../../../acceptance.md) records both historical and superseding gates. | Keep the bodies, IDs and thresholds. Consider a single current-applicability pointer in 10. Treat active feature-specific mismatches in their owning slice. |
| Architecture and design | [Architecture v1](../../../design/fathomdb-data-plane-architecture-v1.md) is explicitly superseded by [v2](../../../design/fathomdb-data-plane-architecture-v2.md). The active v2 header still says published artifacts remain .28, and its release allocation names .28 Slice 95. [Lifecycle inventory](../../../design/document-lifecycle.json) already classifies historical designs. | Archive v1 in place with its existing successor. Correct factual live pointers in v2 after Slice 9 selection, while preserving historical sections and accepted semantics. |
| Authoritative decision and interface set | The [ADR index](../../../adr/ADR-0.6.0-decision-index.md) still labels record-identity implementation and qualification pending. The [identity decision](../../../adr/ADR-0.8.29-retrieval-record-identity.md) is accepted, and [.29 Slice 50 requirements](../../0.8.29/features/slice-50/requirements.md) record implemented/source-qualified identity and the scoped score exception. | Keep the ADR and interface contracts. Correct implementation-status metadata only after verifying its scope, with the owner of Slice 30 or selected 10 documentation work. No ADR decision rewriting. |
| Release status and .29 public Pages | Commit `42629c2a1` added the .29 wheel/addon URLs and digests to the [retention manifest](../../../../scripts/release/tegra-pages-retained.json), while the [public compatibility page](../../../../docs/compatibility/index.md) describes the pair as published. The [.29 state](../../release-state-0.8.29.json) still says `CORE_PUBLISHED_TEGRA_DELIVERY_PENDING`, with `tegra_delivery.status=REQUIRED_PREFLIGHT`. | Keep documents and exact claims as evidence of a contradiction. The .29 owner reconciles actual delivery proof and state. Send the dependency to 70/80. No inferred URL, installed-consumer or cleanup result. |
| Program and release history | [Frozen progress](../../../progress/README.md) is clearly historical. The root [.23 todo](../../../../0.8.23-release-todo.md) has a historical banner that points to .25 as active. The [Slice 85 recovery plan](../../../../plan-slice-85-recovery.md) has live inbound history from the .27 master, plan and receipt. | Archive in place. Correct only the stale authority phrase in the .23 banner if selected. Preserve the recovery plans, boards, nonpasses and their stable links. |
| Notes and intermediate artifacts | The old [file geometry scanner](../../refactor-background-check/scan_file_sizes.py) emits `violations.{json,md}` and `structure/*.txt`. The latter still has 113 tracked maps. [Quality roadmap](../../../notes/fathomdb-quality-testing-roadmap.md) and [.29 retrospective](../../../notes/0.8.29-retrospective.md) are used in .30 planning. | Keep reasoning, generator and active notes. Keep review JSON and committed run evidence unless unique claims are accounted for. Postpone deletion of generated maps pending exact regeneration and inbound-reader checks. |
| Prototype and experiments | The [L2 router prototype](../../../prototypes/l2-router/README.md) is a separate prototype. The [S50 allocation](../release-followups.md) links retained raw external evidence needed for this release. | Archive the labeled prototype in place and retain the experiment design, reusable tooling and evidence required for replication. Never classify by extension or discard original nonpasses. |
| Source comments and measurement-only code | [RRF tests](../../../../src/rust/crates/fathomdb-engine/tests/pr_g9_rrf_fusion.rs) open with a body-dedup description. Current [fusion](../../../../src/rust/crates/fathomdb-engine/src/fusion.rs) uses `RecordKey`. The [engine manifest](../../../../src/rust/crates/fathomdb-engine/Cargo.toml) and feature matrix still register private observation and test paths. | Delete or correct the stale comment when Slice 30 touches the relevant file. Keep the feature-scoped helpers until the actual callers and needed witness are replaced. No blanket source deletion by slice number. |
| Test and test-design material | The [query scaffold in v0.8.29](https://github.com/fathomadb/fathomdb/blob/v0.8.29/src/rust/crates/fathomdb-query/tests/property_template.rs) compares a generated string with its own clone. The [Python property](../../../../src/python/tests/test_property_template.py) exercises actual write/search/close/reopen and checks identity and provenance. [Test feature inventory](../../../../scripts/test-feature-matrix.toml) assigns required feature sets. | Deprecate the trivial scaffold as coverage. Real query properties already exist in-crate. Confirm targets/readers in 5/9 before 30 retires it. Keep Python and registered controls. Check actual runtime and semantic overlap before consolidating any suites. |
| Public, consuming-developer and contributor docs | The [public README](../../../../README.md), [installation guides](../../../../docs/install/python.md), [TS package README](../../../../src/ts/README.md), [contributor guide](../../../../docs/contributing/index.md) and [MkDocs navigation](../../../../mkdocs.yml) serve different audiences. The contributor guide duplicates checkout setup in part but correctly calls owned setup and links generated maps. | Keep audience entry points and generated maps. Correct the duplicated “workspace Rust workspace owners” phrase opportunistically. Resolve version and support claims against actual artifacts in 6/70/80. |
| Separate Inspector program | Read-only branch `dev/0.8.30` at `7d3637657` contains Inspector v0.1 source, docs and tests. Its Slice 20 status is ACTIVE, with committed foundation work and final verification pending. The stale overview sentence says implementation has not started. | Keep the branch and its historical records. The Inspector owner handles that local prose drift. Present preserve-separate versus explicitly sequence/integrate as a Slice 9 scope decision. The database release inherits no Inspector completion credit. |

## Candidate validation notes

- The scanner's source documents generation of the old geometry and its
  violation summaries. The old .27 proposal and .21 cleanup plan still refer
  to the investigation. That establishes provenance, not exact regeneration
  at the originating revision or a proof of no unique findings. Keep current
  paths until those checks can justify a specific deletion.
- The query crate's actual generated cases are in [its library tests](../../../../src/rust/crates/fathomdb-query/src/lib.rs). They check FTS string-literal
  terms and independent content-token expectations. The [scaffold checker](../../../../scripts/check-property-test-scaffolds.py)
  names engine, schema and Python, not the query file. This is a narrow
  retirement candidate, subject to checking ordinary target coverage and
  updating its stale contributor pointer. The Python reopen test must stay.
- The retained Tegra manifest now lists exact filenames/digests and branch
  docs assert publication. Separate prework 6/7 evidence reports that GitHub
  run 44246092920 passed all seven jobs, including public URL and actual Orin
  installed checks. Treat the historical state as stale in light of that
  evidence, pending its owner's reconciliation. This review did not execute
  delivery or access the device. [Run](https://github.com/fathomadb/fathomdb/actions/runs/44246092920)
- Inspector's own status reports partial development evidence, including
  explicit GUI and native checks on a named executor. Later branch commits
  change visibility invalidation and packaging. A dated status and an
  implemented source tree coexist with additional independent gates still
  due. Retain both. Slice 4 maps the actual integration surfaces.

## Slice 9 choices

- Include selected low-effort authority corrections in Slice 10. Prefer dated
  pointers over moving historical paths. A completed history remains a valid
  link target even when its phase description is obsolete.
- Postpone physical archive moves and generated-map deletion. If considered
  later, prove that the original input revision reproduces the artifacts and
  that no live link, validator, reader or unique result depends on them.
- Keep Inspector separate by default. Establish an explicit owner and integration
  plan if the owner selects it. Preserve its separate workspace, documentation
  and verification decisions until that choice is made.
- Hand the .29 status contradiction to its owner and the exact affected claims
  to 6/70/80. Do not convert it into general repository cleanup.

Review complete. No delete, move, archive, authoritative edit or runtime check
was performed.
