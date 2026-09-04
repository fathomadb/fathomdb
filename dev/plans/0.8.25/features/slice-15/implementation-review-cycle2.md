---
title: 0.8.25 Slice 15 implementation review — cycle 2
status: COMPLETE
review_cycle: 2
reviewed_on: 2026-09-03
reviewed_commit: 6d88cf1e8fb1514bea6094e1df0493359ae4a50b
verdict: FAIL
---

# Slice 15 independent implementation review — cycle 2

## Verdict

**FAIL.** Two P2 findings remain; no P1 finding remains.

## Findings

| ID | Severity | Finding | Required correction |
| --- | --- | --- | --- |
| I15-09 | P2 | TypeScript's `validateProvenanceFfiTree` eagerly validates every provenance-union string after role dispatch. Malformed strings in illegal derived-only canonical members, illegal locator members, or lower-precedence locator/hash fields can therefore preempt the native closed-object and discriminator errors. | Make the surrogate/NUL guard follow the native parser's precedence. Validate only fields legal for the selected role and locator/hash discriminator, while retaining field-specific provenance errors for fields the parser would consume. |
| I15-10 | P2 | The shared SDK matrix does not pin the remaining closed-object and discriminator precedence cases: malformed derived-only canonical members, `whole_body` offsets, unknown locator/hash discriminators, missing/extra range fields, non-string offsets, and malformed digest variants. | Add shared Python/TypeScript direct and wrapped canonical-node cases plus the remaining locator/hash cases, asserting identical closed reason and canonical JSON-pointer paths. Include wrapped edges only where their variant contract permits them. |

## Confirmed corrections

Cycle 2 confirmed that FIX-2 closes ownerless source-side link corruption for
purge and source erasure, maps direct and wrapped provenance identifiers to the
closed error family, and expands the raw-corruption and legacy-derivation
coverage without adding a public lookup surface.
