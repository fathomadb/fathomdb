---
title: 0.8.26 Slice 2 — cruft review draft notes
status: DRAFT
---

# Slice 2 draft design notes

## Initial proposal seeds

| ID | Surface | Initial action hypothesis | Evidence required before Slice 6 |
| --- | --- | --- | --- |
| C26-01 | `dev/plans/0.8.26-draft-scope.md` | Deprecate-in-place after accepted 0.8.26 plan supersedes it | Every D26 item has a durable disposition. |
| C26-02 | Completed `dev/plans/0.8.25/**` execution corpus | Keep/archive-in-place with clearer historical navigation | Preserve reviews, receipts, and experiment evidence; measure active-index noise. |
| C26-03 | Multiple historical `dev/plans/release-state-*.json` files versus the documented single-writer wording | Keep records but correct authority/navigation model | Identify generator behavior and which file, if any, is active. |
| C26-04 | `dev/progress/` | Archive-in-place/keep frozen | Confirm no active writer and retain history. |
| C26-05 | Public-doc truth checker expecting 0.8.23 after 0.8.25 publication | Update test/fixture, not product truth | Reproduce from clean release worktree and trace expected-version source. |
| C26-06 | Post-tag 0.8.25 documentation commits | Keep; clarify tag/site chronology if confusing | Confirm current public site and release-note authority. |
| C26-07 | Old graph/evidence designs superseded by accepted 0.8.25 contracts | Deprecate-in-place with successor pointers | Verify all current inbound links and unique rationale. |
| C26-08 | Generated build, package, cache, corpus, and model artifacts | Delete only if tracked accidentally and reproducible | Prove ownership and no unique evidence. |

## Review method

Use `rg --files`, tracked-file classification, inbound-reference searches,
public index comparison, test target enumeration, and source-symbol reachability.
Do not use modification time as authority and do not bulk-move historical
records. Final Slice 2 output expands this seed table to the full repository.
