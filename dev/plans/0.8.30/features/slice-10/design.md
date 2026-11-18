---
title: Slice 10 preparation design and acceptance
status: ACTIVE
target_release: 0.8.30
slice: 10
date: 2026-11-18
---

# Slice 10 preparation design and acceptance

Subject: `impl/0.8.30-slice10@7dd493e634033e98db5c66db862510923f089e8f`.
The [approved plan](plan.md) traces seq-399/400. The five commits beyond the
prework completion through activation are planning, scope, and .29 closure.
The tracked `origin/main` is `b86b19847`; it closes .29 Tegra delivery, corrects
public release references and platform metadata, and preserves the recorded
tooling failures. No dependency manifest, lock, workflow, model, or release
script changed from prework subject `18427b7a8`. Preserve the separate
Inspector work and the dated .29 evidence. Use [architecture v2](../../../../design/fathomdb-data-plane-architecture-v2.md),
the [ADR index](../../../../adr/ADR-0.6.0-decision-index.md), and the maintained
[SDK bindings](../../../../design/bindings.md) and [delivery](../../../../design/release.md)
owners. No storage/schema, public API, model or native ABI change is needed.

## Security corrections

Use separate commits and review actual resolved edges. Versions below were
verified against upstream release and package metadata on November 18, 2026.
Dependabot's private alert inventory is unavailable. Advisory IDs and resolved
features establish the specific affected dependencies and control coverage.

