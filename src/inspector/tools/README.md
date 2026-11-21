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

## Installed0.2.0 qualification

`ubuntu22.Dockerfile` prepares local compiler/Chromium/X11 test prerequisites.
Build with pinned Rust 1.95, locked offline dependencies, explicit cached-cpu and
an isolated Ubuntu 22 target; never reuse host compiled artifacts. Package using
`package.sh` inside that same userland and independently extract its tar. Inspect
ELF interpreter/versioned GLIBC requirements and run the extracted executable.

`run-installed.sh ABSOLUTE_EXTRACTED_PACKAGE ABSOLUTE_FIXTURE_BASELINES
ABSOLUTE_UBUNTU22_TARGET [node test arguments...]` runs existing native/workspace/
comparison oracles against actual production launch in a network-disabled Ubuntu22
container. Baselines person/vector/study variants are generated beforehand by
`vector-fixture.py` with the owned Python/native wheel and pinned existing cache.
Ordinary nonvector fixtures use the same-source Ubuntu 22 release fixture example.
`INSPECTOR_UBUNTU22_IMAGE`, `INSPECTOR_CHROMIUM_DIRECTORY`, `INSPECTOR_NODE_MODULES`
can select the exact existing local image/browser/test modules. Defaults are local
`inspector70-jammy:desktop`, Chromium1248 and existing Codex Playwright modules.
Node is mounted as a measurement tool, not bundled or needed by the application.

The test-only `installed-browser.sh` preserves normal sandbox and production
arguments, adding loopback CDP. `installed-harness.mjs` discovers the owned native
window, samples all-thread descendant service/worker and browser process RSS plus
separate canonical heap growth, and closes that actual window with Alt+F4 through
X11/Openbox. It checks supervisor success, private-state removal and an independent
lock acquisition. No debug driver, fixture-control route or SIGTERM qualifies that
close; SIGTERM is failed-run cleanup only. RSS values are25ms samples, not total
instantaneous memory peaks. Ubuntu 22 userland shares the declared host kernel;
inside-runtime Landlock ABI >=3 remains mandatory for cached positives.

`study_fixture.py`, selected via `vector-fixture.py ... study-training` or
`study-unseen`, makes ordinary-write distinct familiarization/unseen F07 databases.
The fixed coordinator key and task/rubric/result form live in Slice70 `study.md`.
The five-person study is removed by INS-020-LOCAL-RELEASE; no human sessions
occurred or are inferred from fixture automation. Keep the coordinator key out of participant-facing documentation.

`qualify-installed-network.sh PACKAGE FIXTURES TARGET NEW_TRACE_DIRECTORY` runs
the actual installed person/CPU investigation, pausing only before attachment
for root tracing inside its isolated container PID namespace. Container-local
SYS_PTRACE permits tracing the exact recorded supervisor and newborn worker;
normal Chromium sandbox remains enabled. It asserts no Internet-family socket/
connect attempts, all three pinned source assets opened O_RDONLY, and no source
cache mutation calls. Runtime/strace raw logs stay in the caller-owned directory.
This does not alter host ptrace/kernel configuration or trace another task.

`lexical-deadline-fixture.py NEW_DIRECTORY` uses ordinary model-free Engine writes
to prepare4800 records containing1332 one/two-character terms. The legal query
fits4096 bytes and conservative64MiB admission; the optimized installed release
then reaches its actual30s native request deadline without a synthetic pause.
The inherited3500-record/800-term debug corpus completes too quickly in release
and is not counted as a deadline pass. `installed.mjs` also executes all8retained
content/chunk/export and all8autonomous absence/telemetry combinations, installed
asset missing/partial/corrupt/replaced/race cases, document/tree/copy/security,
real F06 UI+native controls and frozen study-fixture checks.

## Installed UX evidence output

`run-installed.sh` accepts optional `INSPECTOR_EVIDENCE_DIRECTORY`, an existing
absolute owned directory mounted as `/evidence`. A probe may save screenshots or
read its verification script there without making the source checkout writable.
The installed harness's `review` fixture clones `review/work/fixture.sqlite` from
the selected fixture baseline; preserve the published0.8.29 review original and
place only a disposable closed-store copy at that baseline, including its existing
`fixture.sqlite.lock` sentinel. Inspector correctly refuses an absent lock; the
probe must not reinterpret that as permission to create one. Normal native-window close,
process/lock cleanup and resource assertions still apply.

## Local release readiness

Source/package/runtime checks use one entry point from a clean target checkout:

```sh
PATH=/tmp/inspector40-tools:/usr/local/go/bin:$PATH bash src/inspector/tools/local-ci.sh /mnt/data/inspector80-owned/candidate /mnt/data/inspector80-owned/fixtures /tmp/inspector70-jammy-target
```

Paths above are this executor's owned storage, not universal install paths.
Use `.venv/bin/python` for release tooling; Python3.12 is preflighted.
The existing npm wrapper matches the repository developer-tool pin; other
executors use bootstrap's pinned tools normally. Keep source HEAD unchanged
through the source gate, compatible build and host seal. No model download,
global installation, tag, upload or final-dist release occurs.

