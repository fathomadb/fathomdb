# LOCOMO/PARENT factual input qualification contract

**Tracks:** `LOCOMO-01`, `PARENT-01`  
**Date:** 2026-08-16  
**Status:** content-free preflight only; never an execution release

## Purpose

`experiments.locomo_input_qualification` validates the external LOCOMO inputs
needed by the integrated live executor. It only reads the declared corpus and
control documents long enough to compute SHA-256 values and validate safe
structure. It does not invoke FathomDB, the cell adapter, a model, CUDA, or a
benchmark action.

The qualifier writes only to an empty, declared external artifact root outside
the repository. Its outputs are a self-hashed
`locomo-input-qualification-report.v1` and, where their prerequisites are
unambiguous, a content-free `trace-projection.v1` and
`locomo-parent-relation-proof.v1`.

## Inputs and eligibility

The Phase-B configuration remains the pin authority for the corpus, turn
provenance, session provenance, and 32-question fixed subset. Every present
input records expected and actual SHA-256 values; a mismatch is a durable
blocker, never a replacement pin.

The qualifier also reads the integrated CORPUS-01 matrix. For LOCOMO it records
the `CC-BY-NC-4.0`, external-evaluation-only posture and the matrix's permitted
retrieval/temporal/multi-session claims. It does not make a knowledge-update,
supersession, source-erasure, commercial-distribution, human-gold, or product
claim.

## Derived content-free proof

When both canonical provenance files match their frozen hashes, the qualifier:

1. creates one TRACE source and text projection per canonical session using
   only its safe fingerprint-derived source ID and SHA-256;
2. validates the resulting complete TRACE sidecar with the integrated
   TRACE-01 validator; and
3. attempts one PARENT relation entry per turn, binding child, enclosing
   session, zero-based ordinal, session members, canonical fingerprints, and
   active TRACE source identity.

The parent relation is emitted only when every child identifier is globally
unambiguous and every child belongs to exactly one canonical enclosing session.
It intentionally preserves the integrated executor ABI: it does not silently
namespace or rewrite child IDs to repair an ambiguous manifest.

## Fail-closed report

The report has no corpus text, questions, answers, evidence, predictions,
credentials, external paths, or runtime output. Its `report_sha256` is the
canonical JSON SHA-256 excluding only that self-hash field. It records only
safe status flags, expected/actual hashes, fixed blocker codes, license/claim
eligibility, output hashes/counts, and the explicit no-live-action evidence.

Missing inputs, pin drift, corpus/subset count or ID errors, invalid manifests,
and unconstructable parent membership produce `qualification_status: blocked`.
A blocked report is useful coordinator evidence, but it cannot be used as a
release record or to invoke the fixed-subset dry run.

## 2026-08-16 factual result

The existing turn and session provenance files match the frozen Phase-B pins.
The available LOCOMO payload has a different corpus SHA-256, the frozen
fixed-subset document is absent, and the pinned turn manifest has ambiguous
child identifiers for the existing parent-relation ABI. The generated report
therefore remains blocked; it emits the valid TRACE sidecar but no parent proof.
No live action was performed.
