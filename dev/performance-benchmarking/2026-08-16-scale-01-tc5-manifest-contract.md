# SCALE-01 TC-5 manifest preparation contract

**Track:** `SCALE-01`

**Date:** 2026-08-16

**Scope:** metadata validation and safe planning receipt only. This contract
does not acquire a corpus, read corpus payloads, invoke an embedder or EU7
harness, perform a smoke or measurement, or authorize SCALE-02.

## Purpose

`tc5-manifest.v1` is the all-real input contract for the later TC-5 CPU
characterization. It supplies only stable document identities and hashes, so
the preparation code can prove selection/provenance completeness without
committing corpus text, raw paths, model output, or a result.

The separately named `experiments.tc5_manifest` module never calls the
historical `eu7_real_corpus_ac` test. That test and its
`dev/plans/runs/eu7-latest-measurements.json` output remain historical and
unchanged.

## `tc5-manifest.v1` input

The manifest contains exactly these fields:

| Field | Requirement |
| --- | --- |
| `schema_version` | Exactly `tc5-manifest.v1`. |
| `program_track` | Exactly `SCALE-01`. |
| `manifest_id` | Stable safe identifier. |
| `source_artifact_sha256` | SHA-256 pin for the external source artifact. |
| `documents` | Exactly 18,472 canonical-order rows; each has `document_id`, `content_sha256`, and `origin: real`. |
| `bridge_document_ids` | Exactly the first 7,667 canonical primary IDs; no per-source truncation or unrelated historical subset. |
| `provenance` | Complete source/build/environment/model/SUT metadata listed below. |

Each document identity and content hash is validated, but its payload is never
opened. Duplicate, missing, malformed, out-of-order, synthetic, or unknown
rows fail closed. The primary arm is always 18,472 real documents and the
supporting bridge is always 7,667 selected IDs from the same manifest.

The required provenance includes source commit and Cargo-lock hashes, Rust,
CPU, and OS identity, model identity and asset hash, sorted engine features,
and the following frozen values:

| Field | Frozen value |
| --- | --- |
| Device | `cpu` |
| Model identity | `fathomdb-bge-small-en-v1.5` |
| Candidate breadth | `192` |
| Queries | `100` |
| Query-select seed | `0x0E77C0125E1EC7` |
| Bootstrap | `1000` resamples, seed `0x0E77B007574A9` |
| SUT / ground truth | Pre-fusion 1-bit `K=192` + f32 rerank vector stage / same-model exact-f32 top-10 |

## External-only receipt projection

`tc5-planning-receipt.v1` writes only after both declared corpus and output
roots already exist outside the repository. It has the manifest hash, the two
arm counts, frozen configuration, hashes/identities needed for provenance, and
hashed logical references to the declared roots. It contains no document IDs,
payload text, raw paths, predictions, metrics, or live-result claim.

Its execution fields are permanently preparation-only for this invocation:
`planned_not_executed`, `smoke_performed: false`,
`measurement_performed: false`, zero synthetic documents, and
`historical_eu7_output_used: false`.

The writer rejects an in-repository output, an output outside the declared
external root, and the historical EU7 result path. Writing the receipt neither
creates nor authorizes a corpus, a model load, a smoke, or a measurement.

## Later execution boundary

A later, separately authorized TC-5 execution must consume a qualified
manifest and emit the normal safe experiment receipt/index evidence after both
frozen arms complete. It must not treat this planning projection as a fidelity,
latency, release, or SCALE-02 result.
