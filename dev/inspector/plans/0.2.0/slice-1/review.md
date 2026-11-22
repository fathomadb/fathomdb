---
title: Inspector 0.2.0 dependency review
date: 2026-11-19
status: REVIEW
---

# Library, advisory and pin assessment

Read the slice plan/design/filled prompt and reconcile against approved source
`aef5b2eac19dbcf5850713081ccd427896909eea`. Since draft base `829625818`,
source/manifests/locks/bot/pin content is unchanged except Inspector README
navigation; retain dependency design, adjust provenance and sole-writer execution.
No upgrade, resolver, install, bot change or current-clean claim is made.

## Actual inventory and coverage

`git ls-files` finds 45 Cargo manifests, nine package.json files, two
pyproject.toml files and three requirement-like text files. Cargo workspace
production members (including Inspector) share root Cargo.lock. Other manifests
are explicit release-skew/security fixtures, retained experiments/drivers,
module-boundary tooling, Mermaid tooling and SBOM survey tooling. Python package,
TS SDK/platform packages, root Markdown tools, docs requirements and ONNX export
requirements are distinct owners; neither a fixture nor a research environment
is installed Inspector code. Inspector's bundled DOM/SVG/ES modules have no UI
framework or Node runtime dependency. Browser harness imports existing external
Playwright; S0-TOOLS owns provisioning/provenance. Actions use commit SHA refs;
no unpinned-action change is proposed by the sampled CI/release source review.
No authenticated live Dependabot alert/PR inventory was obtained.

| Direct Inspector package | Requirement → resolved | Reachability/assessment |
| --- | --- | --- |
| fathomdb-engine/schema | local/workspace → 0.8.28 | Selected Engine 0.8.29 source, defaults off; unchanged external owner. |
| rusqlite | 0.41 → 0.41.1 | Runtime bundled/hooks/blob; libsqlite3-sys 0.39.1, shared native link boundary. |
| sqlite-vec | =0.1.9 → 0.1.9 | Runtime exact shared pin; retain accepted ADR coupling. |
| serde / serde_json | 1 → 1.0.228 / 1.0.149 | Runtime serialization; JSON raw_value; JSON also build input. Retain. |
| sha2 | 0.11 → 0.11.0 | Runtime/build digest. Other consumers retain 0.10.9. No dedup proposal. |
| getrandom / libc | 0.3 → 0.3.4 / 0.2 → 0.2.186 | Runtime nonce/process/FFI; other getrandom 0.2.17/0.4.2 are separate consumers. Retain. |
| tempfile / proptest | 3 → 3.27.0 / 1 → 1.11.0 | Test closure only; retain existing harness. |

Exact pin reasons were checked in `scripts/pinned-override-rot.json`, embedder
manifest and dependency ADR: Candle fork crates =0.11.0 and cudarc-fathomdb
=0.19.10 preserve qualified pool/device/static-link work; candle-kernels Git
revision `182ceab0547348993b6dbec4b1aed610dcc8b23f` owns static CUDART linkage
and does not propagate through published Rust dependencies. These are external
Engine/fork ownership. Optional tokenizer 0.20.4 versus lock's other 0.22.2,
ONNX exact rc.10 and model revision/assets are deliberate separate closures,
not automatic Inspector upgrades. Existing Inspector defaults do not enable
model/GPU/ONNX stack; future 40 must re-evaluate its approved CPU closure.
Root npm overrides are empty; Ruff 0.15.17/Pyright 1.1.410 are reproducibility
pins and maturin >=1.9,<2 is the packaging compatibility range. Preserve them.

## Advisory and release evidence actually checked

Research retrieval date is 2026-11-19. Existing `cargo audit --no-fetch --stale
--format json` checked all 475 locked dependencies against local RustSec commit
`5a0ebedfe8bdd2e295b171f4162f8c977bcad9a5`, dated 2026-09-02. It reported
zero vulnerabilities and one unmaintained `paste` 1.0.15 warning
RUSTSEC-2024-0436. This is stale/offline evidence, not a current clean audit.
No DB fetch, cache update or tool install ran.

Current primary-source comparison produced these attributable conclusions:

| Evidence | Applicability and action |
| --- | --- |
| [RustSec RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html) | Bundled SQLite before 3.54.0 can disclose memory with crafted WAL pairs; patched libsqlite3-sys >=0.39.0. Locked 0.39.1 is outside affected range. Retain selected pin; admission restrictions are additional protection, not reason to skip dependency checks. |
| [rusqlite release notes](https://github.com/rusqlite/rusqlite/releases) | 0.42/0.43 include collation/function registration, session, column_metadata and virtual-table use-after-free fixes. No matching API consumer found in Inspector/Engine/schema source search for column_metadata, in_values, create_collation/create_scalar_function/create_aggregate_function or rusqlite Session. 0.43 is an EXTERNAL optional review candidate, no asserted reachable Inspector defect; shared link/API compatibility needs owning Engine qualification before any bump. |
| [sqlite-vec 0.1.9](https://github.com/asg017/sqlite-vec/releases/tag/v0.1.9) | Official fix concerns DELETE with long metadata text columns; selected version already includes it. Retain exact pin/accepted sole-maintainer decision; no monitoring/fallback program. |
| [rmcp RUSTSEC-2026-0462](https://rustsec.org/advisories/RUSTSEC-2026-0462.html) | Lock has rmcp 3.5.1, affected version interval, patched >=3.9.0. Advisory is Streamable HTTP client buffering. MCP manifest enables server/macros/transport-io, test client/async-rw, with no affected HTTP transport feature declared. Inspector does not link MCP. EXTERNAL package-range match; owning MCP team must confirm resolved feature closure before upgrade/mitigation decision. Not an Inspector runtime vulnerability claim. |
| [markdown-it GHSA](https://github.com/advisories/GHSA-6v5v-wf23-fmfq), [js-yaml first GHSA](https://github.com/advisories/GHSA-h67p-54hq-rp68), [js-yaml chain GHSA](https://github.com/advisories/GHSA-52cp-r559-cp3m), [linkify-it GHSA](https://github.com/advisories/GHSA-v245-v573-v5vm) | Current authoritative affected ranges exclude root locks markdown-it 15.0.1, js-yaml 5.4.1, linkify-it 6.1.0. Retain; four checked advisories do not certify the whole npm tree. |
| [Node official release schedule](https://nodejs.org/en/about/previous-releases) | Node 25 is EOL, Node 26 is supported. Local 26.8.2 conflicts with TS >=25 <26 and CI 25.9.0. Prefer separately scoped shared SDK/toolchain reconciliation over selecting EOL solely to silence engines. S0-TOOLS/7 alias. |
| [Playwright browser contract](https://playwright.dev/docs/browsers) | Versions require specific browser binaries; preserve exact pair qualification, not merely any Chromium executable. S0-TOOLS alias. |

RustSec's current index was searched for direct package names and native SQLite/
rmcp risks. Absence of a text hit is not proof of safety. Current full transitive
Rust/npm/Python/action alert evaluation remains missing evidence; the offline
npm snapshot dated 2026-08-04 has only four governed advisories. Do not refresh
that policy snapshot merely to imply a universal scan. No newer target is
recommended for serde/JSON/hash/random/libc/test tools without demonstrated need.

## Decision proposals

| Key / owner | Proposal, validation and classification |
| --- | --- |
| S1-POSTURE / owner/shared dependency maintenance | Retain paused Dependabot (all five weekly ecosystems PR limit 0) under standing ruling; ask include/postpone for a separately commissioned current full alert/feature-closure review. Do not unpause by implication. Data gap remains explicit. |
| S1-MCP / MCP external | Refer exact rmcp advisory/version/feature question to owning scope; Inspector Slice 10 must not bump it. Acceptance: actual resolved HTTP transport reachability and owner mitigation/retain decision. Medium risk, small evidence effort, update effort unknown. |
| S1-SQL / Engine external | Postpone routine rusqlite 0.43 upgrade; retain 0.41.1/0.39.1. If owning review finds an affected reachable API, separately commission compatibility/security qualification across real SQLite/vec/format-37 paths. Medium risk, medium effort. |
| S0-TOOLS / shared SDK alias | Reconcile supported Node major/package/CI before SDK gates; no change to Inspector installed runtime. |

All are UNDECIDED proposals for 9. No pin change is required by the checked
Inspector-specific evidence; this does not waive missing current full audit or
mandatory feature positives. Independent review and Markdown limits are in [the shared record](../slice-9/prework-verification.md). Canonical needs/requirements/acceptance remain unchanged.
