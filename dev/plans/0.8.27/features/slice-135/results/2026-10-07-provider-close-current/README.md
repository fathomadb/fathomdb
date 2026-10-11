# Slice 135 provider failure and close boundary — 2026-10-07 UTC

**Status:** focused current-product robustness result. The two existing
real-database test targets passed 14/14 cases against source
`87d8659f18260a19b9f2932f412a42f50f71a692`, whose Rust crate tree is
`a25573dbe1efffcda1c907dff88e478a87645719`. The [identity](identity.json)
records the exact test binary hashes, Cargo lock blob and direct invocation.
These are robustness tests, not latency observations or a full test gate.

## Execution and oracles

`cargo test --offline --locked -p fathomdb-engine --test slice90_projection_capacity
-- --nocapture --test-threads=1` and the equivalent
`slice90_foreground_dispatch` target ran successfully. Their exact binaries
were then run directly under GNU Time with the same test options. The retained
[projection output](projection_capacity.stdout), [foreground output](foreground_dispatch.stdout),
[expected panic output](foreground_dispatch.stderr), [resource files](projection_capacity.resource)
and [independent log summary](summary.json) show 4 and 10 passing tests, zero
child swaps, and three expected injected-provider panic messages. Both target
processes exited zero. [SHA256SUMS](SHA256SUMS) binds these files.

The projection cases use temporary real SQLite databases. A timed-out provider
retains its physical slot, late output cannot commit, pending work survives,
and a released slot permits both rows to project. A close during a held
provider call stops admission and waits for the provider; reopen drains the
pending row with no recorded projection failure. A provider retry honors its
absolute delay amid wakeups; close interrupts a long retry wait, no retry starts
after close, and reopen recovers the row. The foreground cases test bounded
started-call timeout, queued saturation, close cancellation, sparse fallback
after timeout/error, snapshot retention while the provider waits, and
vector-only results under eight concurrent readers.

Three tests intentionally inject a provider panic and assert the current Rust
caller-boundary behavior or worker reuse. The panic text in stderr is expected
for those tests; it is not an unhandled test failure. This receipt does not
establish that a panic cannot cross Python or TypeScript FFI boundaries.
The test sources contain the detailed deadlines, state and result assertions;
this receipt did not add a new assertion or fault point.

## Limits

These cases cover the named provider and close boundaries at this exact
product tree. They do not prove every one-shot/persistent fault position,
process kill during commit, power-loss behavior, or FFI panic containment.
The full Phase 1 robustness matrix remains open.
