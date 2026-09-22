# Slice 40 TDD chronology

## Characterization RED and GREEN

The existing error-taxonomy characterization asserted that both public error
enums had exactly four variants even though each had five. Commit `caffbca6`
replaced that stale count with exhaustive matches over every variant. Before
the production move, a temporary `CorruptionKind::MutationProbe` variant made
the focused test fail to compile with Rust error E0004 (non-exhaustive match).
The mutant was reverted and the corrected characterization passed. No product
behavior changed in this step.

Existing identity, temporal, and hook suites already exercised their assigned
contracts, so the plan rejected duplicate layout tests. Those tests were run
before and after their owning moves. A parallel `pr_g0_identity` run once hit
the suite's process-global runtime configuration guard; the unchanged target
passed with `--test-threads=1` and was treated as fixture scheduling, not a
product failure.

## GREEN batch 1: errors

Commit `7fd72c84` moved the shared error taxonomy and carriers into private
`errors.rs`, retained root re-exports, and left runtime configuration ownership
at root. Error taxonomy, lifecycle observability, facade re-export/no-recovery,
and operator/test-hooks compile checks passed.

The clean candidate `7fd72c843c815b2e9191d360e0cc50d6c298789a`
compared equal to the immutable Slice 30 baseline: empty metadata and row
diffs. Its disposable capture SHA-256 was
`a4bdba710147a2d7fd9c9375ca662333adcdfe4929cd4ccf27a329f1c9823a0d`.

## GREEN batch 2: identity

Commit `40b8179e` moved shared ID spaces, source/revision/dependency IDs,
canonical hash, caller-ID grammar, and stable/logical derivation into private
`identity.rs`. Identity swap, source writes, revision grammar, provenance,
dependency, registration-inertness, serial G0 identity, facade, and feature
compile checks passed.

The clean candidate `40b8179e7833aa82433f971e45d98c33beebf6d8`
compared equal with empty metadata and row diffs. Its disposable capture
SHA-256 was
`c19818e97151408a25c5faf71169942b0bf136cf88189920cea0ad7f94d4d35c`.

## GREEN batch 3: temporal

Commit `a2bfd0dd` moved read views, frozen-view support, clock sampling,
temporal SQL, strict timestamp parsing/rendering, and normalization into
private `temporal.rs`. The first compile after relocation was intentionally
recorded RED: 16 E0624 diagnostics showed that `FrozenView::edge_now` needed
crate-parent access. Narrowing it to `pub(crate)` made the existing read-view,
node/search validity, graph epoch, ISO-8601, timestamp round-trip, graph-arm,
frozen-read, facade, and feature checks GREEN without semantic changes.

The clean candidate `a2bfd0dd1ebebb8f91f864bfd017a2566e286ea7`
compared equal with empty metadata and row diffs. Its disposable capture
SHA-256 was
`5013a46604afa523c74bf2a8b78a54ec51fa8030e7e04da2547e02e25649979a`.

## GREEN batch 4: test hooks

Commit `f6d24719` moved the existing hook implementations and debug projection
pause into private `test_hooks.rs` while preserving every root name and cfg
gate. The default hook targets passed; the 23-test evidence target was rerun
serially because its process-global one-shot hook can interfere when the whole
target runs in parallel. Feature-gated graph evidence, explanation, Slice 60,
projection completion/generation, dependency closure, and Slice 72 rendezvous
tests passed under their owning features. Default-surface and combined-feature
compile checks passed.

The clean candidate `f6d247190ef059e38d763370d59b01eadf3e536c`
compared equal with empty metadata and row diffs. Its disposable capture
SHA-256 was
`fc3f90bb10e0d7f550d357b46fe8c918fc5f1a792d015f6f48c9566deff8affb`.

All four comparisons used the tracked Slice 30 baseline captured from
`def7d894d6439c4dd223d972963613c097d27eea`; none regenerated or modified it.
