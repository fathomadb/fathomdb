---
title: "0.8.29 Slice 1 — dependency sweep and pin proposals"
status: PROPOSED
target_release: 0.8.29
slice: 1
---

# Slice 1 — dependency sweep and pin proposals

## Current evidence, goal and boundary

- Goal: give Slice 9 a bounded dependency backlog with explicit retain/update/investigate/postpone proposals and enough evidence to commission selected work in 10 or 140.
- This is proposal-only prework under the [drafting brief](../../planning-brief.md). No install, upgrade, lockfile regeneration, advisory-database refresh, PR closure, Dependabot unpause, fork edit or environment change is authorized here.
- Inspected local source at `6edc1155f039f87cd0a37660cb27ae4373bbce71`: manifests/locks, local remote-tracking Dependabot tips, pin metadata, architecture and prior qualification design. No live registry, alert or PR-state query was made; candidate versions below are observed targets, not assertions of latest or mandatory versions.
- Current manifests are W=0.8.28, E=0.8.0; the [master plan](../../../plan-0.8.29.md) preserves their independent version axes. Propose the coordinated W=0.8.29 bump in approved preparation; independent E changes only with its trait contract.
- Current Rust is 1.95.0/MSRV 1.95, Node contract `>=25 <26`, npm 11.12.1, Python package floor 3.10. Tool/runtime changes belong jointly with Slice 0, not an implicit sweep side effect.

## Sources actually inspected

- [Workspace/patched source](../../../../../Cargo.toml), [Cargo lock](../../../../../Cargo.lock), engine/schema/embedder/MCP/Python/native crate manifests and the MCP manifest pin/confinement tests.
- [Root npm manifest](../../../../../package.json), root lock, [TS manifest](../../../../../src/ts/package.json), TS lock, [Python manifest](../../../../../src/python/pyproject.toml), docs requirements and SBOM mini-project manifest.
- [Dependabot configuration](../../../../../.github/dependabot.yml), tracked Mermaid manifest/lock, [ONNX export requirements](../../../../../dev/tools/onnx/export-requirements.txt), CI/release action pins and local tool installers/version constants.
- [Pin register](../../../../../scripts/pinned-override-rot.json), [advisory snapshot](../../../../../scripts/pinned-override-advisories.json), [cargo-deny policy](../../../../../deny.toml), [survey README](../../../../../scripts/sbom-survey/README.md) and its tracked tier rules.
- [MCP architecture](../../../../design/mcp-adapter/architecture.md), [SQLite ADR including exit](../../../../adr/ADR-0.8.28-sqlite-per-connection-page-cache.md), .28 Slice 42 requirements/design, [reconciliation](../../reconciliation-plan.md), quality roadmap and CUDA review procedure.
- Memory read: `library-bump-steward-pattern.md`, `forks-coreyt-cudarc-and-candle-fathomdb.md`, `cudarc-fork-qualification-and-tegra-pool-facts.md`, `0.8.28-slice42-sqlite-bump-lessons.md`; verified files in the canonical external memory store. Apply owned coherent groups and fork-first qualification, adapted to this release's current-.29 integration base and proposal boundary.

## Draft local needs, requirements and acceptance

- `S1-N1`: maintainers need a current, owned upgrade backlog without discarding qualified compatibility/security exceptions.
- `S1-N2`: release reviewers need to distinguish shipped runtime exposure, optional features, build/export tools, developer tools and intentionally excluded fixtures/eval environments.
- `S1-R1`: enumerate tracked manifests and local candidate refs; record declared range, locked/source version, tier, direct/transitive reachability, proposed target or unknown, coupling and evidence date.
- `S1-R2`: every exact dependency/tool/action pin has a rationale and review trigger; unknown rationale becomes investigate, never automatic removal.
- `S1-R3`: distinguish fresh advisory evidence, historical snapshots, unassessed sources and candidate branch names; urgency requires applicability evidence.
- `S1-R4`: every proposal names owner role, blast radius, qualification, disposition, hard/conditional prerequisites and Slice 9 decision needed; preserve the existing MCP/queue/provider contracts.
- `S1-AC1`: independent reconciliation can account for every tracked manifest, all nine observed Dependabot refs and every pin group listed below; omissions fail proposal completion.
- `S1-AC2`: every update proposal identifies exact edit sites and falsifiable compatibility/security checks; unknown targets or evidence have a named research task rather than an invented version.
- `S1-AC3`: the proposal separates approval from execution and makes no fresh security-clean, product-green, host-access or publication claim. These are release-local draft IDs, not canonical AC additions.

## Observed baseline and pin register

