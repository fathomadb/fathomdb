# FathomDB Inspector

Inspector is a local Rust application with an embedded browser UI.
Slice 20 implements owned local launch, guarded current-schema attachment,
canonical and operational browsing, document/tree views, scalar attributes,
metadata, exact copy, scoped samples and finite session cancellation/close.
Slice 30 adds native indexed-text inspection and body/edge, named-property and
qualified BM25F retrieval. Slice 40 adds stored vectors, exact L2 and cached CPU
hybrid serving. Slice 50 adds directed stored graph exploration, recorded
dependency/source investigation, retained history and actual frozen graph evidence
in [adopted 0.2.0](../../dev/inspector/plans/0.2.0/scope.md).
Slice 60 adds bounded A/B observation comparison and complete person investigation.
The source package is `0.3.0`, targeting FathomDB `0.8.31` (compiled Schema
`38`) on native Ubuntu 24.04 (`glibc 2.39`) with outside-`body` node/edge
`properties` inspection, read-only reserved `scope` display, historical `as_of`
stored and native graph exploration, default `local-ci.sh --scoped` checks, and
GitHub Release artifact delivery via the `build-inspector` job in
`.github/workflows/release.yml`. See the
[0.3.0 release notes](../../docs/inspector/releases/0.3.0.md) (and the earlier
[0.2.1 plan](../../dev/inspector/plans/0.2.1/README.md) and
[Slice 30 release evidence](../../dev/inspector/plans/0.2.1/slice-30/status.md));
local candidate distributions promote to ignored `dist/inspector/0.3.0/`
(`dist/inspector/0.2.1/` and `dist/inspector/0.2.0/` remain separate). No
human-validation claim is made.

This branch targets the `release/0.8.31` Engine/schema (compiled format `38`).
Inspector `0.3.0` admits Schema `38` stores and cleanly refuses Schema `37`
(`0.8.30`) and older or missing-layout stores without mutating files.

## Build and launch

The selected development target is Linux x86_64 with an existing supported
Chromium browser and a graphical session. Rust builds the application and
embeds its assets; Node and Python are verification tools, not product runtimes.
From the repository root:

```sh
cargo build -p fathomdb-inspector --features cached-cpu --locked --offline
./target/debug/fathomdb-inspector --help
./target/debug/fathomdb-inspector --build-description
./target/debug/fathomdb-inspector /absolute/path/to/existing.sqlite
```

Launch without a path to select one in the UI. Stop external writers first.
Inspector owns an ordinary Engine session; allowed startup, background work
and shutdown can change product files. Initial nonempty WAL/journal, unsupported
schema or encoding, missing lock, held ownership and incompatible layout/profile
are actionable refusals. Preserve the original files and sidecars; Inspector
provides no migration, recovery or repair control.

The three independent launch flags are `--no-show-inactive`, `--show-erased`
and `--show-payloads`. Defaults show retained inactive/history revisions and
withhold retained erasure content and telemetry payloads. One flag cannot bypass
another applicable gate. Physically deleted content stays unavailable. Narrow
local erasure export is conditional on legitimate readable retained content;
the Autonomous Engine fixture deletes atomically. Existing ExplicitMaintenance
stores can have legitimately queued exact-cursor targets: their content obeys
`--show-erased`, without changing persisted Engine policy or starting maintenance.
Queued intent does not itself mark an active record inactive or removed.
No general database export is provided.

The UI uses a one-use private bootstrap file, then a local credential bound to
the exact launch/build/protocol/attachment. Database paths and credentials do
not appear in browser URLs. Launch opens a separate private browser profile.
Detach releases the owned session; Cancel expires it and terminates its worker.
Observations can become stale while allowed Engine work progresses. Invalidated
or evicted content requires explicit refresh, preserving the original context.
Record detail separates stored representation facts, inspection eligibility,
representation-read readiness and session runtime; unobserved retrieval facts
remain unknown. Materialized scalar values link to their exact canonical origin.
Back preserves the permitted original tree/chunk/association observations and
keyboard focus; Refresh this view explicitly starts a new read.

Text retrieval opens globally or from Indexed text in an exact record. Native
body/edge RRF, property negated SQLite BM25 and BM25F values have distinct labels.
Body snippets retain native UTF8 byte spans; property/BM25F spans are unavailable.
Stored index values/effective DDL tokenizer and configured overrides are separate.
BM25F is an unfiltered active/current substrate operation with native dependency
eligibility; it ignores own-node validity windows and has no native result limit.
Its displayed limit truncates native output. Later hydration/origins are labeled
with their separate context. Resource admission may refuse work above conservative
corpus/scratch bounds; no model is needed for these lexical operations.

