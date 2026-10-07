---
title: Slice 135 installed TypeScript capability exercise
status: SELECTED_FUNCTIONAL_39_OF_44
target_release: 0.8.27
---

# Installed TypeScript capability exercise — 2026-10-07

The [canonical operation map](../../../../../../../src/conformance/governed-operation-parity.json)
has 44 live IDs (SHA-256 `da673d3de1c7962e6ccd1c0d2d2c552cf3941338b5e4cd67f13b7ad87ab9bb17`).
The [final raw receipt](final.json) accounts for every ID:

| Outcome | Operations |
| --- | ---: |
| Executed with selected real-database assertions | 39 |
| Failed | 0 |
| Gap | 2 |
| Unavailable provider/model condition | 3 |
| **Supported** | **44** |

The package was built from candidate product source
`ddd05221a5dd38bd076bfad48e7f57717d8af791`; the runner checks that
`src/rust`, `src/ts/src`, the package manifests and `Cargo.lock` have no diff
from that commit. The package metadata still says `0.8.26`. The locally
packed main archive SHA-256 is
`13c5c384b401ec3d7c66c5b28473b17877bd5f896c2b85d8aa7d0605b1cf4f54`;
the Linux x64 GNU native archive is
`90e070acfe403ca47280a5166be0e3fc35526c6f60873105626a1d6e2b2174da`.
The installed `.node` SHA-256 is
`2d26e58cda598d702c3dc333beaeec355e268c093c342fbf1efd26e4ccd5186d`.
The main installed module SHA-256 is
`cc0ea0b2ae641aa5162a223985d9afb5ba1233b073899e66a2847f92f36bf432`.
The [SHA manifest](SHA256SUMS) covers every retained receipt and archive.

Node v25.9.0 and npm 11.12.1 ran the isolated consumer. The
[runner](../../../../../../../scripts/slice135_ts_capabilities.mjs) adapted only
compiled test imports to `fathomdb`; two evidence tests deferred temporary
directory cleanup so their real SQLite databases could be independently
reopened. The pagination adaptation disables an unused internal binding
import for its two selected real-database tests. The original TypeScript test
source files and assertions were unchanged. All 41 selected named tests
passed exactly once through the installed package and native addon, producing
41 SQLite databases. Independent reopen checks compared 110 active canonical
bodies with SDK reads and checked a guaranteed absent ID. Another real
database backed 27 direct typed-refusal or idempotence controls; the exact
error class and available reason/field path are in the raw receipt. There
were no mocks of the database or native addon.

`engine.read_dependency_closure` remains a **gap**: an installed test asserted
typed absence and refusal, but no committed closure result. `engine.trace_dependency`
remains a **gap**: an installed test asserted schema refusal, but no successful
trace result. Qualified extraction and consolidation providers and a
standalone cross-encoder reranker were unavailable. The default embedder
itself did run for `engine.embed`.

The [live audit](live-audit.json) checked installed files byte for byte
against both npm archives, each selected test source and adapted test hash,
all 44 rows, method spellings and the reopen links. The [retained archive audit](retained-audit.json)
rechecked the committed archives and [adapted tests](adapted-tests.tgz)
without relying on the temporary consumer. Both rejected controls with a
missing operation, false count, changed archive digest and misattributed
positive route.

Earlier [attempt 1](attempt1.json), [attempt 2](attempt2.json),
[attempt 3](attempt3.json) and [attempt 4](attempt4.json) retain harness
qualification failures or attribution corrections; none count as product
results. The final run recorded no performance timing, full contract
coverage, platform matrix, contention or paired 0.8.26 result.

Exact commands, from the repository root except the native build:

```sh
cd src/ts
npm ci --offline --ignore-scripts --no-audit --no-fund
/home/coreyt/.nvm/versions/node/v25.9.0/bin/node node_modules/@napi-rs/cli/scripts/index.js build --platform --release --cargo-cwd ../rust/crates/fathomdb-napi --features default-embedder --js false
cd ../..
bash scripts/slice135_ts_capability_consumer.sh /tmp/slice135-ts-capability-sealed "$PWD/src/ts/fathomdb.linux-x64-gnu.node" ddd05221a5dd38bd076bfad48e7f57717d8af791
python3 scripts/slice135_ts_capability_audit.py --raw /tmp/slice135-ts-capability-sealed/raw.json --canonical src/conformance/governed-operation-parity.json --output /tmp/slice135-ts-capability-sealed/audit.json
```

The retained audit can be replayed from this directory with
`--raw final.json --canonical src/conformance/governed-operation-parity.json
--receipt-dir dev/plans/0.8.27/features/slice-135/results/2026-10-07-ts-capability-exercise
--repo . --output /tmp/slice135-ts-retained-audit.json` from the repository root.
