---
title: FathomDB 0.8.27 Slice 60 - independent verification
status: PASS
target_release: 0.8.27
candidate: d5a5bd39b3ee8a04bd080df451204564c6849bd1
---

# Slice 60 independent verification

The original independent read-only verifier (Sonnet) returned **PASS** at clean
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

## Original unavailable evidence (2026-09-25)

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
| Strict security | The aggregate 0/0/0 claim conflicted with the 2026-09-25 AC-037 unavailable receipt; it is not accepted as final evidence and is superseded below |
| `scripts/agent-verify.sh --tier=fast` | 122/122 suites passed; 0 skipped, 0 excluded |

The first security invocation was intentionally run in the restricted tool
sandbox and could not exercise AC-036 because ptrace was denied there. The
unchanged canonical security gate was rerun outside that sandbox and passed;
the aggregate verifier then completed in the same environment. This was an
execution-environment limitation, not a product failure or waived gate. The
security aggregate did not preserve enough per-layer context to reconcile its
0/0/0 summary with the earlier AC-037 result, so the final correction below
reruns and names the exact environment instead of relying on that summary.

## Final correction verification (2026-09-26)

The final clean implementation candidate is
`d5a5bd39b3ee8a04bd080df451204564c6849bd1`. The owner follow-up RED/GREEN
commits are `81d723b1` and `d5a5bd39`.

| Gate | Result |
| --- | --- |
| Write-boundary atomicity, debug `test-hooks` | 15/15 passed |
| Write-boundary atomicity, release `test-hooks` | 14/14 passed; the debug-only pre-transaction-hook case is absent by design |
| Slice 30 public surface | Equal; 13 rows; empty metadata and row diffs |
| Hidden structural surface | All eight rustdoc rows equal; 41-item release probe equal |
| Hidden test inventory | 0 removals; 0 changes; reviewed additions only; both new carry-over tests appear in the three applicable `test-hooks` rows |
| Fast-tier target coverage | 261 targets; 53 feature-complete-only |
| `scripts/agent-verify.sh` | 127/127 suites passed; 0 skipped, 0 excluded |
| Security on 2026-09-26 unconfined executor | 0 violations; 0 blockers; 0 downgrades; AC-037 live netns layer ran and passed |
| Candidate-bound Python receipt | PASS; candidate `d5a5bd39`; native module SHA-256 `0d56d9c926a7def6f056041e1277a3951dc2f12e83a4f6170f3c6cbcec7391d4` |
| Workspace Clippy, warnings denied | PASS |
| Workspace Cargo all-target check | PASS |

The public capture SHA-256 was
`dba5b143b362f84887913ad9d51600c0c9d91614b98ef19531cfc8fc43e289ff`.
The hidden capture SHA-256 was
`132c5176e82e590a457716960345ab29a62afdfb2e90f41079ffa54798ec1895`.
The hidden comparator's only differences from
`baseline-8e2afb29.json` were additive test-target and test-inventory entries:
193 feature-row occurrences covering 26 unique added test paths accumulated
since that baseline. There were no removed or changed tests, and every hidden
structural or release-probe row was byte-equal. No baseline was regenerated.

The first final-gate attempts were rejected by disposable-environment
controls: a linked venv could not authorize a candidate rebuild, the first
worktree-owned venv lacked pinned console entries, the retained capture files
made Git dirty, and the non-editable package plus checkout-local `src/python`
path were initially absent. Each attempt stopped on that environment defect.
After the focused affected suite passed 24/24 and Git was clean, the unchanged
full verifier passed. No environment failure was reported as a product pass.

The 2026-09-26 security executor was Codex CLI session
`01a0db63-a1ab-78b3-8ece-1ba0ba36a0a3`, on host `windchill3` (the same host as
the 2026-09-25 run). It ran `./scripts/agent-verify.sh` from the release
worktree through `exec_command`, with `sandbox_permissions: require_escalated`,
which runs outside the Codex Linux sandbox. The final run was around
2026-09-26T14:00Z. Its output recorded `AC-037 catch OK (live netns)` as PASS
and a security summary of 0 violations, 0 blockers, and 0 downgrades. The
host's `kernel.apparmor_restrict_unprivileged_userns` stayed at `1`. The
2026-09-25 Claude Code runs, including the one with the Bash sandbox disabled,
could not create the unprivileged user namespace on this host. To reproduce
the live-layer pass, use the same Codex escalated-exec route.
