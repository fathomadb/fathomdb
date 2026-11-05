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

## Lifecycle follow-up: open is not always once per application launch

Inspected SDK source at `548854b87` and MCP source on `dev/0.8.29` at
`6c177cf9095e06e9d99bb8f0e7aaf74c98bcd5b0`. The Rust/Python/TypeScript SDK
objects retain their engine across calls and have no built-in idle expiry.
They can live for the application lifecycle; explicit close or final object
release ends the handle. A closed handle does not silently reopen. Opening a
new handle for every request is an application choice, not an SDK requirement.
Internal SQLite connection creation is not another Engine.open verification.

The MCP-managed lifecycle is different and makes the earlier amortization
argument incomplete:

- Default `idle_timeout_secs = 60` closes a managed ready store after that much
  inactivity and no active calls. The next request lazily reopens it and waits.
- `max_open_stores = 1` is currently enforced, not merely a configurable default.
  Sequential A/B/A requests close and reopen stores even without an idle pause.
- `idle_timeout_secs = 0` disables idle expiry, but does not prevent switching
  from evicting the previous store. Host-supplied engines are host-owned.
- Default MCP search is text-only with no embedder, so it pays no live numeric
  verification. The concern applies to provider-enabled stores requiring the
  check (including configured hybrid mode).

Witnesses on that MCP branch: `src/rust/crates/fathomdb-mcp/src/config.rs:118`
(defaults), `:274` (single-store restriction), `src/registry.rs:538` (reuse),
`:723` (switch eviction), `:896` (idle expiry), `:985` (SDK open), and
`src/lib.rs:75` (one-second expiry sweep). Existing `tests/lifecycle.rs:95`
covers idle close/reopen; `:264` covers switching and the new open epoch on
return. This is source/test inspection, not a measured production reopen rate.

Under unconditional fresh-per-open verification, the next hybrid query after
an idle close would pay the measured verification cost again, plus other open
work. Alternating stores could put that cost on every request. The 80 timing
samples remain valid; the claim that they are necessarily amortized once per
application launch does not.

**Revised recommendation:** do not approve unconditional fresh verification on
every Engine.open under the current MCP lifecycle yet. Keep the compatibility
check and the proposed classified failure policy, but settle verification reuse
and lifecycle together before implementation. Evaluate explicit process/session
verification reuse bound to the actual runtime and current store verification
inputs, with stated invalidation rules, or a deliberate runtime-lifecycle change.
Neither is designed/qualified by these timing results, and neither may silently
reintroduce the current persistent cache's live-backend blind spot.

Disabling idle expiry alone does not solve switching. Retaining runtimes across
store closes changes resource ownership/memory behavior and needs explicit
review against the engine-owned runtime ADR. Do not invent a universal runtime
fingerprint for arbitrary custom providers or promise a safe cache without that
contract. No cache, pooling, eviction, idle timeout or engine behavior changed.

The [open/recovery proposal](../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
now records this unresolved lifecycle/cadence decision. This follow-up refines
steward `seq-352` rather than treating its fresh-open recommendation as approved.

Targeted actual-feature release clippy and the two real-model test processes
passed. The full gate stopped at its missing checkout-owned `.venv` preflight;
see [full-gate.log](full-gate.log). This is not a full-workspace verification pass.
