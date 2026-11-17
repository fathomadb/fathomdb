---
title: "0.8.29 release continuation prompt — 2026-11-17"
status: ACTIVE
target_release: 0.8.29
---

# Continue FathomDB 0.8.29

Complete the remaining 0.8.29 implementation and qualification, including
appropriate CI, **except registry publication**. Finish required local
qualification first, then review CI scripts for correctness before running
CI. Do actual remaining work; do not create another planning or receipt phase.
The previous chat wrapped up at the checkpoint below. The release is incomplete.

## Start here

- Canonical worktree: `/mnt/data/worktrees/fathomdb-0.8.29`.
- Branch: `llm/0.8.29-chat`; local release ref: `release/0.8.29`.
- Clean checkpoint before this handoff-only commit:
  `7888893c39bc6554a41ffd0197bc026c6366ec1f`.
- Run `python3 scripts/release-current.py`; read its selected
  `dev/plans/release-state-0.8.29.json` and `dev/plans/runs/STATUS-0.8.29.md`.
  Those files and actual Git/runtime results override this snapshot.
- Read `AGENTS.md`, applicable ADRs and the verified memory index at
  `/home/coreyt/.claude/projects/-home-coreyt-projects-fathomdb/memory/MEMORY.md`.
  Especially read `dont-be-distracted-by-ceremony.md` and
  `full-gate-run-frequency.md` in that directory.
- The previous goal is BLOCKED on GPU availability. There are no running
  verification/native sessions, held GPU leases or unfinished Git operations
  from this chat. Previous subagents finished; do not depend on their chat IDs.
  Recheck current processes and ownership before assuming this remains true.
- Keep one writer per checkout. This worktree already has the correct location;
  do not move it again. Main thread owns release-state edits. For source fixes,
  give each concurrent implementer a fresh isolated worktree.

## Working rules and settled scope

- Use direct GPT-6.1 Sol high implementers and independent Astra medium reviews.
  Do not invoke Steward/Orchestrator roles or skills, or Ai CLI MCP.
- Review scope/design against preceding changes and requirements/acceptance;
  use genuine RED→GREEN for defects. Batch reviews around meaningful work.
  Already-correct controls are not fabricated REDs.
- Feature/function work first. No new receipts, calibration, census, hash
  inventories or hygiene campaigns. Update existing state/docs concisely.
  Do not rerun passed scientific, artifact or broad gates for bookkeeping.
- Busy CPU host is accepted. No quiet-machine or CPU-controller prerequisite.
  GPU measurements still require their actual exclusive selected GPU.
- Use owned storage under `/mnt/data/`; clean only this task's unneeded data.
  Preserve raw results, active subjects and anything used by other work.
- Format37; **no database migration**, compatibility shim or Memex snapshot gate.
  Retain the query clone scaffold and existing hints/span storage. No inferred
  calibrated verdict or weak-result threshold.
- Fork B/C and actual reset qualification are .30; new CUDA13 delivery,
  quantitative work and Tegra CUDA13 qualification are .31. Non-Tegra ARM+GPU
  is excluded from .29 gates/known-working claims. Slice150 remains unselected,
  not an inferred mandatory experiment or a newly owner-deferred study.
- Never change R580/primary CUDA12.6, reset GPUs, interfere with foreign work,
  start the Windows VM or restore its already-ended ballooning hold.
- No registry uploads, publishing tags, Pages publication or post-publish claim.
  A `v*` tag push triggers real publication. Authorized ordinary branch pushes
  to this repository and `coreyt/cudarc` / `coreyt/candle-fathomdb` need no repeat
  permission; never force-push or push upstream/other repositories.

## What is already finished

- Slices0–10,20,30,40,60,70,80,100,110,130 and140 are complete in their selected
  scope. Slice40 is conditional non-adoption, not an optimization awaiting a vote.
- Slice30 is CLOSED independently of50: G84, D24, M46 and actual TC5 F/Q8 are
  accepted. M records36,795 RPC outcomes; F/Q8 uses7667 real documents/100 queries,
  K192/top10/exact-f32, recall .958 with CI[.938,.974]. Windows A6 is complete.
  Preserve original failures and original subjects, including G's outer status1.
- Slice50 identity, atomic ambiguous feedback refusal, exact winner hydration
  and bounded record-start snippets are implemented and source-qualified.
  Only its paired measurement exit remains, followed by combined closure.
- Full90 behavior is implemented and independently accepted. Its A1–A11 audit
  found no additional local behavior/artifact gap. A12 still needs the50/Block1
  exit and combined qualification. Do not reopen source entry or80 approval.
