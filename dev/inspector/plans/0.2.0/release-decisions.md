---
title: Inspector 0.2.0 release decisions
status: ACTIVE
desc: Owner decisions governing Inspector-local release readiness and distribution
---

# Inspector 0.2.0 release decisions

## INS-020-LOCAL-RELEASE — ruled

Authority: [Inspector decision ledger](decisions.jsonl), **seq-1**, explicitly
ruled by the owner. This Inspector-local record is the decision home; Engine
release state and publication receipts remain separate.

- The five-person UI/UX study will not be performed for Inspector0.2.0. Remove it
  as a release requirement/blocker. No session has occurred and none is passed.
- Slice80 updates project/user/developer documentation, establishes and actually
  tests CI (local execution is acceptable), and establishes packaging/distribution
  methods with a documented path toward future registries.
- Slice90 performs the Inspector0.2.0 release locally to a gitignored `dist/`
  directory. It follows completed80; it deploys to no registry.
- Both slices are authorized for sequential execution using the existing prompt
  template. Routine in-scope work needs no further ratification.

This ruling supersedes the human-study gate in the current requirements,
acceptance, matrix, scope, study and release-status documents. Slice80 reconciles
those current views while retaining historical results. Functional T01–05 and
High INS07/08 goals, actual expert/browser evaluation and all technical safeguards
remain required; this decision does not invent participant or statistical evidence.

No unruled Inspector release decision is introduced by this change. Publication
tags, remote registry uploads, Engine publication, other platforms and paid
infrastructure are outside this local release mandate.
