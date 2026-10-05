---
title: Slice 110 Tegra allocator integration intake
status: OPEN
target_release: 0.8.27
date: 2026-10-05
---

# Slice 110 Tegra allocator integration intake

On 2026-10-05, the clean
`/home/coreyt/projects/fathomdb-worktrees/slice-110-tegra-allocator-fix`
worktree was fast-forwarded to
`llm/slice110-tegra-allocator-fix` at `77d742153`. This is a branch intake,
not a release merge or Slice 110 qualification. At intake, the release branch
had ten commits absent from the Tegra branch, and the Tegra branch had eighteen
commits absent from release. Release state still has Slice 110 `IN_PROGRESS`
and `next_slice: 110`; Slices 114 and 115 completed under separate HITL
sequencing exceptions. Preserve that state when integrating branch documents.

## Branch result and limits

The branch's `dev/plans/0.8.27/features/slice-110/tegra-allocator-handoff.md`
and `dev/plans/runs/0.8.27-slice-110-tegra/receipt.md` report an
aarch64-Linux-only vendored cudarc fallback when the Orin default CUDA memory
pool cannot be obtained. Reported final-code checks include the fragmented
address-space regression 12/12, vendored unit tests 15/15 with a RED witness,
Node 25.9.0 in-tree and installed 10/10 each, forced CPU 3/3 in each form, and
a rebuilt Tegra Python wheel 10/10 (five normal, five fragmented). These are
branch receipts; integration must bind fresh results to the merged code.

The latest branch receipt reports that `agent-verify` stopped at the Slice 90
checkpoint lint failure also seen at its baseline; separate typecheck and
security legs passed. All-tier `agent-test` reported 178/184 suites. The six
failures were attributed there to missing local release state, Python 3.13
`cgi`, shallow-clone transport, Python loader path, and worktree Python
installation restrictions. This is not a full gate pass. Fix round 3 measured
the fallback's synchronous path about 1.8–2.8 times slower per steady embed.
A late import into a very large Node heap can still fail at `cuInit` before
allocator selection. The updated branch has now compiled and run on x86_64
as recorded below; hosted CI compilation remains part of integration.

## Work before integration and closure

1. **Independently review the completed Tegra follow-up.** The six low-severity
   fix-2 findings were addressed on the candidate branch: the teardown probe
   no longer holds an explicit pool; claims are scoped to the measured Orin
   and name unmeasured aarch64 Linux CUDA hosts; the tautological test was
   removed; the regression skip is limited to absent, stub or unmeasured
   devices; and the measured slowdown is consistently about 1.8–2.8 times.
   The remeasurement found that early `cuInit` does not reserve the default
   pool's eventual 20.47 GiB range, supporting the need for both changes.
   Review the source and retained evidence before promoting those findings.
2. **Review and qualify the final candidate.** After the Tegra agent's final
   push, independently review the exact vendored delta, patch
   reproducibility, governance exception, tests and evidence. Run real x86_64
   CI compilation and applicable hosted platform builds, then repeat
   installed Node forced-CUDA and allocation-witness checks on the Orin. Run
   the required full source gate on a capable release executor; classify any
   environmental failure against an unchanged baseline rather than counting
   it as green.
3. **Reconcile branch state before merge.** Bring the allocator commits onto
   the current `release/0.8.27` without replacing the completed Slice 114/115
   records or the live `next_slice: 110`. The branch handoff reports its todos
   ledger at seq 270 versus origin/main at seq 242. Use `ledgerwatch` and
   `ledgerwrite` to inspect the full current tails and reconcile the single
   appended row before changing any ledger or generated release-state view.
   Ledger order agreed on 2026-10-05: the Slice 110 revisit todo is seq 270
   on this release lineage (base seq 269), and the Slice 117 todo on
   `llm/slice117-jetson-node-cuda-plan` is seq 271 after a byte-identical
   copy of that seq-270 row, so the two branches merge cleanly in either
   order. If `release/0.8.27` gains another ledger row first, both must be
   re-appended through `ledgerwrite` rather than hand-edited.
   Recheck the branch and release tips at integration time; the SHAs above are
   this intake snapshot.

Only after these items have candidate-bound evidence should Slice 110 status
and the release ladder advance. No publication follows from this intake.

## Early `cuInit` is in 0.8.27 Slice 110

**Correction, 2026-10-05.** The earlier text of this section recorded early
`cuInit` as deferred out of 0.8.27. The repository owner has since ruled the
opposite after reviewing the evidence: Slice 110 ships **early `cuInit` at
Node addon load together with the aarch64-Linux synchronous allocation
fallback**, and both are part of the 0.8.27 integration. Integrate, review
and qualify them as one Slice 110 candidate. What remains undecided is only
whether to go further to recover stream-ordered allocation speed (an
explicit memory pool, or early `cuInit` with a lazily created pool); that is
outside Slice 110.

The two changes address different failures, and both are needed:

- The synchronous fallback handles the driver's default memory pool being
  unavailable (it needs one contiguous 20960 MiB range in the
  [8 GiB, 128 GiB) address window).
- Early `cuInit` handles `cuInit` itself returning
  `CUDA_ERROR_OUT_OF_MEMORY` once a grown V8 heap leaves no 4 GiB hole in
  that window. The fallback cannot help that case; without early `cuInit`,
  heap-heavy Node processes cannot use CUDA at all.

