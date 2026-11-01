# Fresh verification timing: findings and recommendation

Owner requested measured cost before deciding whether to retain verdict caching.
Source/tools: `e49609fac`, release build, SQLite 3.54.1, real pinned BGE-small
384-dimensional weights, settled **Mean** pooling. No production behavior changed.

## Measured results

Both strict device processes passed every instrument assertion. Eighty measured
opens: two devices, two provider lifecycles, ten alternating fresh/cached pairs
each. Every fresh cell executed exactly 45 real probe embeddings; every cached
cell executed zero. Vector-less control: zero. A 300-row store supplied a real
pinned centering mean. All cached file hashes matched current loader pins before
and after; every construction asserted three cache hits and no downloaded bytes.

| Device / provider lifecycle | Cached open median (ms) | Fresh open median (ms) | Paired added time median (ms) | Added time observed min / p95 (ms) |
| --- | --- | --- | --- | --- |
| cuda:0 / reuse | 9.18 | 166.38 | 157.10 | 152.82 / 163.58 |
| cuda:0 / reload | 9.74 | 191.43 | 181.51 | 173.14 / 196.13 |
| cpu / reuse | 9.22 | 2383.04 | 2373.95 | 2258.89 / 2634.44 |
| cpu / reload | 9.63 | 2571.61 | 2562.05 | 2408.65 / 2612.63 |

GPU: RTX 3090. CPU: AMD Ryzen Threadripper PRO 5945WX, 12 cores/24 logical CPUs.
No explicit Rayon/OMP/MKL/OpenBLAS thread override was set. Full machine/device
observations, asset pins and every timing are in [results.json](results/results.json);
raw outputs are [CUDA](results/cuda-0.log) and [CPU](results/cpu.log).

These are **warm end-to-end open** measurements. `reuse` isolates a loaded
provider; `reload` reconstructs another provider while the fixture model remains
resident. Reload totals are not cold-process startup or last-provider pool trim.
Small SQL preparation differs (fresh deletes the marker before timing); do not
label the delta pure neural inference time. The nearest-rank p95 from ten pairs
is the observed maximum, not a well-estimated fleet tail. This one host does not
qualify Jetson, other CPUs, cold filesystem caches or busy GPU behavior.

## Pooling decision recovered

Use **Mean** for the production engine. This is an existing ruling, not a new
choice inferred from this timing run:

- [Accepted default-embedder ADR](../../adr/ADR-0.6.0-default-embedder.md), critic
  resolution EMB-1, explicitly records that Mean stays.
- [IR-C measured follow-up](../../notes/IR-C-embedder-options-research.md), commit
  `ff4a59e67`, compared Mean/no-prefix with CLS plus query prefix. Exploratory
  median rank was 99 versus 121 and top-50 was 37% versus 34%; exact-fact top-50
  was 78% versus 80%. This was a combined-configuration comparison, not an
  isolated causal pooling comparison. Its conclusion keeps Mean.
- [IR-C closeout](../../plans/runs/IR-C-retrieval-findings.md), `f370695f3`, says
  ship BGE-small/Mean. Current slice, main and dev/0.8.29 constructors agree.
- CLS evaluation helpers are a distinct path. Commit `813e525ad` explicitly did
  not change Engine embedding's default. Historical U3 explicitly selected CLS
  for its matched ONNX calibration; it does not supersede the Mean decision.

Do not repeat a quality sweep to reconfirm the settled choice, switch existing
stores implicitly, or call the engine CLS merely because a helper is CLS.

## What the store pins

[README SQL](README.md#inspecting-a-stores-metadata) inspects current internal
tables read-only. The declared profile stores name, revision, dimension and an
optional centering mean. Kind/profile mappings, 45 probe references and a cached
accepted-verdict fingerprint live in separate internal tables.

**`mean_vec` is a corpus-centering vector, not a record of Mean/CLS pooling.**
The profile is a declared compatibility identity, not a proof of the exact
runtime that generated every vector. It can exist before a provider ran.
Neither pooling nor backend/device/architecture/driver/library versions are
pinned as a complete per-store embedding-function descriptor. OpenReport is
per-open evidence, not a persisted vector-writer history. Model config/tokenizer/
weight SHA-256 pins are enforced by the default loader, but not copied per DB.

The actual code permits Mean/CLS selection under the same identity triple.
That concrete identity gap reinforces the need for checking live output; it
is not evidence that production stores accidentally underwent such a switch.
Future semantic identity should distinguish pooling and equivalent function
settings. Runtime provenance would help diagnosis, but hard-pinning one GPU
architecture would unnecessarily prevent numerically compatible portability.
No metadata schema/API change is implemented by this investigation.

## Recommendation after measurement

Do **not** retain the present cached-verdict bypass unchanged as a guarantee of
current-backend compatibility. Fresh verification at each relevant provider-enabled
open is the simplest defensible default, with an explicit startup cost:
approximately **0.16–0.18 s on this GPU** and **2.37–2.56 s on this CPU**.

Recommend fresh per-open verification for stored/prospective dense state, with
no per-call probe tax. The CPU cost is material and requires the owner's budget
choice; no latency threshold was supplied and these measurements do not claim
it is universally acceptable. Reuse a healthy open engine across calls rather
than re-opening for every query. A thousand calls in one session incur the check
once, not a thousand times.

If CPU startup requirements rule out that cost, the alternative is a separately
specified cache keyed to a sufficiently complete embedding-function/runtime
identity. The current identity/reference hash omits live implementation changes;
adding another checksum over the same inputs does not solve this. Arbitrary
caller-supplied providers cannot be assumed to supply trustworthy complete runtime
provenance. Do not claim such a replacement cache is designed or qualified yet.

The [proposed open/recovery contract](../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
remains pending owner approval. These results revise its earlier recommendation
to preserve TC-68 cadence: cache retention/removal must now be decided explicitly
using this cost evidence. No verdict-cache policy, thresholds, pooling or
production open behavior changed during measurement.

Targeted actual-feature release clippy and the two real-model test processes
passed. The full gate stopped at its missing checkout-owned `.venv` preflight;
see [full-gate.log](full-gate.log). This is not a full-workspace verification pass.
