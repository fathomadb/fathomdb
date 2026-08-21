# Experiment executor template

Use this template to author a new external experiment executor after its
track is authorized and its plan-only contract is integrated. It is a few-shot
authoring guide, not an execution authority. Nothing in this document grants a
release, selects a corpus, supplies a credential, or permits a model, GPU,
external write, or index append.

## Boundary to preserve

Start from a safe preparation contract and add a separate, track-specific live
executor. The preparation module must remain safe on its own. The live executor
must refuse to run unless it receives one self-hashed coordinator release
record that binds all of the following:

- the reviewed, integrated commit SHA;
- frozen execution and preparation configuration digests;
- qualified external manifest and provenance digests;
- declared external corpus and output roots;
- one named authorized action and its exact cell set; and
- the executor source digest plus an accepted, bound review-evidence record.

Validate the release before loading an adapter, creating an output directory,
loading a corpus, selecting a device, or writing any artifact. A release is
single-purpose: a dry subset, CPU grid, GPU grid, smoke, and long
characterization are distinct actions unless the track contract explicitly
freezes them together.

## Few-shot shape

```text
tests/<track>_live_executor.py          # red first: release and safety failures
experiments/<track>_live_executor.py    # separate external-only runner
experiments/configs/<track>/live.v1.json
dev/performance-benchmarking/<date>-<track>-live-executor-contract.md
dev/performance-benchmarking/<date>-<track>-live-executor-worker-handoff.md
```

The runner should expose four distinct operations:

1. `validate`: parse only local configuration and release-independent shapes.
2. `preview`: emit only cell IDs, action IDs, and fixed public-safe counts.
3. `execute`: require the complete release record and produce one
   content-free, external-root-bound arm or action result.
4. `finalize`: combine only the contract's exact complete result set and make
   a receipt/index projection eligible. It must reject partial, duplicate,
   cross-release, cross-config, or cross-provenance collation.

Do not make `preview` perform a weak version of `execute`. Do not let
`finalize` manufacture a completion verdict from anonymous summaries.

## Fail-closed implementation checklist

- Write human-intended tests first and commit the failing checkpoint before
  implementation. For a review remediation, add a new red checkpoint that
  reaches the defect being fixed; do not mask it behind an earlier validator.
- Parse JSON with duplicate-key rejection. Require exact top-level and nested
  key sets, safe identifier grammar, standard JSON output, and finite numeric
  metrics.
- Resolve every input and every generated destination before `mkdir`, adapter
  invocation, or write. Prove it remains under the resolved declared external
  root and outside the repository; test symlink escapes for arm directories and
  receipt paths.
- Treat corpus text, raw document IDs, raw paths, predictions, credentials,
  and model output as forbidden from repository artifacts, receipts, handoffs,
  and coordinator status. Store only IDs when the specific safe contract
  permits them; otherwise use hashes, counts, and fixed diagnostics.
- Bind every result to action ID, cell ID, mode, configuration digest, manifest
  digest, release digest, executor digest, and complete provenance. A complete
  receipt requires the exact expected unique result set and all required metric
  and class summaries.
- Validate device reality rather than a declarative field. GPU actions require
  selected-device availability and adapter attestation of the same device;
  reject fallback or ambiguity.
- Validate upstream lifecycle and provenance contracts in full. For a derived
  parent/child treatment, prove source lifecycle, canonical membership, parent
  relation, ordinal adjacency, and bounded context from the hash-pinned
  manifests rather than accepting a separately invented inventory.
- Keep historical outputs immutable and explicitly reject their paths. Never
  append the experiment index during executor preparation; only the
  coordinator-approved complete receipt may become index-eligible.

## Review and handoff

The worker hands off a clean commit with owned paths, red/green commands,
full-verifier evidence, exact no-live statement, external prerequisites, and
review focus. An independent reviewer verifies the diff and reenacts focused
safe checks before integration. The coordinator then integrates, verifies from
Git, updates Track Runner status, and—only when factual external prerequisites
are qualified—issues a separate release record.

Do not copy a prior executor's release record, external root, SHA, corpus pin,
device selection, or result into a new track. Reuse the safety pattern; freeze
the new track's own facts and authority.
