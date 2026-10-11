---
title: Slice 135 TypeScript native suite and installed consumer qualification
status: NARROW_FUNCTIONAL_SMOKE
target_release: 0.8.27
---

# TypeScript native and installed consumer qualification

The checkout was at `9597dff3a5d57969fccdd726d6d5b8fd42f4876f` on
`llm/0.8.27-slice-135`. Product sources under `src/rust`, `src/ts` and
`Cargo.lock` were byte-identical to the clean candidate source commit
`b2ac8081e79a6e626d01f5f97331be331f1cc16d`. The package still declares
version `0.8.26` while the release branch is being qualified for 0.8.27.

Node `v25.9.0` and npm `11.12.1` were used. Dependencies came from the
lockfile with `npm ci --offline --ignore-scripts --no-audit --no-fund`; its
SHA-256 is `36ed30648b87f581f4abb608ff1774dbcd5a79e7b3c94524d2bcf55749eb5709`.
The native debug artifact was built by `npm test` with `test-hooks` and
`default-embedder`; its SHA-256 is
`c068038f946fda625da1e4abc00f83655201a95edbf0408a9a3e14f3e89bb359`.
The artifact is local and is not a published package or release build.

`FATHOMDB_SKIP_NETWORK_TESTS=1 npm test` passed. A retained direct TAP run of
the compiled suite has **474 passed, 0 failed and 6 skipped**; see
[native-suite.tap](native-suite.tap). Its real database tests cover many
search, read, graph, lifecycle and error cases, but the suite also includes
pure shape and fixture tests. The skip setting omits default embedder cases.

The existing `scripts/slice120-node-consumer.sh` then compiled the public
TypeScript entry point, packed the main and Linux x64 GNU packages, installed
both into an isolated offline consumer and ran
`src/ts/tests/fixtures/consumer-package-root.mjs` against a real temporary
database. It passed open, configure, write, read, search, graph neighbors,
frozen context, an expected evidence error and close. See
[installed-smoke.log](installed-smoke.log). The script SHA-256 was
`4c6344b953f70c97062d1473b167e4c8a1293c523a01153e23b4ff3429c035be`;
the consumer fixture SHA-256 was
`2ae34d7f5ce3c847d0822b4fffccd0364d0e92056993d624d5532b7f8c573a4b`.
The temporary package archives were removed by the existing script after
the run; their digests remain in the log but cannot be independently
rechecked from retained bytes.

These results establish that the candidate can execute the ordinary
TypeScript suite and a **narrow installed-package real-database smoke**.
They do not qualify installed TypeScript S01/S02 latency, the whole operation
register, the release package, a 0.8.26 comparison or the full Phase 1
checkpoint. The broader functional gate remains open.
