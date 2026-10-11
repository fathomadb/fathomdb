---
title: FathomDB 0.8.27 Slice 40 - independent verification
status: PASS
target_release: 0.8.27
candidate: fdd7fb646b0fb922b9b8fea134ef7ce7e71a5aae
---

# Slice 40 independent verification

The independent read-only verifier returned **PASS** at clean candidate
`fdd7fb646b0fb922b9b8fea134ef7ce7e71a5aae`.

| Gate | Result |
| --- | --- |
| Fresh Slice 30 surface comparison | 13 rows; equal; empty metadata and row diffs |
| Locked correction-safe erasure | 5 passed, 0 failed |
| `scripts/agent-verify.sh` | 124/124 suites passed; 0 failed, skipped, or excluded |
| Security summary | 0 violations, 0 blockers, 0 downgrades |
| Python candidate receipt | PASS; exact candidate SHA and native module hash verified |
| Workspace Clippy, warnings denied | PASS |
| Workspace Cargo check | PASS |

The fresh surface capture SHA-256 was
`832bc78957c20bf8efd36ec97f6f91bbff098a821b3525bfae67bb2179ef6473`.
It compared against baseline source
`def7d894d6439c4dd223d972963613c097d27eea` with candidate source
`fdd7fb646b0fb922b9b8fea134ef7ce7e71a5aae`.

The verified Python native receipt recorded module SHA-256
`e3aa511109eb562342e970492f5daa01193130c1e000c704d74e6d9d1d58dfd2`.
The sandboxed security run reached the expected AC-036 ptrace denial; the
unchanged strict gate passed unconfined, as required by repository policy.

Final tracked Git status was clean and HEAD remained the exact candidate. The
temporary worktree-owned Python environment, candidate receipt, generated
Python/Node native artifacts, comparator cache, and disposable surface
captures were removed after verification.
