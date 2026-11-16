---
title: Slice 160 local qualification status
status: IN_PROGRESS
target_release: 0.8.29
---

# Slice 160 local qualification status

Current source is `d259d84c59e38861de11192cb8764979458fb127` on the isolated
`llm/0.8.29-s160-local-20261116` worktree. The [design](design.md) selects the
existing two required CPU long-test variants and offline verified leaf packages.
Independent Astra-medium design review accepted this batch before execution.
No source behavior or test oracle changed.

## Completed independent local batch

All commands used Rust1.95, the unchanged locked source above, an owned offline
Cargo cache/target and ordinary CPU execution. Raw logs and archives remain under
`/mnt/data/tmp2/fathomdb-0829-continuation/slice160-local-20261116/`.

| Existing check | Actual result |
| --- | --- |
| Q10 AC021, `--features operator`, `AGENT_LONG=1`, exact selector |1test passed;60-second window,59 projection add/drop cycles,59 rebuilds, zero `SQLITE_SCHEMA` events. `logs/ac021.log`. |
| Q10 cursor race, `AGENT_LONG=1`, exact selector |1test passed;1000 searches,401 committed writes including400 during reads,999 advancing hits; zero row-count/cursor and hit-cursor violations. `logs/cursor.log`. |
| Offline leaf packages, `cargo package --offline --locked --allow-dirty` |All3 archives verify-compiled: embedder-api0.8.0, schema/query0.8.29. Cargo's unused optional fork-patch warnings are retained in `logs/package-*.log`; no verification bypass. |
| Q4 missing AC037 live components |Current locked `security_cycle` rebuilt; both canonical netns engine and demonstrate-the-catch scripts exited0, including the live deliberate-egress catch. `logs/security-build.log`, `logs/check-netns-deny-egress.log`, `logs/check-netns-deny-egress-catch.log`. No host settings changed or optional downgrade selected. |
| Q14 current Linux x64 production consumers |At `b07a161317ff753b86b5eb706e7c5e761adecb9a` (unchanged runtime source), the existing owned Python setup built an abi3 .29 wheel with `pyo3/extension-module,default-embedder`; the existing production N-API route and TS build passed. No test-hooks. The existing local native smoke passed isolated offline Python and matched Node installs, strict installed graph types with TS6.0.3, write/search/dependency/frozen-read and Node graph-evidence controls; Python3/Node3 runtime-configuration cases passed, zero skipped. `logs/production-{python-build,napi-build,typescript-build,installed-consumers}.log`. |

The long variants close the missing Q10 observations on these inputs. AC037 closes
the missing live component alongside applicable accepted unchanged security
controls; no aggregate battery or full-workspace green run is claimed. The three
leaf archives are local package preparation, not all shipping artifacts,
Python/Node/platform consumers, complete rehearsal or registry readiness. Reassess
affected runtime/package inputs after90; unchanged observations need no replay.

The Linux wheel is host-native `cp310-abi3-manylinux_2_39_x86_64`, built with
Python3.13.13/Rust1.95; Node25.9 consumed its matched production package. This
runtime observation does not qualify the required manylinux2.28 distribution
floor, CUDA artifacts, all five targets, final90 bytes or registry consumers.
The wheel remains in `tmp/fathomdb-owned-python-3d8ud5k0/` under the raw-log root;
the built addon/declarations remain in this worktree's `src/ts/`.

## Remaining qualification

- 20's four Windows rows and the assembled238-suite source gate are complete;
  ordinary smoke/skip behavior remains distinct from the actual long runs above.
- 60/70/80 and100 have accepted selected exits;110/140's selected .29
  source/healthy/OOM/consumer scope is independently accepted.30's remaining
  G13/84, M4/46 and D12/24 campaigns and50's ordinary6/136, diagnostic0/24 paired
  observations remain incomplete. These are execution progress, not outcomes.
- 40 awaits the actual owner choice,72's approved model episodes have not run,
  and130E's Node soak is active.90 remains behind the actual Block1 exit.
- Final integrated qualification, five-target installed packages, selected
  CUDA12.6 consumers and the complete local rehearsal still require applicable
  actual inputs. Earlier accepted observations retain their scope; no automatic
  full replay follows document edits.
- After local passes, review CI scripts and execute CI. The earlier blanket CI
  exclusion is superseded. No publishing tags, registry uploads or published
  registry-consumer proof is authorized; readiness and publication remain distinct.

Owner dispositions in [requirements](requirements.md#owner-dispositions) retain
format37 with no migration, reset work in .30, CUDA13 including Tegra in .31,
and non-Tegra ARM+GPU excluded from .29 gates/known-working claims. No private
Memex population census or new capacity/admin ritual blocks this local batch.