Evidence the ruling rests on, measured on the Jetson AGX Orin 64 GB
(L4T R36, CUDA 12.6) and recorded on `llm/slice110-tegra-allocator-fix`:

- Plain C, no Node: with `cuInit` first, 100–2000 later scattered mappings
  passed 160 / 160; with 500 or more pages mapped before `cuInit`, it failed
  60 / 60.
- Node experiment: import, then grow the heap to 200k–4M objects — 200 / 200
  passed with early `cuInit`, against 26 of 40 failing without it.
- Implemented change: heap-growth runs on Node 24, 25 and 26 passed 50 / 50;
  the red/green check went from 3 / 3 refused to 5 / 5 passing; a late
  import into a large heap is still refused (5 / 5) with a message naming the
  cause, and `node --import fathomdb` rescued it 8 / 8.

The implementation is compiled only for aarch64 Linux with a CUDA feature,
is silent and non-failing at import, and is skipped when every CUDA-built
component's policy is `cpu`. The typed error, `cuda_probe_failed` kind and
no-CPU-fallback refusal contract are unchanged. A review round moved the
call out of the ELF constructor (loader-lock hazard) into module
registration; fix round 3 re-established the import-first, late-import,
`node --import`, installed-package, and worker results on that source. Residual
limitations to carry: a late import into a large heap still fails; the
synchronous path is about 1.8–2.8 times slower per steady embed; only the
AGX Orin 64 GB was measured. Both changes carry a ledger obligation to be
revisited at the next micro release, and loudly at the next minor release.

The opening branch-result counts describe the `77d742153` intake. The
candidate has since gained early `cuInit` and further review fixes; its
fix-round-3 receipt reports installed and in-tree Node 25 checks at 10/10
each, import-first heap runs across Node 24, 25 and 26, and a rebuilt Tegra
Python wheel at 10/10. Recheck the branch tip at integration time.

## AMD64 regression round on the updated branch

**2026-10-05, candidate `8b76f6115`.** Both origin refs were refreshed:
`release/0.8.27` remained `80246a567` and
`llm/slice110-tegra-allocator-fix` remained `8b76f6115`. The candidate
includes the aarch64-Linux cudarc fallback, the early `cuInit` call moved
from an ELF constructor to N-API module registration, and the review fixes
listed in its intervening commits. Their target gates exclude x86_64 Linux;
this round exercised the x86_64 CUDA path on a real GPU. This is
candidate evidence, not a merge or Slice 110 closure.

- On x86_64 Linux, the vendored cudarc test script passed 11/11. Workspace
  `cargo clippy --workspace --all-targets -- -D warnings -A missing-docs`
  and `cargo check --workspace --all-targets` passed. The N-API crate also
  passed `cargo check --locked -p fathomdb-napi --features
  embed-cuda,rerank-cuda --all-targets`, and the TypeScript typecheck passed.
- The official `build-napi-cuda.sh` built the release N-API binary with
  `embed-cuda,rerank-cuda`, CUDA 12.6.68, GCC 13.3.0, and Node 25.9.0.
  The binary SHA-256 was
  `c9da045b736886411a9fa1a2d2ba0ba1005c173f547842447cc34e8005bd08f2`.
  Separate main and Linux x64 platform tarballs were packed and installed
  into an isolated local consumer; their SHA-256 values were
  `296a4b4157418e3a66fd194e8be45c882d7035d4d06cf9463f7e9d8d1a1092c6`
  and
  `cad97c828bf953ffb4673fc7bd611fc018f800c68635695bede424ef03e4145e`.
- In three fresh installed-package processes, forced CUDA embedding and
  reranking passed 3/3 on an NVIDIA GeForce RTX 3090, UUID
  `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`. Each open selected CUDA,
  the explicitly requested allocation witness recorded a 134,217,728-byte
  GPU delta, embedding returned 384 values, and reranking returned two
  finite-score results. Forced CPU embedding and reranking passed 1/1 with
  both device resolutions reporting CPU. The witness requires
  `FATHOMDB_GPU_ALLOCATION_WITNESS=1`; without that opt-in, its `null` report
  field is expected and is not a CUDA failure.
- A follow-up CUDA run on the same candidate built a host-native Python wheel
  with `embed-cuda,rerank-cuda`, installed it into an isolated Python 3.12
  environment, and passed forced CUDA open, embedding, reranking and
  allocation-witness checks in four fresh processes (4/4). The wheel SHA-256
  was `08bb82395d866cf92b274da9a7cccb5f20bdf88bd43420e893215ef34159aee3`.
  Every run selected CUDA for both components, recorded a 134,217,728-byte
  GPU delta on the named RTX 3090, returned 384 embedding values and reranked
  two passages with finite scores. The installed Node package repeated the
  same CUDA checks in three more fresh processes (3/3, 6/6 across the two
  Node rounds). With `CUDA_VISIBLE_DEVICES=-1`, both installed consumers
  refused forced `cuda:0` with `NoVisibleCudaDevice` (1/1 each), rather than
  falling back to CPU. The Python wheel is a host-only regression artifact
  built with maturin 1.14.0; it is not the release manylinux wheel.
- The full `agent-verify` attempt stopped at preflight because this
  worktree lacks `.venv/bin/python`. No full-gate result is claimed. The
  checkout must be prepared for a final source gate without an editable
  install that repoints a shared Python binding. The amd64 round does not
  replace the pending Orin installed-package repetition or merged-candidate
  release gate.
