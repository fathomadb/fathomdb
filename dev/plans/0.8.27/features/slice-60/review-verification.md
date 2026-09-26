---
title: FathomDB 0.8.27 Slice 60 - independent verification
status: PASS
target_release: 0.8.27
candidate: 5627c78b185d8fae9febe9f642a3b729363c75ea
---

# Slice 60 independent verification

The independent read-only verifier (Sonnet) returned **PASS** at clean
candidate `5627c78b185d8fae9febe9f642a3b729363c75ea`.

| Gate | Result |
| --- | --- |
| Write-boundary atomicity suite | 11 passed |
| Virtual-mutation manifest | 1 passed |
| Focused owners (serial) | 255 default, 2 `operator`, 37 `test-hooks`, 78 lib; all equal to the chronology |
| Slice 30 public surface | Equal; 13 rows; empty metadata and row diffs |
| Hidden structural surface | No removals or changes. Additive only: 10 approved Slice 50 tests, 11 new boundary tests, 1 new test target |
| Fast-tier target coverage | 261 targets; 53 feature-complete-only |
| `scripts/agent-verify.sh` | Lint and typecheck pass; 127/127 suites passed; 0 skipped or excluded |
| Security | 0 violations; AC-036, AC-038, AC-050a, AC-050c pass; AC-037 live layer **unavailable** (below) |
| Workspace Clippy, warnings denied | PASS |
| Workspace Cargo check | PASS |

The public capture SHA-256 was
`e7cecbd4f8174126bd1ce265adc0d9276694053f6d6edf114ed38925b186bfca`. The
hidden capture SHA-256 was
`a0b351dab23ff4a6283928ab8fc03cf1644e67c08cdaf3ea3f0d510168182e82`. The
candidate-bound Python native module SHA-256 was
`b349d7b146a7e349bdb887bb992dce9f4889fcab79d93ad4845eeb5cc9090b49`, identical
to the implementer's rebuild. No baseline was regenerated.

## Unavailable evidence

AC-037's live network-namespace layer could not run on this host, including
with the tool sandbox disabled. A direct `unshare -rUn true` fails with
`write failed /proc/self/uid_map: Operation not permitted`, because the host
sets `apparmor_restrict_unprivileged_userns=1`. The canonical gate classifies
this as an environmental downgrade (0/0/1). A plain `STRICT=1` rerun
classifies the same fact as one blocker. It is recorded as unavailable
evidence, not as a pass. AC-037's offline catch and policy self-test layers
passed. Slice 150 qualification must re-run the live layer on a host that
permits unprivileged user namespaces.

## Environment notes

The first `agent-verify` attempt ran without the documented disposable
`.venv`, so its environment was invalid. It was stopped and superseded. The
second attempt used a non-editable `[dev]` install plus a `.pth` entry naming
`src/python`, and passed. All environments, captures, caches, the in-place
native build byproduct, and receipts were removed. The tracked tree was clean
at the unchanged candidate.

## Post-closeout adversarial verification

The 2026-09-25 adversarial review supersedes only the boundary-suite and
candidate-specific portions of the original receipt. Its implementation
candidate was clean commit `100fa230`.

| Gate | Result |
| --- | --- |
| Write-boundary atomicity, debug `test-hooks` | 13/13 passed |
| Write-boundary atomicity, release `test-hooks` | 12/12 passed; the debug-only pre-transaction-hook case is absent by design |
| Commit-exit non-vacuity mutants | Each targeted mutant failed only its corresponding full-snapshot assertion; restored cases passed |
| Rustdoc broken-link baseline | 58 baseline, 58 candidate; sorted warning-message multisets identical |
| Formatting and markdown lint | PASS |
| Workspace Clippy, warnings denied | PASS |
| Workspace Cargo all-target check | PASS |
| Engine feature profiles | Default, `operator`, `test-hooks`, `slice72-test-hooks`, `migration-test-hooks`, and `tc5-benchmark` all passed |
| Strict security, outside the restricted tool sandbox | 0 violations, 0 blockers, 0 downgrades |
| `scripts/agent-verify.sh --tier=fast` | 122/122 suites passed; 0 skipped, 0 excluded |

The first security invocation was intentionally run in the restricted tool
sandbox and could not exercise AC-036 because ptrace was denied there. The
unchanged canonical security gate was rerun outside that sandbox and passed;
the aggregate verifier then completed in the same environment. This was an
execution-environment limitation, not a product failure or waived gate.
