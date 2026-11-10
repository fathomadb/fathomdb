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
