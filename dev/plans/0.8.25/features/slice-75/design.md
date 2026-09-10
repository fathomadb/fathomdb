---
title: 0.8.25 Slice 75 — integrated closure design
status: APPROVED
design_version: 5
target_release: 0.8.25
depends_on: 73
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 75 design

## Boundary

Slice 75 is a release-evidence adapter around existing product contracts. It
adds focused integration/upgrade witnesses and a fail-closed closure manifest;
it does not add product capability, change public wire shapes, lower a limit,
or reproduce earlier performance investigations.

The frozen product candidate is the Git tree excluding Slice-75 receipts and
status records. Receipt-only commits may follow only when their product-tree
digest equals the tested digest. The current workspace version remains the
pre-cut Axis-W version; production package/version work is a later release
operation outside this ladder.

## Closure manifest and evidence

`Slice75ClosureManifestV1` is checked in before execution. It contains:

- product-tree and candidate identity rules;
- required current, retained, hosted, and explicitly unavailable cells;
- exact argv, cwd, controlled/set/unset environment, timeout, features,
  platform, expected test count, required output markers, and thresholds;
- fixture, model, package, and source-receipt SHA-256 inputs;
- invalidation paths for every retained or reusable result; and
- the permitted exclusions listed in the plan.

The validator seals declarations only: it accepts closed keys and known cell
IDs, checks fixed in-repository and retained-receipt digests, and pins the
complete execution declaration so command, timeout, environment, expectation,
input mapping, or exclusion drift fails. It does not pretend to
execute commands or infer a pass from prose. Execution records the exact
candidate SHA, command, exit status, positive count/marker, threshold, and
raw-log digest for each cell. Independent evidence review checks those records
against the sealed manifest; unknown, missing, timed-out, zero-test, skipped,
or mismatched evidence is not pooled into a pass.

At candidate freeze, the status records the full tracked-tree identity.
Separately, the manifest maps each cell to a narrower closed invalidation-path
set so a fix does not force unrelated expensive work to repeat. Retained cells
use explicit `git diff --quiet <source> <candidate> -- <paths...>` checks.
External BGE/TinyBERT files and the gitignored EU7 corpus are checked against
their declared SHA-256/file-count inputs before their consumers run. Generated
package/model outputs are hashed before their first consumer. The combined
CUDA package set has a canonical manifest binding all four artifact hashes to
the checked-out candidate SHA and verified preflight witness; the installed
smoke rejects substituted bytes. Raw logs are
temporary; the committed compact receipt retains their digests and the
observations used for the verdict. Hosted helpers bind the repository, branch,
exact SHA, run ID, workflow/job name, matrix label, and conclusion. The Jetson
helper captures the run ID after dispatch before watching it. The Windows
helper starts the VM only when needed, verifies the guest service over SSH,
and then confirms the exact repository runner identity, labels, online state,
and idle state through the GitHub API.

## Focused final-code interaction target

`slice75_feature_interactions` contains four deterministic real-SQLite tests:

1. A multi-row governed write exercises cached statements, transaction-level
   canonical visibility coalescing, projection batching, an internal custom
   trigger target, drain/readiness, close, and reopen. The custom trigger must
   fire per affected row while the built-in invalidation advances only at its
   designed boundary.
2. A hybrid search fixture selects the deferred-identity and linear-fusion
   route and compares it with the complete ranked control. Adding a source
   dependency, lifecycle-ineligible row, and frozen context must select the
   safe eligibility route and never expose the hidden dependent.
3. A bounded writer/projection/search fixture first checks completed mutations
   and readiness at deterministic boundaries, then runs an explicit overlapping
   writer/search phase. Each search must be internally consistent and contain
   no duplicate identities; settled results contain no lifecycle-ineligible
   rows.
4. Source erasure followed by recreation must remove old canonical, vector,
   dependency, and search visibility, settle projection state, and remain
   correct after close/reopen.

The fixture reuses existing test-only hooks and APIs. It does not add a 50k
benchmark or derive latency limits from test timing.

## Populated schema-26 upgrade target

Each `slice75_schema26_upgrade` test independently creates a database by
applying the exact registered migration prefix through step 26 with
`migrate_with_steps`. A narrow raw-SQL fixture helper then populates
schema-valid canonical nodes/edges, logical/source identities, lifecycle
states, FTS rows, and projection terminal/cursor state. The record calls this
a migration-prefix fixture, not an actual prior binary-produced artifact.
`PRAGMA user_version=26` and an ordered canonical table/row census are asserted
before each independent upgrade.

