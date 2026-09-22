---
title: FathomDB 0.8.27 Slice 20 - independent verification
status: PASS
target_release: 0.8.27
candidate: 3943cb64dc2d1b99ef9fc4ec2131dca59a71b337
---

# Slice 20 independent verification

The independent read-only verifier returned PASS at exact candidate
`3943cb64dc2d1b99ef9fc4ec2131dca59a71b337`.

| Gate | Result |
| --- | --- |
| Focused Rust correction + blast radius | 52 passed, 0 failed, 0 ignored |
| Isolated Python candidate wheel | 8 passed |
| TypeScript native debug binding | 8 passed, 0 skipped |
| Frozen Memex consumer oracle | 1 passed |
| `scripts/agent-verify.sh` | 119/119 suites passed, 0 skipped, 0 excluded |
| Security summary | 0 violations, 0 blockers, 0 downgrades |
| Workspace Clippy, warnings denied | PASS |
| Workspace Cargo check | PASS |

The default isolated Python wheel SHA-256 was
`beef52e1d4b2d6fe5e44be057c3fbd4b6fae43fe9f236ee70e2238c70c7ec405`.
The canonical gate used a worktree-owned non-editable test-hook wheel with
SHA-256
`aad6f119bd3031e098ab2b4dd8115b0d903fa4a8a11150168b649afd5976b1c5`.

The consumer gate ran the versioned
`dev/plans/0.8.27/features/slice-20/memex-consumer-oracle.py` from the Memex
`release-0.6.0` checkout with the isolated candidate wheel and Memex `src` and
`tests` on `PYTHONPATH`.

Earlier canonical-gate attempts failed only because the worktree lacked a
local Python environment, then because an external environment symlink
violated the repository layout check and omitted the source-only `eval`
helper. The unchanged gate passed after using the established real
worktree-local, non-editable candidate environment. No source or test was
changed to obtain the pass.

The temporary environment and generated TypeScript test artifacts were
removed. Final Git status was clean and HEAD remained the exact candidate.
