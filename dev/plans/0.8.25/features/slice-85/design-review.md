---
title: Slice 85 independent design review
status: PASS
---

# Slice 85 design review

The independent read-only review checked the plan, design and execution matrix
against the original Slice 75 allocation/manifest, Slices 79–80, release state,
current runners/tests, public interfaces and relevant ADRs.

The review required and verified these corrections:

- seal exact commands or immutable manifest keys, timeouts, positive counts,
  feature sets, executor ownership and dispositions;
- rerun both protected 71B workloads because Slice 80 changed their inherited
  broad Engine-source invalidation set;
- explicitly map draft N25-04 and R25-75/AC25-75 to final evidence;
- cover zero-test/skip, relaxed-threshold and false-reuse RED cases;
- add merge-to-release-branch and temporary-worktree cleanup;
- rerun CE with a Slice 85 overlay whose only allowed change from the immutable
  Slice 72 manifest is `candidate_sha=FINAL_SHA`.

After those corrections, the reviewer returned **PASS**. The design is complete,
falsifiable and bounded: one completeness validator and one installed-runtime
smoke extension, with no scheduler, product feature, oracle change, version cut
or publishing action.

## Follow-up findings

A subsequent review correctly reopened the package for five bounded fixes:

- stage and explicitly authorize a push of the exact GREEN commit at the remote
  release-branch head before hosted dispatch, while deferring closure cleanup;
- bind CE to the locally accessible original `windchill3` RTX 3090/CPU-affinity
  envelope and runtime floors to installed uv CPython 3.10/3.11 plus system 3.12;
- compile TypeScript tests with `tsconfig.json` before the focused Node selector;
- let positive broad-round counts satisfy equivalent runtime source rows; and
- distinguish conservative write-evidence invalidation from an observed
  release-path risk because the intervening source edits are debug-only.

These changes are applied above; the independent reviewer must refresh the
verdict before execution proceeds.

The refresh found one remaining executor mismatch: legacy runtime-floor
commands named aliases absent from `PATH`. The matrix now seals absolute paths
for uv CPython 3.10.20/3.11.15, system Python 3.12 and current Node 25.9.0. The
reviewer verified those bindings and returned **PASS**.

The release owner subsequently narrowed the public Node contract to Node 25
only, matching the runtime used throughout earlier 0.8.25 development and
testing. Other Node release lines are explicitly deferred. The package engine
range, public docs, local/CI/release pins, N-API build contract and runtime-floor
matrix are aligned. The independent review then found two registered broad-gate
tests that still asserted the superseded target. Both were corrected under
RED/GREEN, their focused guards passed, and the reviewer returned **PASS** with
no remaining material design blockers.

The final refresh reviewed the completed plan, design and execution matrix at
`f4d57d8e`. It confirmed that the exact-head CI sequencing, registered
GPU/executor comparability, absolute runtime-floor bindings, TypeScript test
compilation, broad-round deduplication, protected-write rationale and Node
25-only scope are closed. It treats the
[CUDA Engine p95 exception](ce-engine-p95-exception.md) only as the accepted
Slice 72 CE disposition reference. The independent verdict remains **PASS**.

The execution-ready refresh at `137cf86e` also verified that the candidate TC-5
runtime is recreated, force-installs the pinned wheel, binds both the lexical
interpreter path and `sys.prefix` to that venv, and rejects the base prefix.
Focused coverage passed 39 tests and 36 subtests. No design findings remain.
