---
title: FathomDB 0.8.27 Slice 130 - Python SDK decomposition status
status: COMPLETE_ON_RELEASE_BRANCH
target_release: 0.8.27
source_sha: 58b0bd192f6bc7cc87ea6e8099bfe1c52e68893f
---

# Slice 130 Python SDK decomposition status

Slice 130 is complete on local `release/0.8.27` at `58b0bd192`. It started
from `3b809b75a` in a temporary worktree. The [plan](plan.md) reconciles the
release draft with Slices 30, 90, 100, 120, the current Python facade, and
later slice allocations. The [design](design.md) passed initial independent
`gpt-6.1-sol` high review and subsequent `gpt-6-sol` high review after two P2
planning fixes. No public interface, native stub, PyO3 source, schema, or
package export changed.

## Result and acceptance

| Acceptance | Evidence |
| --- | --- |
| AC27-130A | `engine.py` is 575 lines, with one `Engine` class retaining the exact prior 40 public method signatures and docstrings. Seven private modules own the substantial write, search, graph, evidence, projection, open, and instrumentation bodies. |
| AC27-130B | The exact pre-move and final captures match all 119 package exports and 1,131 resolved Python wrapper declarations; the native stub SHA-256 remains `a040f7e2ec067fea102433999164e7f35ad95bf523aed132abee4c4ff21afbcd`. A candidate-source Python run passed 1,582 tests with 27 documented skips. |
| AC27-130C | The private owner map parallels the TypeScript SDK by concern, while Python's existing public `read`, `graph`, and `admin` modules and root standalone embedding exports remain in place. `graph.expand` now calls its graph owner directly, closing the former runtime import back into `engine.py`. |
| AC27-130D | A test was committed at `31c3a7711` before the source move. The boundary guard rejected a temporary `Engine.search` signature defect, then passed after restoration. A standalone graph test exposed and repaired a fake-module import-order false pass. Independent `gpt-6-sol` high code review and Terra verification approved `58b0bd192`. The required repository gate was run; its two environment-provenance failures passed after checkout-owned environment repair, as detailed below. |

The source move committed at `3994bcff8`; the review-driven test fixes at
`58b0bd192` made the 1,131-row comparison executable and replaced a stale
private mapper monkeypatch with a real public `graph.expand` fixture. A
temporary change of `Engine.search`'s `limit` default from 10 to 11 produced
RED at comparator row 177; restoring the default returned GREEN. The graph
file failed alone before its fixture repair and passed alone afterward.

## Verification and limits

On clean `58b0bd192`, Ruff passed, Pyright reported zero errors or warnings,
and the full Python suite using candidate Python sources and the qualified
native binary passed 1,581 tests with 27 skips before the review-driven test
change. The final checkout-owned environment run passed 1,582 with 27 skips;
the candidate native test-hooks receipt passed with native SHA-256
`db2f1613947b6e8ab422518e2d02cf25bab5b6cbe9bc1e8946aa7e171167b003`.
The wheel was built from `58b0bd192` and installed non-editably only in that
worktree's virtual environment. Candidate package, `Engine`, and native module
paths resolved inside the slice worktree. No shared release environment was
modified.

The strict unconfined `./scripts/agent-verify.sh` run passed lint, typecheck,
AC-036/AC-037 security (zero violations, blockers, or downgrades), and Rust.
Of 184 registered suites, 180 passed, two were skipped, and two Python suites
failed solely because the initial `.venv` symlink resolved outside the slice
worktree. After replacing that ignored symlink with a checkout-owned
environment, both failed legs passed: `test-python` (1,582 passed, 27 skipped)
and `test-python-native-receipt` (exact candidate and native hash). Source did
not change between the broad run and these focused reruns. This is a staged
gate closure, not a claim that one uninterrupted `agent-verify` invocation
passed all 184 suites.

Terra independently confirmed candidate-source provenance, exact 119/1,131
surface equality, the unchanged native stub, standalone graph and boundary
tests, focused functional checks, SDK parity, and comparator checks. The final
Sol code review found no remaining actionable issue. The hidden Rust surface
was not recaptured because no Rust source, manifest, or tooling changed; the
only test-inventory addition is the Slice 130 boundary fixture. Slice 150
retains integrated release qualification and publication remains unapproved.