Vector inspection opens from Stored vectors or the retrieval navigator. Stored
profile/f32/head-window facts and bounded L2 neighborhoods preserve exact origins;
distance bars include numeric values and lower-is-nearer labels. Native hybrid
scores and zero-based arm ranks have a separate view. Back restores the original
permitted observation and keyboard focus; late replies cannot replace newer data.

Cached CPU serving uses production Default only when existing pinned assets,
probe baseline, materialization and Linux LandlockABI3+ are available. Bytes are
sealed before the loader sees them; source cache is never downloaded/repaired.
No persisted embeddings are created by Inspector. A failed Default open is not
retried None; deliberately selected model-free mode leaves stored/text
inspection usable. CPU/reranker/GPU choosers are absent. Inspector `0.3.0` is
qualified natively on Ubuntu 24.04 / glibc 2.39 userland, kernel
`7.0.0-43-generic` / Landlock ABI 8 and Chromium `156.0.8078.4` (Ubuntu 22.04 /
glibc 2.35 is dropped). Cached CPU requires Landlock ABI >= 3.

## Verify this feature

Use the target checkout's own environment. Do not borrow another checkout's
Python environment or install editable native bindings from a worktree.

```sh
cargo clippy -p fathomdb-inspector --all-targets --offline -- -D warnings
cargo test -p fathomdb-inspector --offline
cargo build -p fathomdb-inspector --bins --examples --features cached-cpu --offline
node --test --test-concurrency=1 src/inspector/tests/browser/lexical.mjs src/inspector/tests/browser/workspace.mjs
node --test --test-concurrency=1 src/inspector/tests/browser/document.mjs
node --test --test-concurrency=1 src/inspector/tests/browser/metadata.mjs
node --test --test-concurrency=1 src/inspector/tests/browser/projection.mjs
node --test --test-concurrency=1 src/inspector/tests/browser/catalog-caps.mjs src/inspector/tests/browser/operational-copy.mjs src/inspector/tests/browser/association-choice.mjs
```

The shipping package and production qualification select `cached-cpu` explicitly.
Bare Cargo development/workspace builds are model-free and never probe model
assets; `--build-description` reports and hashes that compile-time choice.

Build binary and examples together with the same feature selection before
browser checks; build pairing intentionally rejects mixed artifacts. Tests
require existing Playwright and Chromium. Set `INSPECTOR_PLAYWRIGHT` to the
absolute module path and `INSPECTOR_CHROMIUM` to the absolute executable path
when the documented test defaults do not match this executor. No browser or
model download is performed by these commands. Run clipboard tests serially.

Private fixture-control cases require a matching explicit build:

```sh
cargo build -p fathomdb-inspector --bins --examples --features fixture-control --offline
node --test --test-concurrency=1 src/inspector/tests/browser/cache.mjs src/inspector/tests/browser/resources.mjs
```

Fixture controls use private owned IPC and actual Engine writes; no HTTP
mutation is exposed, and default artifacts contain no fixture-control feature.
Rebuild without that feature before checking or packaging a default artifact.

Resource qualifications are explicit rather than ordinary fast tests:

```sh
cargo test -p fathomdb-inspector --test worker --offline -- --ignored --nocapture
node src/inspector/tools/qualify-startup.mjs
AGENT_LONG=1 node src/inspector/tools/qualify-scale.mjs
AGENT_LONG=1 node src/inspector/tools/qualify-lexical.mjs
cargo test -p fathomdb-inspector --test launch --offline -- --ignored --nocapture
```

The [qualification tool index](tools/README.md) records prerequisites and scope.
A focused feature pass is not a full-workspace or final installed-release pass.

## Ownership

All application code, UI, transport, launch, tests and packaging belong here.
Engine/schema/SDK/MCP/agent-access code, public contracts and dependency pins
remain unchanged. Existing dependencies are consumed as-is. The
[canonical design](../../dev/inspector/design/high-level-design.md),
[requirements](../../dev/inspector/requirements.md) and
[single verification matrix](../../dev/inspector/plans/0.1/verification-plan.md)
own feature boundaries and independent oracles.

## Package and local release readiness

```sh
bash src/inspector/tools/package.sh /absolute/existing/parent/new-output-directory
node --test src/inspector/tests/browser/package.mjs
bash src/inspector/tools/local-ci.sh /absolute/new/candidate /absolute/fixtures /absolute/build-target
```

`package.sh` builds a locked offline cached-CPU release with fixture controls
absent. `CARGO_TARGET_DIR` may explicitly select an isolated absolute target;
default is this checkout's target. The new output includes tar, extracted bundle,
license/install notes/build description and inner/outer checksums. Failure removes
only its newly created output and preserves compiler exit status.

