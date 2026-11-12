---
title: Slice 80 residual Block 2 needs
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 needs

This package reconciles residuals against integrated source
`5e4b5b61765aa9ce2557b2349218f44e74803f05`; it grants no implementation
approval. [Census](census.md), [requirements](requirements.md),
[acceptance](acceptance.md), [design](design.md) and [test map](test-map.md)
form one review unit.

Current documentary source boundary is
`5e4b5b61765aa9ce2557b2349218f44e74803f05`. Historical bounded Phase A,
aggregate-v2 PASS (proposal SHA-256
`a5e4851df2fedcbcce3e7d1b93138980c25e172d0ceb802792d218940bca0e53`)
and consolidation consistency PASS remain recorded in [review log](review-log.md).
Bounded v1 design PASS covers owner patch SHA-256
`3620e6c185568752a6c422e1d37a7c9a577d7c7fe40fef7a8dda8dc45bab2a7e`.
A different fresh reviewer issued FINAL DOC/OUTPUT PASS with no P1/P2 on the
14-file parent diff SHA-256
`709240de171103f26951c123265bbfcdddfa5938df9fea6f2e617e052dda4389`.
Parent scoped Markdown/state/diff checks passed and independently verified78
source pins; [review log](review-log.md) attributes subjects and limits. This v2
updates records/citation only and claims no separate v2 PASS, new ABI/policy,
full80 approval,90 source entry or runtime/resumption GO.
[Current census](census.md) separates integrated code, qualified subsets and holds.

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
