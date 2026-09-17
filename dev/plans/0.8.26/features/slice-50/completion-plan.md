---
title: FathomDB 0.8.26 Slice 50 — completion execution plan
status: ACTIVE
---

# Slice 50 completion execution plan

## Reviewed unfinished work

The implementation and its review remediations are committed through
`0ab1b045`, but the release state still correctly marks Slice 50 `DRAFT`.
Completion requires the following evidence and records:

1. remove only the two proven disposable TypeScript fixtures created by
   `slice55-request-validation.test.ts` and establish a clean candidate tree;
2. run the unchanged full canonical gate at the exact candidate SHA;
3. build fresh local wheel, npm/native, and CLI artifacts from that SHA and run
   the installed wheel profile, Linux native-package smoke, CLI version and
   immutable integrity witness;
4. scan generated evidence directly and the tracked tree separately with the
   pinned Gitleaks configuration;
5. push the existing release branch, dispatch the exact-SHA CI workflow, and
   collect the five-target native matrix plus the distinct Windows WAL receipt;
6. assemble and validate the durable candidate manifest from local artifacts,
   scans, and CI receipts;
7. obtain independent read-only verification of the evidence packet and
   acceptance criteria;
8. write the Slice 50 status record, update the single-writer release state,
   regenerate its declared views, verify them, and commit the closeout; and
9. confirm the existing release worktree is clean. No tag, registry write,
   npm dist-tag change, GitHub release, merge, or publication is authorized.

## Execution order and stop gates

1. **Candidate preparation:** clean generated fixtures, commit this reviewed
   execution record, and freeze that commit as the implementation candidate.
2. **Local verification:** run `agent-verify`, then fresh package and security
   witnesses using Rust 1.95.0, Python 3.12, and Node 25.9.0. Stop on any
   source/package/version mismatch, test failure, secret finding, or mutation
   of the schema-33 refusal fixture.
3. **Platform verification:** push only `release/0.8.26`; dispatch `ci.yml`
   with `candidate_sha` equal to the remote branch tip. Stop on candidate
   drift, any target failure, a missing/duplicate receipt, or an unavailable
   Windows WAL receipt. An unavailable executor is `UNRESOLVED`, not a pass.
4. **Evidence binding:** assemble the schema-validated manifest only while the
   checkout remains at the candidate commit. Every artifact and receipt must
   carry an exact digest and candidate binding.
5. **Independent verification and closeout:** verify the packet read-only,
   resolve any finding with focused RED/GREEN work, then update status and
   generated release-state views. Documentation-only closeout happens after
   the candidate and is recorded separately from it.

The prior GPT-6 Astra medium review and remediation rereview are accepted as
the code-review gate. This plan adds no product scope and preserves the
approved Slice 50 design.
