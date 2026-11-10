# Slice 7 proposed verification mapping

Existing files below were located/source-inspected, not executed as product gates.
Read each complete fixture before implementing a behavior change.

| Proposed change / retained obligation | Existing control or required delta |
| --- | --- |
| Owned tools/venv/pins | `scripts/tests/test_dev_environment_tools.sh`, `test_bootstrap_heavy.sh`, `test_agent_typecheck_pyright_version.sh`, `test_agent_lint_ruff_version.sh`; add linked-worktree owned-wheel behavior controls if scope is approved. |
| Full versus heavy verifier | `scripts/agent-verify.sh`, `scripts/tests/test_verify_release_gates.sh`; preserve lint/typecheck/security ordering and truthful preflight failure. |
| W/E and publishing topology | `scripts/tests/test_set_version.sh`, `test_release_version_surfaces.sh`, `test_runtime_release_alignment.sh`; use set-version W operation, retain independent E. |
| Exact first-tag CUDA route | `scripts/tests/test_cuda_release_contract.sh`, `test_release_workflow_scope.sh`; RED controls for wrong tag, API/network failure, missing allowed workflow and empty reviewers before a narrow approved amendment. |
| Package/registry behavior | `scripts/tests/test_wait_for_crate_version.sh`, `test_verify_release_python_wheel.sh`, `test_linux_aarch64_release_artifacts.sh`, existing registry smoke scripts; final real registry checks follow publication. |
| Gitleaks policy/classification | `scripts/tests/test_gitleaks_guards.sh`, `test_gitleaks_history_environment.sh`, `test_gitleaks_history_classifier.py`; retain scanner-failure versus findings distinction and redaction. |
| Planning documents | Scoped Markdown/local links and existing planning/architecture/state validators; missing tooling remains a setup failure, not a pass. |

Full source/executable phases retain per-behavior observed RED before GREEN,
independent review and one final clean-tree full gate. Run affected/scoped checks
while iterating. Ptrace denial requires unchanged strict execution on a capable
executor. No numerical target, broad history rescan or full .28 matrix is added.
Independent review and Slice 9 acceptance are pending.
