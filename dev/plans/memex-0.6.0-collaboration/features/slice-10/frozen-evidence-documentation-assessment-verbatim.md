---
title: Frozen and evidence search documentation assessment — verbatim capture
status: DRAFT SOURCE CAPTURE
date: 2026-09-12
applies_to: published FathomDB 0.8.25 tag documentation
---

# Frozen and evidence search documentation assessment

The text below is a verbatim capture of the earlier assessment. It is preserved
as planning input; it is not rewritten to reflect post-tag documentation edits.

## 3a. Do the frozen/evidence searches need better documentation?

Yes—substantially better public documentation.

The internal contract in `dev/interfaces/python.md` is reasonably precise, but the tagged public `docs/reference/python-api.md` does not document:

- `freeze_read_context`
- `search_frozen`
- `search_with_evidence`
- `resolve_evidence`
- Their request/result types
- Drift and retry behavior
- Ranking equivalence
- Evidence provenance requirements

The changelog reduces all of this to a few release bullets.

The public documentation should include a comparison such as:

| API | Frozen authority | Evidence references | Exact source resolution | Primary use |
|---|---:|---:|---:|---|
| `search` | No | No | No | Ordinary current-state retrieval |
| `search_frozen` | Yes | No | No | Reproducible ranked retrieval |
| `search_with_evidence` | Yes | Yes | Yes, via `resolve_evidence` | Grounded retrieval and answer support |

It also needs an end-to-end guide:

```text
freeze_read_context
→ search_with_evidence
→ resolve_evidence
→ record consumer use receipt
```

That guide should explain:

- A frozen context is reproduce-or-fail authority, not a long-lived snapshot lease.
- Evidence references are not authorization capabilities.
- Resolution must use an equivalent frozen context.
- Search and evidence result entries are positionally associated.
- Evidence search has the same ranking/order as equivalent frozen search.
- Consumers should not call both merely to match their results.
- Drift means restart the complete read attempt.
- Evidence references should not be the sole durable audit record.
- Which incomplete/legacy provenance conditions make evidence search fail.
- The explanation-enabled 0.8.25 defect and the first fixed version.
