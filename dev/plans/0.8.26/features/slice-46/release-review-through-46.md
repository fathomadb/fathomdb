---
title: FathomDB 0.8.26 — release review through Slice 46
status: PASS
reviewed_tip: 6d952c9c7616bbd885ff8f1355d153d97d98ee31
---

# Release review through Slice 46

An independent read-only GPT-6 Astra medium review covered the complete
`40f807f5..a01ce852` release diff, requirements and acceptance, architecture,
maintained design, implementation, tests, and closure evidence through Slice
46. The initial verdict found one P1, two P2 findings, and one P3 finding.

## Findings and remediation

| Severity | Finding | RED | GREEN | Final disposition |
| --- | --- | --- | --- | --- |
| P1 | Graph evidence could disclose a lifecycle-revoked target, source, or terminal edge under a newly authenticated equivalent frozen context. | `cf03b120` | `efaab6b5` | PASS: intrinsic source/artifact/edge authorization now precedes provenance and byte detail. |
| P2 | Serving opens and operator inspection derived different product-lock namespaces for symlink aliases. | `cf03b120` | `efaab6b5` | PASS: existing database files resolve their final component before lock and sidecar derivation; missing-path bootstrap remains supported and dangling symlinks fail closed. |
| P2 | Production receipt validation retained upgrade-era missing-generation acceptance, and populated current stores could bootstrap missing generation authority. | `cf03b120` | `efaab6b5` | PASS: pending cursors require the stored generation and populated current stores missing authority fail closed; migration-test admission remains isolated. |
| P3 | Python accepted boolean and integral-float graph-evidence schema versions. | `cf03b120` | `efaab6b5` | PASS: root and dependency decoders require exact integer version 1. |
| P3 | Graph point resolution did not independently reject temporal-fallback edges before provenance detail. | `c85567ea` | `a1db1881` | PASS: graph-specific intrinsic authorization rejects fallback edges while ranked relaxed-window behavior remains unchanged. |
| P3 | The first lifecycle regression combined target and source revocation. | `c85567ea` | existing GREEN plus `a1db1881` verification | PASS: distinct derived-target and source-only pending/deleted cases now protect both authorization axes. |
| P2 integration regression | Five historical projection fixtures deleted both generation-authority tables from populated current schema-34 databases and expected public reopen to upgrade them. | unchanged full workspace gate after `a1db1881` | `1cdea008` | PASS: substantive fixture oracles remain, generation authority stays coherent, and one narrow debug-only hook performs the inert registry mutation plus generation transition atomically. Production fail-closed behavior is unchanged. |
| P2 cross-SDK integration regression | TypeScript and Python parity fixtures retained the same raw authority deletion and registry mutation. | capable full gate after `1cdea008` | `902cc1d6` | PASS: both use narrowly gated internal bindings to the atomic Rust injector, assert configuration-generation advance and reopen stability, and preserve readback/reapply/repair oracles. |
| P3 feature matrix | The new N-API hook compiled under release-profile tests without the engine helper. | Astra release-profile compile reproduction | `6d952c9c` | PASS: the separate N-API implementation now aligns test-hook and debug-assertion gates; ordinary release artifacts omit it. |

No public API, schema, package, publication, or compatibility layer was added.
The maintained retrieval, recovery, actuation, projections, and bindings
designs were updated only where the corrected implementation changed their
as-built truth.

## Independent rereview

The same independent reviewer inspected both RED/GREEN pairs, verified that
GREEN did not weaken the test oracles, and ran the focused current-tip suites.
The final implementation verdict at `6d952c9c` was PASS with no remaining
P1–P4 findings. The full workspace gate, rather than the focused review, found
the stale Rust and TypeScript fixture setups; the implementation agent stopped
instead of weakening production, and independent Astra adjudication approved
each bounded fixture correction before implementation and rereview.

- graph evidence: 34/34 PASS;
- ranked evidence, serialized: 23/23 PASS;
- mutation/projection, fresh-cutover, WAL, operator-lock, compatibility, and
  Python malformed-carrier focused suites: PASS across the remediation cycle;
- architecture authority and design lifecycle: PASS, 186 documents; and
- Git diff/whitespace and worktree cleanliness: PASS.
- fresh non-editable Python `test-hooks` wheel: corrected projection-registry
  parity case PASS with repository source injection disabled.

The historical RED commits were inspected as test-only commits but were not
independently re-executed in a second checkout. The implementation run recorded
the intended failures before each GREEN commit.

## Gate boundary

The unchanged sandboxed `agent-verify` passed lint and typecheck, then stopped
at the documented AC-036 `PTRACE_TRACEME` denial. Focused product suites pass.
The unchanged capable-executor gate is the final closure witness recorded after
`6d952c9c`: strict security reported 0 violations, 0 blockers, and 0
downgrades, and `agent-test.sh` passed 112/112 suites with zero skipped or
excluded. Slice 50 still owns exact-candidate integrated package and platform
verification. This record establishes correctness and completeness through
Slice 46, not publication readiness.
