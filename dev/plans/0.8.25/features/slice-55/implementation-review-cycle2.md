# Slice 55 implementation review — cycle 2

Verdict: **FAIL**

Candidate reviewed: `77333f93702f0692a4d73e70e5dc5c7ddd8790a4`.

## Blocking findings

1. **P1 — execution-boundary validation is incomplete.** Both integrity and
   dependency trace trust already-constructed public request structs. Direct
   struct literals can bypass constructor checks for schema version, hard caps,
   duplicate checks, enum validity, and related invariants. The execution
   boundary must revalidate every public request field or make invalid
   construction impossible. Evidence:
   `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:1246` and
   `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:483`.

2. **P1 — integrity authority is not fully bounded or authoritative.** The
   implementation performs a full projection-registry load, lacks required
   after-key paging, does not consistently reuse retention/closure/readiness
   classifiers, permits nonmembers to consume capacity ahead of members, does
   not enumerate/account every physical generation member, and does not emit
   `projection_member_corrupt` across the complete matrix. Receipt parsing can
   fetch before reserving aggregate work, committed-boundary relationships and
   classifier error mappings are incomplete, overflow/identity handling is
   imprecise, and attribute ordering is not the declaration authority order.
   Required remediation includes genuine corruption, paging, receipt, property,
   generation-member, and accounting tests. Evidence includes
   `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs` and the false
   integrity oracles at
   `src/rust/crates/fathomdb-engine/tests/slice55_data_plane_integrity.rs:297-311`
   and `:430-470`.

3. **P1 — trace nondisclosure and normalized authority remain incomplete.** A
   malformed lifecycle must collapse to hidden absence; the full digest-to-
   canonical-byte provenance chain and endpoint-specific eligibility/closure
   fences must be authenticated. Paging must filter authorization before its
   bounded remaining-plus-one decision and use after-key/indexed shared
   verification. Production query-plan tests and a genuine hidden/corrupt
   matrix are required. Evidence:
   `src/rust/crates/fathomdb-engine/src/dependency_trace.rs` and the false
   hidden-relation oracles at
   `src/rust/crates/fathomdb-engine/tests/slice55_dependency_trace.rs:172-199`.

4. **P1 — wire and SDK codecs are not strict canonical codecs.** Explicit
   nested declaration-order serializers are required together with semantic
   role/endpoint/order/uniqueness/hard-maximum invariants, error schema version,
   RFC 6901 escaping, and elimination of native `TypeError` leakage. Python and
   TypeScript must recursively validate trace and explanation responses. CLI
   check errors must use `/checks/<index>`. The exact full-byte fixture must be
   consumed as an oracle rather than reconstructed by the implementation.
   Evidence:
   `src/rust/crates/fathomdb-engine/tests/slice55_wire.rs` and the binding/CLI
   codec paths.

5. **P1 — structural explanation lacks the complete live authority.** A
   dependency is registered only when the complete valid dependency,
   provenance, hash, and generation chain authenticates; lifecycle comes from
   the live record. `graph_bound_reached` must be produced by a real graph-bound
   search. Deterministic enable/finalization races and mixed explained versus
   non-explained telemetry must prove one same-snapshot source. Evidence:
   `src/rust/crates/fathomdb-engine/src/lib.rs` and
   `src/rust/crates/fathomdb-engine/tests/slice55_explanation.rs:116`.

6. **P2 — false oracles remain.** Empty happy paths, either-outcome assertions,
   synthetic structural values, and reconstructed wire bytes do not prove the
   required contracts. Replace the hidden trace tests at lines 172-199, the
   integrity plan/receipt/bounded cases at lines 297-311 and 430-470, the wire
   fixture, and the explanation enable race with genuine real-database,
   candidate-native, deterministic assertions. Never accept either outcome.

## Required FIX-2 disposition

Address every P1/P2 with committed RED witnesses before production changes,
retain the hard privacy and work-cap boundaries, and present exact focused and
writer-gate evidence to independent review cycle 3. P3 verification-review
closure remains owned by the independent verifier.
