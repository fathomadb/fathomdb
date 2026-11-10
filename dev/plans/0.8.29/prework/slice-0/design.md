# Slice 0 workspace and setup design proposal

One writer per checkout. Mutable venv/native outputs/npm installs/build/scratch
belong to the selected checkout/task. Immutable model/download caches may be
reused only after input identity and availability are known. Never borrow a
venv or repoint another checkout's editable bindings.

The old planning worktree was archived during this session. Its snapshot
`a4fb5173a4bfa0bf83ff0d7bf1bcb4bec1abee5c` preserved all 35 changed/new planning
files. This chat now owns attached worktree
`/home/coreyt/.codex/worktrees/fathomdb-0829-work/dev-0.8.29`, on the original
`llm/0.8.29-chat` branch at `6edc1155f`. Recovery verified byte equality for all
35 files before prework edits. The snapshot remains; no force/reset or product
commit was made. Canonical integration stays `dev/0.8.29` in its other checkout.

Use noneditable source-bound wheels in an owned linked-worktree venv, subject
to reviewed preparation. Generic bootstrap/build currently run editable installs;
resolve that conflict before invoking them. Preserve R580 and primary CUDA 12.6.
No fault/reset action or GPU readiness claim belongs to environment inventory.

## Current substrate and proposed delta

Read-only source inspection at `6edc1155f` confirms format37, existing MCP
`embed-cuda` and `publish=false`, Rust1.95 and pinned Node/npm/Python developer
tools. Source-contract evidence differs from inherited host probes in the
[environment register](environment-register.md). No new product platform,
provider contract, migration or second CUDA artifact is designed in0. Owner
seq-368 forbids .29 migration work; seq-367 confines140 repairs to A and moves
broader fork maintenance to .30. Existing query scaffold remains.

0 changes these proposal documents only. Approved10 owns actual environment,
state and bounded setup-guidance corrections. Keep1's security/dependency choices
and7's delivery fixes with their owners. Current bootstrap/build editable-install
sites conflict with linked-worktree isolation; noneditable owned wheels are the
proposed route, not a claim that existing scripts already implement it. Resolve
and review the affected invocation path in10 before running it.

## Reproducible preparation checks for10

Before initial state bootstrap in an assigned existing idle checkout, inspect
`git status --short`, `git worktree list --porcelain`, branch, full HEAD and cwd.
Confirm the owner, preserve existing changes, and compare the target to refreshed
allowed .29 integration refs. Do not create a worktree to evade unavailable live
state. Bootstrap approved state/board from real decisions and Git facts, keep10
open, then validate discovery and generated views. Only afterwards may strict
`preflight.sh --worktree <path> --expect-closed <actual-dependency> --plan dev/plans/plan-0.8.29.md` establish dependent eligibility. The actual dependency
must be genuinely closed in the activated state and ancestral to the target.
Do not manufacture a closure or substitute plain health for this check.

For a later source checkout, the parent verifies current release/dependency refs,
checks baseline/dependency preflight before worktree creation, creates a fresh
task-owned `llm/` worktree from the verified integration SHA, then verifies its
ancestry, required files and strict target-worktree/dependency preflight before
the source owner begins. Move the execution session cwd into that checkout. One writer owns each checkout. No agent
switches or prepares another session's checkout.

Before invoking selected tooling, compare executable versions and resolved paths
to source pins; confirm `.venv`, root/TS modules, target and native outputs are
owned by that checkout. A wheel/native build must identify exact source (SHA plus
any dirty delta), lockfiles, features, target, interpreter/ABI and output digest;
installed import/module paths must resolve to its owned outputs. No editable
rebinding or manual native-library copy is an alternative. Import/open/close
verification belongs to10 or the affected downstream owner, not this draft.

## Resources, venues and scope stops

Task-owned /mnt/data scratch/build and command-local tools are already
scope-approved by [decision batches1/2](../slice-9/decision-log.md). The
[direct mandate](../../direct-execution.md) advances approved work through review
gates without another generic permission request; exact
reservation/access and measured phase budget remain10 prerequisites. Start with
one native build/full gate at a time; do not infer a Cargo job count from logical
CPUs. Retain the existing preflight disk floor, and require10 to set a measured
phase reservation/stop threshold on every output filesystem before heavy work.
Without that information the affected build does not start. Check current free
space, memory pressure and competing GPU/VM users before each campaign; high
swap occupancy alone does not prove active thrashing.

Missing corpus location/license/hash or model revision/digest blocks the affected
measurement. Missing GPU UUID/ordinal/driver/access blocks the affected GPU work,
not unrelated CPU proposals. Preserve R580 and primary12.6 and the standing
.28 installation. No reset/fault/VM reconfiguration, cleanup, policy/toolkit change,
priced run, optional experiment or publication is authorized by this inventory.
No new generic approval gate is inferred from an old role document.