| Group / observed value | Proposed posture and reason / review trigger |
| --- | --- |
| rusqlite 0.41.1; libsqlite3-sys 0.39.1 / SQLite 3.54.1; sqlite-vec `=0.1.9` | Retain baseline; investigate available sqlite-vec targets and changelogs. .28 already removed vendoring. Any change must preserve per-connection page-cache compile policy, Windows WAL/path behavior, vector deletion/eligibility and installed Rust consumers. |
| rmcp `=3.5.1` | Retain pending scoped research. Exact edge confinement is contractual; majors require migration/conformance/determinism review, excluded from automated version-bump PRs. Target unknown here. |
| candle-{core,nn,transformers}-fathomdb `=0.11.0`; cudarc-fathomdb `=0.19.10` | Retain qualified fork family. Fork Candle 0.11.0 is based on upstream 0.10.2; matching version numbers do not establish an upstream rebase. Owner seq-367 limits .29/140 to required repairs; broader Candle rebase/marker/kernel work moves to .30. Qualify included repairs in forks before adoption. |
| candle-kernels 0.10.2 Git patch `182ceab0547348993b6dbec4b1aed610dcc8b23f` | Retain static CUDART/driverless CPU mapping boundary. Pin register authenticates source, registry checksums and fork receipts; root patch does not propagate to published Rust consumers. Changed pin requires fresh human advisory/source review. |
| ort `=2.0.0-rc.10`; tokenizers 0.20.4 plus transitive 0.22.2 | Retain/investigate separately. ort pin was approved after rc.12 ort-sys/VitisAI mismatch; dynamic-load CPU-only eval backend, no production provider injection. A stable target remains unverified. Tokenizer updates need model/tokenization compatibility. |
| PyO3 0.29.0 / abi3-py310; napi 2.16.17, derive 2.16.13, build 2.3.2; @napi-rs/cli 2.18.4 | Retain; investigate needed fixes, treat any napi major as a coupled migration across Rust/CLI/loader/platform packages. No inferred mandate from historical major-upgrade plans. |
| Ruff `==0.15.17`; Pyright `==1.1.410`; maturin `>=1.9,<2` | Retain reviewed reproducibility pins and PEP-639 build floor; tool bumps require clean-checkout guards/tests and matched declarations, not opportunistic lint churn. |
| TS 6.0.3; @types/node 26.1.0; markdownlint-cli2 0.23.3 | Separate SDK and Markdown tool groups. TS types version does not change Node runtime contract; preserve AST-neutral Markdown fixing. Root npm has no active overrides. |
| actionlint 1.7.12; ShellCheck 0.11.0; gitleaks 8.30.1; SHA-pinned workflow actions; mkdocs 1.6.1 | Retain pending primary-source review. Coordinate exact versions/checksums/guards with Slice 7 and environment fit with 0; action release tags alone cannot replace commit provenance. |
| dunce `=1.0.5`; ONNX export exact wheel hashes | Retain; investigate explicit-pin rationale and affected Windows behavior for dunce before alteration. Export tools are tracked development dependencies with a reproducible model SHA, not runtime Python package dependencies. |

## Local Dependabot candidate register

All rows are remote-tracking observations, not verified open PRs or security alerts. Compare each tip to the current .29 tree before proposing a change; do not merge stale branch ancestry wholesale.

| Local tip / manifest | Observed target and proposed disposition |
| --- | --- |
| `96a47c598` / Cargo.lock | rustls 0.23.40 → 0.23.46; investigate transitive parents/features and advisory applicability, then consider bounded update. |
| `70b93881c` / root npm | markdownlint-cli2 0.23.3 → 0.24.0 and markdown-it 15.0.1 → 15.0.3; candidate update after neutrality/lint/advisory review. |
| `97286238c` / Mermaid lock | DOMPurify 3.4.11 → 3.4.17; investigate renderer exposure/advisory evidence and update if justified. |
| `d75aa9e0a` / Mermaid lock | puppeteer 25.3.0 → 25.16.0 and zod 3.25.76 → 4.6.7, plus transitive churn; investigate as coupled renderer/browser group, not a guaranteed contained patch. |
| `8f89fdd2e` / ONNX export requirements | certifi 2026.7.22 → 2026.10.18; investigate certificate/security applicability and reproducible export. |
| `dc92d053b` / ONNX export requirements | filelock 3.32.3 → 4.2.1; investigate major API/tool compatibility and hashes. |
| `17d90b9f8`, `49d959742` / ONNX export requirements | setuptools 78.1.0 → 83.0.0 / 84.1.0; competing proposals, not two independent upgrades; select or defer after review. |
| `790a5bba7` / ONNX export requirements | transformers 4.44.2 → 5.10.1; wide model-export migration with torch/tokenizers/hub coupling; postpone unless justified by explicit need or applicable security evidence. |

## Research, design preparation and classification