- **Paid72 completed ONCE** on actual222/b3c6:72 valid episodes/144 grades,
  no invalid/pending/unknown outcomes, reported total$0.13444673. Independent
  final acceptance is recorded. Never replay them. Existing experiment report:
  `dev/experiments/agent-remeasure/README.md`.
- Current installed artifacts, GPU2+4 controls, MCP14 and archive consumers have
  accepted evidence and explicit producer/applicability limits in160 status.
  Frozen50 query comparisons use snippets=None. Old setup/RSS and snippet-enabled
  MCP timings describe their original binaries; do not retag them as full90 timings.
- Throughput is resolved: matched OLD1495.233/current1482.090 writes/s both pass
  the unchanged1000/s gate. Historical760/993 failures remain. No statement-cache
  production change or further throughput rerun is needed.

## Current code phase is closed — do not repeat its full gate

The portable dependency fix preserves exact343 cudarc0.20,2ee Candle0.12,
K9 kernels0.10.2, logical Git IDs, root lock bytes, source bytes and features.
Root/SDK-driver manifests own the qualified patches; `.cargo/config.toml` has
source-only same-commit HTTPS mirrors. SDK-driver stale lock was Cargo-reconciled.
The task's redundant global Cargo patches were removed; do not restore them.

- Final portable implementation: `ef6364fa03657e89305b4b2cb64949923e762829`.
  Independent SOURCE+LOCAL METADATA ACCEPT; real RED `02cec6839`.
- Full code-phase subject: `b6749b4bcf33816e068c61c270ae531ef6968cf7`.
  Actual terminalEXIT1 after2781s:239 registered,236 ran,235 initial passes,
  one obsolete CUDA-contract failure, three explicitly unselected private G/M/D
  live-control skips, zero excluded.
- Correction: `a55ec51330816e5c7fecabc55cb1d4371404b4fc`, real RED `d8c2d18fb`;
  independent SOURCE+FOCUSED CONTRACT ACCEPT. It admits only the exact
  source-owned approved patch cohort, retaining all CUDA/K9/artifact safeguards.
- Canonical failed-only CUDA suite then passes121 checks, terminal0.
  All236 executed outcomes are resolved **as assembled qualification**;
  original wrapperEXIT1 and three skips remain, not an uninterrupted green claim.
- Workspace lint/types, strict security0/0/0, Rust workspace, selected adapters/
  observers/CLI, Python/native receipt, TS and hermetic N-API passed.
  Final state/board scoped Markdown, local links and state validators passed.
- Raw root: `/mnt/data/tmp2/fathomdb-0829-continuation/root-release-20261116/logs/`.
  Main files: `portable-code-phase-b6749b4bc.log`,
  `portable-code-phase-cuda-only-final.log`, `portable-code-phase-close-md.log`.
- Exact dependency graph/byte proofs:
  `/mnt/data/tmp2/fathomdb-0829-continuation/portable-fork-delivery-20261116/`.
  Ten baseline/portable pairs preserve package IDs, features and dependency edges.
  Root lock SHA256:5f16be9eea49a68e196a4b0d8df029177db084cf296d7a0219ff30d5ac784899.
- Owned canonical tooling: source
  `/mnt/data/tmp2/fathomdb-0829-continuation/root-release-20261116/env.sh`.
  Use each checkout's own Python environment. Node25.9/npm11.12.1, Rust1.95,
  Ruff0.15.17 and Pyright1.1.410 are selected; do not upgrade for warnings.

## Next action: resume the original Slice50 remainder when the GPU is free

Snapshot: **58/136 ordinary credited and complete;0/24 diagnostics**.
Remaining:78 ordinary +24 diagnostic members. No existing runtime handle is live.
Last native session50129 endedEXIT1; do not poll or restart that old handle.

Paths to retain:

- Frozen controller worktree:
  `/mnt/data/worktrees/fathomdb-0.8.29-s50-report-recovery-20261116`.
  It must remain clean at `c2ed2e8550635ec17291e95c0008f7511e04639c`.
  Native subject stays09c534/compiler8465; no rebuild or source mutation.
- Lane: `/mnt/data/tmp2/fathomdb-0829-continuation/s50-g-visibility-20261116`.
- Paired root: `/mnt/data/tmp2/fathomdb-0829-continuation/slice30-g-20261113/gd-allocation/s50-required-p1-10k-r0-09c53495b`.
- Shared ledger: `/mnt/data/tmp2/fathomdb-0829-continuation/slice30-g-20261113/gd-allocation/g-budget.json`.
- Selected3090: `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`, R580.178.04/CUDA12.6.
- Physical FD9 lock:
  `/mnt/data/tmp2/fathomdb-0829-continuation/slice30-gd/locks/GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b.lock`, inode8532374.

