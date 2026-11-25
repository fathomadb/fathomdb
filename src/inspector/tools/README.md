# Inspector qualification tools

Run from the task checkout with its own target and Python environment. These
probes qualify behaviors with existing RED/GREEN histories; they do not replace
independent final review, Terra verification or the repository source gate.

| Tool | Prerequisites | Scope |
| --- | --- | --- |
| `node src/inspector/tools/qualify-startup.mjs` | Linux, ptrace-capable executor, existing `/usr/bin/strace` or absolute `INSPECTOR_STRACE`, checkout `.venv/bin/python`, offline Rust dependencies, current default binary | Calibrate socket denial; run the real worker environment/header/lock/close oracle under hostile parent; trace owned worker/thread network/cache access and preserve a partial cache. Model-free startup only; model-operation positive qualification remains40. |

The startup probe creates and removes only its own fresh temporary directory.
It never reads a production database or model cache. It reports build identity
and measured outcomes as JSON. Denial is applied to actual socket syscalls,
and calibrated using an Internet-family socket; no Internet traffic is sent.
The parent producer intentionally sees hostile performance PRAGMAs; traces and
effect assertions distinguish it from the cleared session worker. Captured
producer stdout/stderr are checked for the two specified fixture canaries; no
general log-leak qualification is implied. Worker output streams
are closed by the production process owner.

The source allocation and F06 scale fixtures are committed in
[worker.rs](../tests/worker.rs), use real temporary Engine databases, and run
only with `--ignored`. Actual browser/clipboard and lifecycle harnesses live in
[tests/browser](../tests/browser/). Never treat an ignored qualification as run.

File-syscall tracing includes access/stat/readlink and creation/removal calls,
not only opens. The cache snapshot compares all entries, directory membership,
type, mode, owner/group, device/inode, size, mtime/ctime, symlink target and file
content hashes. Atime is excluded because measuring contents can update it.

## Additional committed tools

| Command | Preconditions and parameters | Evidence scope |
| --- | --- | --- |
| `AGENT_LONG=1 node src/inspector/tools/qualify-scale.mjs` | Paired default debug binary/examples, checkout Python, Chromium/Playwright overrides as source README; optional `INSPECTOR_SAMPLE_MS`1–100(default 25). | Real 100k-node/100k-log/10k-edge/>10MiB fixture, real main-dispatch-thread catalog progress, native cancel/fault/detach, actual keyboard/large clipboard/Back/refresh, sampled serviceRSS and CDPheap. Receipts require successful bounded cleanup. This is default/debug composition, not installed release qualification. |
| `node src/inspector/tools/clipboard-diagnostic.mjs` | Same owned fixture/browser prerequisites. `INSPECTOR_COPY_INTERVAL_MS`1–1000(default 100), `INSPECTOR_COPY_SAMPLES`1–100(default 10), optional absolute `INSPECTOR_SCREENSHOT`; runs over 60 s require `AGENT_LONG=1`. | Preserves the earlier timing diagnostic: seeded fixture clipboard, exact expected identity, API statuses and notice at each sample. Optional actual document screenshot. It supplements the completion-aware five-second exact-byte browser oracle; it is not a human qualification. |
| `bash src/inspector/tools/package.sh ABSOLUTE_NEW_OUTPUT_DIRECTORY` | Linux x86_64, existing parent, locked offline dependencies. | Builds selected release/default package in checkout-owned target, stages exact manifest/checksums/license/install notes, no publish or global install. Actual extract/help/identity/default-feature oracle is `node --test src/inspector/tests/browser/package.mjs`. |

The mandatory positive native GUI command is explicit because headless ordinary
Rust checks must still execute the noGUI refusal. Run
`cargo test -p fathomdb-inspector --test launch --offline -- --ignored --nocapture`
on the declared graphical executor. Optional absolute `INSPECTOR_QUALIFY_BINARY`
selects the actual extracted binary; it is never a worker environment variable.
The tested artifact's own supervisor preserves build pairing. Ignored does not
mean run or passed. Source/runtime/installed/human qualifications remain distinct.

