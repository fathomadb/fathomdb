---
title: 0.8.25 Slice 55 independent implementation review — cycle 3
status: FAIL
reviewed_commit: 786476ebd2a3bee53aacc7d0c25bba0c705111c6
---

# Slice 55 independent implementation review — cycle 3

Verdict: **FAIL**

Candidate reviewed: `786476ebd2a3bee53aacc7d0c25bba0c705111c6`.

## Blocking findings

1. **P1 — new explanation test hooks are public production behavior.** The
   hook module and public arm functions lack any `test` or `test-hooks` gate
   (`src/rust/crates/fathomdb-engine/src/lib.rs:2949` and `:3008`). Every
   explained search executes them, including the after-hook while holding the
   telemetry mutex (`src/rust/crates/fathomdb-engine/src/lib.rs:10944`). A
   downstream caller can block finalization or deadlock by re-entering
   telemetry. Gate the module, public functions, and callsites behind test-only
   configuration, and never execute arbitrary callbacks while holding the
   production telemetry lock. The chronology's test-only description near
   line 622 is inaccurate.

2. **P1 — integrity scanning remains neither classifier-authoritative nor
   comprehensively bounded.** Body and edge checks scan raw prefixes rather
   than shared retention and closure classifiers
   (`src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:637` and
   `:714`). Dense and attribute scans similarly apply `LIMIT` before
   classification, allowing excluded rows to hide later required members
   (`:798` and `:906`). Physical dense enumeration omits terminal-only residues
   (`:865`). Projection-generation enumeration loops over all candidates
   without remaining-cap-plus-one accounting and omits terminal authority
   (`:1071`). Receipt first-pass decoding still materializes unguarded
   `operation_id`, only checks a nonnegative boundary rather than
   `boundary >= max(pending)`, lacks the legacy exception, and calls a
   completion helper without receipt operation or generation authority
   (`:1153`, `:1245`, and `:1292`). The advertised after-key query-plan hook
   tests invented SQL rather than production statements
   (`src/rust/crates/fathomdb-engine/src/lib.rs:13912`). Implement the exact
   shared-classifier, two-direction stable after-key scans and receipt
   point-classifier contract in design lines 389–445 and 489–574.

3. **P1 — trace authorization still occurs after raw candidate `LIMIT` and
   decoding.** Candidate SQL contains no endpoint authorization
   (`src/rust/crates/fathomdb-engine/src/dependency_trace.rs:20`). The
   `to_dependents` path pages and decodes stored IDs, schema, and generation
   before `include_candidate` proves visibility (`:728` and `:570`). Malformed
   hidden rows can therefore produce storage errors and hidden rows can affect
   paging, contrary to the nondisclosure contract at design line 243. Authorize
   both independently proven endpoints in the SQL `JOIN` and `WHERE` before
   eligible-output limiting and relation-corruption classification.

4. **P1 — wire and SDK validation remain inconsistent and permissive.** Rust
   decodes edges but does not reject duplicate dependency IDs or noncanonical
   node and edge ordering
   (`src/rust/crates/fathomdb-engine/src/dependency_trace.rs:1090` and `:1149`).
   Both Rust error records omit required `schema_version`
   (`src/rust/crates/fathomdb-engine/src/dependency_trace.rs:230` and
   `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:254`). Python
   request validation omits root, direction, and context checks and accepts
   boolean `true` as schema version 1 (`src/python/fathomdb/types.py:486`,
   `src/python/fathomdb/engine.py:260`, and `:1567`). TypeScript dereferences
   null or malformed context before typed validation
   (`src/ts/src/index.ts:1283`). Explanation SDKs do not validate complete
   numeric, count, and correlation semantics, and TypeScript silently maps
   unknown arms to `text` (`src/python/fathomdb/engine.py:800` and
   `src/ts/src/index.ts:1741`). Enforce one strict recursive contract and exact
   typed precedence across Rust, Python, and TypeScript.

5. **P1 — `GraphBoundReached` is false-positive at exactly the cap.** The
   traversal sets `bound_reached` upon inserting the cap-th candidate without
   proving an additional eligible candidate was suppressed
   (`src/rust/crates/fathomdb-engine/src/lib.rs:19093`). Set degradation only
   when a further eligible candidate is actually omitted.

6. **P2 — required RED oracles remain vacuous or disconnected from
   production.** Examples are the empty searchable matrix
   (`src/rust/crates/fathomdb-engine/tests/slice55_data_plane_integrity.rs:303`),
   an empty-database macro standing in for multiple receipt and generation
   requirements (`:412`), a receipt-malformed test that runs projection
   generation (`:631`), a tautological empty-chain property (`:645`), and a
   corrupt-endpoint test containing only a happy-path assertion
   (`src/rust/crates/fathomdb-engine/tests/slice55_dependency_trace.rs:328`).
   The single-edge wire fixture cannot expose ordering or duplicate-dependency
   defects (`src/rust/crates/fathomdb-engine/tests/slice55_wire.rs:11`). Replace
   these with adversarial fixtures exercising the stated contracts and observed
   production paths.

## Nonblocking verification finding

1. **P3 — preserve exact performance-mechanism evidence.** The 50,000 hidden
   dependents are seeded outside measurement
   (`src/rust/crates/fathomdb-engine/src/lib.rs:13990` and
   `src/rust/crates/fathomdb-engine/tests/slice55_dependency_trace.rs:529`).
   The SQLite handler counts 1,000-step quanta and interrupts above 10,000
   callbacks (`src/rust/crates/fathomdb-engine/src/lib.rs:13957`); peak RSS and
   reset are present (`:13964`). The chronology records 1,000,000 VM steps,
   1.156 seconds, and zero peak-RSS delta near line 721. Final verification must
   preserve platform, kernel, and process-high-water interpretation.

## Resolved since cycle 2

Execution-time request revalidation, canonical nested trace serialization and
edge decoding, root/digest/lifecycle/endpoint-closure checks, the CLI later-
duplicate path, live explanation dependency/lifecycle sourcing, and genuine
telemetry race fixtures are materially resolved.

FIX-3 must address every P1/P2 through genuine committed RED witnesses before
production changes and return a clean exact candidate for independent cycle 4.