The known blocker was `coralsmol-250-lfm230-v37-r2g-sft.service`, active/running,
MainPID1564817, selected-GPU child1564859 using5988MiB. The other3090 also had
its child1564860; switching devices would not help and is not approved here.
Service start23:00:24CST November16; RuntimeMax4h43m20 is a configured upper
limit, **not an ETA**. Inspect only resource/unit metadata; do not read foreign
logs/source, signal it or change its settings. Require this known unit actually
terminal/inactive AND selected GPU empty, not a transient empty gap during its
training, before spending the one remaining resumption attempt.

Original054–064 credited four new members, then the next winner's observer
raised `selected GPU has foreign compute work`. The payload exit left its own
native child for normal containment; attempt125 `owned work survived leader`
was secondary. Cleanup is certain: owned actors gone, exact cgroup empty/removed,
all five ownership fieldsNULL, marker absent, physical FD9 released. No source
bug or timeout was established. Preserve original failure and passed54–58.

Existing explicit approval permits **ONE attempt0002** of failed
`s50-g-w-b1-s100-engine-hybrid-r0-ordinary-winner`, then its unrun counterpart
and the remaining exact original[58:64] members. Do not ask again or replay
passed members. Preserve failed charge120,597,189,483ns: member remaining
779,402,810,517ns; pair remaining1,679,402,810,517ns. Shared used at handoff
61,444,866,903,705ns. Re-read actual ledger; never edit/reset charges or custody.

After actual job/GPU clearance and fresh resource/source/ownership admission,
run the already-prepared entry once, capturing output in a new owned log:

```bash
bash /mnt/data/tmp2/fathomdb-0829-continuation/s50-g-visibility-20261116/prepared-original-prefix-058-064-c2ed2e8550-20261116-entry.sh
```

Inspect its sibling selection JSON first. The entry validates exact next IDs,
clean c2 source and physical lock; the public guardian owns dispatch/accounting.
Limits:3pairs/5600s phase/common600s,900s/member and1800s/pair CUMULATIVE,
24GiB growth/256MiB common output, original48h/192GiB TOTAL and all old charges.
Do not hold a lease while waiting for resources. Stop at any failure/refusal;
no automatic third attempt. Poll the actual returned live handle to terminal;
an observation timeout is not permission to restart.

After qualified terminal and strict cleanup, the existing standing GO continues
original finite prefixes serially:064–074,074–084,084–094,094–104,104–114,
114–124,124–134,134–136,136–146,146–156,156–160. Use ONLY the existing
`*-c2ed2e8550-20261116-entry.sh` / selection files in the lane, fresh admission
at each entry and their actual unchanged limits. Do not use older a0/359/270
scripts, widen sample counts or manufacture extra calibration. No per-prefix
human/model review is required. Stop first failure; no automatic retries.
Retirement is only the existing qualified-copy retirement, never manual cleanup.

The original044–054 report-only cutoff recovery already credited ten completed
members without native replay and survived later ordinary reporting. Keep its
original guardian124, separate successful assessment, history and charges.
Natural G-W comparisons intentionally retain `equal=false` for observed cursor/
readiness differences and `result_equal=true` for exact results; fixed-state
full equality is unchanged. Do not reopen these settled source/oracle corrections.

## Finish after50, then CI

1. Review the actual final136+24 outcomes once as a coherent cohort; complete50
   and the90 Block1 exit, then160's combined applicability/acceptance joins.
   Existing release/slice plans and requirements remain controlling.
2. Only after required local qualification passes, review CI scripts for
   correctness, including trust, immutable candidate, dry-run and publication
   protections. Then execute appropriate nonpublishing CI. macOS x64/ARM64 and
   hosted release rehearsal remain; Linux/ARM/Windows local installed evidence
   retains its exact producers. Do not silently relabel CI-only rows as local PASS.
3. Cudarc343 is remotely available on
   `coreyt/cudarc`, branch `llm/0.8.29-cudarc-version-20261115`.
   Candle2ee public availability/fresh HTTPS fetch is still pending. Its fork has
   an unconditional push scan with Actions enabled: review before the permitted
   source-branch push. This staged split was independently accepted; do not add
   a circular local hold, bypass CI order or claim an unperformed network fetch.
4. Keep343=`343b04fa73f85ee78754d6ee2baa07b959cc9ccc`,
   2ee=`2ee39305a78ed1463ff913f71dcd3d45ecbfd84f`,
   K9=`182ceab0547348993b6dbec4b1aed610dcc8b23f` and their source identities.
   No speculative registry checksums or registry-ready downstream claim.
5. Close only when every selected nonpublishing requirement has actual evidence.
   Registry publishing and post-publish consumers remain outside this mandate.
   Do not mark the release complete merely because the user wants it finished.
