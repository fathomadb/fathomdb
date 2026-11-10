---
title: 0.8.29 planning evidence and limits
status: PROPOSED
date: 2026-11-09
target_release: 0.8.29
---

# Planning evidence and limits

This register identifies planning inputs, not execution completion. Read current
source and accepted decisions before acting on historical transcript statements.
No credentials or raw tool transcripts are copied here.

| Evidence | Observed fact | Allocation and limit |
| --- | --- | --- |
| Git at 6edc1155f and .28 f43435241 | Current .29 includes completed .28; format37, existing MCP/queue/Block1 | All plans; no new performance or test pass inferred |
| Available Markdown validator | Shared root node_modules reports markdownlint-cli2 0.23.0 while manifest requires ^0.23.3 |0/1/10: record tooling mismatch; scoped checks do not establish an owned reproducible environment |
| Current manifests | Rust1.95 minimum, Python>=3.10 package, developer Python>=3.11 path, Node>=25<26, npm11.12.1; Ruff/Pyright and fork pins | 0/1/10; actual source versions, not recommendations to upgrade everything |
| Slice0 drafter's read-only host probes | Active Node26.8.2/npm12.0.2 disagree with source contracts; no checkout .venv or TS node_modules; root node_modules symlink; 18GiB free at98%; GPU query unavailable in executor | 0/7/10; environments/resources need resolution before builds; do not infer host GPU failure |
| .28 Slice95 completion/status | Published consumers and assembled qualification, actual failures/scoped corrections retained; trusted-publisher repair complete | 6/7/160; do not rerun whole old matrix for documentation-only preparation |
| Chat “Complete Slice 90”, 01a1a80a-e240-7e82-9ced-d701d105a3be, recent four turns read November9 | Latest completion says SDK trusted publisher configured; OIDC-only run42750034084 passed,35 build/publication jobs skipped; temporary check removed | 6/7: avoid proposing the same repair again; authentication-only proof is not a package build/publish proof |
| Same chat, earlier publication and runner-permission turns | Registry recovery and protected GPU workflow routing delayed .28 after product qualification | 7/10/160: bounded prerequisite checks before rehearsal; preserve protection restrictions and separate publish authorization |
| Chat “Find unfinished 0.8.28 work”, 01a1a8aa-844e-7ed0-84bf-5f6669d85986, three turns read November9 | Older audit described .29 before final forward merge, then format36; cleanup reported historical disk relief | 2/6/7: historical snapshot, superseded by current Git/format37/disk observations; no fresh cleanup permission |
| Existing reconciliation and independent Astra review | Block2 already has B2-2/3 and part6; one maturity overstatement corrected and recheck approved | 3/4/80/90; approval covers reconciliation revision, not the newly expanded slice set |
| CUDA review in reconciliation | Primary-source signals refreshed, quantitative adoption share unavailable; importance change unratified | 0/1/120: required usage refresh remains work; keep accepted policy while research is incomplete |

## Additional source findings for the next reviewer

- Slice50: historical-view contracts retain body deduplication; explanation maps
  and graph seed exclusion also key on body. Node FTS rowid=cursor is shipped,
  but the edge FTS table does not share that invariant. Identity design must
  reconcile these seams before exact deferred-content promotion.
- Slice70: current query-vector error handling reports overload but otherwise
  drops some errors, including Closing/Cancelled, rather than matching the old
  EP03 draft's claimed propagation. Reconcile the runtime contract and require
  a failing behavioral control for any correction; do not declare an executed
  defect reproduction or broaden all EP03 reasons by inference.
- Slice60: compact score precision must be reconciled with the existing
  ample-budget cross-format score-parity criteria before implementation.
- Slice70: Python/TS already contain UTF-8 examples and the five snippet-option
  fields. Plan parity checks and specific residual edits, not duplicate exports
  or documentation assumed absent from old ledger prose.

## Evidence collection plan

Slices6/7 may read older turns from these chats using read_thread pagination,
and other relevant FathomDB chats discovered by title and project. Record
which turns were actually read, their source dates, baseline and known limits.
A sidebar summary is not a reviewed transcript. Missing/unavailable transcripts
remain explicit evidence gaps; do not invent their contents or stall bounded
source-based proposals.

Cross-check delivery claims with .28 Slice95 and dev/design/release.md. Compare
cruft/stale-doc claims to inbound references and current source. Apply the
quality roadmap and CUDA support review as planning inputs, not new automatic
gates beyond the accepted contracts.