Prerequisites: checkout-owned Python 3.12 `.venv` and pinned dev tools, Rust 1.95/
GNU target and offline dependencies, Node/Playwright, existing Chromium156,
Docker and prepared `inspector70-jammy:desktop` Ubuntu 22 image, `readelf`, and
existing pinned BGE cache. `ubuntu22.Dockerfile` documents image preparation.
`INSPECTOR_UBUNTU22_IMAGE`, `INSPECTOR_CHROMIUM_DIRECTORY` and
`INSPECTOR_NODE_MODULES` select existing local installations; nothing is pulled
implicitly. Cargo/rustup homes are mounted read-only; the Ubuntu 22 build target
is isolated from host objects. Docker runs network-none with the invoking UID.

Prepare a new owned fixtures directory. Existing frozen70 fixture baselines can
be copied there while closed, including their original lock sentinels. To create
fresh baseline stores, use this checkout's installed native wheel (never editable
worktree bindings): `vector-fixture.py NEW_DIRECTORY` for vector; append `person`,
`study-training` or `study-unseen` for those variants. These last names are legacy
fixture names; tests assert structural facts, not participant results.
`lexical-deadline-fixture.py NEW_DIRECTORY` builds the natural native deadline
corpus. F06 uses the existing production fixture example's `scale` variant,
appended after ordinary vector preparation as in the existing qualification tools.
Preserve generated `work/fixture.sqlite` plus sidecars/sentinel; producers must
close normally and leave no nonempty WAL/journal. Never manufacture a missing lock.
For the published0.8.29 review corpus, preserve the original and copy its closed
`notes.sqlite` and existing `.lock` sentinel to `review/work/fixture.sqlite` and
`fixture.sqlite.lock`; the harness uses only its disposable copies.

Default local CI runs the full typed gate once, tool filesystem/failure tests and
JS unit tests, then builds package and release examples inside the Ubuntu22
userland/target consumed by `run-installed.sh`. It seals the candidate with the
same clean source SHA and actual executable build identity, checks ELF compatibility,
and freshly extracts the actual tar for installed native/lexical/vector/graph/
workspace/comparison/UX tests.
Failures report `stage=...` and preserve status; missing prerequisites fail2.
Raw test output is the result. Long scale/cache campaigns retain their prior
artifact/source scope; this path does not silently claim to rerun every campaign.

After a source-stage failure, fix with focused tests and preserve the failure.
Use `--installed-only` solely for a separately completed source phase and an
already sealed candidate. It verifies clean source equal to the candidate or an
Inspector documentation-only descendant (excluding packaged README), makes no
source-gate claim, and rechecks package/ABI/installed behavior. Substantive source,
packaging or test changes require a freshly qualified source/candidate.

### Candidate contract and90 promotion

Stable top-level candidate contents:

```text
fathomdb-inspector-0.2.0-linux-x86_64.tar.gz
SHA256SUMS                       # archive checksum
candidate.json                   # actual source/build/hash/support metadata
fathomdb-inspector-0.2.0-linux-x86_64/
  fathomdb-inspector
  build-description.json
  LICENSE
  README.md
  SHA256SUMS                     # four bundle files
```

`candidate.json` is unsigned local provenance. It binds actual archive/executable
bytes to clean packaging HEAD, selected Engine commit/schema, features and declared
support. It is not a registry signature or proof of bit-reproducible compilation.
Verify candidate source before releasing and retain the real source SHA even when
later status/docs commits have a different HEAD. The candidate source remains
reviewable in Git. Finaldist contains the same metadata and exact approved bytes.

For a rehearsal, choose a new destination beneath an existing owned scratch parent.
For90 only, create the ignored destination parent and use the same tested command:

```sh
.venv/bin/python src/inspector/tools/local-release.py verify /absolute/candidate --source
mkdir -p dist/inspector
.venv/bin/python src/inspector/tools/local-release.py promote /absolute/candidate "$PWD/dist/inspector/0.2.0"
.venv/bin/python src/inspector/tools/local-release.py verify "$PWD/dist/inspector/0.2.0" --source
git check-ignore dist/inspector/0.2.0/candidate.json
git ls-files dist
```

The existing root ignore covers all dist payloads; do not add duplicate rules.
Promotion refuses any existing destination, even identical/empty, without replacing
it. If already present, verify and inspect it explicitly; choose no overwrite.
A fresh sibling stage is revalidated before Linux atomic RENAME_NOREPLACE. Failed/catchably
interrupted copy removes only that self-created stage; unrelated entries remain.
After90 promotion, independently extract/check the final tar and run its actual
native person/all-view/compare workflow/close using the installed harness, then
record release availability. No final dist is created by80 rehearsal.

The public0.2.0 release notes document future archive/registry channels and the
remaining credentials, Inspector-specific version/tag/signing and supported CI
work. No uploader, credential request or Engine publishing tag is implemented.
