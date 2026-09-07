---
title: 0.8.25 Slice 60 design review — FIX-2 response
status: FIX2_AWAITING_REVIEW
review_cycle: 2
candidate: 031aced1b9187e309d6234d9b98be9bebaa1550d
design_version: 4
---

# Slice 60 design review — FIX-2 response

## Disposition

Design v4 resolves exactly the five Cycle-2 findings without authorizing source
or test implementation and without adding a migration, continuation, full path,
diffusion, or publication work.

| Finding | FIX-2 disposition |
| --- | --- |
| P1-1 vector query seed eligibility | Schema 33 has no indexed pre-KNN logical-identity discriminator, so graph query seeding declines the complete vector arm before embedding and KNN. One bounded logical-node FTS statement applies identity and indexed eligibility before its limit. The RED matrix requires more than `TOP_K_BIT_CANDIDATES` nearer nonlogical vector rows, a returned farther logical FTS seed, and proof that embedding/KNN did not execute. |
| P1-2 temporal relaxation | The one effective instant remains universal. `include_out_of_window` relaxes only node validity for seeds, intermediate nodes, and outputs; every edge always satisfies recency. Current/frozen and all-direction liveness fixtures cross node relaxation with live, expired, and not-yet-valid edges. |
| P2-1 Rust constructability | Every new public Rust type now has exact derives and exact exhaustiveness. Request structs/unions are exhaustive and directly externally constructible with all declared fields; response structs alone are `#[non_exhaustive]`. No hidden-default constructor or builder is introduced. |
| P2-2 explanation/projection composition | Graph explanation reuses the shipped `x<32-lower-hex-open-nonce>-<canonical-u64-seq>` allocator, with exact allocation timing and deterministic-test normalization. Seed route, projection origin, and readiness mappings are exhaustive, sorted by enum order, and independently composed; `legacy_unverified + degraded` retains both projection codes after the query fallback code. |
| P2-3 response coherence | Decoders require every origin ordinal to index an existing seed, the origin seed ID to equal that seed, target ID equality, positional explanation cardinality/index equality, and field-for-field explanation-origin equality. Exact failure paths and shared Rust-wire/Python/TypeScript malformed fixtures are required. |

## Readiness

The design and plan are `FIX2_AWAITING_REVIEW`. A clean independent Cycle-3
review is still required before either may become `READY`. There is no unresolved
HITL or repository-authority choice in this response.