| Local acceptance | Adopted draft and scope | Qualification and limits |
| --- | --- | --- |
| PREP-SEC Markdown | Adopt #257's resolution goal. Root `package.json` and `package-lock.json` move from `markdownlint-cli2` 0.23.3 / `markdown-it` 15.0.1 to 0.24.0 / 15.0.3. This CLI requires Node ≥22 and specifies markdownlint 0.42.0, js-yaml 5.4.3, smol-toml 1.9.1 and globby 16.2.4. Retain other existing resolutions unless forced by the new exact edges. The parser is consumed by repo/PR Markdown tooling. [Advisory](https://github.com/advisories/GHSA-gfr2-rv44-r23c) · [CLI registry metadata](https://registry.npmjs.org/markdownlint-cli2/0.24.0) | `npm ci`, `npm ls markdownlint-cli2 markdown-it js-yaml smol-toml`, `bash scripts/check-pinned-override-rot.sh`, `bash scripts/tests/test_check_pinned_override_rot.sh`, `bash scripts/agent-lint-md.sh`. Run `npm run format:md -- <disposable normal Markdown fixture>` with root-owned linter. Confirm a neutral style fix, independent AST equality, and no unrelated modifications. Keep AST guard rules and test safe fixtures. |
| PREP-SEC TLS roots | Advance only the 1.x `webpki-roots` from 1.0.7 to 1.0.12, compatible at Rust 1.71. The 0.26.11 shim re-exports 1.x; 1.0.10+ removed the affected root. Keep ureq 2.12.1, rustls 0.23.46, rustls-webpki 0.103.16 and the native/model pins. The thin embedder lacks the edge; both opt-in default model graphs use ureq’s rustls configuration. [Bundle advisory](https://rustsec.org/advisories/RUSTSEC-2026-0381.html) · [Verification advisory](https://rustsec.org/advisories/RUSTSEC-2026-0382.html) · [Crate](https://crates.io/crates/webpki-roots/1.0.12) | `cargo update -p webpki-roots@1.0.7 --precise 1.0.12`; compare `cargo tree --locked -p fathomdb-embedder -F "$feature" -e normal,features -i webpki-roots@1.0.12` independently for default-embedder/default-reranker; inspect the removed trust anchor against upstream and verify the feature-off absence. Closure requires actual trusted HTTPS success and untrusted-certificate refusal through the resolved ureq TLS configuration. Use the shared probe below once equivalence is confirmed. Record local HTTP/cache controls independently. |
| PREP-SEC Tokio | Advance 1.52.3 → 1.54.1 (MSRV 1.85). This is above the AsyncFd advisory floor and skips the yanked 1.54.0 interval release. 1.54.1’s macro constraint stays `~2.7.0` and exact rmcp 3.5.1/httpmock 0.8.3 stay. [AsyncFd advisory](https://rustsec.org/advisories/RUSTSEC-2026-0386.html) · [Interval advisory](https://rustsec.org/advisories/RUSTSEC-2026-0384.html) · [Crate](https://crates.io/crates/tokio/1.54.1) | `cargo update -p tokio --precise 1.54.1`. Use inverse `cargo tree --locked -i tokio -e normal,features` on MCP and N-API independently and the loader with dev edges. MCP enables runtime/sync/time/stdin-stdout; N-API enables its async runtime; the loader test graph gets Tokio networking through httpmock/hyper/h2. Check target-gated features before attributing advisory reach, and exercise process/lifecycle, async errors, and real local HTTP below. |

Affected Rust and TypeScript controls, from the worktree with the final lock:

- `cargo test --locked -p fathomdb-mcp --test process --test protocol --test admission --test lifecycle`.
  Includes real stdio clean exit and admission/deadline/idle behavior.
- Confirm from both loader call sites that `AgentBuilder::new()` uses the
  default rustls configuration. Neither builder sets custom trust or a
  custom TLS connector. Use one disposable, matched-graph ureq probe with
  `default-features = false`, `features = ["tls"]` and the exact selected
  ureq, rustls and roots resolutions. Require successful HTTPS to an authorized
  public package registry endpoint and a certificate-trust failure against a
  loopback TLS server with a name-matching, untrusted test certificate.
  Distinguish certificate rejection from network failure, hostname mismatch
  and HTTP errors. Use no model transfer or private token. Record versions,
  both observed outcomes, and the equivalent loader configuration. If either
  leg is unrun, Slice 10 owns the outstanding TLS qualification and holds
  PREP-SEC TLS closure until that leg is proved. Local HTTP, cache, digest and
  inference evidence remain separately reportable.
- `cargo test --locked -p fathomdb-embedder -F loader-test-hooks --test loader -- --test-threads=1`.
  Local HTTP fixtures cover correct and corrupt hashes, partial resume,
  locking, cache admission and refusal. They do not negotiate TLS.
- `cargo test --locked -p fathomdb-embedder -F default-reranker --test slice71_reranker_policy`
  and `FATHOMDB_RERANK_DEVICE=cpu FATHOMDB_SKIP_NETWORK_TESTS=1 cargo test --locked -p fathomdb-embedder -F default-reranker --lib candle_reranker::tests:: -- --nocapture`.
  Check that exact cached TinyBERT weights are used and cases actually score.
  No cached model or internal early return is an unrun score case. Inspect the
  independent reranker retry, staged write and verified-digest call path in
  `candle_reranker.rs`; the embedder HTTP fixtures do not cover it. Record
  any unrun reranker body or inference path independently of TLS.
- In `src/ts`, run `npm ci`, `npm run typecheck`, `npm run build:native:debug`
  and `npx --no-install tsc -p tsconfig.json`. Then run `node --test` on
  `dist/tests/{npm-platform-split,npm-platform-tegra,typed-errors,vector-open-errors,surface}.test.js`
  and run `FATHOMDB_SKIP_NETWORK_TESTS=1 npm test` for the broader async
  lifecycle. Focused loader cases exercise negative package resolution
  and typed errors as well as successful dispatch.

## Development runtime and local packed SDK

PREP-ENV / PREP-NODE. Select Node **26.13.1** Lithium LTS (November security
release) with its bundled **npm 11.20.0**. The npm registry reports Node
`^20.17.0 || >=22.9.0` for npm 11.20.0. npm 12.2.3 is compatible with
26 but adds a major without an adopted need. Keep TypeScript 6.0.3,
`@types/node` 26.1.0, `@napi-rs/cli` 2.18.4 and napi8; confirm against actual
build and Node-API 10 on the candidate. The candidate booted on the owned
Linux x64 glibc 2.39 host. Preserve Rust 1.95.0 and use owned Python 3.12.
The first supported provisioning pass succeeded on auto-selected uv Python
3.13.13. Select 3.12 locally (system interpreter is 3.12.3), then repeat the
owned wheel setup for final source/lock inputs. This does not change the
repo's Python floor, interpreter-selection policy, or public ABI.
[Node distribution index](https://nodejs.org/dist/index.json) ·
[Node-API](https://nodejs.org/api/n-api.html) ·
[npm 11.20.0 metadata](https://registry.npmjs.org/npm/11.20.0).

- Update `.nvmrc`, root and TS `packageManager`, and development/test CI
  `verify-fast`, `verify`, `default-embedder-tests`,
  `native-artifact-runtime-validation`, and `markdownlint`. Set runtime
  receipt expectations for the candidate execution. Leave source build
  flags, Action SHAs, path routing, and cache/timeout oracles intact.
  Use exact settings for platform setup; a hosted job definition is not a
  completed platform test.
- Release role uses Node 25.9.0 / bundled npm 11.12.1 for normal publishing.
  Keep `release.yml` production build, packaging, publishing, post-publish
  and promotion jobs, manylinux, AArch64 release rehearsal, and Tegra
  release/evidence pins on their current exact inputs. Recovery OIDC
  promotion continues to pin npm 11.21.0. Keep released `engines.node`
  `>=25 <26` and published support prose through Slice 70. Local candidate
  install captures the expected engine warning under normal npm settings;
  do not rewrite the tarball engine range or claim strict-engine support.
- The Tegra build and static CUDA contract currently couple the project
  manifest's `packageManager` to the release npm. Remove that cross-role
  dependency and keep the real version check in `build-napi-cuda-tegra.sh`,
  the exact pinned contract, workflow checks, driverless image digest, and
  wrong-node/wrong-npm rejection. Verify that the exact old npm CLI still
  reports 11.12.1 and honors the existing install path with the candidate
  manifest. Do not delegate the release npm choice to the project
  `packageManager`. Slice 20 owns normal release runtime/OIDC qualification.
- Establish a genuine RED for each changed executable role guard before
  editing its implementation. Extend the existing
  `scripts/tests/test_tegra_napi_build_contract.sh` fixture to accept a
  development manifest with the new npm using the exact release runtime
  while wrong release npm still fails. Check
  `scripts/tests/test_cuda_release_contract.sh` for the same role split.
  Reconcile `test_runtime_release_alignment.sh`, `test_actionlint_fixture.sh`,
  `test_slice85_final_gate.py`, `test_ts_cache_coverage_split.sh` and
  `test_native_artifact_runtime_validation.sh` where they encode the old
  CI/release equality or exact receipt runtime. Preserve their independent
  wrong-publisher/routing/metadata controls. Run actionlint and scoped shell
  and Python suites. Run `test_setup_python_owned.sh` with `missing`,
  `foreign`, `stale` and `default`, plus the real ownership and tool checks.
- Build with `npm run build` and confirm `test_napi_build_hermetic.py --execute`.
  In disposable staging, pack the clean JS build and matching
  `src/ts/npm/linux-x64-gnu` platform package with the actual release-mode
  native bytes and existing matching package versions. Inspect both pack
  listings. Install both local tarballs in a fresh outside-source consumer
  with exact npm and no registry lookup, source symlink, or local-dev
  fallback. Diagnose installed native resolution and compare real path and
  hash to the staged binary. With bounded process exits, use the public
  `Engine.open`, provenance-bearing `write`, `read.get` and `close`
  against a disposable file. Assert exact stored body and missing
  `null`, then start a fresh Node process to reopen and read the same body,
  close and exit naturally. Use the existing installed-smoke data shape
  and retain reproducible probe instructions with the result. This trial
  qualifies the local host's exact bytes, not manylinux or any other
  platform, strict engine metadata, shared registry-smoke code, or
  publishing. Slice 70 uses the checks from20 for Windows, macOS, other
  Linux and selected Tegra/CPU/CUDA artifacts after60 settles actual native
  inputs. Slice 80 independently proves installed public release bytes.

## Exporter and renderer dispositions

PREP-TRIAGE. Inspect the real test/tool graphs and report the final resulting
versions and open advisory exposure. Neither surface is a production SDK
dependency based on the tracked paths. The hosted/registry metadata establishes
published versions, not selected behavior.

- **#263 certifi**: 2026.7.22 is affected; trial 2026.10.18 or the compatible
  2026.10.23 wheel whose bundle addresses the emSign advisory. This concerns
  Requests TLS roots when networking is used, while the recorded export reads
  local-only pinned inputs. Compare one updated hash pin against the
  authoritative wheel. Use a disposable CPython 3.12 exporter with
  `pip install --require-hashes --no-deps -r <candidate lock>` and `pip check`.
  Use the existing safetensors snapshot, verify the loader's three digests and
  require no pickle weights or remote code. Run with `HF_HUB_OFFLINE=1`,
  `TRANSFORMERS_OFFLINE=1` from the checkout root, invoking
  `"$exporter_venv/bin/python" dev/tools/onnx/export_bge_small_onnx.py --out "$export_scratch/model.onnx" --verify`.
  Compare *both* output graph and the exported input tokenizer independently
  to the accepted graph SHA `c92689ecdefc35c3b533f68449cf1a50e7a1301736d8031372b0fa70e862c5a5`
  (recorded in the exporter lock) and tokenizer SHA
  `d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66`
  (pinned Candle loader). Current cached inputs match both in a read-only
  check. If install/refusal/byte proof fails or remains unrun, retain the old
  pin and record the exact limitation. Any changed accepted bytes require a
  separate owner decision. [Certifi advisory](https://github.com/advisories/GHSA-6q48-6c7w-9gw2) ·
  [Wheel releases](https://pypi.org/project/certifi/#history).
- **#260 setuptools / #259 filelock / #256 Transformers**: defer. The
  selected Linux wheel-only exporter does not run the affected legacy
  Windows installer. The exporter uses local `AutoModel.from_pretrained`
  and Hugging Face cache metadata scanning. Audit the pinned helper imports
  for simple locks versus the affected `ReadWriteLock`; no such use is
  visible in the export script. The cited Transformers advisory involves
  chat-template names in `save_pretrained`, which this BERT-only export
  does not invoke. Retain the independently relevant Torch safety prerequisite,
  the existing pinned safetensors-only input and offline admission, and hold
  any reachable advisory requiring a new stack for an explicit scope choice.
  [Setuptools advisory](https://github.com/advisories/GHSA-xmj3-9gf7-h52w) ·
  [Filelock advisory](https://github.com/advisories/GHSA-v4x8-3wr5-3wq8) ·
  [Transformers advisory](https://github.com/advisories/GHSA-xrqw-3rrv-vx5w).
- **#258 DOMPurify / #261 renderer**: #258 is already satisfied by
  `c484a461e` and DOMPurify 3.4.17. Current CLI/Mermaid 11.16.0 locks
  parser 1.2.0 with Zod 3.25.76 and Puppeteer 25.3.0. The advisory catalogs
  the Zod range below 4.6.6 and concerns array-validation issue accumulation;
  the renderer accepts local diagrams and the parser consumes Zod. Reject
  #261’s forced Zod4 substitution. Investigate the upstream CLI 12.2.0 +
  Mermaid 12.2+ + parser 2.0.1 route, whose parser has no Zod dependency and
  whose CLI supports Puppeteer ^25. Retain current Puppeteer and test a
  coherent graph with `npm ls --all` and `npm audit --json`; prove every
  parser path, including the ZenUML plugin’s peer graph. Retain DOMPurify’s
  resolved fix and explain unavoidable transitive changes. Require
  `dev/tools/mermaid/render-mermaid.sh check`,
  `bash dev/tools/mermaid/test-render-mermaid.sh` and
  `node dev/tools/mermaid/test-sanitizer.mjs` from the checkout root, with
  real compatible local Chromium. A Chromium cache exists, but check the exact Puppeteer build
  and execution. A required browser download has a separate approval
  prerequisite. Hold an unsupported graph or unrun render instead of
  forcing compatibility. [Zod advisory](https://github.com/advisories/GHSA-mrhx-c7xc-m3w5) ·
  [Mermaid CLI](https://github.com/mermaid-js/mermaid-cli/releases/tag/12.2.0) ·
  [Parser registry metadata](https://registry.npmjs.org/@mermaid-js%2fparser/2.0.1) ·
  [DOMPurify advisory](https://github.com/cure53/DOMPurify/security/advisories/GHSA-cg2g-xh64-v4q7).

P9-06 keeps general dependency and Action refresh deferred. Retain exact
SQLite/Candle/cudarc/ORT/model inputs, zero routine Dependabot version PRs and
qualified release action pins. The tokenizer and zip active-feature inventory
belongs with60; broad rustix/chrono/serialization/tool refresh has no selected
use here.

## Targeted documentation

PREP-DOC. Correct [structured search hits](../../../../../docs/guides/structured-search-hits.md)
against `fusion.rs`, `slice50_record_identity.rs`,
`slice71_search_statement_trace.rs` and MCP `feedback.rs`. Fusion admits
one unit per stored node/edge version or ownerless FTS row. Distinct versions
can share a body or public ref. Ownerless hits can have a null source ID;
governed writes still require provenance. Preserve current logical-get and
atomic otherwise-recordable named-feedback semantics. Align
[Python install](../../../../../docs/install/python.md),
[contributing](../../../../../docs/contributing/index.md), and setup pointers
with the owned noneditable worktree wheel, direct-checkout editable path,
AC-036 ptrace-capable executor and `AGENT_LONG=1 ./scripts/check.sh` long-test
scope. Refresh engineering entry/architecture/ADR-index applicability
around current schema 37, the twelve-member workspace, .29 publication
evidence and live release selector. Preserve historical verdicts. Defer
behavior-coupled publication design copy to20 and later occurrence labels
to30. Run root and public docs lint, generated-docs check and strict MkDocs.

## Receiving requirements and acceptance

PREP-ALLOC. Adopt the bounded prework3–7 drafts below under the master
requirement IDs. Each receiving design selects its mechanism and actual
negative/positive tests. Canonical acceptance criteria remain authoritative.

| Receiving slice and input | Need and required proof |
| --- | --- |
| [20](../slice-20/plan.md), P9-09/10 and prework3/4/5/6/7 | R30-CI/DIAG. Retain full first diagnostics even when a later unit is cancelled, distinguishing cancelled and unrun. Pair artifact receipt with actual runtime/native identity. Prove wrong/cancelled CI and exact tag routing, bounded exact-version visibility, pinned normal OIDC npm promotion and persisted Rust/Python/Node consumers with wrong/missing native refusal. Measure Windows static check, tool-only bootstrap and duplicate builds while retaining trust/coverage. Own known .29 terminal-state, preflight and commission-manifest assumptions. |
| [30](../slice-30/plan.md), P9-11 and prework3/4/5 | R30-COR/S50. One bounded hunt with archived state and independent ranks/fields/score bits. Distinct same-body versions, ownerless provenance and zero/NULL extent, failed publication and active-reader consistency use real stores. Preserve atomic named-ref refusal; review occurrence labels separately if selected. Replace or remove trivial query scaffold only after mapping existing in-crate properties and meaningful new composition coverage. |
| [40](../slice-40/plan.md), P9-12 and prework3/5 | R30-LAT. Attribute current latency and evaluate one measured treatment on matched interleaved state, config and useful completions. Report caller tails, throughput, errors and resources; require fixed-state exact outputs and positive reader/writer progress. Record a negative or inconclusive comparison without forced adoption. |
| [50](../slice-50/plan.md), P9-11 and prework3/4/5 | R30-ROB. Second bounded hunt, covering queued versus started timeouts, restored capacity, simultaneous actual writer/reader progress, close and fresh reopen, plus current default Node growth. Provider drain timing follows SQLite quiescence. Retain the settled B30-01 placement. Slice 60 must prove final `CudaSlice` drop safety after context loss on the exact consumed pins before Slice 50 exercises or ships that CUDA release path. Independent safe source and CPU work can proceed. |
| [60](../slice-60/plan.md), prework1/3/4/7 | R30-FORK. Record upstream bases, retained delta, actual feature and downstream graph, supported native/PTX target emissions, unsupported outcomes, exact artifact/pin identities and private-pool behavior; no assumed GPU or toolchain readiness. |
| [70](../slice-70/plan.md), prework3/5/6/7 | R30-QUAL/TEST. Account for both hunts, concrete wrong-result oracles, and measured reductions. Run actual full, feature, strict security, long, performance and artifact rows for the integrated candidate. Consume shared20 installed checks on verified60 inputs before changing published Node engines/support. Reuse only evidenced matching scope, and keep skipped/unavailable rows open. |
| [80](../slice-80/plan.md), prework3/4/5/6/7 | R30-REL. Separately authorized exact candidate publication; eight public consumers, exact visibility and native resolution, meaningful body/missing/close/exit/fresh-process reopen and Pages links and bytes. Required publication or smoke failures leave delivery open. |

## Implementation order

Review this design before changing implementation. Conclude owned provision,
then use separate small security updates, runtime role RED/GREEN, bounded
tooling dispositions and the documentation corrections. Mechanical version
and lock edits are exempt from RED; executable behavior changes require an
observed failure at the old behavior and a separate GREEN. Verify scoped
controls during development, then let the parent schedule one independent
full gate on the clean coherent candidate. Keep raw exits, counts, real
selected inputs and platform limits with the slice result. An official
release date, registered dependency or rendered configuration alone does
not establish runtime qualification.