The first test opens those bytes through the normal current migration path
with a deterministic test embedder,
requiring `schema_version_before=26`, `schema_version_after=33`, and migration
step IDs 27 through 33 exactly once. It proves preserved canonical reads,
search and projection state, then configures current vector projection and
proves materialization/readiness without pretending a synthetic v26 vector
blob came from a prior runtime.

The second test closes and reopens the upgraded database, requires an empty
migration-step list, then exercises new provenance and dependency registration,
frozen-read invalidation, supersession, source erasure/purge, recreation,
projection drain/readiness, and a final reopen. Old source/dependent/vector/FTS
state must not reappear. This is the authoritative prior-release upgrade
witness; individual step tests remain focused supporting coverage.

## Long and performance paths

The default aggregate gate runs without `AGENT_LONG`; explicit cells activate
only the selected long bodies. AC-021 is brought into agreement with its
registered 60-second DDL workload by observing both administrative schema
mutation and projection rebuild activity. AC-059b runs the full 1,000-read
race. AC-034a/b runs 100 process-kill trials; AC-034c stays unavailable because
the accepted VM/sysrq fixture is absent.

One release-built `perf_gates` target is reused for AC-076, AC-072, and AC-020.
`AC_FULL_SCALE` is removed, not set to `0`, because presence selects the
tracked million-row tier. AC-076 uses 10k and 1,000 measured searches at
20/150 ms. AC-072 uses 10k, 384 dimensions, and 1,000 measured full
`Engine.search` calls at 80/300 ms. AC-020 retains its registered parallelism
predicate.

One EU7 body on 7,667 real documents, real BGE, 100 queries, 1,000 bootstrap
resamples, 1,000 latency samples, and 8×250 stress searches supplies both real
verdicts. The body must fail rather than return successfully when the long
flag, corpus, or model cache is absent. It asserts AC-073 against the same-run
baseline and AC-075 with the accepted `recall_ci_hi >= 0.90` predicate. Fused
recall and synthetic recall remain descriptive.

## Models, artifacts, and GLOBAL-01

The live-model cell performs cache preflight first and forbids
`FATHOMDB_SKIP_NETWORK_TESTS`. It runs the existing seven TypeScript modules,
engine, Python selectors, and CLI selectors with expected counts 33, 9, 13,
and 2. TypeScript also binds per-file counts 2/1/5/2/11/6/6. The receipt records
model identity/cache digest. Hosted model-job green is supporting evidence only
unless these positive markers are present.

The Linux x64 wheel and N-API package are built once from the candidate using
the workflow feature contracts. An isolated environment installs those bytes;
runtime attestation records the Python package, extension, CLI, JS package, and
`.node` paths and hashes. Repository paths on `PYTHONPATH`, editable installs,
and source-tree native fallback fail.

That same wheel is installed and smoke-tested under CPython 3.10, 3.11, and
3.12; the same packed N-API bytes are installed and smoke-tested under Node 18
and 25.9.0. These are reuse cells, not rebuild rows. The release workspace and
CLI are also built once, the three independent leaf crates
(`fathomdb-embedder-api`, `fathomdb-schema`, and `fathomdb-query`) run
normal `cargo package`, and the built CLI opens, checks, and closes a fresh
database through its integrity verbs. Candidate write/search behavior is
covered by the installed SDK and GLOBAL-01 cells because the CLI intentionally
has no general write/search commands. Dependent-crate package resolution waits for the
separate version-cut and ordered registry rehearsal; no historical registry
dependency is misrepresented as this candidate.

The installed cross-SDK fixture is bounded but representative. Python records
positive write/search, dependency, and frozen-read markers. The N-API route
reuses the fixed Slice 73 13-module set and requires 180/180 tests across the
broader actuation, dependency/closure, projection-readiness,
pagination/operational-state, evidence, lifecycle/erasure, frozen-read, and
constrained-graph areas. Both SDKs exchange one canonical
frozen-context/database fixture and require wire-equivalent values.

