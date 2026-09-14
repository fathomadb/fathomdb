---
title: FathomDB 0.8.26 Slice 10 — frozen explanation repair status
status: COMPLETE
completed_on: 2026-09-13
---

# Slice 10 status — frozen explanation repair

## Outcome

Slice 10 is complete at reviewed implementation commit `c2795caf`. Both frozen
explanation paths now assign a valid non-empty correlation identity through the
existing observability finalizer. Explanation-disabled calls remain telemetry-
silent, and ranking, eligibility, frozen authority, evidence identity,
persistence, schema, and public method shapes are unchanged.

The maintained interfaces and public guide now describe the canonical frozen-
evidence workflow. The release wheel verifier executes that workflow from a
fresh non-editable installation and retains its previous ordinary-search and
frozen-expansion coverage.

## Delivered contract

- N26-01 and R26-10A through R26-10D are implemented.
- AC26-10A proves `q...` identities with telemetry enabled, unique `x...`
  fallback identities with telemetry disabled, and exactly one finalization.
- AC26-10B proves ordered-result and resolved-evidence equivalence without
  comparing randomized evidence-reference bytes; disabled calls emit no
  telemetry.
- AC26-10C is covered by maintained Rust, Python, TypeScript, and wire
  interfaces plus the public frozen-evidence guide and its executable wheel
  profile.
- AC26-10D proves a fresh wheel, one direct dependency, context/reference reuse
  after unchanged-state restart, import provenance, clean close, and exit.

The locked global `dev/acceptance.md` was not changed. Slice 15 retains graph-
evidence performance/erasure decision support; Slice 20 retains graph-target
point evidence; Slice 40 retains derived-edge actuation; Slice 50 retains the
combined P0–P2 installed-artifact profile.

## TDD and commit chronology

| Phase | Commit | Evidence |
| --- | --- | --- |
| RED | `2ebaabec` | New Engine and binding regressions reproduced an empty explanation correlation identity (`unexpected fallback identity: ""`). |
| GREEN | `4c90d107` | Both successful frozen paths conditionally invoke the existing finalizer only when returning explanation. |
| Guidance/witness | `361c29e7` | Maintained contracts, public guide, external profile, and verifier fixture landed. |
| Guide correction | `dd007b8b` | The public Python example became self-contained. |
| Review FIX-1 | `c2795caf` | Restored prior ordinary-search/frozen-expansion wheel assertions and corrected the guide-index version claim. |

## Verification

Green independent evidence at `c2795caf`:

- focused Rust Engine regression: 1 passed;
- focused Python public-call regressions against the clean wheel: 2 passed;
- focused TypeScript native regressions: 2 passed;
- current installed-wheel profile: ordinary search, frozen expansion, both
  explained paths, disabled controls, exact resolution, direct dependency,
  retained context/reference after reopen, close, and exit passed;
- wheel-verifier fixture: all six cases passed;
- wheel module and native imports resolved inside the fresh virtual environment
  and the editable flag was `false`;
- public-document lint and strict MkDocs build passed; and
- full-workspace clippy and check passed.

The broad `agent-verify` attempt is recorded accurately as non-green: 109 of
110 suites passed. `test-python` failed during collection with 45 occurrences
of `ModuleNotFoundError: No module named 'eval'`; no Python test body ran. The
attempt combined a source-path override, a fresh release wheel, and a virtual
environment not owned by the implementation worktree. The exact current eval
module subsequently collected 37 tests, the corrected focused matrix passed
29 tests, the clean-wheel product tests passed, and both independent reviewers
classified the failure as environment/configuration-only. No editable install
or `maturin develop` workaround was used, and this result is not represented as
a green full gate.

## Reviews

The design received independent PASS after one clarification cycle. Initial
code review found two P2 regressions: loss of two pre-existing package-smoke
assertions and a stale 0.8.25 guide-index claim. FIX-1 corrected only those
items; independent re-review passed with no remaining finding. Independent
verification returned `PASS_WITH_ENVIRONMENT_LIMITATION`, with every focused,
package, documentation, and workspace product gate green and the broad Python
collection limitation classified above.

## Performance and next work

The explanation-disabled path adds only an explanation-presence check. The
enabled path reuses an existing finalizer after successful assembly. No read,
write, concurrent, erase, or persisted-state algorithm changed, so this narrow
defect repair did not add a performance spike or run the unrelated long
platform/performance matrix.

D26-01 remains the sole open release decision and is intentionally waiting on
Slice 15. Slice 15 is next and unblocked. Publication remains unauthorized.
