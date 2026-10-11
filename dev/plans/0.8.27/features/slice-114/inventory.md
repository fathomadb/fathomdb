---
title: FathomDB 0.8.27 Slice 114 — engine declaration census
status: COMPLETE
target_release: 0.8.27
source_baseline: eefc2e3d8f9b446750f142180f1bf63742d7b6b4
---

# Slice 114 declaration census

The source census captures every unindented engine `const`/`static` declaration at this candidate. `Reference` is the first non-import code occurrence other than the declaration, preferring the owner's file; it is a search witness, not proof of reachability. Test-only and test-hook declarations are identified from their own cfg attributes or explicit test modules. Function-local production constants/statics and intentionally suppressed bindings are listed below.

**152 declaration rows**; each retained value has a source reference and remains internal unless the public setting matrix says otherwise. No constant is promoted to a user knob by this audit.

| Owner and declaration | Symbol | Category | Reference | Disposition |
| --- | --- | --- | --- | --- |
| `src/rust/crates/fathomdb-engine/src/actuation.rs:12` | `MAX_OPERATIONS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/actuation.rs:117` | Retained: use at src/rust/crates/fathomdb-engine/src/actuation.rs:117 |
| `src/rust/crates/fathomdb-engine/src/actuation.rs:13` | `MAX_AFFECTED_REVISIONS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/actuation.rs:726` | Retained: use at src/rust/crates/fathomdb-engine/src/actuation.rs:726 |
| `src/rust/crates/fathomdb-engine/src/actuation.rs:14` | `MAX_PENDING_CURSORS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/actuation.rs:749` | Retained: use at src/rust/crates/fathomdb-engine/src/actuation.rs:749 |
| `src/rust/crates/fathomdb-engine/src/actuation.rs:15` | `MAX_SOURCE_REFS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/actuation.rs:22` | Retained: use at src/rust/crates/fathomdb-engine/src/actuation.rs:22 |
| `src/rust/crates/fathomdb-engine/src/actuation.rs:16` | `MAX_SOURCE_REFS_PER_OPERATION` | internal operational limit | `src/rust/crates/fathomdb-engine/src/actuation.rs:20` | Retained: use at src/rust/crates/fathomdb-engine/src/actuation.rs:20 |
| `src/rust/crates/fathomdb-engine/src/connection_runtime.rs:12` | `READER_LOOKASIDE_SLOT_SIZE` | internal operational limit | `src/rust/crates/fathomdb-engine/src/connection_runtime.rs:404` | Retained: use at src/rust/crates/fathomdb-engine/src/connection_runtime.rs:404 |
| `src/rust/crates/fathomdb-engine/src/connection_runtime.rs:18` | `READER_LOOKASIDE_SLOT_COUNT` | internal operational limit | `src/rust/crates/fathomdb-engine/src/connection_runtime.rs:405` | Retained: use at src/rust/crates/fathomdb-engine/src/connection_runtime.rs:405 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:19` | `SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:556` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:556 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:20` | `MAX_WORK_UNITS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:543` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:543 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:21` | `MAX_FINDINGS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:549` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:549 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:23` | `FIRST_SQLITE_ROWID` | internal invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:197` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:197 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:26` | `NODE_BODY_OWNER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:157` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:157 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:33` | `EDGE_BODY_OWNER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:158` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:158 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:41` | `SEARCH_V1_MEMBER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:159` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:159 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:44` | `SEARCH_V2_MEMBER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:160` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:160 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:47` | `EDGE_SEARCH_MEMBER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:161` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:161 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:51` | `ATTRIBUTE_MEMBER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:162` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:162 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:55` | `PROPERTY_MEMBER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:163` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:163 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:59` | `ATTRIBUTE_OWNER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:233` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:233 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:66` | `DENSE_TERMINAL_MEMBER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:164` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:164 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:70` | `DENSE_SIDECAR_MEMBER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:165` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:165 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:73` | `DENSE_VECTOR_MEMBER_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:166` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:166 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:76` | `CURRENT_GENERATION_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:167` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:167 |
| `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:83` | `RECEIPT_GUARD_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:168` | Retained: use at src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:168 |
| `src/rust/crates/fathomdb-engine/src/dependency_closure.rs:4` | `CLOSURE_SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/dependency_closure.rs:1124` | Retained: use at src/rust/crates/fathomdb-engine/src/dependency_closure.rs:1124 |
| `src/rust/crates/fathomdb-engine/src/dependency_closure.rs:5` | `CLOSURE_SEQUENCE_KEY` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/dependency_closure.rs:1271` | Retained: use at src/rust/crates/fathomdb-engine/src/dependency_closure.rs:1271 |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:25` | `SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:231` | Retained: use at src/rust/crates/fathomdb-engine/src/dependency_trace.rs:231 |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:26` | `DEFAULT_MAX_RELATIONS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:235` | Retained: use at src/rust/crates/fathomdb-engine/src/dependency_trace.rs:235 |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:27` | `DEFAULT_MAX_WORK_UNITS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:236` | Retained: use at src/rust/crates/fathomdb-engine/src/dependency_trace.rs:236 |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:29` | `TO_SOURCE_CANDIDATE_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:447` | Retained: use at src/rust/crates/fathomdb-engine/src/dependency_trace.rs:447 |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:57` | `TO_DEPENDENTS_CANDIDATE_QUERY` | SQL invariant | `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:447` | Retained: use at src/rust/crates/fathomdb-engine/src/dependency_trace.rs:447 |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:1522` | `DEPENDENCY_GENERATION_KEY` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1591` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1591 |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:1523` | `SOURCE_DEPENDENCY_SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1585` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1585 |
| `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:1524` | `DEPENDENCY_LOOKUP_LIMIT` | internal operational limit | `src/rust/crates/fathomdb-engine/src/dependency.rs:1178` | Retained: use at src/rust/crates/fathomdb-engine/src/dependency.rs:1178 |
| `src/rust/crates/fathomdb-engine/src/embed_dispatch.rs:14` | `DEFAULT_EMBED_TIMEOUT_MS` | configuration default / capacity | `src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:209` | Retained: use at src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:209 |
| `src/rust/crates/fathomdb-engine/src/embedding.rs:8` | `EDGE_FACT_KIND` | internal invariant | `src/rust/crates/fathomdb-engine/src/embedding.rs:87` | Retained: use at src/rust/crates/fathomdb-engine/src/embedding.rs:87 |
| `src/rust/crates/fathomdb-engine/src/erasure.rs:11` | `ERASURE_WAL_TRUNCATE_ATTEMPTS` | internal operational limit | `src/rust/crates/fathomdb-engine/src/erasure.rs:1037` | Retained: use at src/rust/crates/fathomdb-engine/src/erasure.rs:1037 |
| `src/rust/crates/fathomdb-engine/src/erasure.rs:14` | `ERASURE_WAL_TRUNCATE_BACKOFF_MS` | internal invariant | `src/rust/crates/fathomdb-engine/src/erasure.rs:1079` | Retained: use at src/rust/crates/fathomdb-engine/src/erasure.rs:1079 |
| `src/rust/crates/fathomdb-engine/src/erasure.rs:20` | `REDACTED_STABLE_ID` | internal invariant | `src/rust/crates/fathomdb-engine/src/erasure.rs:217` | Retained: use at src/rust/crates/fathomdb-engine/src/erasure.rs:217 |
| `src/rust/crates/fathomdb-engine/src/erasure.rs:38` | `ERASURE_AUDIT_COLLECTIONS` | internal invariant | `src/rust/crates/fathomdb-engine/src/erasure.rs:89` | Retained: use at src/rust/crates/fathomdb-engine/src/erasure.rs:89 |
| `src/rust/crates/fathomdb-engine/src/erasure.rs:46` | `ERASURE_PENDING_REDACTION_COLLECTION` | internal invariant | `src/rust/crates/fathomdb-engine/src/erasure.rs:88` | Retained: use at src/rust/crates/fathomdb-engine/src/erasure.rs:88 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:11` | `SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:633` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:633 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:12` | `TOKEN_PREFIX` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:2409` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2409 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:13` | `TOKEN_MAX_BYTES` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:178` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:178 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:14` | `TOKEN_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:2407` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2407 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:15` | `DATABASE_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:515` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:515 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:16` | `CONTEXT_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:516` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:516 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:17` | `ARTIFACT_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:614` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:614 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:18` | `SOURCE_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:617` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:617 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:19` | `LOCATOR_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:618` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:618 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:20` | `HASH_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:619` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:619 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:21` | `GENERATION_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1215` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1215 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:22` | `GENERATION_TAIL_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1216` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1216 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:23` | `GRAPH_EDGE_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:567` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:567 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:24` | `GRAPH_DATABASE_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1386` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1386 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:25` | `GRAPH_CONTEXT_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1387` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1387 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:26` | `GRAPH_REQUEST_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1396` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1396 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:27` | `GRAPH_TARGET_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:2031` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2031 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:28` | `GRAPH_PREDECESSOR_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:2036` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2036 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:29` | `GRAPH_EDGE_KIND_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:2041` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2041 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:30` | `GRAPH_TARGET_REVISION_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:2046` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2046 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:31` | `GRAPH_EDGE_REVISION_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:2051` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2051 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:32` | `GRAPH_STREAM_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1981` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1981 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:33` | `GRAPH_MAC_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1994` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1994 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:34` | `GRAPH_TOKEN_PREFIX` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1990` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1990 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:35` | `GRAPH_SELECTOR_BYTES` | internal operational limit | `src/rust/crates/fathomdb-engine/src/evidence.rs:2018` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2018 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:36` | `GRAPH_TOKEN_BYTES` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:2057` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:2057 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:1308` | `INTRINSIC_NODE_PREFLIGHT_SQL` | SQL invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1874` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1874 |
| `src/rust/crates/fathomdb-engine/src/evidence.rs:1340` | `INTRINSIC_EDGE_PREFLIGHT_SQL` | SQL invariant | `src/rust/crates/fathomdb-engine/src/evidence.rs:1883` | Retained: use at src/rust/crates/fathomdb-engine/src/evidence.rs:1883 |
| `src/rust/crates/fathomdb-engine/src/filter.rs:43` | `PREDICATE_PATH_ALLOWLIST` | internal invariant | `src/rust/crates/fathomdb-engine/src/filter.rs:168` | Retained: use at src/rust/crates/fathomdb-engine/src/filter.rs:168 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:20` | `FROZEN_READ_SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:69` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:69 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:21` | `TOKEN_PREFIX` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:252` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:252 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:22` | `TOKEN_MAX_BYTES` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:253` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:253 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:23` | `CONTEXT_MAX_BYTES` | internal operational limit | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:176` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:176 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:24` | `CONTEXT_MAX_ATTRIBUTES` | internal invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:169` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:169 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:25` | `CONTEXT_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:242` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:242 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:26` | `TOKEN_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:251` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:251 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:27` | `PAGE_CURSOR_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/pagination.rs:155` | Retained: use at src/rust/crates/fathomdb-engine/src/pagination.rs:155 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:28` | `REGISTRY_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:489` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:489 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:29` | `SERVING_DOMAIN_V1` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:533` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:533 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:30` | `SERVING_DOMAIN_V2` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:531` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:531 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:31` | `SERVING_DOMAIN_V3` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:529` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:529 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:32` | `DATABASE_ID_KEY` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:311` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:311 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:33` | `READ_CONTEXT_KEY` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:249` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:249 |
| `src/rust/crates/fathomdb-engine/src/frozen_read.rs:34` | `VISIBILITY_TRIGGER_TABLES` | internal invariant | `src/rust/crates/fathomdb-engine/src/frozen_read.rs:377` | Retained: use at src/rust/crates/fathomdb-engine/src/frozen_read.rs:377 |
| `src/rust/crates/fathomdb-engine/src/fusion.rs:9` | `RRF_K` | internal invariant | `src/rust/crates/fathomdb-engine/src/fusion.rs:99` | Retained: use at src/rust/crates/fathomdb-engine/src/fusion.rs:99 |
| `src/rust/crates/fathomdb-engine/src/fusion.rs:16` | `RRF_WEIGHT_VECTOR` | internal invariant | `src/rust/crates/fathomdb-engine/src/fusion.rs:129` | Retained: use at src/rust/crates/fathomdb-engine/src/fusion.rs:129 |
| `src/rust/crates/fathomdb-engine/src/fusion.rs:18` | `RRF_WEIGHT_TEXT` | internal invariant | `src/rust/crates/fathomdb-engine/src/fusion.rs:132` | Retained: use at src/rust/crates/fathomdb-engine/src/fusion.rs:132 |
| `src/rust/crates/fathomdb-engine/src/fusion.rs:26` | `RRF_WEIGHT_GRAPH` | internal invariant | `src/rust/crates/fathomdb-engine/src/fusion.rs:139` | Retained: use at src/rust/crates/fathomdb-engine/src/fusion.rs:139 |
| `src/rust/crates/fathomdb-engine/src/fusion.rs:45` | `RECENCY_WEIGHT` | internal invariant | `src/rust/crates/fathomdb-engine/src/fusion.rs:177` | Retained: use at src/rust/crates/fathomdb-engine/src/fusion.rs:177 |
| `src/rust/crates/fathomdb-engine/src/graph_expand/execution.rs:37` | `GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE` | test-only / test-hooks | `src/rust/crates/fathomdb-engine/src/graph_api.rs:129` | Excluded: fixture or feature-gated test seam |
| `src/rust/crates/fathomdb-engine/src/graph_expand/execution.rs:40` | `GRAPH_EXPAND_SQL_STATEMENTS` | test-only / test-hooks | `src/rust/crates/fathomdb-engine/src/graph_expand/execution.rs:45` | Excluded: fixture or feature-gated test seam |
| `src/rust/crates/fathomdb-engine/src/graph_expand/traversal.rs:63` | `GRAPH_NEIGHBORS_HARD_CAP` | internal operational limit | `src/rust/crates/fathomdb-engine/src/graph_expand/traversal.rs:93` | Retained: use at src/rust/crates/fathomdb-engine/src/graph_expand/traversal.rs:93 |
| `src/rust/crates/fathomdb-engine/src/graph_expand/types.rs:7` | `SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/graph_expand/types.rs:238` | Retained: use at src/rust/crates/fathomdb-engine/src/graph_expand/types.rs:238 |
| `src/rust/crates/fathomdb-engine/src/index_projector.rs:10` | `SEARCH_INDEX_TOKENIZER_SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1139` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1139 |
| `src/rust/crates/fathomdb-engine/src/index_projector.rs:21` | `SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/index_projector.rs:216` | Retained: use at src/rust/crates/fathomdb-engine/src/index_projector.rs:216 |
| `src/rust/crates/fathomdb-engine/src/lib.rs:434` | `PROJECTION_WORKERS` | test-only / test-hooks | `src/rust/crates/fathomdb-engine/src/lib.rs:787` | Excluded: `#[cfg(test)]` fixture |
| `src/rust/crates/fathomdb-engine/src/mean.rs:26` | `MEAN_VEC_PIN_THRESHOLD` | internal operational limit | `src/rust/crates/fathomdb-engine/src/vector_storage.rs:696` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_storage.rs:696 |
| `src/rust/crates/fathomdb-engine/src/open.rs:165` | `ADMISSION_LOCKED_HOOK_FOR_TEST` | test-only / test-hooks | `src/rust/crates/fathomdb-engine/src/open.rs:243` | Excluded: fixture or feature-gated test seam |
| `src/rust/crates/fathomdb-engine/src/open.rs:182` | `POST_PROBE_STARTUP_FAULT_FOR_TEST` | test-only / test-hooks | `src/rust/crates/fathomdb-engine/src/open.rs:227` | Excluded: fixture or feature-gated test seam |
| `src/rust/crates/fathomdb-engine/src/open.rs:195` | `POST_PROBE_VISIBILITY_FAULT_FOR_TEST` | test-only / test-hooks | `src/rust/crates/fathomdb-engine/src/open.rs:206` | Excluded: fixture or feature-gated test seam |
| `src/rust/crates/fathomdb-engine/src/open.rs:1378` | `EDGE_VECTOR_PRUNE_MARKER_KEY` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1394` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1394 |
| `src/rust/crates/fathomdb-engine/src/open.rs:1635` | `DEFAULT_EMBEDDER_NAME` | internal invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1907` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1907 |
| `src/rust/crates/fathomdb-engine/src/open.rs:1636` | `DEFAULT_EMBEDDER_REVISION` | internal invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1908` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1908 |
| `src/rust/crates/fathomdb-engine/src/open.rs:1637` | `DEFAULT_EMBEDDER_DIMENSION` | internal invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1909` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1909 |
| `src/rust/crates/fathomdb-engine/src/open.rs:1648` | `BGE_SMALL_EMBEDDER_NAME` | internal invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1289` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1289 |
| `src/rust/crates/fathomdb-engine/src/open.rs:1650` | `EXPLANATION_OPEN_NONCE_SEQUENCE` | internal invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1654` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1654 |
| `src/rust/crates/fathomdb-engine/src/open.rs:1787` | `ENV_GPU_ALLOCATION_WITNESS` | opt-in operator diagnostic | `src/rust/crates/fathomdb-engine/src/open.rs:1799` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1799 |
| `src/rust/crates/fathomdb-engine/src/pagination.rs:7` | `PAGE_SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/pagination.rs:125` | Retained: use at src/rust/crates/fathomdb-engine/src/pagination.rs:125 |
| `src/rust/crates/fathomdb-engine/src/pagination.rs:8` | `PAGE_LIMIT_MAX` | internal invariant | `src/rust/crates/fathomdb-engine/src/pagination.rs:130` | Retained: use at src/rust/crates/fathomdb-engine/src/pagination.rs:130 |
| `src/rust/crates/fathomdb-engine/src/pagination.rs:9` | `CURSOR_PREFIX` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/pagination.rs:157` | Retained: use at src/rust/crates/fathomdb-engine/src/pagination.rs:157 |
| `src/rust/crates/fathomdb-engine/src/pagination.rs:10` | `CURSOR_MAX_BYTES` | internal operational limit | `src/rust/crates/fathomdb-engine/src/pagination.rs:170` | Retained: use at src/rust/crates/fathomdb-engine/src/pagination.rs:170 |
| `src/rust/crates/fathomdb-engine/src/pagination.rs:11` | `SELECTOR_DOMAIN` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/pagination.rs:148` | Retained: use at src/rust/crates/fathomdb-engine/src/pagination.rs:148 |
| `src/rust/crates/fathomdb-engine/src/projection_generation.rs:16` | `PREFIX` | internal invariant | `src/rust/crates/fathomdb-engine/src/projection_generation.rs:35` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_generation.rs:35 |
| `src/rust/crates/fathomdb-engine/src/projection_rebuild.rs:33` | `REBUILD_DRAIN_TIMEOUT_MS` | internal invariant | `src/rust/crates/fathomdb-engine/src/projection_rebuild.rs:70` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_rebuild.rs:70 |
| `src/rust/crates/fathomdb-engine/src/projection_registry.rs:287` | `ROW_OWNED_PROJECTIONS` | internal invariant | `src/rust/crates/fathomdb-engine/src/projection_registry.rs:347` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_registry.rs:347 |
| `src/rust/crates/fathomdb-engine/src/projection_registry.rs:1013` | `ATTR_VEC0_PRESENT_MARKER` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/projection_registry.rs:1021` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_registry.rs:1021 |
| `src/rust/crates/fathomdb-engine/src/projection_runtime.rs:121` | `PROJECTION_RUNTIME_STARTUP_TIMEOUT` | internal operational limit | `src/rust/crates/fathomdb-engine/src/projection_runtime.rs:369` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_runtime.rs:369 |
| `src/rust/crates/fathomdb-engine/src/projection_runtime.rs:122` | `DEFAULT_PROJECTION_RETRY_DELAYS_MS` | internal invariant | `src/rust/crates/fathomdb-engine/src/projection_runtime.rs:459` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_runtime.rs:459 |
| `src/rust/crates/fathomdb-engine/src/projection_runtime.rs:123` | `PROJECTION_TEMPORAL_WAKE_POLL` | internal operational limit | `src/rust/crates/fathomdb-engine/src/projection_worker.rs:242` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_worker.rs:242 |
| `src/rust/crates/fathomdb-engine/src/projection_runtime.rs:124` | `PROJECTION_CURSOR_KEY` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/projection_commit.rs:38` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_commit.rs:38 |
| `src/rust/crates/fathomdb-engine/src/projection_runtime.rs:125` | `PROJECTION_COMMIT_BATCH` | configuration default / capacity | `src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:237` | Retained: use at src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:237 |
| `src/rust/crates/fathomdb-engine/src/projection_worker.rs:890` | `PROJECTION_CANDIDATE_PAGE` | internal invariant | `src/rust/crates/fathomdb-engine/src/projection_worker.rs:910` | Retained: use at src/rust/crates/fathomdb-engine/src/projection_worker.rs:910 |
| `src/rust/crates/fathomdb-engine/src/provenance.rs:230` | `DEFAULT_PROVENANCE_ROW_CAP` | configuration default / capacity | `src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:216` | Retained: use at src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:216 |
| `src/rust/crates/fathomdb-engine/src/read.rs:65` | `READ_COLLECTION_MAX_LIMIT` | internal operational limit | `src/rust/crates/fathomdb-engine/src/read.rs:158` | Retained: use at src/rust/crates/fathomdb-engine/src/read.rs:158 |
| `src/rust/crates/fathomdb-engine/src/read.rs:413` | `OPERATIONAL_STATE_POINT_SQL` | SQL invariant | `src/rust/crates/fathomdb-engine/src/read.rs:440` | Retained: use at src/rust/crates/fathomdb-engine/src/read.rs:440 |
| `src/rust/crates/fathomdb-engine/src/read.rs:417` | `OPERATIONAL_STATE_PAGE_SQL` | SQL invariant | `src/rust/crates/fathomdb-engine/src/read.rs:479` | Retained: use at src/rust/crates/fathomdb-engine/src/read.rs:479 |
| `src/rust/crates/fathomdb-engine/src/reader_pool.rs:4` | `READER_POOL_SIZE` | internal operational limit | `src/rust/crates/fathomdb-engine/src/open.rs:1320` | Retained: production reader-pool construction at `open.rs:1320` |
| `src/rust/crates/fathomdb-engine/src/reader_pool.rs:500` | `READER_WORKER_CHANNEL_CAPACITY` | internal invariant | `src/rust/crates/fathomdb-engine/src/reader_pool.rs:955` | Retained: use at src/rust/crates/fathomdb-engine/src/reader_pool.rs:955 |
| `src/rust/crates/fathomdb-engine/src/record_lifecycle.rs:7` | `LIFECYCLE_DRAIN_TIMEOUT_MS` | internal invariant | `src/rust/crates/fathomdb-engine/src/runtime_lifecycle.rs:102` | Retained: use at src/rust/crates/fathomdb-engine/src/runtime_lifecycle.rs:102 |
| `src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:63` | `SQLITE_RUNTIME_STATE` | process SQLite mode state | `src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:86` | Retained: use at src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:86 |
| `src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:139` | `MAX_SAFE_INTEGER` | internal operational limit | `src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:218` | Retained: use at src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:218 |
| `src/rust/crates/fathomdb-engine/src/search.rs:373` | `_` | compile-time assertion | `src/rust/crates/fathomdb-engine/src/search.rs:373` | Retained: zero-size assertion for `NoEvidenceCapture` |
| `src/rust/crates/fathomdb-engine/src/search_types.rs:348` | `TOP_K_BIT_CANDIDATES` | internal operational limit | `src/rust/crates/fathomdb-engine/src/filter.rs:580` | Retained: use at src/rust/crates/fathomdb-engine/src/filter.rs:580 |
| `src/rust/crates/fathomdb-engine/src/search_types.rs:356` | `SEARCH_RERANK_LIMIT` | internal operational limit | `src/rust/crates/fathomdb-engine/src/search_types.rs:359` | Retained: use at src/rust/crates/fathomdb-engine/src/search_types.rs:359 |
| `src/rust/crates/fathomdb-engine/src/search_types.rs:359` | `DEFAULT_SEARCH_RESULT_LIMIT` | internal operational limit | `src/rust/crates/fathomdb-engine/src/search_api.rs:676` | Retained: use at src/rust/crates/fathomdb-engine/src/search_api.rs:676 |
| `src/rust/crates/fathomdb-engine/src/search_types.rs:362` | `MAX_SEARCH_RESULT_LIMIT` | internal operational limit | `src/rust/crates/fathomdb-engine/src/search_types.rs:365` | Retained: use at src/rust/crates/fathomdb-engine/src/search_types.rs:365 |
| `src/rust/crates/fathomdb-engine/src/telemetry.rs:3` | `DEFAULT_SLOW_THRESHOLD_MS` | configuration default / capacity | `src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:223` | Retained: use at src/rust/crates/fathomdb-engine/src/runtime_configuration.rs:223 |
| `src/rust/crates/fathomdb-engine/src/temporal.rs:10` | `EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/open.rs:1195` | Retained: use at src/rust/crates/fathomdb-engine/src/open.rs:1195 |
| `src/rust/crates/fathomdb-engine/src/temporal.rs:257` | `CLOCK_READS` | test observation seam | `src/rust/crates/fathomdb-engine/src/temporal.rs:287` | Retained: production clock-read increment supports test observation |
| `src/rust/crates/fathomdb-engine/src/temporal.rs:541` | `MIN_RENDERABLE_EPOCH` | internal invariant | `src/rust/crates/fathomdb-engine/src/temporal.rs:585` | Retained: use at src/rust/crates/fathomdb-engine/src/temporal.rs:585 |
| `src/rust/crates/fathomdb-engine/src/temporal.rs:542` | `MAX_RENDERABLE_EPOCH` | internal operational limit | `src/rust/crates/fathomdb-engine/src/temporal.rs:585` | Retained: use at src/rust/crates/fathomdb-engine/src/temporal.rs:585 |
| `src/rust/crates/fathomdb-engine/src/test_hooks.rs:329` | `PROJECTION_TRANSACTION_TEST_PAUSE_RELEASE_TIMEOUT` | test-only / test-hooks | `src/rust/crates/fathomdb-engine/src/projection_commit.rs:178` | Excluded: fixture or feature-gated test seam |
| `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:7` | `VECTOR_EQUIVALENCE_PROBE_FIXTURE` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:48` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:48 |
| `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:16` | `VECTOR_EQUIVALENCE_L2_EPSILON` | internal invariant | `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:366` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:366 |
| `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:21` | `VECTOR_EQUIVALENCE_P1_FLIP_FLOOR` | internal invariant | `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:365` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:365 |
| `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:35` | `VECTOR_EQUIVALENCE_VERDICT_CACHE_KEY` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:429` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:429 |
| `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:42` | `VECTOR_EQUIVALENCE_FINGERPRINT_RECIPE` | schema / protocol invariant | `src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:353` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_equivalence.rs:353 |
| `src/rust/crates/fathomdb-engine/src/vector_storage.rs:3` | `DEFAULT_VECTOR_PROFILE` | internal invariant | `src/rust/crates/fathomdb-engine/src/vector_storage.rs:9` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_storage.rs:9 |
| `src/rust/crates/fathomdb-engine/src/vector_storage.rs:4` | `DEFAULT_VECTOR_PARTITION` | internal invariant | `src/rust/crates/fathomdb-engine/src/vector_storage.rs:61` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_storage.rs:61 |
| `src/rust/crates/fathomdb-engine/src/vector_storage.rs:476` | `KIND_TO_SOURCE_TYPE_CASE_SQL` | SQL invariant | `src/rust/crates/fathomdb-engine/src/vector_storage.rs:534` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_storage.rs:534 |
| `src/rust/crates/fathomdb-engine/src/vector_storage.rs:586` | `VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES` | internal invariant | `src/rust/crates/fathomdb-engine/src/vector_storage.rs:603` | Retained: use at src/rust/crates/fathomdb-engine/src/vector_storage.rs:603 |

