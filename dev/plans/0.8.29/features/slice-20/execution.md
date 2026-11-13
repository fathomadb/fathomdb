# Slice20 source execution evidence

## Ownership and actual entry

Sole writer: same Sol-high owner. Parent created
/mnt/data/worktrees/fathomdb-0.8.29-s20-test-hygiene on
llm/0.8.29-s20-test-hygiene at02e30d861beddc8061af0dc22f12106d5e09dd32.
Actual branch/top-level/HEAD/clean status, reviewed plans and Astra design PASS,
actual104dcbc10 ancestry and documentation-only02e30d delta were verified.
Own strict --worktree, --expect-closed 10, --plan and --min-disk-gb 32 exited0 with zero
warnings; initial receipt/log is task logs/entry-preflight.log. Parent receipt:
/tmp/fathomdb-0829-direct/slice-20/source-entry.json. No reset/rebase/ref movement
or other-checkout write. Parent/release stay frozen; state remains parent-owned.

## Owned setup and operations

Task root /mnt/data/tmp2/fathomdb-0829-01a1c5a5-s20 is mode700, with distinct own
Cargo home/target, npm/pip/XDG/browser caches, tmp, logs and receipts. Own real
venv/root/TS/Mermaid modules live in this checkout; no borrowed10/parent install.
Exact Node25.9.0/npm11.12.1 and locked npmci passed. Reviewed common helper
--provision selected installed immutable UV Python3.13.13 (>=3.11), installed
Ruff0.15.17/Pyright1.1.410 and source-bound noneditable W0.8.29 wheel with actual
interpreter/ABI/module digest receipt. Production import is own venv site-packages,
not ignored source candidate. No model/GPU/toolkit/driver action occurred.

