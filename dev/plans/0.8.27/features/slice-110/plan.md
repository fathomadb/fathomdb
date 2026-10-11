---
title: FathomDB 0.8.27 Slice 110 execution plan
status: IN_PROGRESS
target_release: 0.8.27
entry_commit: 9340a8246e39127c255b55a3f97b674553eb6908
---

# Slice 110 execution plan

## Entry reconciliation

The prospective design was reviewed at `63091b6249e5805ea71b44de59f0ac6703b786a9`.
The clean `release/0.8.27` worktree now points at `9340a824`. Slice 103's
qualified source is `c2e80ff`; subsequent commits record closeout evidence.
This inventory is the approval basis for the Slice 110 draft:

| Change since draft | Effect on Slice 110 |
| --- | --- |
| Slice 90 added the five-field EngineConfig bridge, native validation, `test-hooks` accessors and installed Node qualification. | Capture these exact current exports, config representations and feature gates; preserve them during extraction. Do not implement another runtime pool. |
| Slice 100 implemented a bounded Python logging subscriber and accepted its own heartbeat disposition. | Use its engine registry and SQLite callback safety lessons. Its ADR expressly leaves TypeScript undecided. |
| Slice 103 changed engine SQLite/WAL behavior, dependency pins and lockfile, and added CLI-only recovery; it changed no NAPI or TypeScript source. | Build fresh Node artifacts from the current source. The Python Tegra and Windows wheel receipts do not qualify Node packages; recovery adds no NAPI export. |
| The runtime-topology successor superseded the old async ADR's exact pool/ThreadsafeFunction mechanism. | Keep Tokio `spawn_blocking` for ordinary engine work and test its outcomes. ThreadsafeFunction is appropriate only for host subscriber delivery. |
| Active binding/interface docs now identify the inert native TypeScript subscriber. | Resolve the callback and heartbeat contract with an accepted successor and RED tests before moving its owner. |
| The master plan reserves TypeScript SDK domain decomposition for Slice 120; no Slice 111 exists. | Restrict TypeScript edits here to the subscriber contract, native declarations, loader and packaging corrections needed for native closure. |

The assigned native families are `errors`, `execution`, `engine`, `types`,
`write`, `read_search`, `graph_evidence`, `projection`, `embedding`, `admin`
and `test_support`, with a registration-only root. The draft's R27-110A–F
remain the slice requirements. BC-3/4/5/7/8/9 in
`binding-closure-review.md` are allocated here. The current 5,505-line
`fathomdb-napi/src/lib.rs` has one primary Engine impl, three test-hook
Engine impls, free functions and generated object registrations. Enumerate
every item and every generated/runtime export before moving it; no family
placeholder is a sufficient inventory.

## Approved scope and acceptance

Approve the draft's six requirements with these entry clarifications:

- **R27-110A/B:** Reconcile source items, cfg attributes, macro registrations,
  production declarations, loaded runtime exports and installed package
  exports independently. A root/facade retention needs a concrete macro or
  ownership reason. Mechanical changes preserve exact names and call shapes.
- **R27-110C:** Keep the accepted `spawn_blocking` handoff. Separately make
  `attachSubscriber` deliver bounded diagnostics on the JS thread, with an
  explicit TypeScript heartbeat decision, callback fault containment,
  replacement/close and process-exit policy. The subscriber contract delta
  is decided in a successor ADR and mirrored in the public interface/docs.
- **R27-110D:** Characterize native conversion and error precedence using
  actual signatures and existing human-authored tests. Add focused witnesses
  for uncovered risky cases; avoid a synthetic exhaustive matrix unrelated
  to the binding's accepted input types.
- **R27-110E/F:** Qualify current-candidate feature/platform rows and fresh
  thin-main plus matching platform package installation. Exact source,
  artifact hashes and toolchain bind the review. An unavailable required row
  stays open rather than being represented as a pass.

## RED → GREEN → review sequence

1. Freeze entry item/export/declaration/platform manifests. Read active ADRs,
   TypeScript interface, packaging recipe and source consumers. Record the
   executor and subscriber dispositions. Review the design before coding.
2. Add a native-artifact RED subscriber delivery test and missing
   executor/lifetime/conversion witnesses. Stage or commit RED tests visibly.
   Fix the callback contract, then run focused Node and native tests.
3. Extract private owners in cohesive batches, keeping one Engine class and
   the existing macro registrations. After each batch run format, the owning
   typecheck/tests and runtime/declaration surface comparison. Retarget any
   source scanner using a failing hidden-owner fixture first.
4. Build production NAPI using the checked wrapper, compare production
   declarations and loaded exports with the reviewed entry plus named
   subscriber delta, and verify test hooks are absent. Pack and install the
   matching main/platform pair in an external consumer.
5. Run the applicable clean-candidate repository and platform gates. Obtain
   independent `gpt-6-sol` high code review and separate verification agent
   evidence. Repair focused findings with focused tests; rerun broad gates
   only when the repair changes their covered behavior.
6. Write a candidate-bound status and Slice 120 handoff. Advance release
   state only after R27-110A–F pass. Keep this release worktree as the branch
   checkout; no extra slice branch or worktree is needed.

Disk free space was 16 GiB at entry, so capacity is checked before native
artifact builds. The prior full check's optimized AC-013 settings and ptrace
executor are recorded as environmental gate conditions, not Slice 110 passes.

The required shipping rows frozen at entry are Linux x64 GNU (ordinary and
CUDA), Linux arm64 GNU/Tegra, macOS x64 and arm64, and Windows x64 MSVC,
subject to the existing release workflow's exact feature selections. Also
exercise default-embedder, no-default-embedder refusal, test-hooks and
applicable reranker feature paths. Each row needs an exact candidate and
installed Node package witness; a prior Python wheel is not a substitute.
