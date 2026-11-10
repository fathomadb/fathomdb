# Slice 10 preparation requirements and local acceptance

Authority: actual [Slice9 decision log](../../prework/slice-9/decision-log.md),
[direct mandate](../../direct-execution.md), reviewed0–8 proposals and
[master release](../../../plan-0.8.29.md). Earlier generic “final acceptance”
waiting language is superseded. These are local preparation criteria, not new
canonical ACs. [Execution evidence](execution.md) maps actual implementation and
assembled qualification; fresh independent review and parent acceptance remain
pending. PhaseA executed no setup or tests.

| Need / local requirement | Binding acceptance and source mapping |
| --- | --- |
| S10-N1 reproducible ownership / S10-R1 | S10-A1: exact approved scope, final9 closure SHA, source owner, fresh branch/base/cwd and owned paths recorded; strict preflight exits0 before/after new worktree. N29-D/R29-REL-D; R29-3P01/O3-01. |
| S10-N2 valid tools/native / S10-R2 | S10-A2: Node25.9.0/npm11.12.1, Rust1.95, developer Python>=3.11, Ruff0.15.17/Pyright1.1.410 and required tools verified before workload. Owned modules/venv; source-bound noneditable wheel/native receipt records SHA/tree, features, interpreter/ABI/target, artifact/module digests and resolved import paths. Missing/foreign/wrong/stale setup fails, never system fallback or timestamp-only readiness. R29-3P01. |
| S10-N3 durable truth / S10-R3 | S10-A3: exactly one live.29 tracked state/board pair, declared generated master/board views, real0–9 completion SHAs reachable from local release ref;10 IN_PROGRESS. release-current/view/dependency validators exit0; historical.28 bytes unchanged. N29-D; R29-3P01. |
| S10-N4 contained dependencies / S10-R4 | S10-A4: current lock resolves rustls0.23.46/necessary webpki0.103.16 and renderer DOMPurify3.4.17 with exact integrity; no stale-lock replacement or unrelated changes. Actual feature/provider/config and changed-node source/license/advisory/yank evidence plus affected downloader/renderer checks recorded. R29-3P02/O3-02; NEED-002/005, REQ-033, AC-037 remain. |
| S10-N5 complete inventory / S10-R5 | S10-A5: exact tracked exporter discovered, dispatched,28 named/versioned direct origins preserved; valid inline hashes handled, malformed/unsupported hash syntax names manifest/exit3. Explicit tiers/exclusions account for every recognized path, unknown path exits2; excluded files are neither parsed nor looked up. Independent CycloneDX validator rejects its invalid control. R29-3P03/O3-03. |
| S10-N6 accurate authority / S10-R6 | S10-A6: enumerated applicability/setup/package/MCP-rationale/profile corrections match source/ADRs, preserve IDs/thresholds/current adapter refusals/historical evidence/generator ownership. N29-D,K; R29-3P04/O3-04. AC052 same-version conflict remains160's allocated reconciliation;10 never changes E or locked AC by inference. |
| S10-N7 protected delivery / S10-R7 | S10-A7: exactv0.8.29 absent-tag bootstrap passes offline with GPU approval HELD; retained.28 and normal route pass, unauthorized/mismatched tags, API failures, missing workflow/policy/reviewers all fail. No live route/dispatch/publication claim. N29-D; R29-3P05/O3-05. |
| S10-N8 coherent release identity / S10-R8 | S10-A8: set-version --workspace0.8.29 then --check-files and version/runtime/platform-package fixtures pass. E0.8.0 and format37 unchanged. REQ-047/049, AC-051/053; independent axes in release design govern. W coordination is implied release work under direct mandate, not a new generic question. |

[Register](preparation-register.md) binds each criterion to exact edit sites,
checks and stops. No broad dependency security clearance, new SDK contract,
exporter execution/model hash regeneration, CUDA toolkit choice, fault venue or
publication is inferred.100/110 remain included with item-specific reviewed
fault/remedy/venue controls;120 artifact choice and optional150 stay with owners.
Missing their future choices does not block unrelated10 preparation.