Historical Slice20 development evidence used Ubuntu 24/glibc 2.39 and its binary
required GLIBC 2.39.70/75 separately built and qualified Ubuntu 22/glibc 2.35
compatible artifacts (maximum GLIBC 2.34) on kernel 7.0.0-43-generic/Landlock ABI 8
and Chromium 156.0.8078.4. No broader platform claim follows. Raw logs/screenshots may live in owned scratch; probe source
and parameter definitions are committed here.

## Native lexical qualification

`AGENT_LONG=1 node src/inspector/tools/qualify-lexical.mjs` uses paired default
binary/examples and the checkout-owned Python lock oracle. Ordinary Engine writes
produce3500 unique800-term records. Actual pending lexical dispatch-thread CPU
progress precedes queue overflow/cancel/fault/close; an800-term native query reaches
the unchanged30s deadline. No synthetic pause or model is used. It reports ACK,
quiescence/lock release, native3500→100 output and10ms sampled total RSS; all owned
workers and scratch must cleanly close. These are default/debug results, not
installed or instantaneous-memory-peak qualification. The full F06 lexical row-cap/
RSS case is `cargo test -p fathomdb-inspector --test worker --offline scale_lexical_refusal -- --ignored --nocapture`.

## Vector qualification

`tools/vector-fixture.py` runs with this checkout's owned native wheel and existing
pinned cache. It copies bytes into sealed memfds, confines the private loader view
and denies sockets, then uses ordinary Default writes/drain and a second Default
session for the Engine-owned45-probe baseline. Optional `queued`/`history-queued`
use legitimate ExplicitMaintenance. F06 first builds ordinary vectors, then
appends the existing scale fixture with the provider deliberately absent. No model download/repair or new product API.

`AGENT_LONG=1 node src/inspector/tools/qualify-vector.mjs` requires paired standalone
`cached-cpu` debug binary/examples, own Python, existing cache and LinuxABI3+. It checks
real pending CPU query work, queue/cancel/fault/close, model/RSS bounds and F06
stored-positive/native-admission-bound behavior. Stored L2 success is not native
serving success. Natural native timeout is not manufactured with a pause; report
actual shorter positive deadline bounds and inherited strict timeout separately.

Mandatory ignored Rust cases are named in `tests/worker.rs` (cached CPU positive,
asset matrix, export, canonical aliases) and `tests/integration/vector.rs` (f64/
properties, all8D01/sign bits/unknown coverage/overflow). Subprocess helper test
names are not standalone qualifications. Actual Chromium vector workflow is
`node --test src/inspector/tests/browser/vector.mjs`. Run relevant named ignored
cases explicitly; ignored does not mean passed. Source cache inventories exclude
atime and preserve entry/type/owner/mode/inode/content/mtime/ctime facts.

Build production vector qualification artifacts with
`cargo build -p fathomdb-inspector --locked --offline --bins --examples --features cached-cpu`.
Execute named cached qualifiers with the same `--features cached-cpu`; ordinary
workspace tests retain the separate feature-off SDK contract.

## Graph qualification

`AGENT_LONG=1 node src/inspector/tools/qualify-graph.mjs` uses the paired standalone
`cached-cpu` debug binary/examples, own checkout Python and configured Chromium/
Playwright. It builds the ordinary100k-node/100k-log/10k-incident-edge/>10MiB F06
fixture, checks actual output and examined-work bounds, and witnesses owned main
thread CPU progress with a pending real graph call before cancel/fault/detach.
It measures queued cancellation, ACK versus quiescence/lock, next-read usability,
repeated real SVG collapse/expand/Back, sampled serviceRSS and browser heap.
Only a worker PID recorded as this driver's own child may be signaled. No
synthetic pause, global process signal, cache cleanup or external request is used.
This is development qualification; installed/all-view70 remains separate; human study is removed.

## Observation comparison and person qualification

`node --test src/inspector/tests/unit/comparison.mjs` exercises independent typed
physical/revision/namespace identities, origin cardinalities, large decimal IDs,
Unicode, multiset union/intersection properties and400-entry pagination. Actual
Chromium qualification is `node --test src/inspector/tests/browser/comparison.mjs`
with the paired standalone `cached-cpu` binary/examples. It covers actual A/B,
original native return, person/graph/native evidence, policy, budgets, stale/late
owners and real validity expiry under polling and Back. The actual heavy
`comparison-envelope` fixture exercises shared20MiB refusal while each original
response remains<=2MiB/200 items. Twelve native/compare cycles sample JSheap
before/after GC and verify DOM release; those samples are not browser-processRSS.

