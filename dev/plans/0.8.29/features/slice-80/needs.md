---
title: Slice 80 residual Block 2 needs
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 needs

This package specifies the residual contract against the integrated source
below; full80 owner acceptance remains open. [Census](census.md), [requirements](requirements.md),
[acceptance](acceptance.md), [design](design.md) and [test map](test-map.md)
form one review unit.

This revision uses integrated source
`e1cb1bd5311f68da8be98bbe133b9cecd5ab56d4`, selected identity/packing
seq-374 and feedback seq-375, and Slice50's independently reviewed
`10a72e5a9` carrier design. Selected design is distinguished from implemented API.
The earlier aggregate-v2 and consolidation consistency reviews passed; they are
not repeated. The updated contract needs its required Astra-medium review and
full80 owner acceptance. No implementation or90 source entry follows.

| Local need | User outcome | Release allocation / existing authority |
| --- | --- | --- |
| S80-N1 | Agents see real readiness and bounded suggestions without mistaking unavailable state/verdict for a failure. | N29-A/R29-REL-A; accepted hints ADR decisions (a), (c), (f); B2-1–3. |
| S80-N2 | An invalid argument or foreign feedback ref produces usable, deterministic correction data. | N29-A/R29-REL-A; hints design §6 B2-4/5; MCP MR-012, MR-024, MR-071. |
| S80-N3 | Agents can use the session default without repeated discovery. | N29-A/R29-REL-A; B2-6; MCP MR-004/005. |
| S80-N4 | Agents receive truthful stored mapped-span totals and availability, while per-hit extents never claim that unseen text or another record influenced a hit. | N29-A/R29-REL-A, N29-K/R29-REL-K; accepted B2-8 head-span storage and guard intent; format37/no migration. |
| S80-N5 | Rust, Python, TypeScript and both MCP formats carry the same accepted semantics, with bounded read work. | N29-K/R29-REL-K; REQ-029/056/064/065, AC-031/060a/068a/b/069/074 and existing performance gates. |
| S80-N6 | Only accepted residuals enter implementation after actual prerequisite closure. | N29-D/R29-REL-D; approved80 AND the separate Block1 follow-up exit before90; independent design and final implementation reviews. |

Compact text default, omission of unavailable-only state/verdict, stored hints,
queue policy and B2-8 approval are settled. No weak-result threshold, calibrated
verdict, chunking, ranking change, fresh embedding or new experiment follows.
SDK hints Stage2/default placement remains conditional on150.
