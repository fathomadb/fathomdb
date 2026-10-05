---
title: FathomDB 0.8.27 Slice 114 — configuration audit design
status: APPROVED
target_release: 0.8.27
source_baseline: eefc2e3d8f9b446750f142180f1bf63742d7b6b4
---

# Slice 114 — configuration audit design

## Boundary and existing substrate

This slice traces each effective setting from the caller through validation to
its consuming engine owner and observable result. It reconciles source,
accepted decisions, interfaces, and public documentation. It does not add a
setting, change a default, or tune performance. `EngineConfig` and
`ResolvedRuntimeConfiguration` already implement five per-engine open values
in `runtime_configuration.rs`. The process-wide SQLite mode is a separate
startup choice governed by
`dev/adr/ADR-0.8.25-sqlite-runtime-configuration.md`. The Slice 90 runtime
topology and its qualification receipt already own scheduler and embedder
worker semantics. Slice 115 owns measurement, and Slice 120/132/140 own their
allocated SDK and test-seam work.

## Census and disposition

The audit artifact has one row per production engine module-level `const` or
`static`, plus separately enumerated public inputs, environment reads,
setters, operational limits, and intentionally suppressed unused bindings.
Each declaration row identifies source location, owner, references, category
(setting, internal limit, SQL/schema/protocol invariant, test-only), and
disposition. A declaration inside a `#[cfg(test)]` module or function is
explicitly excluded from the production census. A production declaration
used only with a feature gets its feature named. Compiler warnings in default
and relevant feature builds and reverse searches from docs/interfaces check
the census for missing or dead entries. SQL text and schema versions are
classified rather than presented as potential user controls.

For public settings, a companion matrix records all three language spellings,
unit, default, range, omission and zero semantics, effective precedence,
mutability, consumer, invalid outcome, and fallback behavior. The accepted
numeric values remain those implemented by `ResolvedRuntimeConfiguration`.
Operational limits without a justified user range remain internal. A real
contract mismatch receives a focused test and correction; a proposed tuning
change becomes a named Slice 115 or later finding.

## Known drift and testable correction

`docs/reference/config.md` currently says any post-open SQLite mode request
raises. In code, `configure_runtime_locked` returns the effective mode when
the requested mode equals it, including after Engine open. The accepted
SQLite ADR also says repeating the effective choice succeeds. The existing
subprocess test proves pre-open repetition only. Add a post-open subprocess
scenario that opens a real temporary Engine, repeats the effective choice,
asserts the returned mode, and confirms a conflicting choice still reports
`Conflict`. The existing documentation claim is the failing witness against
that observed behavior; do not encode its incorrect expectation in a test.
Keep the contract-correct test unchanged after adding it, amend the public
guide and Rust API comment, and rerun the focused selector. This is a
correction to documentation and test coverage, with no runtime behavior
change. A later code defect, if found, uses a failing contract-correct test
first and fixes production code to turn it green.

The census may reveal other mismatches. For each one, decide from the actual
consumer and accepted contract: retain and explain; remove only with a RED
behavior witness followed by GREEN; correct stale prose; or hand off a
nonessential improvement to its named owner. The audit does not remove a
declaration merely because its name looks unused in a text search.

## Verification and handoff

The implementation order is discrepancy witness, minimal correction, affected-path
checks, independent code review (`gpt-6-sol`, high effort), and independent
Terra verification. Markdown, docs, and release-state validators apply to
their respective changed files. The Slice 114 status records exact candidate,
row dispositions, tests, review findings, and any limits. It hands concrete
measurement questions to Slice 115 without asserting unmeasured performance.