`local-ci.sh` is the documented local source+package+installed path: preflight,
a default `--scoped` Inspector source check (with fail-closed diff guard
against `--base` / `INSPECTOR_SCOPED_BASE`; `--full-workspace` selects the full
workspace source gate), tooling/JS tests, native Ubuntu 24.04 (`glibc 2.39`)
offline production build/examples, clean-source candidate seal, ELF
`GLIBC <= 2.39` check and actual installed native/browser qualification under
host `Xvfb`, `/proc` process discovery and CDP `Browser.close` (no Docker, no
`openbox`, no `xdotool`). The candidate stays outside final dist. An explicit
`--installed-only` path rechecks existing bytes after permitted docs-only closure;
it makes no source-gate claim. No required positive stage silently skips.
Hosted CI runs `build-inspector` in `.github/workflows/release.yml` on
`ubuntu-24.04` to produce and attach `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz`
and its `.sha256` to the GitHub Release; it does not claim the local installed
target has run remotely.

[Tool instructions](tools/README.md#local-release-readiness) give exact
preparation, commands and promotion steps. `local-release.py` verifies strict
contents/hashes/actual executable build identity, seals unsigned source-bound
metadata, and copies to verified same-filesystem staging before atomic
no-overwrite promotion. Existing or mismatched destinations refuse;
failures/interrupts clean only owned staging. Local verified `0.3.0` bytes
promote to ignored `dist/inspector/0.3.0/`. See
[0.3.0 release notes](../../docs/inspector/releases/0.3.0.md) for artifact
verification and support limits.

The native Ubuntu 24.04 build uses an isolated build target directory. The
`0.3.0` binary's maximum GLIBC symbol is `2.39`. Runtime support remains
the exact qualified kernel/browser/userland pairing (`Ubuntu 24.04 / glibc 2.39`,
`7.0.0-43-generic`, Chromium `156.0.8078.4`). Historical `0.2.1` and `0.2.0`
artifacts retain their original identities and are not relabeled.

## Stored graph and retained investigation

Open a canonical record, then **Stored graph** or **Provenance and history**.
The directed SVG and synchronized keyboard list preserve parallel/self/cyclic
edge identities. Select a node or edge, open complete detail, or expand/collapse
an exact node. Incoming/outgoing/both and exact kind filters create new reads.
ArrowUp/Down/Home/End move through relationship buttons; Enter selects; Back
restores the original permitted observation and focus.

Endpoint logical IDs can resolve to zero, one, many or unknown retained node
revisions. A single candidate does not prove historical membership. Source,
registered dependency, representation origin and retained supersession facts
remain separate. Missing targets stay unavailable; physically erased bytes are
never reconstructed. Existing disclosure gates also govern graph/history,
copy and selected retained-content export.

**Run native graph traversal** performs a separate default-eligible frozen Engine
read with actual selected target and terminal-edge evidence (both stored and
native graph views accept mutually exclusive historical `as_of_write_cursor` or
`as_of_epoch_ms` coordinates). Full provenance is
required by that public route; legacy records can be inspected in stored graph
while native evidence refuses. Later detail/policy reads have separate contexts;
bounded absence has an undetermined cause. Conservative pre-call admission
accounts for repeated source hydration. No diagram geometry implies a metric.

Graph responses share 200 objects / 2 MiB, 1000 work units and depth 3. Historical
logical-ID resolution scans at most 100k canonical rows / 16 MiB keys because the
current format has no all-revision ID index; exhausted coverage is explicitly
unknown. Aggregate retained graph observations stay within 500 objects and the
shared 10-observation / 20 MiB limit. Collapse/replacement releases hidden state.

```sh
cargo build -p fathomdb-inspector --locked --offline --bins --examples --features cached-cpu
node --test src/inspector/tests/browser/graph.mjs
AGENT_LONG=1 node src/inspector/tools/qualify-graph.mjs
```

Compare observations pins completed or honestly partial actual browse, lexical,
vector or graph observations. Pin/Replace A/B and Unpin are explicit. Union and
intersection pages retain typed physical identities, separate exact origins,
original ranks/own values and each side's request/read scope. Missing means not
returned in that bounded observation. There is no common score scale, inferred
absence cause or shared snapshot. Open original observation restores admitted
native evidence without rerunning; an evicted source is labeled separately.
Pins share ordinary FIFO/byte/object limits and current disclosure checks; they
can evict and never preserve content through invalidation. No saved experiments
or general export is added.
