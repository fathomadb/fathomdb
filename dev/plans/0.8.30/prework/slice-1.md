---
title: Slice 1 library and Dependabot sweep
status: COMPLETE
target_release: 0.8.30
slice: 1
date: 2026-11-18
---

# Slice 1 library and Dependabot sweep

**Disposition:** bounded proposal investigation complete. Source subject
`18427b7a8` on `release/0.8.30`, including `origin/main@42629c2a1`.
GitHub and primary registry, upstream release and advisory lookups retrieved
2026-11-18 America/Chicago. Decisions go to Slice 9. Implementation,
qualification, dependency installation and lockfile updates remain pending.

## Automation and current queue

The connected GitHub API author-filtered open PR search returned seven
Dependabot PRs, cross-checked against the unfiltered open PR search and each
candidate's diff. The eighth open PR (#254) is a Tegra delivery change.
The five configured weekly entries cover Cargo at root, Python at
`/src/python`, npm at root and `/src/ts`, and Actions. The zero version-PR limit
pauses routine updates. Security updates have separate controls. Repository
Dependabot alert settings and the private alert inventory were unavailable
through this lookup. ONNX exporter and Mermaid PRs concern tracked tooling
outside the five entries. Keep routine version PRs paused unless Slice 9
explicitly changes the standing policy. [Dependabot config](../../../../.github/dependabot.yml) · [GitHub security-update controls](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/manage-your-dependency-security/customizing-dependabot-security-prs)

