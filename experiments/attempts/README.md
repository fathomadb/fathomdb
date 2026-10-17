# COMP external attempt data

Non-database contents of the COMP-01/02/03 external attempt roots that the
receipts in `experiments/runs/` bind by digest. Each archive packs one attempt
root from `~/.cache/fathomdb-comp/<root>/`, packed on 2026-10-17, as
`<root>.tar.gz` with `<root>/` as its top-level directory.

A receipt binds its attempt by the SHA-256 of the attempt's `summary.json`
(`artifacts[].summary_sha256`) or `manifest.json` (the `manifest` artifact).
The binding column below is that digest, computed over the archived file, so
each archive is checkable against its receipt. Receipts keep their digest-only
artifacts; no receipt locator was changed.

## Format

- Gzip-compressed POSIX tar, members in sorted path order, original file
  mtimes, owner 0, mode 0644, gzip header mtime 0.
- One archive per attempt instead of loose files: the 33 roots hold 11,648
  small JSON files, which would nearly double the repository's tracked-file
  count, and the per-question files carry model output that the experiment
  conventions keep out of the browsable tree.
- Extract with `tar -xzf <root>.tar.gz`, or read in place with Python's
  `tarfile`. Verify with `sha256sum -c` against the table below.

## Exclusions

- Every database and store: `*.sqlite*`, `*.db`, `*.lance` files and
  directories, WAL and SHM files, and lock files. Each latency root held
  `fathomdb.sqlite` and `fathomdb.sqlite.lock`; nothing else was skipped.
- Every `work/` directory (all were already empty).
- The input packs `tc5-10000`, `tc5-17272`, and `nfcorpus-input`, which the
  configs pin by digest. Each rebuilds byte-for-byte from the local external
  corpus root (checked 2026-10-17): `python -m experiments.comp02_inputs --pack
  data/performance-benchmarking/scale-02/inputs/input-pack-v1/tc5 --size N
  --output DIR` for N = 10000 and 17272, and `python -m
  experiments.comp01_nfcorpus --source data/corpus-data/raw/beir/nfcorpus
  --output DIR`.
- The LongMemEval-S cleaned dataset the COMP-03 runs read, which the configs
  pin by digest.

## Archives

### COMP-01 NFCorpus campaigns

| archive | files | bytes | binding | receipts |
| --- | --- | --- | --- | --- |
| `comp01-run1.tar.gz` | 10 | 424939 | `manifest.json` `1e4af2dff21718c4` | `comp-01-comp01-3633-20261016T1240Z-b9aa1214` |
| `final-comp01.tar.gz` | 10 | 425967 | `manifest.json` `026bec939836b05f` | `comp-01-comp01-3633-20261016T1755Z-b9aa1214` |
| `final-comp01-hybrid.tar.gz` | 10 | 454432 | `manifest.json` `498d2404ed0a6107` | `comp-01-comp01-hybrid-3633-20261016T1755Z-c1693bec` |

### COMP-02a ingest-scale campaigns

| archive | files | bytes | binding | receipts |
| --- | --- | --- | --- | --- |
| `comp02a-10000.tar.gz` | 9 | 2497169 | `manifest.json` `836b66cd930c3652` | `comp-02a-comp02a-10000-20261016T1240Z-09ac6bd7` |
| `final-comp02a-10000.tar.gz` | 9 | 2497155 | `manifest.json` `3edddc188d700d94` | `comp-02a-comp02a-10000-20261016T1756Z-09ac6bd7` |
| `comp02a-17272.tar.gz` | 9 | 2499511 | `manifest.json` `f42b6846ff3ce188` | `comp-02a-comp02a-17272-20261016T1240Z-eede35ef` |
| `final-comp02a-17272.tar.gz` | 9 | 2498790 | `manifest.json` `797c8bf9489821e6` | `comp-02a-comp02a-17272-20261016T1756Z-eede35ef` |

### COMP-02b filtered campaigns

| archive | files | bytes | binding | receipts |
| --- | --- | --- | --- | --- |
| `comp02b-10000.tar.gz` | 9 | 680862 | `manifest.json` `2bdbc1b3d6d911ff` | `comp-02b-comp02b-10000-20261016T1240Z-d91933b9` |
| `final-comp02b-10000.tar.gz` | 9 | 679743 | `manifest.json` `8854341798db1b88` | `comp-02b-comp02b-10000-20261016T1757Z-d91933b9` |
| `ui-s40-comp02b-10000.tar.gz` | 9 | 679709 | `manifest.json` `1e63beef0292ded5` | `comp-02b-comp02b-10000-20261017T1510Z-19d8c470` |
| `ui-h3-comp02b-10000.tar.gz` | 9 | 680454 | `manifest.json` `ea19889d9e3acfc9` | `comp-02b-comp02b-10000-20261017T1659Z-19d8c470` |
| `ui-base-comp02b-10000.tar.gz` | 9 | 680579 | `manifest.json` `b68d213f03c2fe36` | `comp-02b-comp02b-10000-20261017T1700Z-19d8c470` |

### COMP-02b latency

