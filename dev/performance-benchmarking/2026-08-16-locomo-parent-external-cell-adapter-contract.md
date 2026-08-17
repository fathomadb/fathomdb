# LOCOMO/PARENT external cell-adapter contract

**Tracks:** `LOCOMO-01`, `PARENT-01`
**Date:** 2026-08-16
**Status:** synthetic implementation preparation only; not a release or run

## Purpose

`experiments.locomo_external_adapter` is the external executable consumed by
the integrated `experiments.locomo_live_executor`. It is not another runner,
receipt writer, corpus acquirer, or index writer. The coordinator copies the
reviewed executable and its deployment declaration to an access-controlled
external location, hashes that executable, and binds that hash in a later
`locomo-live-executor.release.v1` record.

The executable reads exactly one strict
`locomo-live-executor.request.v1` JSON object from stdin and writes exactly one
strict `locomo-live-executor.cell-result.v1` JSON object to stdout. It rejects
duplicate JSON keys and unknown request, cell, runtime, provenance, and result
fields. Standard error carries fixed diagnostic text only.

## Inputs and execution boundary

The request is the sole runtime authority. It contains only the release ID,
frozen action/mode/cell, six absolute external input paths, and one absolute
per-cell output root. The adapter has no environment-based corpus, question,
provenance, credential, model, output, or device configuration. It accepts
only:

- the released action/mode pairing (`fixed_subset_dry_run`/`dry_run`, or a
  full-grid action/`full_grid`);
- one exact frozen Phase-B cell, including every treatment parameter,
  ingestion unit, CPU/GPU device, cache state, program track, and the exact
  approved `parent_child_turn_session_v1` mapping;
- an action partition containing that exact cell: the five listed dry-run
  cells, all 26 CPU cells, or all 26 GPU cells; and
- external corpus, turn/session manifests, fixed subset, TRACE sidecar, and
  parent-relation proof paths supplied by the released runner.

At actual runtime it reconstructs transient LOCOMO turn or session ingestion
payloads from the supplied external corpus, requires each payload to resolve
one canonical provenance entry, and uses the existing FathomDB Python `Engine`
surface:

- FTS cells call `search_text_only(..., limit=10)`.
- Hybrid cells call `search(..., limit=10)` and enable the frozen cross-encoder
  depth/pool/alpha only for the named CE treatments. A dense-disabled engine is
  an error, never a fallback.
- PARENT cells use the frozen hybrid child top-10, then require each child hit
  to resolve through the canonical parent relation and active TRACE source.

The relation proof is additionally bound to the byte hashes of the supplied
turn and session provenance files. Every relation fingerprint, exact parent
session, ordered member list, and child ordinal must agree with those manifests.
Member ordinals must equal their zero-based position in the canonical ordered
session list; a forged non-adjacent ordinal is rejected.
This is a second fail-closed check; it does not replace the full lifecycle and
relation validation already performed by the released live executor.

## GPU rule

The current runner request carries a GPU/CPU class but no CUDA ordinal. This
adapter therefore supports a GPU cell only when the external runtime exposes
exactly one visible physical GPU as `cuda:0` (`CUDA_VISIBLE_DEVICES` absent or
`0`, and `nvidia-smi` reports only device zero). Otherwise it fails before
opening FathomDB and emits no CPU result. Its GPU result attests exactly
`{"device":"cuda:0","cuda_available":true}`.

Consequently, a coordinator GPU release must select `cuda:0` until the
release-request ABI itself is extended and independently reviewed to carry the
selected CUDA ordinal. A working NVIDIA driver and a GPU-capable, pinned
FathomDB runtime remain factual preflight requirements; this task did not
probe either.

## Safe output

The per-cell output root must be empty and external. The adapter creates only
its external FathomDB working database and one
`locomo-external-adapter-metrics.v1.json` aggregate. The metric file and stdout
result contain only the frozen cell ID/mode, a logical metrics reference,
SHA-256, finite scalar M1/M2/M4-proxy/M6/M7/class summaries, and—only for a
PARENT result—the safe parent proof required by the live executor.

No path, corpus text, question, evidence text, retrieved passage, prediction,
model output, credential, receipt, or index row is emitted. The external
database is raw runtime state and remains in the access-controlled output root;
it is never copied into the repository or a campaign receipt.

The adapter independently rejects an output root in the repository, including
the historical `experiments/runs/` tree, before loading a corpus or creating a
directory. This repeats, rather than relies upon, the live executor's external
root boundary.

For the fixed dry subset, the adapter requires exactly 32 unique external
question IDs. A full-grid cell requires exactly 1,536 evidence-backed
questions. It rejects incomplete class coverage rather than inventing a class
metric. It cannot make an action complete: only the live executor's exact
action closure and the coordinator's ordinary receipt/index operation may do
that.

Rank is retained end-to-end. A retrieved logical hit is expanded in its original
FathomDB rank order; each turn keeps its first position after expansion and
deduplication. Unknown hit provenance is an error, not an omitted rank. M1/M2
therefore score R@1, MRR, and nDCG against the actual child ranking rather than
a set.

For PARENT cells, `parent_hits` is the content-free proof of the first selected
query's actual hybrid child top-10 ordering. It is not an aggregate or a second
retrieval result. Each listed child retains its original rank and yields the
frozen deduplicated, at-most-five session bundles. The parent metrics aggregate
those real per-question bundles: `duplicate_rate` is the share of child hits
removed by session deduplication, and `context_expansion_count` is the mean
number of bounded neighbor turns added per query. Neither field may be a
constant or a completion claim.

## Deployment prerequisites

Before the first authorized invocation, the coordinator must qualify all of
the following outside the repository:

1. a copied executable matching the reviewed source digest, executable bit, and
   `external-adapter.v1.json` declaration;
2. the external normalized LOCOMO corpus, both byte-matching provenance
   manifests, 32-ID fixed subset, accepted TRACE sidecar, and canonical
   parent-relation proof;
3. an empty access-controlled per-cell output root; and
4. a pinned FathomDB Python distribution with its required local embedder and
   CE assets already available. GPU cells additionally need the healthy,
   exactly-visible CUDA device above.

This implementation used only synthetic files and injected fake FathomDB
engines. It did not acquire or read LOCOMO, invoke FathomDB, load a model,
select a GPU, write an external artifact, produce a receipt, or append an
experiment index row.
