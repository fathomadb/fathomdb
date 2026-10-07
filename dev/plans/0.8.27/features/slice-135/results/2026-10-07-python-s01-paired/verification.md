---
title: Slice 135 installed Python S01 paired result verification
status: FULL_GATE_HAS_TWO_OPEN_FAILURES
target_release: 0.8.27
---

# Paired S01 verification and gate disposition

Source commit: `2fbc268bd` on `llm/0.8.27-slice-135`. The complete gate ran
from a clean checkout with compact Rust build settings:

```sh
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 \
  CARGO_PROFILE_DEV_DEBUG=0 ./scripts/agent-verify.sh
```

Lint, type checking and the security checks passed. The test loop registered
185 suites, ran 183, passed 181, failed 2 and skipped 2. Rust passed. The
new `test-slice135-harness` ran in the standard fast tier and passed; the
focused direct suite also passed with 42 tests and 3 subtests. TypeScript and
the executable N-API hermetic suite were skipped because this checkout has
no `src/ts/node_modules` yet. **The full gate is red**; these scoped passes are
not a full-workspace green claim.

| Failed suite | Exact observation | Disposition |
| --- | --- | --- |
| `test-steward-orient` | The real-checkout briefing was 4,353 bytes against an arm that still asserts a 4,096-byte cap; the same suite reports that it is within its 5,120-byte cap. See the [retained log](full-gate-briefing-failure.log). | Existing gate inconsistency, not changed by the S01 measurement scripts. Resolve before a full green release claim. |
| `test-python` | 1,582 passed, 30 skipped, 3 failed. One test pins 1,131 declarations while current source yields 1,132; the two other failures are `ModuleNotFoundError: eval` in child processes. See the [retained log](full-gate-python-failure.log). | The [previous exact declaration diff](../2026-10-07-python-suite-clean/slice135-python-declaration-diff.json) shows two changed `search_frozen` signatures and one added error alias against the older Slice 130 snapshot. The two child-import cases pass when rerun with `PYTHONPATH=src/python` from this checkout (`2 passed`). The declaration pin remains open; no golden test was rewritten to obtain a green result. |

The independent S01 [audit](independent-audit.json) and
[hash manifest](SHA256SUMS) passed again after the commit. They prove the
twenty retained blocks and paired calculations at this workload boundary,
not the full Phase 1 functional or release gate.