`vector-fixture.py DIRECTORY person` calls `person_fixture.py` inside the existing
sealed-cache/network-denied ordinary Default session. Supported `doc` records
carry full structured biographies, equal names/bodies, retained revisions, real
head vectors, recorded sources/dependencies and complete directed edge content.
It uses no Caller, synthetic vector row, model download or Engine change.
`fixture DIRECTORY comparison-policy` uses legitimate ExplicitMaintenance queued
content; it does not delay an autonomous erasure worker. `comparison-expiry` uses
an ordinary provenanced target with a near-future validity bound, verifying fresh
policy independently of generation updates. Neither fixture is a product control.
Installed/all-view technical checks completed70/75; the unperformed human
study is historical and removed.

## Installed 0.3.0 qualification

Inspector targets FathomDB `0.8.32` (compiled schema `39`, `TABLE_MANIFEST`-backed
admission, refusing Schema `38` and older) and retains
the `D4 = B` Docker-free native Ubuntu 24.04 (`glibc 2.39`) qualification stack
(Chromium + bundled SQLite + Landlock; zero Docker, Ubuntu 22.04, `openbox`,
`xdotool`, or `sudo`). Build with pinned Rust 1.95.0, locked offline
dependencies, explicit `--features cached-cpu`, and an isolated build target
directory. Package using `package.sh` (which derives `VERSION` dynamically from
`src/inspector/Cargo.toml`), inspect ELF versioned GLIBC requirements
(`<= 2.39`), and independently extract the tar to run the extracted executable.

`run-installed.sh ABSOLUTE_EXTRACTED_PACKAGE ABSOLUTE_FIXTURE_BASELINES
ABSOLUTE_BUILD_TARGET [node test arguments...]` runs existing native/workspace/
attributes/comparison/graph oracles against actual production launch directly on
the host under an isolated `Xvfb -displayfd` display and temporary `HOME`
(unsetting host `WAYLAND_DISPLAY`, `XAUTHORITY`, `DBUS_SESSION_BUS_ADDRESS`,
and `XDG_RUNTIME_DIR`). Both `run-installed.sh` and
`qualify-installed-network.sh` enforce bounded `timeout --kill-after=10s`
deadlines (`INSPECTOR_RUN_TIMEOUT_SECS` / `INSPECTOR_NETWORK_TIMEOUT_SECS`,
default `900`), 2 s `SIGKILL` escalation on `Xvfb` teardown, and bounded
`browser.close()` + `/proc` descendant-tree `SIGKILL` reaping in
`installed-harness.mjs` (avoiding `strace` stdout pipe deadlocks and orphan
processes). Baselines (`person`, `vector`, `study-training`, `study-unseen`)
are generated beforehand by `vector-fixture.py` with the owned Python/native
wheel and pinned existing cache. Ordinary nonvector fixtures use the
same-source release `fixture` example. `INSPECTOR_CHROMIUM_DIRECTORY` and
`INSPECTOR_NODE_MODULES` select existing local Chromium/Playwright installs.
Node is an external measurement tool, not bundled or needed by the application.

The test-only `installed-browser.sh` generates a wrapper inside the temporary
`HOME` that preserves Chromium's normal sandbox and production arguments under
`env_clear()`, adding `--remote-debugging-port=0`. `installed-harness.mjs`
discovers the owned native browser PID via `/proc/<pid>/cmdline`, samples
all-thread descendant service/worker and browser process RSS plus separate
canonical heap growth, and closes the native browser cleanly via CDP
`Browser.close` / `page.close()`. It checks supervisor exit `0`, private-state
removal (`fdi-*`), and nonblocking `flock` reacquisition. No debug driver,
fixture-control route or `SIGTERM` qualifies that close; `SIGTERM` is
failed-run cleanup only. RSS values are 25 ms samples, not total instantaneous
memory peaks. Landlock ABI >= 3 remains mandatory for cached CPU positives.