| archive | files | bytes | binding | receipts |
| --- | --- | --- | --- | --- |
| `final-comp02b-latency.tar.gz` | 1 | 1090 | `summary.json` `acf09ed07d52dc46` | `comp-02b-latency-20261016T1700Z-4f96fad0` |
| `ui-s40-latency.tar.gz` | 1 | 1320 | `summary.json` `81224c220a8422de` | `comp-02b-latency-20261017T1509Z-2412d988` |
| `qc-comp02b-latency.tar.gz` | 1 | 1320 | `summary.json` `b04baebc8099d1a4` | `comp-02b-latency-20261017T1653Z-2412d988` |
| `ui-h3-latency.tar.gz` | 1 | 1324 | `summary.json` `8455997a65c96da4` | `comp-02b-latency-20261017T1654Z-2412d988` |
| `ui-base-latency.tar.gz` | 1 | 1321 | `summary.json` `d5ba7c74cf073e76` | `comp-02b-latency-20261017T1655Z-2412d988` |
| `uh-h2-comp02b-latency.tar.gz` | 1 | 1094 | `summary.json` `29b8927133cd9299` | `comp-02b-latency-20261017T1656Z-4f96fad0` |
| `uh-head-comp02b-latency.tar.gz` | 1 | 1096 | `summary.json` `ee0aff4d57e9fca6` | `comp-02b-latency-20261017T1657Z-4f96fad0` |
| `ab-h2b.tar.gz` | 1 | 1092 | `summary.json` `a7d1d64d3ee49299` | `comp-02b-latency-20261017T1658Z-4f96fad0` |

### COMP-03 answer and retrieval-sweep runs

| archive | files | bytes | binding | receipts |
| --- | --- | --- | --- | --- |
| `comp03-run1.tar.gz` | 362 | 250514 | `summary.json` `f129662f6055d604` | `comp-03-hermes-answer-20261016T1239Z-2e410a4f` |
| `comp03-run2.tar.gz` | 542 | 437224 | `summary.json` `2eab2a2baa1844e8` | `comp-03-hermes-answer-20261016T1422Z-c807ed90` |
| `comp03-run-v3.tar.gz` | 542 | 494294 | `summary.json` `3388d68657e4ccd5` | `comp-03-hermes-answer-20261016T1610Z-33d78241` |
| `comp03-run-v3rep.tar.gz` | 542 | 492639 | `summary.json` `1e778f1fbd6d6fde` | `comp-03-hermes-answer-20261016T1732Z-33d78241` |
| `comp03-run-v4a.tar.gz` | 542 | 484260 | `summary.json` `f3e68bc9d5160c05` | `comp-03-hermes-answer-20261016T1609Z-22869ddc` |
| `comp03-run-v4b.tar.gz` | 542 | 483595 | `summary.json` `afa18150030585c9` | `comp-03-hermes-answer-20261016T1609Z-1c856c71` |
| `comp03-run-v4c.tar.gz` | 542 | 487239 | `summary.json` `dc0424a6cdb27b87` | `comp-03-hermes-answer-20261016T1732Z-50579949` |
| `comp03-run-mix.tar.gz` | 542 | 486656 | `summary.json` `533231e82452f3e9` | `comp-03-hermes-answer-20261016T1747Z-4e56afa9` |
| `comp03-sweep-d3.tar.gz` | 1503 | 1128795 | `summary.json` `130e00a6c4c6013f` | `comp-03-retrieval-sweep-20261016T1406Z-17773486` |
| `final-comp03-sweep.tar.gz` | 1503 | 1167183 | `summary.json` `efac4382b779eee6` | `comp-03-retrieval-sweep-20261016T1725Z-17773486` |
| `comp03-run-250.tar.gz` | 1502 | 1596640 | `summary.json` `c57d0ab3142b0631` | pending: `comp-03-hermes-answer-20261017T1216Z-3bd3546c` |
| `pa-m3.tar.gz` | 362 | 540382 | `summary.json` `a5134b1c30545ac6` | pending: `comp-03-hermes-answer-20261017T0308Z-0f2856e9` |
| `pa-combined-sweep.tar.gz` | 2503 | 9134070 | `summary.json` `36e0cae8e578b45e` | pending: `comp-03-retrieval-sweep-20261017T0256Z-78c3c36d` |

The three pending runs (`comp03-run-250`, `pa-m3`, `pa-combined-sweep`) have
harness-written receipts only in scratch storage: the current COMP
classifier has no rule for their metrics, so they are not registered in
`index.jsonl`. Those scratch receipts, with the M3 answer config and the
offline M1, MF1 and AC41-27 outputs it came with, are preserved verbatim in
`scratch-receipts.tar.gz`. The 250-question config reuses `pa-m3` and
`comp03-run-mix` answers.

## SHA-256