The existing GLOBAL-01 native fixture is resolved for Slice 75 so its CLI path
names the built candidate. Candidate-artifact Python performs exactly one
native `Engine.search` over the three frozen records. The receipt requires the
expected source first, recall@3=1, reciprocal rank=1, and
`measurement_layer=data_plane`. It uses no external answerer/judge and incurs
no spend. Historical answer-quality decisions remain separate and no new
answer-quality claim is made by this data-plane release.

## Platform classification

Exact-head installed smoke is required for Linux x64 GNU, Linux ARM64 GNU,
macOS x64, macOS ARM64, Windows x64 MSVC CPU, and Jetson/Tegra Python CUDA.
Each CPU row builds wheel and N-API bytes once, installs them, records resolved
native paths/hashes, and runs open/write/search/dependency/frozen-read/close.
GitHub provides the macOS runners. Before workflow dispatch, Slice 75 starts
`gh-runner-wonl-win11`, requires a runner registered to `fathomadb/fathomdb`
with labels `self-hosted`, `Windows`, `X64`, and `windchill3-windows-11`, starts
and verifies its service, and uses those labels for the supported Windows CPU
row. Missing or wrong registration is a preflight failure. The workflow row
selects that runner only for an exact-SHA manual dispatch on a release branch;
ordinary push and pull-request events retain GitHub-hosted Windows and cannot
allocate the persistent VM. That row supplements, rather than
repeats, the applicable Slice 73 13-module/180-test receipt.

Slice 72 is the retained Linux x64 CE CPU/CUDA proof when its declared model,
reranker, engine, feature, build, lock, and tooling inputs have no relevant
diff. It does not prove the combined release package. Slice 75 also builds and
installs the current Linux x64 `embed-cuda,rerank-cuda` wheel/N-API/CLI package
and runs CPU, auto, and forced-CUDA embed/rerank processes with RTX 3090
allocation and model-forward evidence. Linux ARM64/macOS/Windows packages
carry their documented default-embedder feature, not an implied CE guarantee.

Tegra is an active 0.8.25 and go-forward Python CUDA route. Slice 75 makes
`jetson-tegra-cuda-evidence.yml` release-generic, removing its stale 0.8.24
branch and expected-version literals while preserving exact-SHA and
nonpublication gates. On the Jetson at `10.83.10.13`, it builds and installs
one host-native `0.8.25+tegra` `linux_aarch64` wheel and runs CPU, auto-CUDA,
and forced-CUDA in separate processes. Because the repository version cut is
outside the slice, the builder accepts a validated `candidate_version` and
changes only the copied staging manifest; it records the overlay diff and
never dirties the exact-SHA checkout. The input must equal the release-state
version and branch suffix. The forced path must retain verified Orin allocation
and model-forward evidence. `publish_to_pages=false` is mandatory in Slice 75.

Musl, ARMv7, Windows ARM64/ia32, Windows CUDA, and Tegra npm remain outside the
active routes. Public compatibility docs continue to describe the currently
published release until the separately authorized release cut; the closure
receipt describes candidate routes without rewriting published-state claims
early.

## TDD, review, and rerun policy

RED tests first define manifest closure/count/skip/declaration failures and
any product defect exposed by the interaction/upgrade fixtures. GREEN adds the
minimal manifest validator, fixture hooks, execution helpers, and fail-closed
verdict assertions.
Tests remain fixed during each correction. An independent reviewer checks the
design before code and another checks the final diff.

The manifest tests are added to the already-registered
`test_agent_test_tiers.sh` suite, so the post-implementation aggregate
contract remains structurally pinned at 109 suite labels rather than acquiring
an accidental 110th label.

After candidate freeze, a narrow correction invalidates only cells whose
declared inputs changed. Product/schema/search/write changes invalidate the
interaction, upgrade, affected performance, and installed-consumer cells.
Model/reranker changes invalidate live-model, EU7, and relevant CE cells.
Workflow/package changes invalidate affected hosted rows. Receipt/status-only
changes invalidate none when product-tree equality is proved. The aggregate
default-tree gate runs once on the final product tree; unaffected expensive
campaigns are not repeated.

The aggregate gate requires zero skipped suite labels. Its known nested
network-backed model bodies are an exact manifest exclusion and receive no
credit there; the separate live-model cell must execute them. Any other nested
skip is a failure. AC-072 additionally unsets scale-treatment and drain-timeout
overrides. EU7 unsets its recompute override and writes only to a caller-owned
run-directory path, never the tracked historical `eu7-latest` file.