## Public input to consumer trace

The five `EngineConfig` fields are optional `u64` in Rust, Python snake_case
`int | None` in either `Engine.open` keyword or frozen `EngineConfig` form, and
TypeScript camelCase finite safe integers in a frozen `engineConfig` object.
Omission selects the Rust default. Rust validates before filesystem, SQLite,
locks, provider, or workers; Python and TypeScript reject malformed values
before native open. Requested snapshots remain unchanged by the live slow
threshold setter.

| Rust / Python field; TypeScript field | Default and accepted range | Zero and mutability | Effective consumer and outcome |
| --- | --- | --- | --- |
| `scheduler_runtime_threads`; `schedulerRuntimeThreads` | `2`; `1..=64` workers | Zero invalid; open-time | `runtime_configuration.rs` derives `64 * workers` admission; `projection_runtime.rs` starts exactly that many workers and owned SQLite connections. Invalid Rust input is `EngineConfigurationError`; Python `ValueError`; TS `RangeError`. |
| `embedder_pool_size`; `embedderPoolSize` | `5`; `1..=64` workers | Zero invalid; open-time | `embed_dispatch.rs` / `projection_runtime.rs` start fixed provider workers and `4 * workers` waiting slots. No provider starts none. A full/expired queue reports `Overloaded` for direct embedding and leaves projection pending; hybrid search may use sparse fallback. |
| `embedder_call_timeout_ms`; `embedderCallTimeoutMs` | `30_000`; `1..=u32::MAX` ms | Zero invalid; open-time | `open.rs` and `projection_runtime.rs` pass one absolute queue-plus-service deadline to the dispatcher; started failure/timeout uses existing route errors or projection retry. |
| `provenance_row_cap`; `provenanceRowCap` | `1_000_000`; `0..=2^53-1` rows | Zero disables retention; open-time | `open.rs` initializes the cap; `write_commit.rs` and `actuation.rs` enforce retention after commits. |
| `slow_threshold_ms`; `slowThresholdMs` | `100`; `0..=2^53-1` ms | Zero accepts every positive duration; live setter | `open.rs` initializes the atomic threshold; `telemetry.rs` consumes and updates it for operation/SQLite slow signals. The setter does not rewrite `Engine::config()`. |

