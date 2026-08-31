---
title: 0.8.24 Slice 2 — repository cruft review
status: COMPLETE
target_release: 0.8.24
---

# Slice 2 — repository cruft review

**Observed:** 2026-08-23. This is a repository-wide classification and proposal,
not an edit pass. No file was deleted, moved, archived, or rewritten.

## Method and decision rule

The review examined repository documentation, engineering material,
developer/intermediate records, public documentation, source/test roots, and
their stated ownership. A path is called cruft only when its current role is
unexplained, contradicted, or obsolete without a retained-evidence reason.
Age alone is not evidence. The classifications mean:

- **keep** — current, intentionally historical, append-only, generated, or
  source/test material with a distinct stated role;
- **deprecate-in-place** — retain the path while marking/replacing its active
  instruction or dependency through an owner-approved follow-up;
- **archive** — retain provenance at a named destination; and
- **delete** — remove only with a named replacement or documented absence of
  one plus read/reference proof.

## Classification

| Area and concrete evidence | Classification | Proposal and rationale | Destination |
| --- | --- | --- | --- |
| `dev/archive/`, `dev/progress/`, closed-release plans/prompts/runs, and superseded ADRs | **Keep** | Each has an explicit historical/archived-in-place or supersession rule. Moving them would break path references and obscure decision provenance. | None |
| `dev/release/`, `dev/releases/0.8.0.md`, and `dev/release-policy.md` | **Keep** | These are respectively the canonical internal workflow, the 0.8.0 engineering record, and a compatibility pointer cited by requirements/learnings. | None |
| `dev/perf-history/`, `experiments/`, `dev/performance-benchmarking/`, `dev/experiments/` | **Keep** | The first is machine-read append-only history; the next three are respectively structured run index, durable human evidence, and isolated facilitation/code experiments. They are not duplicates. | None |
| `test/` and `tests/` | **Keep** | `test/` owns cross-cutting assets; `tests/` contains executable corpus/evaluation tests. The root separation is documented, not accidental duplication. | None |
| `dev/prototypes/l2-router/` | **Keep** | `build_registry.py` reads committed experiment outputs to regenerate `registry.json`; prior prune history records this as a non-removable read dependency. | None |
| `dev/turso-watch/` | **Keep** | It is an explicitly dated technology snapshot with a refresh prompt and revisit triggers. No current re-evaluation was performed, so no newer conclusion is claimed. | None |
| Root `prettier` dev dependency and bootstrap wording | **Deprecate in place** | Direct-use search found no active formatter command; policy prohibits it for Markdown. Retain until a clean tooling-install proof supports removal, then remove the dependency/lock entry and obsolete bootstrap wording together. | Candidate Slice 7 item; separate owner decision from Slice 1 security remediation. |
| Former owner URLs in maintained public docs, `mkdocs.yml`, and shipped package READMEs | **Deprecate in place** | The repository moved to `fathomadb/fathomdb`; current public links still use `coreyt/fathomdb`. Correct maintained surfaces while preserving historical links in evidence records. | Candidate Slice 7 item, R24-13 / AC24-13 draft. |
| `0.8.21` described as current in public API and historical-release docs | **Deprecate in place** | The latest published release is 0.8.23. Update reader-facing current-release assertions at implementation time; do not pre-announce 0.8.24. | Candidate Slice 7 item, R24-13 / AC24-13 draft. |
| Conflicting program-navigation claims in `dev/README.md`, `dev/plans/README.md`, and `dev/DOC-INDEX.md` | **Deprecate in place** | `dev/README.md` calls the superseded 0.8.6–0.8.16 schedule the master; `DOC-INDEX.md` names 0.8.20–0.9.0. Replace hard-coded active-release wording with the existing release-state resolution rule where possible. | Candidate Slice 7 item, R24-14 / AC24-14 draft. |
| `dev/DOC-INDEX.md` long-form map and `dev/doc-index/` detail files | **Keep, review with edits** | The layered index is intentional. Any Slice 7 documentation change must update the relevant index row/detail, rather than creating a second inventory. | Coupled to any accepted documentation item. |

## No archive or delete candidate

No reviewed path met the threshold for archive or deletion. The apparent
duplicates each have a declared retention or operational role, and a broad
prune would be higher risk than its benefit. If a later review nominates a
specific file, it must satisfy draft R24-15/AC24-15 before it can be moved or
deleted.

## Concrete follow-up scope

If the owner accepts the recommendations in Slice 6, Slice 7 should use narrow
sets rather than a repository-wide rewrite:

1. Correct only maintained public repository links and actual current-release
   statements, including `mkdocs.yml`, `docs/**`, and package READMEs. Do not
   rewrite historical evidence URLs.
2. Reconcile engineering navigation against the current program schedule and
   release-state contract, including required `DOC-INDEX` maintenance.
3. Verify that Prettier has no direct supported use, then remove it as a
   contained tooling change only if the owner accepts that separate item.

The recommended proof is scoped text/config checks, documentation lint/build
where affected, and the existing Markdown-tooling check for a Prettier change.
It does not justify a fresh full hosted CI or release workflow.

## Evidence

- Former-owner maintained-surface scan: `mkdocs.yml`, public `docs/**`,
  `src/python/README.md`, `src/ts/README.md`, and Rust/package READMEs contain
  current-facing `coreyt/fathomdb` links; workspace metadata already uses
  `https://github.com/fathomadb/fathomdb`.
- Stale-release scan: `docs/embedder.md`, `docs/reference/python-api.md`,
  `docs/reference/typescript-api.md`, and the 0.6.0/0.6.1/0.8.0 release-note
  banners name 0.8.21 as current.
- Navigation scan: `dev/README.md` names the superseded schedule as master,
  while `dev/DOC-INDEX.md` names the newer 0.8.20–0.9.0 schedule.
- Retention evidence is cited in the reviewed README files and in
  `scripts/repo-prune/runs/doc-prune-CLEANUP-MAP.md` for code-read and
  reference material.
