---
title: 0.8.25 Slice 55 independent implementation review — cycle 4
status: FAIL
reviewed_commit: d61a550a02c2a1af0f39a780555d23be1e82d160
---

# Slice 55 independent implementation review — cycle 4

Verdict: **FAIL**

Candidate reviewed: `d61a550a02c2a1af0f39a780555d23be1e82d160`.

## Blocking findings

1. **P1 — integrity authority and bounding remain incomplete.** Raw node and
   edge scans do not use the shared retention and closure classifiers
   (`src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:20-32` and
   `:740-887`). Dense scans limit raw rows before classification (`:889-921`),
   and the attribute owner loop leaves excluded rows uncharged (`:74-123`).
   Projection-generation validation performs five point queries for every raw
   cursor without a bounded candidate scan and charges only selected members
   (`:1184-1258`). Receipt validation accepts retired historical generations
   and invokes current physical classification without the receipt operation
   and expected generation (`:1419-1516`), contrary to design lines 569-586.
   Attribute ordering is cursor/name rather than declaration-name/cursor
   (`:1032` and `:1047`; design lines 531-535). Dependency source-side
   findings omit the source revision identity (`:698-704`). Finding overflow
   reports `/maxWorkUnits` instead of `/maxFindings` (`:383-409`), and request
   validation checks duplicate entries before limits, violating the specified
   precedence and later-duplicate `/checks/<index>` path (`:291-315`; design
   lines 849-852). Use remaining-plus-one bounded scans, shared classifiers,
   and a receipt-aware point classifier for every physical class.

2. **P1 — trace authorization does not authenticate the full normalized chain
   before relation decode and limit.** Candidate SQL validates lifecycle and
   filters only (`src/rust/crates/fathomdb-engine/src/dependency_trace.rs:20-89`).
   Provenance and canonical-byte chain validation occurs later (`:435-549` and
   `:626-652`), after relation fields have been decoded (`:777-839`). A
   visible-lifecycle row with malformed or invalid chain authority can still
   influence decoding and eligible limiting. Perform a normalized-chain
   authorization stage before selected relation-field decoding and the
   eligible `LIMIT`, and prove malformed hidden-chain nondisclosure with a
   genuine BLOB relation fixture.

3. **P1 — SDK validation remains incomplete.** Python accepts boolean response
   schema versions (`src/python/fathomdb/engine.py:260`). Malformed frozen
   contexts do not consistently raise `FrozenReadError`, and nested frozen
   context fields are incompletely validated in both Python and TypeScript
   (`src/python/fathomdb/engine.py:1627` and `:165`;
   `src/ts/src/index.ts:1283-1297`). Explanation validation omits query-level
   finite/range checks, result/per-hit positional coherence, and mandatory
   nonempty native correlation and structural fields
   (`src/python/fathomdb/engine.py:102-129`, `:800-870`, and `:1536`;
   `src/ts/src/index.ts:1048-1088`, `:1748-1848`, and `:2897`). Preserve
   compatibility defaults only for explicitly simulated legacy native objects;
   exact current-candidate native responses must be strict.

4. **P2 — genuine adversarial coverage is required for every authority above.**
   Add real-database RED cases for dense excluded prefixes, generation
   nonmember ceilings, retired receipt generations, completed-closure body
   exclusion, declaration ordering, source revision identity,
   `/maxFindings`, and actual dense/generation/receipt production plans. Add a
   trace fixture whose lifecycle is visible but normalized chain is malformed
   and whose relation fields are BLOBs. Add Python and TypeScript response-bool,
   malformed-frozen-context, and explanation-coherence tests.

## Accepted for verification

The P3 performance implementation is accepted. Independent verification must
still preserve the exact platform, kernel, VM-step, and process-high-water RSS
evidence.

FIX-4 must resolve every P1/P2 through genuine committed RED witnesses before
production changes and return a clean exact candidate for independent cycle 5.
