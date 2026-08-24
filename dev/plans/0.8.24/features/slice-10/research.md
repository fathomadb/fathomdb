---
title: 0.8.24 Slice 10 — primary-source research disposition
status: COMPLETE
target_release: 0.8.24
---

# Slice 10 — primary-source research disposition

## Result

No web research was needed. Code-grounded inspection found no unresolved
GitHub Actions behavior and proposed no new event, reusable workflow,
permission, environment, runner, artifact, or attestation edge. Researching
generic CI practice would not change the no-code decision.

## Questions reserved for an owning target slice

If a later ready design proposes one of the following concrete changes, that
slice must use current GitHub primary documentation before implementation:

| Proposed change | Primary-source question |
| --- | --- |
| New self-hosted target route | How the chosen event obtains the default-branch workflow, how public-repository self-hosted execution is constrained, and which environment/runner-group controls apply. |
| Cross-job or cross-workflow artifact transfer | Artifact identity, retention, permissions, digest/attestation verification, and same-run/cross-run trust boundary. |
| Reusable workflow | Caller/callee permission narrowing, secret inheritance, ref pinning, and artifact ownership. |
| OIDC publisher | Exact subject claim, repository/workflow/environment binding, and job-level `id-token` scope for the selected registry. |

These are research triggers, not Slice 10 prerequisites. Existing workflow
behavior was evaluated from the repository's executable contract and source;
no external fact is claimed by this record.
