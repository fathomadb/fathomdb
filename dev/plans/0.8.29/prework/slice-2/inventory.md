# Slice 2 bounded inventory

Read-only November 10 at `6edc1155f`, before parent added prework outputs.
13,922 tracked paths; all 32 untracked paths were recovered planning drafts.
No ignored cohorts appeared in this new checkout; that says nothing about other
checkouts or external data availability.

| Cohort | Tracked paths | Sample / limit |
| --- | --- | --- |
| dev/plans | 9,614 | .29 master/reconciliation, .28 state, .27 recovery |
| dev/design | 236 | 230 catalog entries:93 historical,5 superseded,2 deferred,28 reference,60 proposal,27 maintained,15 experiment; classifications not all independently verified |
| dev/interfaces / notes | 8 /40 | Contract retention; selected historical/current authority claims |
| dev/archive / progress | 56 /5 | Preservation precedent; frozen progress retained |
| docs | 37 | Navigation, contributor generator and consolidation guide |
| Rust / Python / TS | 673 /260 /138 | Representative package docs, source/property tests; not exhaustive reachability |
| tests / test | 329 /5 | Test routing and asset-slot READMEs |
| performance-benchmarking | 147 | Tracked storage rule and closed CORPUS-01 result |

Commands: `git ls-files -z`, `git status --short --ignored`, `rg --files`,
targeted `rg -n` consumers, manifest/property/source reads, lifecycle JSON counts
and `git merge-base --is-ancestor`. No tests or cleanup performed.