1. Draft `requirements.md`, `design.md`, `candidate-register.md` and `test-mapping.md` alongside this plan; retain raw local ref SHAs and manifest/source evidence. These deliverables remain proposals.
2. Complete discovery with the existing SBOM design/tier rules, including tools/docs, Mermaid, ONNX exporter and the survey itself; enumerate fixtures/eval exclusions explicitly. Plan execution of its offline/online modes only after approved tool setup; neither mode was run here.
3. Plan authorized primary-source retrieval: registry metadata and release/migration notes; RustSec/GitHub advisory records and affected/fixed ranges; official rmcp protocol/conformance notes; fork qualification receipts; upstream SQLite/rusqlite/sqlite-vec changes; exact action commit/release provenance. Save date/source/raw evidence and distinguish declared, locked and published values.
4. For security, map affected version → reachable dependency/feature → artifact or developer tool → exploit preconditions → fixed version/mitigation. Include yanked versions/license policy. Historical zero-advisory comments are not current evidence; an unavailable alert API is an explicit gap.
5. The 2026-08-04 checked-in advisory snapshot covers specific npm overrides only, currently none. Its GHSA entries for markdown-it/js-yaml/linkify-it and `external-source-unassessed` Rust patch posture do not establish today's whole-tree security status.
6. Classify each row as update, retain, investigate or postpone with rationale, blast (`contained`/`wide`/`migration`), effort and owner. Distinguish orphan/satisfied ref proposals after comparison; no autonomous PR closure. Deferred rows need a concrete reopen trigger.
7. Fit review: preserve format 37, driverless mapping, optional-feature closure, MSRV/ABI, supported package platforms, exact MCP behavior and model identity. Apply the quality roadmap's real-database and installed-package upgrade checks; defer broad fuzzing/exhaustive CUDA campaigns to their owned slices.
8. Proposed revision: keep fork maintenance in 140, selected contained approved updates in 10, and larger migrations explicitly placed or deferred by 9. Release-local proposal supersedes no accepted ADR; public-contract changes need reviewed ADR/interface updates.

## Dependencies, conflicts and newly discovered prerequisites

- Hard: current source/pin/ADR evidence is needed for this proposal; Slice 9 requires its completed register. Actual upgrades require recorded 9 decisions and accepted 10 scope. No dependency on implementation of 20–70.
- Conditional: accepted fork changes require 140's fork qualification before pin promotion, then affected 110/120/130 integration/qualification; library candidate discovery alone creates no such product gate.
- Preferred: finish dependency choices before Slice 30 baseline measurement; changing native/scoring dependencies afterwards requires affected remeasurement. Share environment constraints with 0 and workflow/tooling proposals with 7.
- Serialize future edits to Cargo.lock, root/TS/Mermaid locks, version manifests and pin/advisory metadata. Use isolated writers and checkout-owned binding environments; no borrowed venv or editable repointing.
- Newly discovered: Dependabot has five paused entries, but tracked Mermaid and ONNX-tool manifests exist outside their configured directories. Decide intentional coverage/manual ownership versus dedicated paused entries; unpausing is a separate Slice 9 decision, not automatic reconciliation.
- Newly discovered: the survey's documented discovery baseline is tracked `main`; design how to capture this .29 tree without silently substituting main or installing tools. Historical status prose is not evidence of a current completed sweep.
- Resources: local file/ref inspection only for drafting. Later exporter/browser/native qualification needs owned tools, disk and approved platform venues; CUDA policy/usage refresh belongs to 120, no driver/primary toolkit change or K620 execution.

## Proposal completion, validation and decisions

- Complete only when all manifest/ref/pin rows reconcile, unresolved targets/advisories are explicit, each selected group has owner/edit sites/qualification/dependencies, and Slice 9 can accept/defer individual proposals without guessing.
- Have an independent reviewer challenge missing tiers, stale refs, unsupported security urgency, pin rationale and accidental coupling. Record findings and proposed revisions; no review result is claimed here.
- Parent integration uses scoped Markdown/link validation after import. Draft completion does not require running product tests or a network/package-manager sweep.
- Later implementation: mechanical version-only bumps use the repo exception; behavior/API/tool-guard changes require per-behavior failing tests first, then green without weakening tests. Run scoped checks while iterating and required full gate once per final source phase; add group-specific real SQLite/vector, MCP conformance/determinism, ABI/platform, export-SHA and installed-consumer evidence.
- Stop and return to 9 if a proposed contained bump changes public contracts, model identity, output semantics, driver/runtime requirements, support floors or exceeds its approved group. No golden/model hash regeneration to force success.
- Open decisions: which candidates are worth .29 scope; who owns manual security review while paused; whether to extend paused coverage; evidence freshness and resources; which wide migrations/fork changes to defer with triggers. No live vulnerability verdict or HITL decision is asserted.
