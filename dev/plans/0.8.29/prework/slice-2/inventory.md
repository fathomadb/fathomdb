# Slice 2 bounded inventory

## Current census and coverage

November10 read-only census at planning HEAD
`40d82c2e75d1d70fea61f327bf5b2ffcb8a1f479`, product baseline `6edc1155f`:
Initial census:14,019 tracked paths after the104-file planning delta
(97 net added paths), replacing the old13,922 figure. Six parent-owned Slice0
document edits and their new review log were committed during this audit; final
census is14,020 paths/105 changed files/98 net added paths at root recheck
`850f2c15df1ebf3262f0346e340721f9ff2422ad`; the checkout was then clean with
no untracked or ignored path in the status snapshot.
That snapshot says nothing about other checkouts or external payload availability.
The counts are path counts, not independently verified documents or consumers.

| Cohort / audience | Tracked paths | Actual content/consumer sample and coverage limit |
| --- | ---: | --- |
| Program/release planning |9,712 under dev/plans | .29 master/direct mandate/9 rulings/10 tasks;3 mappings; .27 recovery85/90/150/state; .28 publication state. No reread of all historical logs/results. |
| Product/contracts |8 interfaces, plus canonical root docs | needs/requirements/test-plan applicability, acceptance through082d, traceability, Rust/MCP contract ownership. No semantic contract rewrite or executed conformance. |
| Architecture/design |236 under dev/design | active v2, superseded v1/root architecture, format37 ADR, hints ADR,230 catalog records and checker consumer. Catalog counts are classifications, not certification of every entry. |
| Development/history |56 archive,5 progress | archive restoration precedent, frozen progress inbound note, extracted-harness notice/shared tooling. No frozen progress edits or re-import of extracted work. |
| Notes |40 | measured May canonical perf RED, Phase9 dated handoff, quality roadmap. No claim that old performance/support observations apply today. |
| Rust |673 | query compiler/readme/manifest/property tests; engine/schema templates; MCP schema enum and edge consumer; embedder download path. No whole Rust dead-code/reachability analysis. |
| Python |260 | package README/pyproject, reopen property and cursor scaffold. No installed binding or public example execution. |
| TypeScript |138 | package/workspace/generator inputs, maintained cross-binding authority and contributor map. Representative ownership only; no exhaustive TS runtime/test audit. |
| Shared tests/assets |329 tests,5 test | test/ routing READMEs and package discovery/guard source; named substantive properties read. No product tests run, no all-suite adequacy claim. |
| Public/contributor docs |37 | MkDocs nav, contributor map/generator/workflow and worktree operator page; package README publication inputs. No rendered site or external-link qualification. |
| Benchmark/experiment evidence |147 performance-benchmarking | README/PROGRAM, closed CORPUS result, retained positive/negative records, L2 generator's actual JSON reads. No dataset/gold availability or external evidence reconstruction. |
| Branch/worktree metadata |not a file cohort count | Local agent-access/aa-b1/scale/.30 refs and worktree ownership listings inspected. No fetch, owner/liveness census, equivalence proof or retirement recommendation. |

The catalog has230 entries:93 historical,5 superseded,2 deferred,28 reference,
60 proposal,27 maintained,15 experiment. No class edits are proposed merely from
these aggregate numbers. The committed0 review log records design/final PASS and proposal closure,
including its provenance FIX; this does not establish environment or source
readiness. Its six revised proposals retain ownership/preservation prerequisites
for10. The planning census above records actual root HEAD850f2c15d separately
from product baseline6edc1155f.

## Retrieval and limits

Read-only `git ls-files -z`, `git status --short --untracked-files=all --ignored`,
`git diff --stat/--name-only`, branch/worktree lists, targeted `rg`/`git grep`,
source/manifests/JSON content reads and `scripts/release-current.py` informed this
sample. release-current exited0 with no active release. Strict dependent preflight
is not claimed before10 state activation. Parent imports can change the root;
recheck before final handoff and attribute their edits separately.

No network, installs, tests, source edits, ledger reads/writes, file/ref/worktree
moves/deletes or dataset copies occurred. Literal no-hit searches cannot establish
external or dynamic non-use. Ignored/model/raw-capture data, other checkouts,
remote refs, all vendor code and complete source/test adequacy are excluded.
See [dispositions](dispositions.md) and [research](research.md) for concrete proof.
