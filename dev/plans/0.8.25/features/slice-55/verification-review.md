---
title: 0.8.25 Slice 55 independent verification
status: FAIL
verified_candidate: b9d96575
updated: 2026-09-06
---

# Slice 55 independent verification

Independent verification cannot close candidate
`b9d965756909c6a7bedad02fde5c8b58e257ec8a`. The reviewed implementation is
green on Linux and in focused Windows Rust tests, but a fresh Windows wheel
reproducibly retains a corrupt database file after explicit `Engine.close()`.

The exact source archive SHA-256 was
`6a605d11160186e8432a68cf0f4a4c75c038fb96778d5c308f280a352705d415`.
Linux verification ran on x86_64 `windchill3`, kernel `7.0.0-30-generic`,
Rust 1.95.0, Python 3.12.3, and Node 25.9.0. Windows verification ran on the
Windows 11 VM `MEMEX-AFE8V4I00` with PowerShell 5.1.26100.9168, Rust 1.97.1,
Python 3.12.10, uv 0.12.5, and maturin 1.13.1.

## Blocking Windows installed-wheel reproduction

A fresh wheel from the exact candidate was installed into a new isolated
virtual environment without `PYTHONPATH`. The unchanged
`src/python/tests/smoke_slice55_installed.py` completed its Slice 55
trace, explanation, and typed-corruption assertions. After the corrupt trace
path and explicit `Engine.close()`, `TemporaryDirectory` cleanup could not
unlink `corrupt.fathom`:

```text
PermissionError: [WinError 32] The process cannot access the file because
it is being used by another process: '...\corrupt.fathom'
```

The exact test was repeated against a new temporary directory and failed the
same way. The durable raw traces are
`verification-windows-wheel-failure.log` and
`verification-windows-wheel-failure-retry.log`. The Windows wheel SHA-256 was
`aafc7c3c21f16e1026b40149c8486af7091308d2b40119e926e42510f8fbe57c`;
the installed `_fathomdb.pyd` was 15,382,528 bytes.

## Passed functional and artifact evidence

- Linux focused Rust: dependency trace 18, integrity 66, explanation 17,
  legacy 6, wire 10, facade 2, and CLI 3 passed.
- The 50,000-row release fixture passed at 3,200,000 VM steps, 53 ms, and
  zero reported peak-RSS delta.
- Fresh Linux wheel smoke and 49 focused Python tests passed; installed
  predecessor Slice regressions passed 309 tests with 2 documented skips.
- The Linux wheel SHA-256 was
  `b4e3e6d780949f233163a3263dade01ee34734734976b64443817662f8f90f15`.
- Exact-source N-API smoke passed. Its `.node` SHA-256 was
  `9345716ec6def0d933c1d22db5bd3a51d715b32c2ed2d6c6965cb71776069267`.
- Windows focused Rust passed dependency trace 18, integrity 66, explanation
  17, wire 10, facade 2, and CLI 3 tests.
- Ruff, canonical project Pyright, package-local TypeScript, workspace Clippy,
  workspace check, and the selected-feature serial engine route passed.

## Repository gates and classified failures

- The unchanged unconfined `./scripts/agent-verify.sh --tier=fast` passed
  103/103 suites with no skips or exclusions after the sandbox-only ptrace
  denial.
- An exact disposable candidate clone passed
  `./scripts/agent-verify.sh --tier=heavy` at 3/3 suites and
  `./scripts/agent-verify.sh --tier=all` at 106/106 suites, with no skips or
  exclusions. The durable checkout's heavy Python collection failure was the
  known stale-worktree-native trap, not candidate behavior.
- A parallel selected-feature engine run exposed the documented
  scheduling-sensitive WAL-idle oracle. Its unchanged focused control and the
  complete serial selected-feature route passed. No old WAL deadlock or
  watchdog timeout reproduced.
- Windows Node/N-API could not run because the VM has no Node runtime. This is
  informational here and remains a required Slice 75 CI cell.

## Verification-plan defects

The verifier also found three plan defects that must be corrected before the
slice can close:

1. The path-list Pyright command at `plan.md` line 127 does not select
   `src/python/pyproject.toml`; `.venv/bin/pyright -p src/python` passes.
2. The monolithic Linux `--all-features` commands at lines 176–178 combine
   CUDA with Apple-only Metal, contradicting Slice 40's explicit platform
   separation. The valid default-workspace and selected-feature routes pass.
3. The package smokes at lines 193–194 are registry/semver commands rather
   than local-artifact commands. As written, they conflict with the owner's
   hard no-registry boundary.

The candidate remained unchanged and tracked-clean. Verifier-owned temporary
artifacts and Windows staging were removed. No package was staged, uploaded,
tagged, or published.
