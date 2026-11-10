# Slice 10 proposed executable task order

All rows await final reviewed Slice9 acceptance; no setup/code change executed.

| Task | Inputs / concrete change | Verification and stop |
| --- | --- | --- |
| T10-1 resources/tools plan | Approved task-owned /mnt/data location, current capacity/access and command-local exact tool selection | Refuse ownership collision/missing capacity; preserve other sessions/data. No blind bootstrap. |
| T10-2 release activation | JSON single writer at dev/plans/release-state-0.8.29.json; declared board/master generated views; baseline current .29 and actual ladder/decisions | release-current identifies .29; check-release-state-views and strict preflight validate before dependent worktrees. No direct generated-block edits. |
| T10-3 owned setup behavior | Resolve linked-worktree editable conflict; install owned pinned tools/native noneditable wheel through reviewed path | Observed RED for missing/wrong-owned-env/tool or binding identity before behavior fix; scoped bootstrap/tool tests then relevant source gate. |
| T10-4 bounded lock updates | Current Cargo rustls.23.46/webpki closure and Mermaid DOMPurify3.4.17 | Locked feature/build closure, affected downloader/renderer checks; preserve SQLite/MCP/forks/model behavior. No whole stale lock. |
| T10-5 survey correction | Discover/parse exporter file, classify current119 recognized manifests and fixture/history exclusions | RED omitted exporter/untiered/current-checkout cases; then expected inventory with explicit tiers. No deletion to clear discovery. |
| T10-6 current-authority docs | DOC-INDEX .28 publication/no active state→activated .29; needs/requirements/test-plan format37 applicability; v2 workspace12 including MCP; query/Python README clauses; worktree setup wording | Verify actual source/ADRs/inbound consumers; preserve canonical IDs/history. Scoped Markdown, docs/generator checks only where applicable. Query scaffold stays. |
| T10-7 .29 first-tag route | Exact approved route/amendment under existing protections | RED wrong tag, API/network failure, absent allowed workflow/reviewer; actionlint plus existing route/scope tests; no dispatch or external ACL edit by inference. |
| T10-8 coordinated versions | Final acceptance confirms W0.8.29 via scripts/set-version.sh; E0.8.0 retained | Existing version/lock/runtime alignment checks; mechanical-only exception. Retain database format37 (owner seq-368); no migration task. Do not tag/publish. |
| T10-9 assembled preparation close | Reviewed accepted scope, owned tools, state/views, chosen dependency/docs/tool changes | Required clean-tree full gate once for final source phase, affected scoped/native checks and independent code review; classify any environment failure honestly. Then permit20/30. |

Source/behavior tests use real databases and observed failing-first controls;
do not change old expectations without an accepted contract successor. Run scoped
checks while editing, not repeated whole-workspace gates. Ptrace denial requires
unchanged strict rerun on a capable executor. Fork140, GPU faults/artifacts,
conditional experiments and final publication remain their own gated work.