Setup logs/receipts are task logs/{setup-tools,setup-python}.log and matching
receipts/*.json; source start/end02e30d and empty tracked delta match. Installed
real-store write/search/close/reopen/search/close/normal-exit smoke passed at
logs/setup-installed-smoke-typed-id.log. The first operational smoke incorrectly
compared documented IdSpace to string; retained setup-installed-smoke.log exit1
is a harness error, not product RED. Corrected smoke requires exact
(logical,setup-record), preserving the intended independent identity expectation.
No repository test/oracle was edited for that operational correction.

One native phase, Cargo/CMake jobs2; root32/20GiB, taskfs250/50GiB,
MemAvailable16/8GiB start/stop, cumulative owned task+checkout storage200GiB and
growth200GiB cap. Monitor records actual PID/PGID/command/cwd and source/delta,
retries only transient du disappearance and stops only owned child groups.
Parent-requested read-only descriptor retention is configured for own
AGENT_LOG_DIR; canonical passing spills can be retained without runner changes.
Native setup sample summary (bytes, historical not reserved capacity):

```json
{
  "root_free_min": 103132221440,
  "task_free_min": 917312004096,
  "memory_available_min": 39068356608,
  "owned_bytes_peak": 3382359629,
  "resource_stop": null
}
```

## Per-behavior RED and separate GREEN

| Behavior | Observed RED / exact assertion | GREEN / actual checks |
| --- | --- | --- |
| Sidecar engine ownership |542508062fadb30f6162b9e433cda5f33bc161d2; exact evidence_sidecar_hook_belongs_to_its_engine selector exit101,0pass/1fail/0ignored: an unrelated engine consumed the owner's sidecar hook. |3dd8e7a24b907b82bb96da3db3579f1cb387bfe2; unchanged regression1pass; test-hooks engine clippy, feature-off check and original linearization control exit0. |
| Resolver engine ownership |7f56e4e7ddba01672fae58ea7977e3afefa5b04b; exact evidence_resolver_hook_belongs_to_its_engine selector exit101,0pass/1fail/0ignored: an unrelated engine consumed the owner's resolver hook. |e3d3406ae879da2e0ff8af702cbfb825b3ce1cae; unchanged regression1pass; clippy/feature-off check plus complete ordinary-parallel evidence target29pass/0fail/0ignored. |

Sidecar RED includes only reviewed owner-argument compile scaffolding while
retaining global storage/fire semantics; its failure is runtime theft, not a
missing method/compile error. GREEN does not edit the staged regression. The
engine-owned handle travels only in evidence request/capture, keeps transaction
placement and ordinary search layout, invokes callback without hook mutex, and
updates the public test-hook note in Rust interface. Normal hooks were used.
Raw logs/receipts: task {logs,receipts}/{red-sidecar,green-sidecar,scoped-sidecar,
red-resolver,green-resolver,scoped-evidence-hooks}.
Parent independently reproduced exact542508062f runtime theft RED using audit-
owned cache/target: exit101,0pass/1fail/0ignored, no timeout/resource stop, clean
SHA/deltas. Receipt /tmp/fathomdb-0829-direct/slice-20/red-542508062f-audit.json.
Parent owns that immutable audit worktree; no child-created tree.

## Windows resource and existing-CI disposition

Read-only existing approved libvirt VM is running: gh-runner-wonl-win11,
192.168.122.51, MEMEX-AFE8V4I00, Windows11 Pro, Rust/Cargo1.95. SSH known-host
checking and key authentication succeeded. Guest free RAM30531876KiB, no active
cargo/rustc/cl/ninja processes at observation. C: free11646398464B (~10.85GiB);
D/E/F media report0free. No adequate alternate guest filesystem exists. An own
C:\temp workspace shares C: with OS, below reviewed32GiB start floor. No transfer,
build, cleanup, disk expansion or provisioning was attempted; A6 stays held.

Existing ci.yml push triggers main only, not this llm branch; PR/manual are
separate. Existing Windows jobs run named WAL attribution/checkpoint cases and
installed-artifact smokes, not four A6 startup/opener selectors or a full Windows
engine --lib suite. A normal branch push would not supply A6 proof. No workflow
change/dispatch/release campaign or inferred CI pass follows this discovery.
Actual Windows evidence needs an adequate approved owned venue; Linux never
qualifies Windows. Parent has the concrete resource finding.

## Remaining hypotheses: actual unchanged controls

Task logs/receipts residual-controls record exact source e3d3406ae:
TC57 ordinary parallel5pass and serial5pass; Slice40 fresh-database cutover13pass,
all0failed/0ignored. Configure-first helpers were already correct; no rewrite,
serialization policy, SQLite shutdown or retry-until-green was added. Inherited
.28 owned-file FD repair stays intact; no global FD oracle is restored.

Feature-off non-test MCP release build and separate CPU default-embedder
non-test release build both exit0 without warning/dead_code diagnostic. The
historical warning's exact item/features are unknown; these are two current
CPU configurations, not a claim every historical CUDA configuration was tested.
No speculative cfg removal, suppression or new blanket warning lint was added.

Task affected-boundaries log/receipt: existing independent public test-seam
consumer check1pass across dev/release feature-positive/default-negative arms;
MCP admission25pass and lifecycle18pass,0failed/0ignored. Scoped queue controls
use TOKIO_WORKER_THREADS=2 to bound per-runtime worker resources, ordinary Rust
harness parallelism, unchanged timing/permit/cancel/shutdown assertions. No
pre-enqueue seam or new queue policy is added. Logs retain all actual output.

## Final Linux repair-subset checkpoint

No demonstrated remaining in-scope Linux defect exists in these controls. The
code slice consists of the two independently observed ownership repairs, their
unchanged meaningful regressions, bounded existing pauses and test-only interface
contract. The final clean metadata commit and external receipts will identify
its exact source SHA. Fresh source-bound production Python/TS builds, installed
lifecycle, canonical fresh test-hooks candidate/nonce/child identity and a single
required stable full agent-verify remain the next qualification phase at this
checkpoint. Do not interpret this checkpoint as those future checks having run.

Full-gate operations use STRICT=1 AC037_LIVE_OPTIONAL=1 with AGENT_LONG unset,
one native phase and owned monitored resources. AGENT_LOG_DIR is own real spill
directory; the read-only descriptor collector retains complete successful native
suite output even when canonical wrappers unlink their passing files. No runner
change or passing-spill assumption substitutes for raw receipts. Future results
live in task receipts/logs and source-bound final report; no source edits while
the final gate runs. Source metadata records this pre-gate timing explicitly.

## Outstanding scope and no-close boundary

A3 new pre-enqueue race23 instrumentation/proof and A5 five-constant guard remain
unanswered policy holds; neither exception, artificial RED, passing coverage nor
automatic defer is implemented. Actual Windows A6 remains resource-held; its
capacity proposal/FIX1 is read-only, independently reviewed before any owner
choice and entirely outside this source candidate. No storage/ACL/guest action.
Existing queue controls are preservation evidence, not A3 discharge.

The release state/master/generated board/ledgers stay parent-owned and unchanged
here. Fresh final independent implementation review and actual criterion/disposal
proof still gate any later integration/closure. Return REPAIR_SUBSET_READY only
when actual qualification supports it; never20 COMPLETE,30 dependency clear,
Block1 exit, platform matrix or publication from this partial scope.

## First full-gate attempt and field-map correction

At clean23cacac6037f5814f5344eefe09e5d99f354f492 the required full gate
passed owned-tool preflight and shell lint, then stopped at module-boundary:
Engine field map missing evidence_before_resolve_return_hook and
evidence_before_sidecar_hook. Actual exit1, no resource stop,63s; native suites
had not run. Raw task logs/final-full-gate.log and retained module-boundary spill
remain intact. The correction adds only these two private fields to the existing
root-owner map, matching the existing engine-owned graph evidence hook. No
dependency restriction, threshold, test or parser rule is weakened. Regeneration
and failed/affected checks precede the new clean freeze; the unexecuted gate
phases continue afterward, with completed shell lint retained as earlier evidence.
The first attempt is never reported as a full pass or silently discarded.

## Completed collector and reviewed affected corrections

The clean6eee2e9a66943f94397fac5b9bdc1c7cc330f5d9 continuation completed
all214 registered suites:212passed,2failed,0suite-skipped,0excluded. Lint, all
typecheck variants and STRICT security passed; security had0violations,0blockers
and0downgrades. The two failed suites are retained, never relabeled green:

- test-slice90-root-reconciliation: exact current root inventory expected46 but
  found48 after the two reviewed cfg test-hook fields. Parent conveyed independent
  final-reviewer design approval for exactly these two named inventory entries,
  current48 total, current lib.rs blob2303100aa5363f09f57629864b61a4ffd1553d66,
  literal structural count46to48 and a dated .29 note. Historical counts, exact
  field multiset/blob equality and every other assertion/mutant stay unchanged.
  Only this reviewed amendment is applied after collector completion.
- test-rust: Cargo exit101 because the exact fathomdb-py --lib executable exited127
  before tests with missing libpython3.13.so.1.0; all other Rust targets continued.
  Readelf NEEDED/SONAME and own venv/sysconfig/base-runtime path+hash identities
  match immutable UV3.13.13. The identical executable45419a05ed279a81bae8e5aa418d43a7d3d3b0b76f02691699e5afe9faf70a99
  reran with only command-local verified LIBDIR:39passed,0failed,0ignored,exit0.
  No global loader/config/symlink, borrowed runtime or product change. Its retained
  stderr includes Could not find platform dependent libraries <exec_prefix>; no
  suppression or additional environment alteration hides that diagnostic.

Actual collector logs/receipt: task logs/final-gate-continuation.log and
receipts/final-gate-continuation.json. Retention index final-native-spill-index.json
links complete Rust65288B, Python5422B and TS57970B logs plus NAPI execution output;
collector_errors is empty. Original Rust2390-line failure and all suite exits
remain intact. Loader facts/raw readelf and rerun live in
receipts/failed-python-rust-loader-identity.json and
{logs,receipts}/failed-pyo3-exact-rerun.

Inner-suite limits are distinct from registry-level0skips: Python1746passed and
36skipped with exact reasons; TS625TAPpassed and6TAPskipped, plus15 in-body skip
markers in the full retained log. They do not prove live-model/network/release-
surface behavior. Rust original ignore reasons/counts remain in raw output; the
source-only rust-source-ignore-annotations.json supplement makes no runtime
selection claim. AGENT_LONG was unset. Tegra-only lint and unconfigured ESLint
were explicit lint skips; no platform/GPU/publication qualification follows.

Collector resource minima in bytes: root95397928960,taskfs856798765056,
MemAvailable36065853440; owned task+checkout peak62031249351. No resource stop,
start/end SHA and empty delta match. One native phase and canonical serial
Rust jobs1 were preserved.

Only failed/affected checks follow the reviewed inventory correction; the full
verifier is not broadly repeated. This metadata freeze precedes the final
source-bound native identity/lifecycle refresh; external receipts identify its
actual SHA and outcomes without editing during qualification. A3/A5 remain
unanswered and A6 unqualified. Capacity FIX3 design and core artifact
0e48dca33c9bbb5c27b7d01f137c7a852f73abec934c62c304d66d20572396db
received independent static PASS only; bootstrap/compiler/binary/transfer/controls/
PowerShell and all Windows runtime remain NOT_RUN. Capacity-only owner choice
is still unanswered; no storage/ACL/guest/compiler action occurred.

## Completed native refresh and partial integration

Final clean source657edeb76caaf9c8d2aab433766bec9362e73217 completed the
reviewed root-inventory correction and affected checks, then fresh owned
production Python/TypeScript builds and real write/search/close/reopen/exit
lifecycles. Canonical Python test-hooks candidate and child matched the exact
source, digest and nonce slice20-final-2489444; scaffold1passed. Python suite
1746passed/36skipped. TypeScript625TAPpassed/6TAPskipped and15 additional in-body
skip markers remain limitations. Final source-native refresh receipt is
/mnt/data/tmp2/fathomdb-0829-01a1c5a5-s20/receipts/final-source-native-refresh.json.
The complete handoff remains
/tmp/fathomdb-0829-direct/slice-20/linux-repair-subset-handoff.json.

Fresh independent Astra-medium implementation PASS is recorded in
[review log](review-log.md). Parent integrated exactly this qualified Linux
repair subset by fast-forward. Original failed collector attempts, affected
reruns, runtime warning and skips remain disclosed; this is assembled
qualification, never an uninterrupted verifier exit0. No borrowed parent-native
qualification is claimed. A3/A5 policy remains unanswered and actual Windows A6
unqualified. Slice20 remains open;30 and Block1 exit are not cleared.

## Approved continuation: capacity-only result

Owner seq-369 approves A3/A5 verification and negative controls without fabricated
RED; actual defects retain committed semantic RED/GREEN. Seq-370 approves only
new64GiB image, necessary narrow ACL and live attachment. Parent recorded both
in0a7df81fd4b6f17d716c950e6fd2748daa3565c2 before the fresh sole-writer child.
Strict closed10 preflight and nonmutating owned-environment check passed.

Capacity capability RED a9f85467ad0b466d204dd8a8829033dbfa79eeeb shows the
unchanged healthy boundary test cannot obtain ATTACHED_RAW_CAPACITY_ONLY from
the inherited full helper under narrower authority. Its refusal was correct;
this is missing safe capability, not an old product defect. GREEN
c7e0aec8edf786f42488141bdc398e3f8f812f8a passes14contained controls, including
identity/refusal/UNKNOWN branches and harmless real-child timeout output.
Original failed logger fixture adapter (.cwd missing), correction and raw logs
remain under task approved-runtime/logs. The healthy assertions were unchanged.
Operator manifest9e8b6c4ed02ece44cba59a3557456dc513f02b5856324a49acdffb3cedd7d17a
binds the exact three committed operator/control/README inputs. Fresh distinct
Astra-medium capacity-results review PASS preceded parent live GO.

Parent executed the exact committed operator and actual seq-370 authority;
exit0 ATTACHED_RAW_CAPACITY_ONLY. Actual evidence directory:
/mnt/data/tmp2/fathomdb-0829-continuation/slice20/capacity-only-2ff3bab79d754de89f15039626ef117a.
New Disk1/serialFDB29S20A6/68,719,476,736B/RAW/nonboot/nonsystem is present;
original guest disk metadata and persistent XML remained identical. All25commands
exited0 with no timeout. Image is
/mnt/data/tmp2/fathomdb-0829-continuation/slice20-windows-images-approved-20261113/windows-a6-64g.qcow2.
No initialization, partition, format, compiler/helper/test or cleanup ran. C:
remains below later8GiB start/5GiB stop policy; broader Windows work/A6 remains
held and unqualified. Capacity success does not close20/30/Block1.

A3/A5 proof candidate c13914453 is committed under seq-369, runtime not yet
qualified at this checkpoint. Its own setup/scoped tests, seven serialized
sensitivity controls and final source/native qualification remain to execute.
The indexed verification resource monitor reuses existing Slice60 control logic
with the required64GiB image reservation and optional single control checkout;
no parallel native window or seven-target allocation is introduced.
