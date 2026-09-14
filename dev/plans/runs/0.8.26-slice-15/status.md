# Slice 15 status

Status: corrected spike implementation, measurement, exact teardown, and final
gates complete; ready for independent review.

The committed history preserves both the rejected exploratory shortcut and the
FIX-1 RED → GREEN → corrected measurement → teardown chain. The corrected
decision evidence covers the approved functional, provenance, concurrency,
writer, RSS, erasure, carrier, 10,000-work-unit, 1 KiB, 100 KiB, and
Memex-shaped arms. The recommendation is option A, subject to HITL ruling of
D26-01.

FIX-1 corrected the decision treatment to exactly two class-specific joined
data statements after selection, with plan inspection outside the timed path
and no per-artifact SQL or re-hashing during mint. The intrinsic carrier no
longer reuses ranked-search payloads or fabricates search fields. Writer and
erasure campaigns now use explicit readiness rendezvous and balanced order;
the workload model uses expected mean rather than invalid p50 interpolation.

Teardown restored every transient shipping-code path byte-for-byte to the
pre-spike tree and removed the transient prototype tests. Only the durable
evidence and a behavior-neutral literal ordinary graph-response regression
remain. No public API, schema, binding, persistence, migration, or release-state
decision was made. D26-01 remains open.

Final-tree verification:

- focused graph/evidence/reader/erasure tests: 70 passed serially;
- canonical `agent-verify`: 109 of 110 suites passed, one intended skip;
- `cargo clippy --workspace --all-targets -- -D warnings`: passed;
- `cargo check --workspace --all-targets`: passed.

The canonical verifier used a disposable worktree virtual environment containing
the exact-source test-hooks wheel and pinned dev tools. `PYTHONPATH` named the
worktree source so Python child processes imported the repo-only `eval` package.
Earlier setup attempts are retained in diagnostics: one lacked the worktree
tools/native module, and one used a broken relative native-module link. After
the link was corrected to its verified absolute target, the unchanged canonical
gate passed.

Next step: independent review of the committed chronology and evidence, then
HITL consideration of D26-01.