A non-integer/bool Python request raises `TypeError`; out-of-range raises
`ValueError`. TypeScript non-number raises `TypeError`; fractional, unsafe, or
out-of-range raises `RangeError`. Native conversion and Rust final validation
may return `InvalidArgumentError` in bindings. Exact Rust names and ranges are
in `dev/interfaces/rust.md`; binding forms are in `dev/interfaces/python.md`,
`dev/interfaces/typescript.md`, and `docs/reference/config.md`. A derived
native-width capacity overflow has its own Rust error. No range changes are
proposed.

The process-wide SQLite mode is **not** an `EngineConfig` field. Rust
`admin::configure_runtime(RuntimeSqliteMode::{Performance,Diagnostics})`,
Python `admin.configure_runtime(sqlite_mode="performance" | "diagnostics")`,
and TypeScript `admin.configureRuntime({ sqliteMode: ... })` all select the
loaded SQLite runtime. The first ordinary open chooses performance. A repeat
of the effective mode succeeds even after Engine open; a conflicting request
fails with `Conflict`, and a first request after external SQLite initialization
fails `TooLate`. Its owner is `runtime_configuration.rs:63–126`, with no live
mutation or fallback. The post-open witness and corrected guidance are the
Slice 114 documentation fix.

## Live controls outside EngineConfig

