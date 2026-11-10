# Slice 3 proposed CRUD and behavior allocation

Source-only review November 10 at `6edc1155f`; need→requirement groups below use
N29-X→R29-REL-X. C=create local delta; R=retain; U=proposed semantic successor.
All source changes and final decisions remain future work.

| Local row / owner | Need group / CRUD | Current maturity and proposed falsifiable acceptance |
| --- | --- | --- |
| D29-PREP /10 | D /C,R | Owned tools/state missing. Wrong/missing tools fail before workloads; approved generated views and source-bound installed native identity agree. |
| D29-BLK1 /20 | A,R /R,U residual | Real queue/retained permits already built. Deterministic Ready→enqueue drain/cancel controls prove no old-engine execution or lost queue/permit accounting; retain10s wait/30s run semantics. |
| D29-MEASURE /30 | S,K /C | Current timing unmeasured. Source/corpus/features-bound100k tracked AC083, interleaved writes/open/embed/drain and selected300k probes preserve accepted targets and failed outcomes. |
| D29-CACHE /40 | S /C conditional,R | Format37 visibility/native pre-cap eligibility built; new cache proposed. Independent expected IDs hold across snapshots, rollback, external writers and time boundaries;30 evidence gates adoption. |
| D29-SEARCH /50 | S /U,C | Fusion/public text contract intentionally body-dedup. Selected identity census/successor preserves IDs/provenance/revisions across nodes/edges/explanations/graph seeds; exact optimized/reference and Unicode equivalence follow30 baseline. |
| D29-MCP1 /60 | A /U,R | Adapter rejects text filters; SDK accepts them. Selective eligible match behind>100 stronger nonmatches survives limit1, SDK agrees, malformed filters/paging/both formats preserve typed behavior. |
| D29-MCP2 /60 | A /C | Compact default is settled direction, not implemented. Text has no outputSchema/structuredContent; structured preserves schema/JSON text. All tools/errors honor quoting, framing, budgets and continuations. |
| D29-SDK /70 | K,A /U,R | CUDA faults typed and overload labelled; other dispatch errors may silently omit vector. Decide exact fallback/error vocabulary, then per-behavior RED and Rust/Python/TS byte/outcome parity. |
| D29-B2-DESIGN /80 | A /R,U,C | Pure hints/default-store facts built, placeholders remain. Census built/partial/missing; settle omission/coverage/fields without invented calibration. |
| D29-B2-BUILD,SPAN /90 | A,K /C reporting,R storage | B2-8 span storage/validation built; reporting absent in inspected output. Known body endpoints map faithfully through multibyte snippets/rendering; unknown remains unknown. Approved80 AND Block1 follow-up exit required. |
| D29-FAULT /100 | R /R,C only gaps | Strong recovery/physical/reopen tests exist. Map uncovered reached fault positions to committed/rolled-back target/survivor/history/span/fence/WAL post-state. No reopened erasure SLA/power-loss claim. |
| D29-RESET /110 | R,G /C | Current test expects typed loss then SIGSEGV11. Positive bounded close/drop/subprocess exit plus healthy control proves remedy; permitted venue/design and qualified fork fix if needed. |
| D29-ARTIFACT /120 | G /R,C | Current CUDA13 library evidence is not delivered product. Ratified policy/design identifies bytes/toolkit/PTX/driver/library/architecture/selection and installed consumer behavior. |
| D29-GPUQUAL /130 | G,K /C bounded | Pool-release instrumentation exists, final consumer proof unmeasured. Selected final pins/venues prove actual USED release and frozen plateau criterion; Tegra/non-Tegra and12/13 cells distinct. |
| D29-FORK /140 | G /C required repairs,R pins | Owner seq-367: only required110/120 repairs and necessary release versioning. Broader Candle rebase/marker/multiarch work moves to .30 and does not gate .29. Qualify required fork repairs before pins. |
| D29-EXP /150 | S,A /C conditional | Queue/hints existence is not benefit/bottleneck evidence. Individual9 rulings, held-out frozen rule, native baseline and cost bounds precede experiments; separate product/default promotion. Chunking stays parked. |
| D29-CLOSE /160 | D,all /C | .29 not activated/qualified. Every selected row qualifies or receives explicit disposition; unresolved mandatory work blocks close. Combined docs/artifacts and conditional registry smoke; publication separately authorized. |

Primary source seams and actual test assertions are in
[Slice4 alignment](../slice-4/alignment-matrix.md) and
[Slice5 traceability](../slice-5/traceability.md). No behavior deletion proposed.
