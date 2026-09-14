# Slice 15 status

Status: corrected spike implementation, measurement, exact teardown, and final
gates complete; ready for independent review.

The committed history preserves the rejected exploratory shortcut and both
review corrections through their RED → GREEN → measurement → teardown chains.
The final corrected decision evidence covers the approved functional,
provenance, concurrency,
writer, RSS, erasure, carrier, 10,000-work-unit, 1 KiB, 100 KiB, and
Memex-shaped arms. The recommendation is option A, subject to HITL ruling of
D26-01.

FIX-1 corrected the decision treatment to exactly two class-specific joined
data statements after selection, with plan inspection outside the timed path
and no per-artifact SQL or re-hashing during mint. The intrinsic carrier no
longer reuses ranked-search payloads or fabricates search fields. Writer and
erasure campaigns now use explicit readiness rendezvous and balanced order;
the workload model uses expected mean rather than invalid p50 interpolation.

FIX-2 completes the intrinsic locator, span, direct dependency generation, and
class-specific lifecycle material; aligns node eligibility with shipped target
filters while keeping target-only filters out of terminal-edge resolution; and
counts only successful writer operations during the timed interval. Every
loaded writer arm achieved at least 20 overlapping commits. The corrected
campaign crossed the writer-throughput reconsideration threshold: median
throughput ratios were 0.781 for graph hydration and 0.661 for point
resolution. This strengthens the option-A recommendation but requires Slice 20
to reduce or explicitly bound interference before shipping.

Teardown restored every transient shipping-code path byte-for-byte to the
pre-spike tree and removed the transient prototype tests. Only the durable
evidence and a behavior-neutral literal ordinary graph-response regression
remain. No public API, schema, binding, persistence, migration, or release-state
decision was made. D26-01 remains open.

Final-tree verification:

- focused graph/evidence/reader/erasure tests: 68 passed serially;
- canonical `agent-verify`: 109 of 110 suites passed, one intended skip;
- `cargo clippy --workspace --all-targets -- -D warnings`: passed;
- `cargo check --workspace --all-targets`: passed.

The canonical verifier used a disposable worktree virtual environment containing
the exact-source test-hooks wheel and pinned dev tools. `PYTHONPATH` named the
worktree source so Python child processes imported the repo-only `eval` package.
Earlier setup attempts are retained in diagnostics: one lacked the worktree
tools/native module, and one used a broken relative native-module link. The
final unchanged canonical gate passed with the verified absolute native-module
target. The disposable virtual environment and links were removed afterward.

Next step: independent review of the committed chronology and evidence, then
HITL consideration of D26-01.
