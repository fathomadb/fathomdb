---
title: FathomDB 0.8.26 Slice 65 — semantic ownership gate design
status: REVIEW_CANDIDATE
target_release: 0.8.26
---

# Slice 65 design — enforceable authority topology

## Existing structure and minimal extension

`document-lifecycle.json` remains the structural source for all 186 tracked
design documents: lifecycle class, topic/role uniqueness, owner, release
relevance, and supersession. Slice 65 does not overload that full inventory or
rewrite 161 non-maintained records. It adds
`dev/design/current-owner-authority.json`, an exact-set companion containing
one record for each and only each maintained lifecycle entry.

The companion schema is deliberately small:

```json
{
  "schema_version": 1,
  "profiles": [
    {
      "path": "dev/design/errors.md",
      "profile": "current",
      "semantic_authority": ["dev/adr/ADR-0.6.0-error-taxonomy.md"],
      "implementation_witness": [
        "src/rust/crates/fathomdb-engine/tests/error_taxonomy.rs"
      ],
      "evidence_only": [
        "dev/plans/0.8.25/features/slice-15/design.md"
      ]
    }
  ]
}
```

Records and path arrays are sorted and duplicate-free. `profile` has the sole
V1 value `current`; exact-set matching means a non-maintained document cannot
claim it and a maintained owner cannot omit it. The companion stores paths,
not prose excerpts, digests, line numbers, or inferred keywords.

## Boundary of automation

The lifecycle checker can prove that every maintained owner declares a current
profile, points to existing authoritative inputs and implementation witnesses,
and does not route current semantics through a historical document. It cannot
prove that arbitrary prose accurately describes code. Semantic truth remains a
code-grounded review obligation, recorded through the authority-and-witness
metadata and Slice 60 matrix.

## Relationship classes

Maintained records distinguish:

- **semantic authority:** an accepted ADR/interface or maintained design that
  governs the current statement;
- **implementation witness:** code, test, or bounded enforcement path that
  demonstrates the current implementation seam; and
- **evidence only:** historical plans, experiments, release-local designs, or
  status records that explain provenance but do not own current semantics.

Semantic-authority edges must exist, be unambiguous, and terminate without
cycles at accepted external authority or a maintained owner. Evidence-only
edges may target historical records but cannot satisfy the authority or witness
requirement.

For V1, an accepted external authority is one of:

- an existing ADR under `dev/adr/` whose YAML front-matter `status` starts with
  `accepted` or `locked` after case normalization;
- an existing public contract under `dev/interfaces/` whose YAML front-matter
  `status` is exactly `locked` after case normalization;
- `dev/requirements.md`; or
- the repository invariant file `AGENTS.md`, only when the lifecycle record's
  role is `index` or `method`.

The checker parses the bounded opening YAML front matter itself and fails
closed on absent, duplicate, malformed, multiline, or non-string `status`.
Proposed/draft/decision-ready/superseded ADRs and draft interfaces are not
authorities even though the files exist. `AGENTS.md` cannot authorize an
ordinary subsystem design, policy, or gate merely by being named.

A semantic-authority path under `dev/design/` must be another maintained entry
with its own companion profile. The checker traverses those edges and rejects
cycles. Every path must terminate at one of the external authority classes
above. Historical, proposal, deferred, experiment, reference, or superseded
design records cannot be semantic authorities.

Implementation witnesses are existing regular files in `src/`, `scripts/`,
`.github/workflows/`, or focused test locations under those trees. They prove a
code, test, or bounded enforcement seam exists; they do not prove prose truth.
Evidence-only paths may be any existing repository file, including historical
or release-local records, but cannot duplicate an authority or witness path.
At least one authority and one implementation witness are required per current
profile; evidence may be empty.

The existing lifecycle checker remains the single lifecycle gate. Its fixture
suite constructs tiny repositories and proves exact-set matching, accepted and
locked front-matter termination, rejection of proposed/superseded/malformed
ADRs and draft/malformed interfaces, role-bounded `AGENTS.md`, invalid-class
rejection, cycle detection, witness-root checking, disjoint relationship
classes, sorting, and existing local/docs-only wiring. No second lifecycle
checker or generated prose index is introduced.

## Error-owner reconciliation

Stable error behavior is summarized in `errors.md` or the maintained subsystem
owner and linked to accepted/public contracts. Release-local slice designs stay
available as provenance. This removes the condition where a maintained current
owner requires a reader to treat a historical implementation plan as normative.

The affected taxonomy rows are `EmbedderRequired`, `ProvenanceError`,
`DependencyError`, `DependencyClosureError`, `PageError`,
`DependencyTraceErrorV1`, `RuntimeConfigurationError`, and
`VectorEquivalenceMismatchError`, plus any row found by the final exact scan to
use the same historical/release-local pattern. Their current semantics move to
`errors.md`, the appropriate maintained subsystem owner, or the locked Rust
interface. Historical design paths remain linked as evidence and are not
rewritten. This is an ownership/documentation correction only: variant names,
payloads, precedence, and binding mappings do not change.

## Candidate model

Slice 50 remains valid historical evidence for the candidate it measured, but
it is no longer the final integration candidate once Slices 55–65 modify the
tree. Slice 65 creates the replacement exact candidate record. Expensive native
evidence is reused only when a diff-based input analysis proves it unaffected;
fresh SDK surface smokes and repository/security gates bind the new commit.

The actual post-Slice-50 diff changes Rust engine/facade/CLI code, N-API and
TypeScript binding code, package-facing interfaces, and tests. Therefore the
five native targets and distinct Windows WAL installed-wheel witness are
affected and must be rerun. The Slice 50 assembly/receipt validators and graph
profile are reused unchanged. A thin `slice65-candidate-manifest.py` imports
that module and wraps one newly assembled
`fathomdb.slice50-candidate/v1` payload for the same candidate in:

```json
{
  "schema_version": "fathomdb.slice65-candidate/v1",
  "candidate_sha": "<40-hex>",
  "base_candidate": {"schema_version": "fathomdb.slice50-candidate/v1"},
  "qualification": {
    "design_lifecycle": "pass",
    "sdk_surface_parity": "pass",
    "slice60_owner_probes": "pass"
  }
}
```

Validation first calls the unchanged Slice 50 validator over `base_candidate`,
then requires equal candidate SHAs and the exact three-key qualification set
with only `pass` outcomes. Unknown/missing keys, an embedded non-Slice-50
schema, or candidate mismatch fails. The wrapper assembly delegates all
artifact/evidence/toolchain/command/external/profile arguments to Slice 50 and
adds only the three named outcomes. The existing Slice 50 script and committed
manifest remain byte-unchanged historical evidence.

The manifest RED proves both directions: the original Slice 50 manifest still
passes its original validator, while malformed Slice 65 wrappers fail for
missing, extra, cross-schema, non-pass, and candidate-drift cases. GREEN adds
only the wrapper/validator needed by those tests.

The implementation candidate is a clean commit containing all functional,
catalog, test, error-owner, and review-remediation changes. Artifacts and
receipts bind that SHA. A later documentation-only commit may add the manifest,
review records, status, and release-state closeout; release state records both
SHAs so the self-referential closeout is not mislabeled as the built candidate.

## Compatibility and change class

The product/runtime surface is unchanged. Executable repository changes are
limited to the existing lifecycle checker and fixtures plus the thin candidate-
manifest wrapper and its tests. The catalog is internal engineering metadata,
and the error edit redirects ownership without changing an error contract. No
schema, migration, dependency, package API, operation map, tag, publication, or
main integration is authorized.
