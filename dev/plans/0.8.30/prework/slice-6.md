---
title: Slice 6 — Documentation drift and delivery evidence
status: COMPLETE
slice: 6
target_release: 0.8.30
date: 2026-11-18
---

# Slice 6 — Documentation drift

Reviewed at `18427b7a8`, incorporating main's .29 documentation commit
`42629c2a1`. Used release records, source, current workflow job summaries and
bounded read-only Codex chat pages. The two chat pages cover two recent turns
each from “0.8.29 worktree” and “Finish 0.8.29.” They establish the durable
handoff and the later retrospective. Older turns and raw chat tool logs were
outside this pass. Historical handoff status and chat branch descriptions are
snapshots. [Handoff](../../runs/HANDOFF-0.8.29-2026-11-17.md),
[retrospective](../../../notes/0.8.29-retrospective.md).

## Verified delivery and remaining record drift

Read the seven successful jobs of [Tegra 44246092920](https://github.com/fathomadb/fathomdb/actions/runs/44246092920)
through the GitHub connector on 2026-11-18. This included
[driverless ARM64 import](https://github.com/fathomadb/fathomdb/actions/runs/44246092920/job/134607482758),
[public URL and digest verification](https://github.com/fathomadb/fathomdb/actions/runs/44246092920/job/134607953884)
and [installed public Jetson consumers](https://github.com/fathomadb/fathomdb/actions/runs/44246092920/job/134608027048).
The installed log confirms the 0.8.29+tegra Python CPU/auto/forced CUDA
lifecycle and CUDA witness and the exact 0.8.29 Node Pages addon on Orin with
embed/rerank, witness and bounded heap-smoke checks. The Pages endpoints are
the [wheel index](https://fathomadb.github.io/fathomdb/tegra/simple/fathomdb/)
and [Node package](https://fathomadb.github.io/fathomdb/tegra/packages/fathomdb-linux-arm64-gnu-tegra-0.8.29.tgz).
Direct Pages reads were unavailable to the web reader here. The dated job
evidence establishes the deployed/read/installed results at that run.

The [retained manifest](../../../../scripts/release/tegra-pages-retained.json)
and [public release notes](../../../../docs/release-notes/0.8.29.md) already
reflect delivery. The [.29 release-state JSON](../../release-state-0.8.29.json)
still says REQUIRED_PREFLIGHT and “No build/dispatch yet,” and the
[generated board](../../runs/STATUS-0.8.29.md) still says pending. Correct this
through the .29-owned state and its generated views, with the exact run and
retained artifacts. Keep .30 qualification on its own bytes. Generic ARM64
CPU, CUDA 13/.31, pending real reset proof and the historical default Node
soak are separate claims.

## Corrections proposed to Slice 9

- **Search users and downstream SDK consumers, [structured
  hits](../../../../docs/guides/structured-search-hits.md).** The page says deduplicated on body and
  labels stored-record identity and atomic feedback refusal “implementation pending.” The shipped
  [ADR](../../../adr/ADR-0.8.29-retrieval-record-identity.md), [record
  oracle](../../../../src/rust/crates/fathomdb-engine/tests/slice50_record_identity.rs) and [MCP
  tests](../../../../src/rust/crates/fathomdb-mcp/tests/feedback.rs) speak to the implemented behavior.
  10 can select a documentation correction describing one fusion unit per stored record/version,
  distinct same-body versions, repeated public refs, and the existing context-packing/atomic
  named-feedback rule. 30 owns any later selected occurrence-label contract.

- **Search users, [structured hits](../../../../docs/guides/structured-search-hits.md).** The table and
  final note say provenance is populated on every hit. The currently exercised ownerless FTS path
  returns `source_id=None`. 10 can clarify required provenance on governed writes separately from
  nullable hit provenance. Update both table and note, preserving the write validation rule. [Current
  ownerless
  control](../../../../src/rust/crates/fathomdb-engine/tests/slice71_search_statement_trace.rs).

- **Python consumers building from source, [install guide](../../../../docs/install/python.md) and
  [contributor guide](../../../../docs/contributing/index.md).** The install guide calls editable the
  only supported development native build. The contributor guide already requires the checkout-owned
  noneditable wheel and forbids sharing mutable/editable environments across worktrees. 10 can
  distinguish the direct-checkout development path from isolated source-bound build/provision and
  qualification. Link the supported current entry point. Preserve the manual cargo-build/copy
  prohibition. [Owned setup](../../../../scripts/setup-python-owned.sh).

- **Contributors selecting validation, [contributor guide](../../../../docs/contributing/index.md).**
  The gate list correctly separates source/docs scope, but omits the AC-036 ptrace prerequisite and the
  deliberate long-test split. A developer may read one full default gate as exercising long gated
  cases. 10 can add the actual prerequisite and one line tying AC-021/059b long cases to `AGENT_LONG=1`
  and broader `check.sh` or the selected 70 commands. Keep actual exit/scope language. [Developer
  gate](../../../../scripts/agent-verify.sh), [long entry](../../../../scripts/check.sh).

- **Maintainers, [release design](../../../design/release.md#tiered-publish-order).** The opening text
  still says index-propagation sleeps sit between tiers. The same section and current workflow require
  bounded exact-version polling. The first-tag bootstrap section permits only explicit .28/.29 cases.
  20 owns updates alongside normal OIDC/routing work. Remove the stale fixed-sleep description, keep
  actual Cargo resolution separate, retain the prior exact-tag exception as history and describe only
  the owner-approved .30 route once selected. [Release
  workflow](../../../../.github/workflows/release.yml).

- **Release operators, [.29 state](../../release-state-0.8.29.json) and
  [board](../../runs/STATUS-0.8.29.md).** Seq-398 still looks unstarted despite qualified delivery. The
  pre-delivery retrospective sentence is a historical observation that can mislead if reused as current
  status. .29 owner reconciles the single writer and generated views. 10/70 should cite the dated job
  and consume the prior artifact scope. Add a dated successor pointer where needed instead of rewriting
  historical outcomes.

## Reuse and checks after selection

- Keep the current .29 install commands and actual matching Tegra versions.
  Retain the generic ARM64 CPU distinction and classic Orin detector.
  [Python guide](../../../../docs/install/python.md),
  [TypeScript guide](../../../../docs/install/typescript.md).
- Keep the source map generated from actual inputs. Update generator/source
  links only when the route changes. Public changes should pass scoped
  Markdown/docs checks and strict MkDocs when implemented.
  [Contributor map](../../../../docs/generated/contributor-map.md).
- Review actual Git/runtime proof before updating a delivery claim. Place
  scoped copy edits in 10 and behavior-coupled changes with 20 or 30.
  Record an authority disposition for .29 state reconciliation. This
  review changes no public docs or authoritative state.

The completed [library sweep](slice-1.md) adds a future NodeLTS choice. Current
Node25-only public guidance remains the existing .29 support contract, not
proof that .30 supports26. Any selected migration updates pins, package engine
ranges, build/loader proof and public claims together in10/70; it is not a
cosmetic “latest version” edit.
