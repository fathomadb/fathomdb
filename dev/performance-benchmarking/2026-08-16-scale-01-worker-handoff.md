# SCALE-01 worker handoff

**Track:** `SCALE-01`

**Charter:** [SCALE-01 TC-5 fidelity](tracks/scale-01-tc5-fidelity.md)

**Base:** `3256d207`

**Outcome:** ready for independent read-only review of TC-5 preparation only.

## Scope and ownership

- Owned paths: this handoff, the dated
  [TC-5 manifest contract](2026-08-16-scale-01-tc5-manifest-contract.md),
  `experiments/tc5_manifest.py`, and
  `tests/experiments/test_tc5_manifest.py`.
- Produced contract: `tc5-manifest.v1` and `tc5-planning-receipt.v1`.
- Preserved unchanged: `eu7_real_corpus_ac`, its shared support, and
  `dev/plans/runs/eu7-latest-measurements.json`; no common experiment helper,
  configuration convention, PROGRAM, Track Runner control, status board,
  receipt/index, LOCOMO path, or SCALE-02 path changed.
- The preparation module validates an externally supplied, content-free
  manifest. It never reads corpus payloads, calls an embedder or harness,
  acquires a corpus, executes a smoke/measurement, uses GPU/model/paid or
  external services, writes an experiment receipt/index row, pushes, or starts
  SCALE-02.

## TDD and verification evidence

1. Red checkpoint `3b2417ba` (`test(perf): specify tc5 manifest preparation`):
   `python3 -m pytest tests/experiments/test_tc5_manifest.py -q` stopped at
   collection with `ModuleNotFoundError: No module named
   'experiments.tc5_manifest'`.
2. Green implementation `14792f8a` (`feat(perf): prepare tc5 manifest
   runner`): the focused command passed `10 passed in 0.48s`; the complete
   `tests/experiments` suite passed `80 passed in 0.93s`.
3. `python3 -m ruff check experiments/tc5_manifest.py
   tests/experiments/test_tc5_manifest.py`, `./scripts/agent-lint-md.sh`, and
   `git diff --check` passed.
4. Required full `./scripts/agent-verify.sh` passed unconfined: `agent-test.sh:
   72/73 suites passed (skipped=1 excluded=0)`. AC-037 reported its expected
   local environmental downgrade because rootless netns is unavailable; its
   offline catch passed. The first full-gate attempt failed before tests only
   because this fresh worktree lacked `fathomdb._fathomdb`. It was corrected by
   using the repository's local-only `scripts/earp-build-binding.sh`; the
   retry imported the extension from this worktree and passed.

## Review focus

1. Confirm `tc5-manifest.v1` fails closed for every stated invalid case:
   primary/bridge count mismatch, duplicate or missing IDs, malformed rows,
   synthetic origins, incomplete provenance, noncanonical order, and a changed
   frozen CPU/model/seeds/SUT setting.
2. Confirm the bridge is exactly the canonical 7,667-ID subset of the
   18,472-document all-real primary selection, and the property-style test
   covers unique identifier preservation.
3. Confirm `tc5-planning-receipt.v1` carries no document IDs, corpus text, or
   raw paths; that its execution state cannot imply a smoke or measurement;
   and that repository output, an output outside the declared external root,
   and the historical EU7 result path all fail closed.
4. Confirm the new pure module has no invocation seam into the historical EU7
   harness and did not alter its output contract.

## Authorization boundary and next state

This is a **ready** preparation packet, not a live characterization result.
Independent acceptance may integrate the manifest-only runner. Corpus
acquisition, an external manifest, a smoke invocation, the two all-real CPU
arms, a normal execution receipt/index row, any model/GPU/paid work, and
SCALE-02 all remain separately unauthorized.
