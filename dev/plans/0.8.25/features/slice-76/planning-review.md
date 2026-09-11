---
title: Slice 76 — planning and design review
status: RESOLVED
---

# Slice 76 planning and design review

An independent read-only reviewer required corrections before execution.

| Finding | Resolution |
| --- | --- |
| Missing design and sealed manifest | `design.md` added; the execution manifest is sealed before prototype work. |
| Baseline-green path contradicted R76-3/R76-4 | Valid 7/7 baseline now explicitly discharges them as `NOT_APPLICABLE_BASELINE_GREEN`. |
| Missing history reconciliation | Plan records `5056db9e`, planning-only `b8a1a228`, prior E.1 failure, and the current-path reasons for one retest. |
| Ambiguous capacity | Capacity now uses per-reader cyclic reuse distance, includes existing cached statements, acknowledges rusqlite's default 16 and forbids a sweep. |
| Statistics-enabled was assumed | Design requires a positive controlled-allocation memory-status witness bound to each verdict binary. |
| TDD scope was too broad | New REDs cover observability and statement mechanics; existing fixed tests own dependency, lifecycle, frozen-read and ranking semantics. |

The reviewer otherwise accepted the unchanged AC-020 oracle, focused run caps,
non-shipping prototype boundary and bounded same-file census. Implementation may
start only after the manifest fixes commands, counts, identities and run order.

Final re-review: **PASS**. The sealed manifest scrubs the forbidden environment,
separates and hash-binds B/C verdict and diagnostic binaries, supplies four exact
diagnostic slots, and makes the baseline-green early exit executable. No design
or execution blocker remains.

## Measurement-contract correction

A subsequent independent review at `f26d59c4` found four measurement-contract
issues before any benchmark ran. They are resolved as follows:

- the concurrent profile now performs the registered gate's preceding 1,600
  sequential searches and both profile arms use a ready/go boundary so setup and
  teardown are outside the sampled interval;
- the profile is explicitly on-CPU only; off-CPU waiting remains unresolved and
  cannot justify a dispatch or lock-wait treatment;
- the relevant-tree digest now covers workspace build inputs and every local
  engine dependency, while shared scaffolding is separated from the B-to-C diff;
- the shared protocol now states that Slice 76's four-slot manifest controls,
  distinguishes mandatory from optional signals and names the recommendations
  unavailable signals cannot support.

These are evidence-quality corrections only. The experiment, oracle, run budget
and focused verification scope are unchanged.
