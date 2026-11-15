# Slice 30 read-only input inventory

Historical PhaseA read-only observation against
c0c94b4203ffeb9e573efcc14e8b10df653318c5 on2026-11-11; at that phase no
model/product/native/benchmark run, input acquisition or live Memex read ran.
Subsequent owned LME preparation and P execution are recorded below and in
[results](results-p.md). No model/GPU or live Memex use occurred.
Hashes below identify existing bytes; they are not a new receipt-hardening suite.

## LongMemEval available

Existing read-only source:
`/home/coreyt/projects/fathomdb/data/corpus-data/raw/longmemeval-cleaned/longmemeval_s_cleaned.json`.
277383467B; SHA256
`d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442`.
[Registry](../../../../../tests/corpus/scripts/manifest.json) pins
`xiaowu0162/longmemeval-cleaned` revision
`98d7416c24c778c2fee6e6f3006e7a073259d48f`.
Adjacent LICENSE.txt is1062B, SHA256
`d3c4b9aa54759df6ded337978a6f3b55b75615e5e4525c3b82d7e2627d4b9732`,
MIT, copyright2024 Di Wu; matches the registry. Keep attribution with any copy.

Read-only JSON inspection reproduced historical converter order:500questions,
19195 first-seen sessions,189514 unique stripped nonempty turns,10 empty turns,
9985 repeated turn texts after session deduplication,0 mismatched ID/session
array lengths. First100000 bodies total99124130UTF8 bytes; body byte
min/median/p99(nearest-rank)/max=1/440/3487/42347. This workload contains long
whole records; no chunking or claim of full BGE coverage beyond stored spans.
Default Python json.dumps string plus newline gives selected100k SHA256
`cc5930e525a2af2954353f92dacf720cc4b79f40dbb7965c44bb5c985b5cb481`
and all500 question-lines SHA256
`0eeb5582fc93bee7e2a494054a55d26bb0940d771083b726f999a9682e74dc16`.
These are a cross-check, not autonomous golden regeneration. Actual converted
files/product match counts remain unproduced until authorized tools execute.

After GO, copy these static raw/license inputs into own/mnt/data input directory;
do not borrow mutable cache paths for execution. Source copies stay untouched.
No download is necessary. Matched-row/query compiler statistics, relevance fit,
and actual runtime truncation/coverage remain measurement outputs, not facts
inferred from the raw byte distribution.

## BGE available, execution not yet qualified

Existing read-only snapshot:
`/home/coreyt/.cache/huggingface/hub/models--BAAI--bge-small-en-v1.5/snapshots/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`.
All files match [loader pins](../../../../../src/rust/crates/fathomdb-embedder/src/loader.rs):

| File | Bytes | SHA256 |
| --- | --- | --- |
| config.json |743|094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750|
| tokenizer.json |711396|d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66|
| model.safetensors |133466304|3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad|

Current Candle default identity is fathomdb-bge-small-en-v1.5, dimension384,
512token window, accepted model revision above. Use the already accepted default
model locally; no new model selection, export, redistribution or license ruling.
The snapshot itself has no separate LICENSE file; do not invent a fresh license
verification claim. Own model cache copies follow the existing loader path and
pins, never runtime borrowing from sibling caches.

## Host and capacity

/mnt/data is mounted ext4 on/dev/nvme0n1p2; read-only df showed690799476736B
available (~643.4GiB). This volume is available. The separate historical cold
drive C8D47C11D47C03CA is missing; it is not/mnt/data.
Sandbox nvidia-smi failed with cannot-communicate; unchanged scoped unconfined
query exited0 and showed two RTX3090s, each24576MiB/2MiB used/0%util,
R580.178.04, plus display K620. Primary CUDA resolves/usr/local/cuda-12.6.
This is point-in-time inventory, not reservation or engagement. Before G cells,
recheck/claim one3090 and prove actual memory+utilization engagement. Never use
K620 or change driver/toolkit. Quiet CPU/disk and GPU lock ownership are not yet
reserved; no other campaign is stopped or modified.

