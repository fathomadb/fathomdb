---
title: Slice 10 implementation evidence
status: ACTIVE
target_release: 0.8.30
slice: 10
date: 2026-11-18
---

# Slice 10 implementation evidence

Baseline `7dd493e634033e98db5c66db862510923f089e8f`.
The [design and allocation](design.md) passed its [Astra-medium re-review](planning-review.md).
The independent full developer gate and integration are pending the parent.
The source and platform evidence here is local and uses existing compute.

## Security and runtime

| Area | Result |
| --- | --- |
| Markdown | Root npm ci and npm ls resolved markdownlint-cli2 0.24.0, markdown-it 15.0.3, js-yaml 5.4.3 and smol-toml 1.9.1. Root and public Markdown lint, current pin policy, pin policy fixtures (including 16 unittest cases), guarded neutral formatting and semantic-change refusal all exited 0 at their expected checks. |
| TLS roots | Both opt-in model feature graphs resolve ureq 2.12.1 / rustls 0.23.46 / rustls-webpki 0.103.16 / webpki-roots 0.26.11 → 1.0.12. The thin graph excludes those edges. Both model loaders use the default TLS connector through their ureq builders. The exact-lock disposable probe exited 0. It returned HTTP 200 at npm's trusted registry endpoint and refused the name-matched self-signed localhost certificate as `UnknownIssuer`; the server observed `TLSV1_ALERT_UNKNOWN_CA`. The affected anchor was present in 1.0.7 and absent in 1.0.12. |
| Model bodies | Loader HTTP/cache suite 12/12, reranker policy 10/10, cached TinyBERT CPU scoring 5/5, all exit 0. The real batch/per-pair comparison observed max absolute difference 0 on its base fixture and `1.9073486e-6` across batches. Reranker model-network transfer was outside this proof. |
| Tokio | 1.54.1 is locked with macros 2.7.0, rmcp 3.5.1 and httpmock 0.8.3. The unchanged serial MCP admission/lifecycle/process/protocol run exited 0 with 29/18/3/11 cases. The first parallel run failed during store open, as retained below. The loader HTTP run exercised the httpmock/hyper/h2 Tokio graph. TypeScript scopes are below. |
| Development and release roles | Node 26.13.1 / npm 11.20.0 selected in the development manifests, CI and native receipt. Held Node 25.9.0 and separately staged exact npm 11.12.1 successfully ran offline npm ci on the new TS manifest and original lock (4 packages). Tegra build and static guard REDs `b6c0e6292` / `c708c2bf5` became GREEN `76e199bb4` / `3ddc3a13a`. The wrapper rejects either wrong release version. CUDA static suite reported 123 PASS rows. Development selector RED `2600ec03d` became GREEN `94519bb97`. Actionlint, five shell selector/shape suites (18/13/14/1/11 PASS rows), Node support unittest 3/3 and Python workflow shape 6/6 exited 0. Scoped ShellCheck passed. |
| Owned Python | Python 3.12.3, Ruff 0.15.17 and Pyright 1.1.410. The checkout-owned noneditable ABI3 wheel provision and the actual environment tool check passed. Missing, foreign, stale and default ownership fixtures passed 4/4. |

Initial independent Sol-high review of `a6cf54534bc7323d8f0018c7c2721b7263000640`
found two lock-scope issues. Both offline and online Cargo resolution selected
older compatible Windows-only edges for errno, rustix, tempfile and winapi-util.
Commit `85e597ecf` restores their baseline 0.61.2 edges and was checked with
`cargo tree --locked --workspace --target x86_64-pc-windows-msvc -e normal,dev`.
Commit `195ed9c34` restores the compatible ansi-regex 6.3.0 and
get-east-asian-width 1.6.0 npm resolutions, verified by npm ci and npm ls.
The security and runtime source re-review through `94519bb97` returned PASS.
Final Sol-high code and documentation re-review passed through
`b79b5ca05baf26b80651a1f4e8f979de8302edf8` after correcting reserved source
ID erasure, the Python build entry point, the ONNX CPU qualification scope
and the historical approval wording.

The first parallel MCP run exited 101 at admission (28 passed, 1 failed).
It stopped before the other binaries. The failure was before the cancellation
assertion, under concurrent compilation. The smaller rerun used the same
selection with `--test-threads=1`. The startup timing failure is a lead for
Slice 50's bounded robustness hunt if it recurs.