```text
51572597f93648b3bcb5fc3b89b0978092898d624b2053b6e484804c1802d0d9  comp01-run1.tar.gz
e7f0c33283245ba2d2a67674005f744c30a72963385be43d7cbcd5320c74c5c1  final-comp01.tar.gz
dec65f2379e87595892a0f796a27f72c1060fe384196625b08ae5ed1accb6152  final-comp01-hybrid.tar.gz
380691b6e004f2011c745f39a4ae9cd8b4398c91351ed8342f2144b28ccc9145  comp02a-10000.tar.gz
cb9e61896d7ca4d1e67a9928f447c1a90baec866eea590cf608e26b841e91a2b  final-comp02a-10000.tar.gz
cad8e6e65fd5cabfd6254bb7b9cd1aa3c6a60521d1086030bd53bdca516df6ca  comp02a-17272.tar.gz
7aa548413d75f9a6da14733acd649567dc3a8a20eb76e5a34b1b5f5dc9ac6292  final-comp02a-17272.tar.gz
ca7619db921d0ec24ab6485536411f853c8fa369bf0d510eaab02a8bbc4af267  comp02b-10000.tar.gz
8ce980816b13d61496b6d953849182400c291579013d15152a01e68462c8cb73  final-comp02b-10000.tar.gz
5ed57b5ca055a420cbeb8d13e82219de36b4f93cd04e776699bbbe3bf6f1f372  ui-s40-comp02b-10000.tar.gz
ad90b725a2790299a16c0f714ac362d639a497b139f4ab964c2439a1ad104ffa  ui-h3-comp02b-10000.tar.gz
11490cf82e601eb2558f3e096e82905f8efd89271339294056475bdf19f94384  ui-base-comp02b-10000.tar.gz
c97ab835faee2f276959e022fffcb78dc2dbc42a34bb367e280d504868d74487  final-comp02b-latency.tar.gz
9fb9df30e05627e95979b38fd36f7dc6141db39a568cc221a2a7ecadbb40729d  ui-s40-latency.tar.gz
da98fe46266f7329214272cf44910e394fe362e6479962823f0dda7097c12508  qc-comp02b-latency.tar.gz
2801f9656e0c055204a528e90bf16cfe2e6b28136e26e74906c78b83ffdbb1fe  ui-h3-latency.tar.gz
d2b4cb0f77f56acdb312c28e4895a572766688778104b57eb3e2018070f0cf86  ui-base-latency.tar.gz
726f5d91367814e285b8b3d0d0e6679d10b473f67acec01923886f70f13d84a9  uh-h2-comp02b-latency.tar.gz
89a6d107245f11637105f9d51ea89033f332ab8ecc13e63070dd0b93a97bcb2c  uh-head-comp02b-latency.tar.gz
5952ba6059ea729486180bd15bae7e55855667a46134f8ff891d44256775a23d  ab-h2b.tar.gz
3f179b26313284444375b75349cab4725eb0a82c51a2f106197189d09f27e8b0  comp03-run1.tar.gz
1eba92b9d86623a172869a73c33b64e62676343ae74bea7b6f3b586b489ec653  comp03-run2.tar.gz
ce630dfe2231dde4f1d329fb5dc1d0b9762ddd1740cf090c201c2ab7c8579a07  comp03-run-v3.tar.gz
6a78103cdbb78731c5be83c064b6efb54d7d28194b88709ca2f90156c2aa36a9  comp03-run-v3rep.tar.gz
72c67ded1fe285f3515ede08a7ef0851c6efbc2190a69e6f96677013209500ed  comp03-run-v4a.tar.gz
3186c3103fa1ec6b8a407586c15a95e7a5b0cf96a973d922e1cf5f7395201a7e  comp03-run-v4b.tar.gz
c9cd23e5bf5c96c0d7507c1fb1d9724c29f95780106c12a7b28cee6072008ad1  comp03-run-v4c.tar.gz
ea6962cd93505b286dd499aa5b25f9bb49aaa1a2ad72e68e268f90025c522f0d  comp03-run-mix.tar.gz
830e93a5be6d810ec2e7b6883f893813ad88f916295199170c014a2c49308fa1  comp03-sweep-d3.tar.gz
aedd649a5b054dcbf33b6a9747a38664ca95943b2f05b3b34042fbddcf3b152e  final-comp03-sweep.tar.gz
3d7d7e1d3dae7acac4f2b10e523a64cc36c77ba9815c9f9451139d6b638ecbbc  comp03-run-250.tar.gz
992049af9f1f5ccb4d97cdd80755c0886f9b98b7bc8acedd6a6d540bfca57605  pa-m3.tar.gz
4780e94e15eba2c8fe99f52705e94dd40f478ed09703ec71434936e93ccec2b0  pa-combined-sweep.tar.gz
02c49ffa27dbd25a1eefe86bccc4fcb2cad62d2dca8e391d443470fc14f92f4e  scratch-receipts.tar.gz
```

## Added after packing

- `ui-h3-sweep.tar.gz` and `ui-s40-sweep.tar.gz`: the Unit I gate retrieval sweeps (harness 821e649e5, exported; no receipt). Summary SHA-256 c61418eb… and 8fa6cfa6….
- `*.scratch-record.*.json`: scratch receipts kept as plain data, not registered (the 250 run, M3 and the combined sweep need classifier rules first).