| Open PR and checked-out value | Candidate and primary evidence | Proposal and owner |
| --- | --- | --- |
| [#257](https://github.com/fathomadb/fathomdb/pull/257) root Markdown `markdownlint-cli2` 0.23.3 and `markdown-it` 15.0.1 | PR diff 0.24.0 and 15.0.3. 15.0.3 fixes the identified multiline-reference advisory. | **Upgrade in 10** as a coherent tooling change. Review every additional lock delta and run Markdown lint and the AST-neutrality guard. [Upstream advisory](https://github.com/advisories/GHSA-gfr2-rv44-r23c) · [Parser changelog](https://github.com/markdown-it/markdown-it/blob/master/CHANGELOG.md) |
| [#258](https://github.com/fathomadb/fathomdb/pull/258) Mermaid DOMPurify already 3.4.17 in the checked-out lock | PR proposes 3.4.11 → 3.4.17, which fixes the cited 3.4.16-and-earlier ranges. 3.4.18 is a newer published upstream release. | **Keep in 10** for this PR. Reconcile as already satisfied after confirming the target ref. Treat 3.4.18 as a distinct future candidate. [Maintainer advisory](https://github.com/cure53/DOMPurify/security/advisories/GHSA-cg2g-xh64-v4q7) · [3.4.18 release](https://github.com/cure53/DOMPurify/releases/tag/3.4.18) |
| [#261](https://github.com/fathomadb/fathomdb/pull/261) Mermaid `zod` 3.25.76 and `puppeteer` 25.3.0 | PR lock candidate 4.6.7 and 25.16.0, with other transitive changes. Zod advisory supplies a reason to move and the locked Mermaid parser declares `^3.24.1`. Upstream Mermaid CLI 12.2.0 is a separate candidate. | **Investigate in 10.** Prefer a supported upstream graph, or demonstrate the proposed graph is sound against actual rendering and browser startup. Review the bundled Chromium change. [Zod advisory](https://github.com/advisories/GHSA-mrhx-c7xc-m3w5) · [Zod release](https://github.com/colinhacks/zod/releases/tag/v4.6.7) · [Mermaid CLI release](https://github.com/mermaid-js/mermaid-cli/releases/tag/12.2.0) |
| [#263](https://github.com/fathomadb/fathomdb/pull/263) exporter `certifi` 2026.7.22 | PR 2026.10.18. PyPI also has 2026.10.23. The emSign root advisory applies to the old pin. | **Upgrade candidate in 10** if exporter maintenance is selected. Use the patched cohort and exact wheel hash, verify pinned inputs and compare output digest before accepting. [Maintainer advisory](https://github.com/certifi/python-certifi/security/advisories/GHSA-6q48-6c7w-9gw2) · [PyPI](https://pypi.org/project/certifi/) |
| [#260](https://github.com/fathomadb/fathomdb/pull/260) exporter `setuptools` 78.1.0 | PR 84.1.0, PyPI 84.2.1. The reviewed advisory concerns legacy Windows wheel installation. | **Investigate in 10** within the exporter cohort. Established exporter reproduction is a separate Linux path. Check build backends, platform reach and output before adoption. [Advisory](https://github.com/advisories/GHSA-xmj3-9gf7-h52w) · [PyPI](https://pypi.org/project/setuptools/) |
| [#259](https://github.com/fathomadb/fathomdb/pull/259) exporter `filelock` 3.32.3 | PR 4.2.1, PyPI 4.2.12. The linked vulnerable API is `ReadWriteLock`, so version exposure alone does not establish the exporter invokes it. | **Investigate in 10** within the exporter cohort. Check imported API, Python compatibility, fresh patched version and output digest. [Advisory](https://github.com/advisories/GHSA-v4x8-3wr5-3wq8) · [PyPI](https://pypi.org/project/filelock/) |
| [#256](https://github.com/fathomadb/fathomdb/pull/256) exporter `transformers` 4.44.2 | PR 5.10.1, PyPI 5.23.0. An advisory covering `save_pretrained` applies below 5.10.0. The proposed wheel and new deps need to resolve with the pinned CPU Torch and exporter interpreter. | **Investigate in 10.** A single-line requirements bump proves neither dependency compatibility nor export byte identity. Require reproducible output before replacing the hash-pinned cohort. [Advisory](https://github.com/advisories/GHSA-xrqw-3rrv-vx5w) · [PyPI](https://pypi.org/project/transformers/) |

## Rust workspace and selected transitive chains

Counted 35 external direct dependency names from the 12 crate manifests after
resolving workspace aliases and including target/build/test sections. The
root lock contains 474 package records across features and platforms. A lock
edge establishes possible resolution. Actual activation depends on the target,
feature set and consumer. Versions here are locked unless a manifest constraint
is shown separately. [Root manifest](../../../../Cargo.toml) · [Root lock](../../../../Cargo.lock)

| Group and current resolution | Verified candidate or finding | Proposal and owner |
| --- | --- | --- |
| Downloader `ureq` 2.12.1 with `tls` → `rustls` 0.23.46 / `rustls-webpki` 0.103.16 / `ring` 0.17.14 and root wrapper 0.26.11 → `webpki-roots` **1.0.7** | The wrapper uses the affected 1.x roots. Advisory fix ≥1.0.10, published 1.0.12. The related verification advisory is fixed in locked `rustls-webpki` 0.103.16. ureq 3.5.2 is a separate major API change. | **Upgrade the roots in 10** with a narrow lock change; check both default model loaders, TLS/retry/digest/refusal behavior and feature activation. Keep ureq 2 pending a separate reason. [Roots advisory](https://rustsec.org/advisories/RUSTSEC-2026-0381.html) · [Verification advisory](https://rustsec.org/advisories/RUSTSEC-2026-0382.html) · [Roots package](https://docs.rs/crate/webpki-roots/1.0.12) |
| MCP/Node async and test HTTP `tokio` 1.52.3; `rmcp` =3.5.1 and macros 3.5.1; `httpmock` =0.8.3 with hyper 1.11.1 and h2 0.4.19 | Tokio 1.54.1 includes the AsyncFd fix for the affected 1.52 line and avoids the 1.54.0 interval regression. rmcp 3.11.0 and httpmock 0.9.0 are published. | **Select Tokio update in 10** with MCP stdio/admission, Node runtime/lifecycle and affected HTTP controls. Keep rmcp exact for typed MCP transport/schema and keep HTTP fixture runtime separate pending need. [AsyncFd advisory](https://rustsec.org/advisories/RUSTSEC-2026-0386.html) · [Interval advisory](https://rustsec.org/advisories/RUSTSEC-2026-0384.html) · [Tokio](https://docs.rs/crate/tokio/1.54.1) · [rmcp](https://docs.rs/crate/rmcp/3.11.0) |
| SQLite `rusqlite` 0.41.1, `libsqlite3-sys` 0.39.1, bundled SQLite 3.54.1 and `sqlite-vec` =0.1.9 | Published 0.43.0 / 0.40.0 still bundle 3.54.1. sqlite-vec 0.1.9 is the verified available stable. | **Keep in 10.** Preserve the upstream SQLite WAL fix, page-cache build flags and real Windows verbatim-path and WAL controls. Any selected upgrade needs checkpoint/concurrent/reopen, vec registration and installed native proof. [rusqlite release documentation](https://github.com/rusqlite/rusqlite/blob/master/README.md) · [sqlite-vec](https://docs.rs/crate/sqlite-vec/0.1.9) · [SQLite acceptance test](../../../../src/rust/crates/fathomdb-engine/tests/slice42_sqlite_bump.rs) |
| Bindings `pyo3` 0.29.0; `napi` 2.16.17, `napi-derive` 2.16.13 and `napi-build` 2.3.2 | Published PyO3 0.30.1 and napi 3.16.2, derive 3.7.1 and build 2.6.0. | **Keep current ABI in 10.** A coordinated migration needs Python ABI3/feature and Node ABI/package-loader proof on the selected platforms. Assess a narrowly justified napi-build change separately. [PyO3](https://docs.rs/crate/pyo3/0.30.1) · [napi](https://docs.rs/crate/napi/3.16.2) · [napi build helper](https://docs.rs/crate/napi-build/2.6.0) |
| Text/models `tokenizers` 0.20.4 (direct) and 0.22.2 (Candle); `ort` and `ort-sys` =2.0.0-rc.10 | Published 0.23.2 and ORT rc.15. GHSA-jrf5-f6jp-7vxc is an advisory to check against Rust versus Python package scope and actual pinned inputs. | **Keep current ONNX in 10** due to the opt-in dynamic provider, matched runtime ABI and vector equivalence. **Investigate tokenizer in 10/60** at the direct/fork seams, with exact token IDs, model outputs and download/refusal paths. [Tokenizer](https://docs.rs/crate/tokenizers/0.23.2) · [Advisory](https://github.com/advisories/GHSA-jrf5-f6jp-7vxc) · [ORT](https://docs.rs/crate/ort/2.0.0-rc.15) |
| `zip` 7.2.0 via Candle; defaults disabled in the qualified fork. `chrono` 0.4.45 via rmcp/schemars. `rustix` 1.1.4 via tempfile/which. `rand` 0.8.6 / 0.9.4. | Zip advisories include fixes at 9.0.0/9.0.5, published 9.6.1. Affected operations/features include extraction, AES, legacy compression and streaming readers. Rustix xattr issue fixed in 1.1.6. Chrono reports unmaintained, while locked rand lines meet checked advisory fixes. `libz-sys` is absent from this lock. | **Investigate zip in 60** against fork features and call sites. The observed product model route uses pinned safetensors. **10** may refresh rustix after scope review, keep rand, and record chrono as monitoring rather than a mandatory migration. [zip extraction advisory](https://rustsec.org/advisories/RUSTSEC-2026-0546.html) · [zip streaming advisory](https://rustsec.org/advisories/RUSTSEC-2026-0548.html) · [rustix](https://rustsec.org/advisories/RUSTSEC-2026-0486.html) · [chrono](https://rustsec.org/advisories/RUSTSEC-2026-0543.html) · [rand](https://rustsec.org/advisories/RUSTSEC-2026-0097.html) |
| Serialization, contract and hashing `serde` 1.0.228, `serde_json` 1.0.149, `jsonschema` 0.18.3, `schemars` 1.2.3, `sha2` 0.10.9/0.11.0, `toml` 1.1.3+spec-1.1.0 | Candidates 1.0.229, 1.0.154, 0.61.2, 1.2.3, 0.11.1 and 1.2.1+spec-1.1.0 respectively. | **Keep schema/digests in 10** outside a selected contained batch. A compatible patch still needs schema/refusal/encoding and stable identifier or fixture checks where it can change output. [serde_json](https://docs.rs/crate/serde_json/1.0.154) · [jsonschema](https://docs.rs/crate/jsonschema/0.61.2) · [sha2](https://docs.rs/crate/sha2/0.11.1) |
| CLI/platform `clap` 4.6.1, `dirs` 5.0.1, `dunce` =1.0.5, `fs2` 0.4.3, `getrandom` direct 0.3.4 (also 0.2.17/0.4.2), `libc` 0.2.186, `unicode-segmentation` 1.13.2 | Published candidates 4.6.8, 7.0.0, 1.0.5, 0.4.3, 0.4.3, 0.2.191 and 1.13.3. | **Keep in 10** unless an accepted need justifies a contained change. Preserve target startup, path alias/open ownership, versioned CLI behavior and FTS text boundaries. |
| Errors/logging/fixtures `thiserror` 1.0.69/2.0.18, `tracing` 0.1.44, `tracing-subscriber` 0.3.23, `tempfile` 3.27.0, `proptest` 1.11.0, `trybuild` 1.0.118 | Published candidates 2.0.21, 0.1.44, 0.3.23, 3.27.0, 1.12.0 and 1.0.125. | **Keep by default in 10**, aside from scoped patch choices and approved security resolution. Preserve typed errors, redaction, and the existing test oracles. |

The qualified cohort is `cudarc-fathomdb =0.20.0@343b04fa`, three
`candle-*-fathomdb =0.12.0@2ee39305` crates and `candle-kernels 0.10.2@182ceab0`
(K9). Repository HTTPS mirrors preserve the same logical Git IDs and exact
commits. An upstream `candle-core 0.11.1` and `cudarc 0.19.10` are published
comparison inputs, and confer no fork compatibility. Upstream `cudarc 0.17.8`
is in the lock via `ug-cuda`. The qualified Candle manifest makes `ug` optional
with weak CUDA activation. Verify active selected trees during 60 and keep
the single qualified cudarc family invariant. Registry packages need separate
source/checksum and downstream build proof because workspace patches do not
propagate. **Keep in 10, investigate in 60** with K1–K9 review, static CUDART,
typed library/runtime behavior, ARM CPU and Tegra pool/drop, and actual F32
kernel and installed product controls. [Pin policy](../../../../dev/design/pinned-override-rot-guard.md) · [Fork plan](../fork-maintenance-plan.md) · [Upstream Candle](https://docs.rs/crate/candle-core/0.11.1) · [Upstream cudarc](https://docs.rs/crate/cudarc/0.19.10)

## Python, TypeScript and tools

| Group and current resolution | Verified candidate or finding | Proposal and owner |
| --- | --- | --- |
| Shipped Python `requires-python >=3.10`, empty runtime dependencies. Build `maturin>=1.9,<2`, uv inventory 1.14.1. Ruff 0.15.17 and Pyright 1.1.410 are exact. | PyPI Maturin 1.16.0, Ruff 0.17.5 and Pyright 1.1.414. Checked-in `uv.lock` still labels the project 0.8.20. Owned setup installs manifest dev requirements, so lock entries are a snapshot, and do not attest actual tooling. | **Keep exact tools by default / investigate lock ownership in 10.** Review change separately from PyO3. Verify pin parity and older Python runner and binding compatibility. [Manifest](../../../../src/python/pyproject.toml) · [uv lock](../../../../src/python/uv.lock) · [Maturin](https://pypi.org/project/maturin/1.16.0/) · [Ruff](https://pypi.org/project/ruff/0.17.5/) · [Pyright](https://pypi.org/project/pyright/1.1.414/) |
| Python test and analysis groups: pytest 9.0.3, Hypothesis 6.152.9, PyYAML 6.0.3, NumPy 2.2.6/2.4.6/2.5.1, SciPy 1.15.3/1.17.1/1.18.0, NetworkX 3.4.2/3.6.1 and tomli 2.4.1. | Verified pytest 9.2.0, Hypothesis 6.170.15, PyYAML 6.0.3, NumPy 2.5.2 and NetworkX 3.7. Fresh SciPy/tomli candidates were unavailable in retrieved metadata. | **Keep marker branches in 10**; check selected compatible wheels on Python 3.10 and owned dev interpreter before any refresh. No scientific baseline change follows. [pytest](https://pypi.org/project/pytest/) · [Hypothesis](https://pypi.org/project/hypothesis/) · [PyYAML](https://pypi.org/project/PyYAML/) · [NumPy](https://pypi.org/project/numpy/2.5.2/) · [NetworkX](https://pypi.org/project/networkx/) |
| TS root package declares Node `>=25 <26`; resolved TS 6.0.3, @types/node 26.1.0, @napi-rs/cli 2.18.4. Repo pins Node 25.9.0, recovery pins npm 11.21.0. | Node 25 is EOL. Node 26.13.1 LTS is the November security release. npm 12.2.3 lists Node ≥26 among supported ranges and excludes 25. TS 7.0.2 attaches platform-specific compiler packages. npm shows N-API CLI 3.10.9 as an observed release; newer @types metadata unavailable. | **Select a coordinated Node LTS candidate in 10.** Check engine range, per-platform builds, glibc/ABI and exact-version installed loaders. Keep the qualified npm OIDC mechanism for 20 and advance CLI/TS majors only with separate proof. [Node EOL](https://nodejs.org/en/about/eol) · [Security release](https://nodejs.org/en/blog/vulnerability/november-2026-security-releases/) · [npm metadata](https://registry.npmjs.org/npm/latest) · [TS metadata](https://registry.npmjs.org/typescript/latest) · [N-API CLI](https://www.npmjs.com/package/%40napi-rs/cli?activeTab=dependencies) |
| Root npm toolchain via #257 includes js-yaml 5.4.1, smol-toml 1.8.0 and KaTeX 0.16.47. Mermaid CLI 11.16.0 has separate npm lock. MkDocs is exactly 1.6.1. | Review lock delta and applicable advisory ranges again after resolution; current MkDocs 1.6.1 was verified on PyPI. | **10** owns selected tooling maintenance. Keep root and renderer scopes distinct; require strict docs build and changed rendering behavior proof. [smol-toml advisory](https://github.com/advisories/GHSA-r4xh-jqrq-34v2) · [KaTeX advisory](https://github.com/advisories/GHSA-238p-pmpm-9mq7) · [MkDocs](https://pypi.org/project/mkdocs/1.6.1/) |
| Rust MSRV/CI 1.95/1.95.0; actionlint 1.7.12; ShellCheck 0.11.0; gitleaks 8.30.1; bootstrap's Cargo-installed lychee is unversioned at top level. | Rust 1.100.0 and lychee 0.24.2 confirmed upstream. The lint/secret-scanner release checks matched the pins. | **Keep in 10** unless a selected dependency requires different Rust or reproducibility requires pinning lychee. Preserve existing ownership, versions, hooks, scanners and Markdown validation. [Rust release](https://blog.rust-lang.org/releases/latest/) · [actionlint](https://github.com/rhysd/actionlint/releases/tag/v1.7.12) · [ShellCheck](https://github.com/koalaman/shellcheck/releases/tag/v0.11.0) · [gitleaks](https://github.com/gitleaks/gitleaks/releases/tag/v8.30.1) · [lychee](https://github.com/lycheeverse/lychee/releases/tag/lychee-v0.24.2) |

Model and export inputs remain exact in 10 and 60. BGE-small is
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`, TinyBERT is
`81d1926f67cb8eee2c2be17ca9f793c7c3bd20cc`, and the loaders verify the asset
hashes. The exporter has its own hash-locked Torch 2.4.1+cpu, ONNX 1.23.0 and
toolchain. ONNX identity includes graph/tokenizer digests. An exporter update
must reproduce the accepted bytes or raise a separate model/compatibility
decision with fresh evidence. [Embedder](../../../../src/rust/crates/fathomdb-embedder/src/loader.rs) · [Reranker](../../../../src/rust/crates/fathomdb-embedder/src/candle_reranker.rs) · [Exporter lock](../../../../dev/tools/onnx/export-requirements.txt)

## GitHub Actions pins

Inventory contains 15 third-party action names and 16 exact refs, including two
download-artifact refs. Source is the workflow and composite-action files.
Preserve immutable SHA pinning, permissions, runner support, trusted routes,
artifact identity and currently authorized release boundaries.

| Group | Tracked → observed upstream release | Proposal and owner |
| --- | --- | --- |
| General CI | checkout 7.0.0 → 7.0.1; rust-cache 2.9.1 → 2.9.3; paths-filter 4.0.1 → 4.0.3. cache 6.1.0 latest lookup unavailable. Rust toolchain action uses exact SHA + explicit Rust version; new action SHA unqualified. | **10** may select contained SHA changes after reviewing checkout credentials/ref, cache ownership, and path-routing tests. [Checkout](https://github.com/actions/checkout/releases/tag/v7.0.1) · [Rust cache](https://github.com/Swatinem/rust-cache/releases/tag/v2.9.3) · [Paths filter](https://github.com/dorny/paths-filter/releases/tag/v4.0.3) |
| Runtime setup | setup-node 6.4.0 and setup-python 6.3.0 → 7.1.0 each; maturin-action stays 1.51.0. | **Keep in 10** until action runtime/runner compatibility and candidate toolchains are known. Node LTS pin can be evaluated independently of action major. [Node action](https://github.com/actions/setup-node/releases/tag/v7.1.0) · [Python action](https://github.com/actions/setup-python/releases/tag/v7.1.0) · [Maturin action](https://github.com/PyO3/maturin-action/releases/tag/v1.51.0) |
| Artifact and Pages | upload 7.0.1 → 7.0.2; download 8.0.1 → 8.0.2, plus separately pinned 4.3.0 in CUDA release input jobs. deploy-pages 4.0.5 → 5.0.1 and upload-pages-artifact 3.0.1 → 5.0.0. | **10** may select patch SHA candidates with existing producer/consumer tests. Keep separate CUDA fetch and Pages pins until exact runner/security/artifact compatibility is established. [Upload](https://github.com/actions/upload-artifact/releases/tag/v7.0.2) · [Download](https://github.com/actions/download-artifact/releases/tag/v8.0.2) · [Pages deployment](https://github.com/actions/deploy-pages/releases/tag/v5.0.1) · [Pages upload](https://github.com/actions/upload-pages-artifact/releases/tag/v5.0.0) |
| Publish/auth | PyPI action 1.14.0 → 1.14.3; crates.io auth 1.0.5 → 1.0.6; action-gh-release 2.6.1 → 3.0.4. | **Keep in 10** pending trusted-publisher/finalization evidence. A rehearsal that skips publishers cannot establish their behavior. [PyPI](https://github.com/pypa/gh-action-pypi-publish/releases/tag/v1.14.3) · [Crates auth](https://github.com/rust-lang/crates-io-auth-action/releases/tag/v1.0.6) · [GitHub Release](https://github.com/softprops/action-gh-release/releases/tag/v3.0.4) |

## Decisions for Slice 9 and later execution

1. **Include in 10:** select narrow root Markdown and TLS root fixes and the
   Tokio fix, with separate reviewable lock deltas. Select Node 26 LTS
   investigation with a compatible resolution and platform proof before
   retiring the Node 25 toolchain. Keep native ABI/SQLite/model pins.
2. **Select or defer in 10:** decide whether to take the ONNX exporter and
   Mermaid maintained-graph items now, and which contained tool/Action patches
   merit work. Keep routine Dependabot version updates paused by default.
   Recheck the queue, private alerts if access is available, and advisories
   immediately before changing locks.
3. **Own qualified fork changes in 60:** select rebase and active feature
   graph deliberately. Qualify owned fork commits before changing FathomDB
   pins. Registry or kernel source changes must prove exact consumer bytes,
   architecture coverage and separate publication authority.

For execution, capture current manifests/resolutions first and review the
selected change in its existing checkout-owned environment. Inspect the exact
lock and feature diffs, isolate native/API changes, run scoped regression and
packaging checks, and use the one final coherent-phase gate where required.
Keep unchanged proof by actual inputs. This review checked high-priority
advisories and material direct/transitive candidates. Full advisory-database
and platform graph scans remain execution work. Direct registry HTTP in the
shell was unavailable; web registry/upstream reads and connected PR queries
supplied the cited results. Unavailable registry tags are unresolved.
