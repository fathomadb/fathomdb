---
title: Slice 135 installed TypeScript S01 paired result verification
status: FULL_GATE_HAS_TWO_OPEN_FAILURES
target_release: 0.8.27
---

# Paired TypeScript S01 verification and gate disposition

The full gate ran from clean source commit
`fc2829301cdd55670401b7ffc7b22cd056055d13` on
`llm/0.8.27-slice-135`, with pinned Node `v25.9.0` on `PATH`:

```sh
PATH=/home/coreyt/.local/actions-runner/fathomdb-gpu/_work/_tool/node/25.9.0/x64/bin:$PATH \
  CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 \
  CARGO_PROFILE_DEV_DEBUG=0 ./scripts/agent-verify.sh
```

Lint, type checking and security advanced into the test loop. The full
[gate output](full-gate.log) reports **186 suites registered, 186 run,
184 passed, two failed and zero skipped**. Rust passed. The normal fast tier
ran `test-slice135-harness` and the newly registered
`test-slice135-ts-s01-contracts`, both passed. With lockfile-pinned
`src/ts/node_modules` present, the TypeScript suite and executable N-API
hermetic suite both ran and passed. The TypeScript suite's own offline
setting still skips six network-dependent default-embedder tests. The full
gate remains **red**.

| Failed suite | Exact observation | Disposition |
| --- | --- | --- |
| `test-steward-orient` | A real-checkout briefing is 4,353 bytes; one assertion still requires 4,096 bytes while another in the same suite accepts the 5,120-byte cap. [Full log](full-gate-briefing-failure.log). | The same failure preceded these TypeScript S01 changes. Resolve the conflicting gate expectation or briefing before a full green claim. |
| `test-python` | 1,582 passed, 30 skipped, three failed. A Slice 130 package-boundary test expects 1,131 Python declarations but sees 1,132; two child processes cannot import `eval`. [Full log](full-gate-python-failure.log). | The declaration diff and the successful targeted child-import retry with `PYTHONPATH=src/python` were recorded in the prior [Python S01 verification](../2026-10-07-python-s01-paired/verification.md). The three failures remain open; no golden test was rewritten. |

Separately, 55 focused Slice 135 Python tests and three subtests passed;
four TypeScript S01 contract tests passed. The independent paired auditor
replayed byte-identically from retained receipts, and the hash manifest
passed. These scoped results prove the S01 evidence path, not the full Phase
1 functional exercise or a green release gate.