## Frozen TC5 located and statically qualified

Owner seq382 supplies the actual existing directory
`/mnt/data/fathomdb-performance-benchmarking/scale-02/inputs/input-pack-v1`.
November14 read-only validation matched the registered index and qualified
manifest hashes below, plus full `input-manifest.v1.json` SHA256
`1d1aee57b20f38ed39dc780c44d12be6ec1a18f64f02dbf8ae5fcf79d5f72795`.
All17372 referenced payloads are readable contained regular files with matching
content hashes (29184099B total). Canonical static qualification confirms17272
documents/100queries and deterministic7667-document bridge identity
`9e92d236e44fc7443c1940f6870877a6e6eb07e92136ca2da331f88221e622ed`.
Missing/mismatching references: zero. No further static input qualification is
required. Source-extracted static helpers performed no native/module import,
database open, mutation, regeneration, model or GPU execution. The initial helper
extraction omitted a dataclass decorator; correction made both complete static
validations pass. No benchmark runtime or current candidate proof is claimed.

The missing-input hold is cleared. F/Q8 still need current candidate-bound
native/model/runtime qualification and actual measurements under their selected
protocol. The absent-path observations below are historical, not live requests
to reconnect a drive or search for another copy.

November14 filename/path lookup found the registered durable directory:
`/home/coreyt/projects/fathomdb/data/performance-benchmarking/scale-02/inputs/input-pack-v1/`.
The performance-benchmarking symlink targets
`/media/coreyt/C8D47C11D47C03CA/data/linux-cold/fathomdb-performance-benchmarking`.
Search for `tc5/tc5-corpus-input.v1.json` with sibling
`tc5-qualified-manifest.json`; the older manifest name is
`tc5-manifest.all-real.v2.json`. Preserve the entire referenced `tc5/` payload,
not just its JSON index. The full frozen input pack has17272 documents and100
queries; the benchmark selects its7667-document bridge deterministically.
Registered input-index SHA256 is
`624b6b42e7e1d40866fea48d37a34b0ef8d786f46f4aa2ed9a3dd245d22a0a0a`;
qualified-manifest SHA256 is
`f6180a00d1551a143a7445aa6ed28ed589533400ee702835663729694e393df5`.
These are source input identities, distinct from derived bridge/ground-truth and
benchmark-database digests. This lookup did not search mounted volumes, open a
database, locate the missing payload or regenerate any benchmark data.

Parent's scoped search of/mnt/data/tmp2 and~/.cache/fathomdb-comp found no exact
frozen index624b6b42… or qualified manifestf6180a00… and referenced pack from the
historical cold drive. No repeated broad search or regenerated substitute follows.
The tc5-10000 convenience pack is a distinct fixture. F cells require the actual
accepted frozen7667docs/100queries/K192/top10/exact-f32 bridge inputs and current
candidate-bound CUDA/native identities. LME/P/G latency cells do not consume
that pack. Parent retains the exact-artifact availability question; do not ask
whether/mnt/data exists or require reconnecting a drive for unrelated text work.

## Actual owned P inputs

Only registered LME/license bytes were copied into the assigned/mnt/data task.
The committed converter produced100000selected turns/500questions. Corpus SHA
cc5930e525a2af2954353f92dacf720cc4b79f40dbb7965c44bb5c985b5cb481 and query SHA
0eeb5582fc93bee7e2a494054a55d26bb0940d771083b726f999a9682e74dc16 were frozen
before16Pcells and checked unchanged afterward. Actual source/native/method
identities and raw paths are in [results](results-p.md). Models were neither
copied nor invoked; the exact frozen TC5 pack remains absent/held, not replaced
by LME or the convenience10k corpus. /mnt/data remained available throughout.

## G/D continuation inventory

The [G/D continuation](continuation-gd.md) retains the accepted LME/model pins and records
a bounded November13 known-path metadata check: the registered exact TC5 index
and qualified-manifest paths remain absent; the existing BGE snapshot path exists.
This is neither model execution nor proof that no other TC5 copy exists. No
new dataset/model acquisition or database open was performed.