| Control and binding spelling | Default and accepted input | Consumer, mutability, and failure |
| --- | --- | --- |
| `Engine::set_profiling(enabled)`; Python `set_profiling(enabled=...)`; TypeScript `setProfiling(enabled)` | Off at open; boolean | `telemetry.rs:111` changes an atomic flag for subsequent response-cycle profiling without restart. It does not change `Engine::config()`. |
| `Engine::set_slow_threshold_ms(value)`; Python `set_slow_threshold_ms(value=...)`; TypeScript `setSlowThresholdMs(value)` | `100` ms at open; nonnegative safe integer at binding boundary | `telemetry.rs:121` changes the live threshold for subsequent operation and SQLite-statement slow signals. The Rust setter accepts `u64`; binding validators keep their documented range. It does not change requested config. |
| `Engine::enable_telemetry(sink_path)`; Python `enable_telemetry`; TypeScript `enableTelemetry` | Off at open; caller-supplied local path | `telemetry.rs:131` first verifies the appendable sink, then arms local JSONL capture. A bad path returns storage failure before enable. Re-enable resets the sequence; this is a live instrumentation control, not an `EngineConfig` field. |

Other public methods with configuration-like verbs have distinct contracts:
`configure_projections` persists governed projection declarations;
`register_source_dependency` persists a dependency; `_for_test` setters are
test seams. None is a runtime setting. `drain` and `subscribe` are live
operations, but do not set configuration.