```text
---- a_client_cancel_of_a_running_call_abandons_it stdout ----

thread 'a_client_cancel_of_a_running_call_abandons_it' (2360043) panicked at src/rust/crates/fathomdb-mcp/tests/common/mod.rs:248:5:
assertion `left != right` failed: orient {"store":"a"} failed: {"error":{"class":"not_ready","code":"opening","message":"store \"a\" is not ready: opening","remedy":"retry shortly; the call waited until its deadline"}}
  left: Some(true)
 right: Some(true)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

## Installed native SDK

Clean source `3fb4b2af1febbcc54f92ed5d1a8426063ad35bbe`, Rust 1.95.0,
Node v26.13.1, npm 11.20.0, Linux x64 glibc 2.39 and Node-API 10.
npm ci, TypeScript typecheck, release build and both packs exited 0.
The thin main tarball had 44 files. The platform tarball had 4 files and
carried the matching release-mode addon. Package manifests were 0.8.29 as
stamped in this source checkout.

| Input | SHA-256 |
| --- | --- |
| Cargo.lock | `016b8368ed5ca872f541f5526ca9c312c921885e1bfd6336ac541ffe0f026e6d` |
| TypeScript package-lock.json | `3dcd88a2ef6173acb810150b73b52682433917d4e62e7f3f86b53fb485a8fe92` |
| fathomdb-0.8.29.tgz | `bd6e922cbd3d96d0d4d9b5fa93acbf58e18bf87d61712b0ce0f1bc48bb3ca2bb` |
| fathomdb-linux-x64-gnu-0.8.29.tgz | `289a9e9f2aa78b25c107d3ff27127ef216c1e0bec243313a6ffb3f732dea73ca` |
| Built and installed addon | `d61de587c2bd1779c85a1b67227be688371d4e8f2ab06f1688c7edea1bd2a68f` |

Installed both tarballs with npm offline in a fresh outside-source consumer.
The unchanged `>=25 <26` package engine produced the expected `EBADENGINE`
warning. The first scratch invocation called `engine.read.get` and failed.
The corrected probe used `read.get(engine, id)` with the unchanged tarballs
and a fresh database. Both bounded Node processes exited naturally with exit 0.
PID 2405136 wrote and read the exact UTF-8 body, read a missing ID as null, and
closed. PID 2405189 reopened the same store, read the same body and missing
result, and closed. Both resolved the staged digest at the fresh consumer's
`node_modules/fathomdb-linux-x64-gnu/fathomdb.linux-x64-gnu.node` and confirmed
that path was loaded. To reproduce, run `npm run build` in `src/ts`, stage the
compiled JS in the main package and the matching ELF addon in the Linux x64
platform package, then `npm pack --offline` each staging directory. Install
both tarballs together with `npm install --offline --ignore-scripts` in a
new consumer. Use the [existing canonical write shape](../../../../../scripts/release/smoke/smoke-npm-package.sh),
`Engine.open` with `useDefaultEmbedder: false`, `read.get(engine, id)` for
the written and missing IDs, and a second process for the same-store read.

Direct Terra independently verified the same tarball and addon digests in a
fresh outside-source Linux x64 consumer. Offline install, process 38's
write/read/close and process 84's reopen/read/close all exited 0. Terra
observed the same body, null for the missing ID and loaded addon path.
Evidence is in `/tmp/fathomdb-0830-terra-node/consumer.TuC000/result.json`.

The hermetic build check executed both debug and production configurations,
and exited 0. The five focused TypeScript suites passed 52/52 with zero
skips. The broader offline `npm test` exited 0. The runner reported 643
selected, 637 passing, zero failing and six declared skips. Its output also
contains 15 explicit test-internal skip messages for disabled network or
release-surface checks. Those branches remain unrun.

## Tooling triage and documentation

- Exporter certifi 2026.10.18 is the sole selected exporter update. The exact
  wheel digest is recorded in the lock. Disposable CPython 3.12 hash-locked
  install, pip check and all three verified cached safetensors inputs passed.
  Two offline CPU exports reproduced the accepted 133111201-byte graph
  (`c92689ecdefc35c3b533f68449cf1a50e7a1301736d8031372b0fa70e862c5a5`) and
  tokenizer (`d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66`).
  Hold #256 Transformers, #259 filelock and #260 setuptools. This export
  takes the local BERT load path, uses no chat-template save, and the installed
  Hugging Face helper imports ordinary FileLock/SoftFileLock, not
  ReadWriteLock. The selected wheel-only reproduction is Linux. Current
  Torch's pickle-loading advisory retains the pinned safetensors prerequisite.
- Renderer #258 was already satisfied for its cited advisory by DOMPurify
  3.4.17 at `c484a461e`. #261's supported disposable CLI 12.2.0 /
  Mermaid 12.3.0 / parser 2.0.1 trial rendered 21/21 controls and passed
  its real sanitizer test with cached Chromium 150.0.7871.24. npm audit
  exited 1 with four high entries. Puppeteer 25.3.0 still resolves affected
  Zod 3.25.76 via chromium-bidi 16.0.1, even though the Mermaid parser no
  longer uses Zod. The officially paired browser for Puppeteer 25.16.0 is
  156.0.8078.47 and is absent locally. Retain tracked renderer pins and the
  trusted local-diagram scope. The tooling owner needs a patched supported
  graph and compatible browser startup/render proof to adopt a fix.
- The [receiving allocations](design.md#receiving-requirements-and-acceptance)
  govern follow-up proof. Slice 20 owns installed-consumer checks and the
  publishing/OIDC route. Slice 70 consumes those checks before promoting Node
  engine or platform support. Slice 80 owns public artifact verification.
  Windows, macOS, AArch64, Tegra, CUDA and manylinux delivery were outside
  this local candidate proof. The repository's private Dependabot inventory
  was unavailable.
- Root Markdown lint, public docs lint, generated docs check and strict MkDocs
  passed. The public-doc-truth fixture suite exited 1 with `docs/install/typescript.md says unpublished documentation target 0.8.27 is published`
  after two passes. The exact result reproduced byte-for-byte on the frozen
  baseline's tracked inputs. This synthetic pending-publication fixture and
  the known .29 terminal-state fixtures remain for Slice 20.
- Root reconciled the optional pending-documentation setting in
  `51f20be1c77f0fcb1f633ea0609e05610fbdfbe3`. The live public-doc-truth
  checker selects published 0.8.29 and passed after this metadata merge.
  The release-state generated views passed. Publication-facing runtime
  compatibility remains with the published packages.
