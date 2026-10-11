---
title: FathomDB 0.8.27 Slice 115 code review
status: APPROVED
reviewed_sha: 012e132920147396ac195f14af74444dd698f48e
---

# Slice 115 independent code review

`gpt-6-sol` high reviewed the measurement implementation against the approved
plan and design. Its initial review found three issues:

1. Projection timing began after `write()` returned even though projection
   work can start before that return. The implementation now times from before
   write through drain, retaining separate write and drain measurements; a
   new candidate receipt was collected.
2. Raw-controlled booleans made mandatory model-projection and mutation-state
   checks optional. The validator now enforces them unconditionally, with
   focused mutation tests.
3. The candidate check omitted `Cargo.lock` despite using that lockfile for
   the build. The protocol now pins its digest and the runner rejects drift.

The reviewer reread clean commit `012e132920147396ac195f14af74444dd698f48e`
and approved it with no remaining finding. It also confirmed that the
Gitleaks exception matches only the pinned tokenizer digest and exact Slice
115 evidence paths, with a negative scanner test. Independent recomputation
matched the protocol, runner, lockfile, corpus, binary-bound profile digests,
seven samples for each of twelve cells, and final summary. Fifteen focused
tests passed under normal discovery.