## Environment inputs and function-local declarations

| Source | Input or declaration | Classification and disposition |
| --- | --- | --- |
| `projection_worker.rs:574` | `FATHOMDB_PROJECTION_BATCH` | Opt-in batch route for `1`, `true`, or `on`; other/unset values choose per-row route. Retain existing private behavior; it is not an `EngineConfig` setting. |
| `provider.rs:158` | `FATHOMDB_EXTRACTOR_TIMEOUT_MS` | BYO extractor I/O timeout, default 300 seconds; parseable `u64` milliseconds overrides, malformed input falls back to default. Keep separate from embedder dispatch deadline. |
| `connection_runtime.rs:314,355` | `FATHOMDB_PERF_EXPERIMENTS`, `FATHOMDB_PERF_WRITER_PRAGMAS`, `FATHOMDB_PERF_READER_PRAGMAS` | Experimental SQLite PRAGMA hooks. Gate uses variable **presence**, not `=1`; malformed entries and SQLite failures log without failing open. Stale comments corrected; no production default change. |
| `search.rs:490–1841` | `FATHOMDB_PERF_SEARCH_LIMIT`; `FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST`; `FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST`; `FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST`; `FATHOMDB_FTS_FAIL_STREAM_FOR_TEST` | Search experiment and test controls. `FATHOMDB_PERF_SEARCH_LIMIT` is read only when `FATHOMDB_PERF_EXPERIMENTS` is present; malformed values are ignored. The FTS keys are test/diagnostic controls, not user settings. |
| `open.rs:1787–1832` | `FATHOMDB_GPU_ALLOCATION_WITNESS` | Opt-in operator diagnostic on default-embedder builds. `1`/`true` demand a witness or open fails; `0`/`false`/unset disable; invalid values fail open. Preserve the prior Tegra contract. |
| `wal_attribution.rs:291` | `FATHOMDB_WAL_ATTRIBUTION` | Opt-in WAL diagnostic on variable presence; `cfg(test)` also enables it. Not a process-mode knob. |
| `connection_runtime.rs:212` | function-local `REGISTER: Once` | Retain: guarantees one `sqlite3_auto_extension` registration per process. Excluded from the module-level table because it is function-local, explicitly accounted here. |
| `rerank.rs:287` | function-local `CELL: OnceLock` | Retain: lazy default reranker singleton. |
| `erasure.rs:147`; `open.rs:1541–1543`; `graph_expand/traversal.rs:109,113` | function-local constants | Retain bounded tail folds, WAL-header validation constants, and SQL parameter positions respectively. Not separate settings. |
| `search.rs:914–915`; `reader_pool.rs:1070`; `open.rs:2065` | function-local constants | Retain search fallback calibration, bounded test reply wait, and percent-encoding digits respectively. Test-only where declared under a test module. |
| `tc5_benchmark.rs:98`; `search.rs:493,514`; `evidence.rs:2795`; `write_commit.rs:113` | function-local/thread-local statics | Test/benchmark observation state, excluded from production setting inventory. |