`study_fixture.py`, selected via `vector-fixture.py ... study-training` or
`study-unseen`, makes ordinary-write distinct familiarization/unseen F07
databases. The five-person study remains removed by `INS-020-LOCAL-RELEASE`; no
human sessions occurred or are inferred from fixture automation.

`qualify-installed-network.sh PACKAGE FIXTURES TARGET NEW_TRACE_DIRECTORY` runs
the actual installed person/CPU investigation with parent-wrapped
`strace -ff -e trace=network,%file,process` (working natively under Ubuntu's
`ptrace_scope=1` without `sudo` or Docker root while keeping Chromium's normal
sandbox enabled). It asserts 0 Internet-family socket/connect attempts across
the `--session-worker` tree, 0 outbound connect/send/UDP/IPv6 attempts across
the supervisor tree, all three pinned source assets (`config.json`,
`tokenizer.json`, `model.safetensors`) opened `O_RDONLY`, and 0 source-cache
mutation calls. Raw logs stay in the caller-owned directory.

`lexical-deadline-fixture.py NEW_DIRECTORY` uses ordinary model-free Engine
writes to prepare 4800 records containing 1332 one/two-character terms. The
legal query fits 4096 bytes and conservative 64 MiB admission; the optimized
installed release then reaches its actual 30 s native request deadline without
a synthetic pause. `installed.mjs` also executes all 8 retained
content/chunk/export and all 8 autonomous absence/telemetry combinations,
installed asset missing/partial/corrupt/replaced/race cases, document/tree/copy/
security, real F06 UI+native controls and frozen study-fixture checks.

## Installed UX evidence output

`run-installed.sh` accepts optional `INSPECTOR_EVIDENCE_DIRECTORY`, an existing
absolute owned directory. A probe may save screenshots or read its verification
script there without making the source checkout writable. The installed
harness's `review` fixture clones `review/work/fixture.sqlite` from the
selected fixture baseline; preserve the published `0.8.29` and `0.8.30`
originals read-only (`0444`) and place only disposable closed-store copies at
that baseline, including their existing `fixture.sqlite.lock` sentinels.
Inspector correctly refuses an absent lock; the probe must not reinterpret that
as permission to create one. Normal native-window close, process/lock cleanup
and resource assertions still apply.

## Local release readiness

Source/package/runtime checks use one entry point from a clean target checkout:

```sh
bash src/inspector/tools/local-ci.sh [--scoped | --full-workspace | --installed-only] [--base <ref>] [ABSOLUTE_CANDIDATE ABSOLUTE_FIXTURES ABSOLUTE_BUILD_TARGET]
```

Mode flags are mutually exclusive (combining two or more exits `2`):

- `--scoped` (**default**, ~5 min target): enforces a fail-closed diff guard
  against `--base <ref>` (or `INSPECTOR_SCOPED_BASE`, defaulting to
  `origin/main`; fails closed if the base ref is unresolvable or if any changed
  path lies outside `src/inspector/`, `dev/inspector/`, or `docs/inspector/`).
  Runs `scripts/agent-lint.sh`, `scripts/agent-typecheck.sh`, and
  `cargo test --locked --offline -p fathomdb-inspector` (both default and
  `--no-default-features --features cached-cpu`), followed by `tooling`, `unit`,
  `native-build`, `seal`, `candidate`, `abi`, `extract`, and `installed`. Per
  `AGENTS.md` §3, a `--scoped` pass is **not** a full-workspace green claim.
- `--full-workspace`: replaces the scoped `source` stage with the full typed
  `scripts/agent-verify.sh` gate before running the Inspector packaging,
  sealing, ABI, and installed stages. Required whenever changes cross shared or
  non-Inspector workspace files, or when an explicit release gate requires it.
- `--installed-only`: requires explicit `ABSOLUTE_CANDIDATE ABSOLUTE_FIXTURES
  ABSOLUTE_BUILD_TARGET` for an already-sealed candidate after only permitted
  documentation closure (`dev/inspector/*.md`, `docs/inspector/*.md`,
  `src/inspector/README.md`, `src/inspector/tools/README.md`, `mkdocs.yml`;
  excluding packaged README). Makes no source-gate claim and rechecks
  package/ABI/installed behavior.

