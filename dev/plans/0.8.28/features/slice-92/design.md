---
title: Slice 92 — documentation convergence design
status: APPROVED
target_release: 0.8.28
---

# Documentation convergence design

## Reconciled execution baseline

Execution starts at `77b6f0b4d`, after the planning baseline `896ff59e9`.
The intervening commit only placed this slice and reconciled the 90 → 92 → 95
ladder, requirements and handoff. No product or documentation contract changed.
The seven input categories in the [plan](plan.md) remain applicable; no new
feature or deferred item is allocated. The audit [inventory](inventory.md)
records code-grounded corrections and all maintained-owner dispositions.

## Current owners and edits

Update current user guides and references, package READMEs, architecture and
maintained designs in place. Replace obsolete normative passages rather than
stacking contradictory amendments. Historical releases, experiments and frozen
slice evidence retain their original context. Existing lifecycle and semantic
ownership catalogs remain the only catalogs. An index is navigation, not a
replacement contract or a proof that prose is accurate.

Review the .28 delta against accepted decisions, interfaces and source/tests:
CUDA/fork dependencies; SQLite and fresh format 36; pending-work scans and
built-in-provider failed-open verification; vec0 metadata/head spans; retrieval
ranking, stop words, context pack, property/visibility filters and direct body
FTS; explicit maintenance and operator recovery limits. MCP exposure stays .29.
Preserve existing documented performance limits and owner deferrals.

Code documentation changes describe the existing API behavior only. No API,
runtime, storage or publication change is part of this slice. Real authority/code
conflicts are blockers; prose must not silently select a new policy.

## Public-doc truth correction

The existing checker distinguishes a selected pending documentation target from
the newest historical published receipt. Retain that selection and identity
validation. Correct only the fixed schema-34 assumptions:

- Published receipts use reviewed release-format facts: .26/.27 use 34; .28
  uses 36 under the accepted .28 storage/maintenance decisions. A later unknown
  published release fails closed until its format fact is deliberately reviewed;
  it cannot borrow the current source schema merely by matching its number.
- A pending documentation target must still come from the tracked canonical
  active release and its own-release marker. Its schema must match the single
  literal `SCHEMA_VERSION` declaration in the schema crate. Missing, ambiguous
  or invalid source declarations fail with an actionable diagnostic.
- Compatibility guidance must state that selected schema and preserve the
  no-public-automatic-migration boundary. Historical receipts, malformed-date/tag,
  false-publication, platform, workspace and exact-install checks remain intact.
- The test fixtures explicitly model historical source 34 and pending source 36.
  Normalize copied current landing text to the fixture's intended version/schema;
  do not infer expected behavior from checker output. Keep all existing negative
  controls, including independent wrong published-schema rejection.

RED adds the pending .28 format-36 positive witness and decisive source/state/
compatibility mismatch, publication and missing-source controls to the existing
suite. GREEN updates the existing checker and current docs. Tests remain fixed
while correcting implementation. No new generator or checker framework is needed.

Select `public_docs_target_release=0.8.28` only alongside consistent .28 guidance.
This marker is not a publication receipt. Version manifests, registry/Pages
availability and retained Tegra artifact digests remain unchanged.

## Verification and handoff

Run existing focused checker/generator/authority/lifecycle tests and lint before
broader verification. Build MkDocs without deploying or requiring retained binary
artifacts for a prose-site preview. Verify representative real-database Python/TS
and external Rust SDK/CLI examples, recording source vs installed provenance and
reusing prior evidence only for unchanged relevant inputs. Astra reviews the
actual content/tooling diff; a separate Astra verifier checks S92-1–8 and selected
claims/examples. One final clean-tree `agent-verify` covers executable/source
changes; repair failures with focused reruns. Slice 95 retains final feature,
security, performance, platform and non-publishing release qualification.