## Suppressed unused-binding disposition

The source search found 32 `allow(dead_code)` attributes and 17 underscore
bindings. The attributes are grouped below by owner and exact locations;
these are deliberate production/test seam suppressions, not unexplained
configuration inputs. No compiler-reported new unused production binding was
observed in the default build. The feature-build warning check is recorded in
the status.

| Owner and locations | Disposition |
| --- | --- |
| `lib.rs:55,439,509`; `embed_dispatch/core.rs:331,452,461,477,495,500,509` | Retain runtime diagnostic/config snapshot and standalone dispatcher test seams. `resolved_config` is read by `embed_dispatch.rs` and `wal_runtime.rs`; `reader_lookaside_rcs` is a test observation. Broad `embed_dispatch` module suppression is assigned to Slice 140 seam extraction for narrower gating. The obsolete integration-batch comment was deleted here. |
| `wal_runtime.rs:214,219,228`; `wal_attribution.rs:124,133,144,492`; `projection_runtime.rs:860,959`; `reader_pool.rs:224` | Retain WAL attribution and pause witnesses in test/debug builds; the production WAL diagnostic uses the same redacted types. Slice 140 can narrow visibility and suppression while moving test seams. |
| `evidence.rs:1413,1452,2088`; `filter.rs:735,885,949`; `identity.rs:40`; `dependency.rs:204,221,586` | Retain evidence selector/preflight shapes, filter functions used by evidence/dependency tracing, legacy revision derivation and prospective-write helpers. Some fields/helpers are used only by a feature or test; removing them here would cross the Slice 140 test-seam boundary. |
| `lifecycle.rs:299,304`; `operator/data_plane.rs:76`; `tc5_benchmark.rs:269,412`; `projection_worker.rs:166,374,1062`; `runtime_lifecycle.rs:18`; `mean.rs:421`; `evidence.rs:1689,2283`; `erasure.rs:918`; `projection_commit.rs:134,158`; `reader_pool.rs:515,527`; `embed_dispatch/d27_observation.rs:33` | Underscore bindings retain guards, registrations, observers, or validation side effects. The two `evidence.rs` values and `erasure.rs` validated value intentionally reject invalid input despite an unused returned object. These are not unused settings. |
| `lib.rs:1187` | Test-only binding, excluded from production. |

`let _ = ...` result discards are separately used for cleanup/notification or
best-effort telemetry; they are not unused named bindings. The census does not
infer that a suppressed helper is dead solely from a lexical search. No
configuration default or operational limit needs a speculative new accepted
range in this slice; Slice 115 receives measurement questions in status.
