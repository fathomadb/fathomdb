---
title: 0.8.25 Slice 50 design review cycle 1
status: CHANGES_REQUIRED
reviewed_commit: 7fde4b20
---

# Slice 50 design review cycle 1

The independent review found no P0 issue and five P1 design defects. Design v5
was not implementation-ready.

| Finding | Required correction | Disposition in design v6 |
| --- | --- | --- |
| Graph-arm provenance was ambiguous | Capture the returned body artifact separately from the exact graph edge that introduced it; never infer an edge from `source_id` after search. | Added a private typed origin carrier, graph seed/traversal variants, and an explicit body-evidence association. |
| Lifecycle was node-only | Define artifact class at candidate creation and provide total node/edge lifecycle wire semantics. | Added closed artifact-class and node/edge lifecycle variants. |
| Source authorization was undefined | Define eligibility semantics separately for the hit artifact and canonical source bytes. | The full origin filter applies to the artifact; validity, lifecycle, closure, plus `created_after`, `status`, and every attribute term apply to the source. |
| Visible hashes enabled dictionary attacks | Key every content- or identity-derived commitment and test low-entropy candidates. | Added field-domain HMAC commitments and an explicit visible-metadata inventory. |
| Nested wire contracts were incomplete | Specify every projection-origin/contribution variant, field, null rule, encoding, and error. | Added closed v1 types, JSON rules, query-level fallback placement, and unknown handling. |

The review also required an executable same-snapshot seam. Design v6 now
specifies a transaction-owning wrapper around a generic, monomorphized search
core. Ordinary search uses a zero-sized no-op capture implementation, so it
does not gain a runtime evidence branch, provenance SQL, or evidence
allocation.

All five findings require re-review. They are not treated as accepted until an
independent reviewer passes the revised design.
