# Slice 3 local needs and authority

The groups below retain the master IDs. Canonical references constrain the
proposals; this document neither passes nor amends their acceptance criteria.
Current applicability corrections preserve historical rewrite IDs and criteria.

| Need → release requirement | Audience / concrete outcome | Canonical constraint / local rows |
| --- | --- | --- |
| N29-A → R29-REL-A | Agent retrieves constrained evidence with usable, bounded, truthful output and fair admission. | NEED-010/012/020/023/026; REQ-021/023/029/030/034/053; AC-023a/b,025,031,032a/b,038,074; BLK1,MCP1,MCP2,SDK,B2-DESIGN,B2-BUILD,SPAN,EXP. |
| N29-S → R29-REL-S | Application gets exact eligible retrieval and evidence-based scaling choices. | NEED-012/022; REQ-010/011/013/014/018/029; current AC-072/073/075/076/081a/b/c successors, not retired budgets; MEASURE,CACHE,SEARCH,EXP. |
| N29-K → R29-REL-K | Rust/Python/TS caller observes coherent identity, typed errors and UTF-8 coordinates. | NEED-010/016/023; REQ-052/053; AC-056/074/079 and maintained SDK contracts; MEASURE,SEARCH,SDK,SPAN,GPUQUAL. |
| N29-R → R29-REL-R | Application can trust declared post-fault state and bounded cleanup/exit. | NEED-001/020/021; REQ-020a/b,021,023,025a,026; AC-022a/b/c,023a/b,025,027a/b/c/d,028a/b/c; BLK1,FAULT,RESET. Physical erasure is complete except defects and included bounded100 qualification. |
| N29-G → R29-REL-G | Consumer selects a genuinely supported GPU artifact and gets explicit lifecycle/support behavior. | NEED-003/016/024; REQ-052/068; AC-056/082a/b/c/d plus accepted CUDA/provider/pool ADRs; RESET,ARTIFACT,GPUQUAL,FORK. |
| N29-D → R29-REL-D | Contributor/release owner can prepare and deliver selected work without repeating solved setup/authority failures. | NEED-005/006/015/023/025/026; REQ-047–052; AC-051a/b,053/054/056 under accepted independent W/E version axes; PREP,DEPS,SURVEY,DOCS,ROUTE,CLOSE. |

Row names abbreviate D29- prefixes in the [CRUD register](crud-register.md).
No persona implies a browser runtime SDK, new model/provider API, policy engine,
new migration path or new performance threshold. Accepted opt-in verified default
weight fetch qualifies historical REQ-033/AC-037 wording; no query egress follows.

N29-3-1: reviewers distinguish built, residual, unqualified and unresolved claims.
N29-3-2: implementers receive one owner and falsifiable contract for each delta.
Both support N29-D and every product group through [local requirements](requirements.md).

Historical REQ-048/AC-052 same-version co-tag wording conflicts with accepted
independent W/E release axes.10 preserves those axes;5/9 reconcile the exact
canonical wording against release architecture before any governed amendment.
No E bump or locked-AC edit is inferred merely to make the old prose agree.
