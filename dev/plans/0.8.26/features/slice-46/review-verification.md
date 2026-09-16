---
title: FathomDB 0.8.26 Slice 46 — independent verification
status: PASS
verified_tip: 801586b2
---

# Slice 46 independent verification

An independent read-only verification agent tested committed tip `801586b2`.

## Results

- Initial and post-cleanup Git status: clean.
- Lifecycle checker: 186 tracked Markdown documents, exactly cataloged once.
- Classes: maintained 25, reference 26, experiment 15, historical 93,
  proposal 21, deferred 2, superseded 4.
- Focused lifecycle fixtures, Python compilation, and touched-shell syntax:
  PASS.
- Active local/docs-only CI wiring, Markdown gate, strict MkDocs build, diff
  check, and no-deletion preservation check: PASS.
- Unchanged unconfined `./scripts/agent-verify.sh`: exit 0; lint and typecheck
  PASS; security 0 violations, 0 blockers, 0 downgrades; tests **112/112 suites
  passed (skipped=0, excluded=0)**.

The first sandboxed canonical run failed only at the documented AC-036 ptrace
boundary:

```text
strace: test_ptrace_get_syscall_info: PTRACE_TRACEME: Operation not permitted
strace: ptrace(PTRACE_TRACEME, ...): Operation not permitted
strace: PTRACE_SETOPTIONS: Operation not permitted
strace: cleanup: waitpid(-1, __WALL): No child processes
AC-036 BLOCKER: strace ptrace access was denied; rerun from a ptrace-capable unconfined executor. Do not disable AC-036.
```

The unchanged unconfined rerun passed AC-036 and AC-037 strictly. The canonical
test run created two untracked Slice 55 malformed-context fixtures; the primary
agent verified their exact paths and removed them. They were generated test
artifacts, not user data, and are not recoverable or needed. Ignored `site/` and
`scripts/__pycache__/` outputs were refreshed; the Rust spill log is
`/tmp/fathomdb-agent-test-rust-1773139.log`.

## Verdict

**PASS.** AC26-46A through AC26-46G are independently verified at `801586b2`.