When `ABSOLUTE_CANDIDATE ABSOLUTE_FIXTURES ABSOLUTE_BUILD_TARGET` are omitted
in `--scoped` or `--full-workspace`, `local-ci.sh` stages deterministic
temporary paths under `${TMPDIR:-/tmp}` (respecting optional
`INSPECTOR_FIXTURE_DIR` and `CARGO_TARGET_DIR`) and cleans them up on exit.

Use `.venv/bin/python` for release tooling; Python >= 3.12 is preflighted. Keep
source HEAD unchanged through the source gate, native build and host seal. No
model download, global installation, tag, upload or final-dist release occurs.

Prerequisites: checkout-owned Python >= 3.12 `.venv` (system `/usr/bin/python3.12`
recommended so `fathomdb-py --lib` links against system `libpython3.12`) and
pinned dev tools, Rust 1.95.0 / GNU target and offline dependencies,
checkout-owned `node_modules` and `src/ts/node_modules`, Node/Playwright,
existing Chromium 156, host `Xvfb`, `flock`, `readelf`, `strace`, and existing
pinned BGE cache. `INSPECTOR_CHROMIUM_DIRECTORY` and `INSPECTOR_NODE_MODULES`
select existing local installations; nothing is pulled implicitly.

Prepare a new owned fixtures directory using this checkout's installed native
wheel (never editable worktree bindings): `vector-fixture.py NEW_DIRECTORY` for
`vector`; append `person`, `study-training` or `study-unseen` for those
variants. `lexical-deadline-fixture.py NEW_DIRECTORY` builds the natural native
deadline corpus. `f06` first builds ordinary vectors via `vector-fixture.py`,
then appends the release `fixture` example's `scale` variant. Preserve
generated `work/fixture.sqlite` plus sidecars/sentinel; producers must close
normally and leave no nonempty WAL/journal. Never manufacture a missing lock.

After a source-stage failure, fix with focused tests and preserve the failure.
Substantive source, packaging or test changes require a freshly qualified
source/candidate.

### Candidate contract, local promotion, and GitHub Release delivery

Stable top-level candidate contents (with version derived dynamically from
`src/inspector/Cargo.toml`):

```text
fathomdb-inspector-0.3.0-linux-x86_64.tar.gz
SHA256SUMS                       # archive checksum
candidate.json                   # actual source/build/hash/support metadata
fathomdb-inspector-0.3.0-linux-x86_64/
  fathomdb-inspector
  build-description.json
  LICENSE
  README.md
  SHA256SUMS                     # four bundle files
```

`candidate.json` is unsigned local provenance. It binds actual
archive/executable bytes to clean packaging HEAD, workspace engine version,
compiled schema `39`, features and declared `D4 = B` support. It is not a
registry signature or proof of bit-reproducible compilation. Verify candidate
source before releasing and retain the real source SHA even when later
status/docs commits have a different HEAD. The candidate source remains
reviewable in Git.

To promote a verified candidate locally to the gitignored local release
directory:

```sh
.venv/bin/python src/inspector/tools/local-release.py verify /absolute/candidate --source
mkdir -p dist/inspector
.venv/bin/python src/inspector/tools/local-release.py promote /absolute/candidate "$PWD/dist/inspector/0.3.0"
.venv/bin/python src/inspector/tools/local-release.py verify "$PWD/dist/inspector/0.3.0" --source
git check-ignore dist/inspector/0.3.0/candidate.json
git ls-files dist
```

The existing root ignore covers all dist payloads; do not add duplicate rules.
Promotion refuses any existing destination, even identical/empty, without
replacing it. A fresh sibling stage is revalidated before Linux atomic
`RENAME_NOREPLACE`. Failed or interrupted copy removes only that self-created
stage; unrelated entries remain.

For FathomDB `0.8.31` release automation, `.github/workflows/release.yml`
includes the `build-inspector` job on `ubuntu-24.04`, which runs
`cargo fetch --locked`, `package.sh`, `scripts/check-glibc-floor.sh --floor 2.39`,
`local-release.py seal`, and `local-release.py verify --source`, stages
non-colliding `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` assets via
`upload-artifact-retry` (`inspector-linux-x64-gnu`), gates `all-builds-passed`,
and attaches both files in `github-release`.
