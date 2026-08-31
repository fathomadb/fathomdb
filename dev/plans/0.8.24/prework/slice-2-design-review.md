---
title: 0.8.24 Slice 2 — repository-hygiene design review
status: PROPOSED
target_release: 0.8.24
---

# Slice 2 — repository-hygiene design review

## Review boundary

This review turns the Slice 2 plan into an evidence-led hygiene proposal. It
does not treat age as cruft, does not move historical evidence merely to make
the tree smaller, and authorizes no deletion or rewrite. Formal edits to user
needs, requirements, acceptance criteria, or architecture remain Slice 3
drafts; an owner decision in Slice 6 is required before Slice 7 implements any
proposal below.

## Review of proposed work and allocation

| Proposed item | Surfaces reviewed | Disposition | Allocation |
| --- | --- | --- | --- |
| Broad repository cleanup | Program records, `dev/`, public docs, package READMEs, source and test roots | **Approve, narrowed.** A cleanup must correct misleading current instructions or remove a demonstrably unneeded artifact; bulk age-based pruning is rejected. | Slice 2 inventory; owner-approved repairs only in Slice 7. |
| Historical plans, prompts, run records, ADRs, and archive | `dev/plans/README.md`, `dev/archive/README.md`, `dev/progress/README.md`, ADR supersession records | **Reject relocation/deletion.** These paths intentionally preserve cross-referenced decisions and provenance. | Keep in place; no 0.8.24 implementation item. |
| Root Prettier dependency | `package.json`, lockfile, `scripts/bootstrap.sh`, Markdown guard documentation | **Approve deprecate-in-place candidate.** No active command invokes it, and repository policy forbids using it for Markdown; removal still needs a package/install proof. | Candidate Slice 7 item, separate from the Markdown security update. |
| Former GitHub-owner links | `mkdocs.yml`, public `docs/`, shipped binding/package READMEs | **Approve correction candidate.** The 2026-08-10 transfer makes current-facing `coreyt/fathomdb` links misleading. Historical evidence links may retain their original target. | Slice 7, with a bounded maintained-surface scan. |
| Stale “current release” assertions | Public docs and release-note archival banners | **Approve correction candidate.** The checked-in assertions name 0.8.21 although 0.8.23 is released. | Slice 7, update to the actually published release at implementation time. |
| Engineering-navigation conflicts | `dev/README.md`, `dev/plans/README.md`, `dev/DOC-INDEX.md` | **Approve correction candidate.** `dev/README.md` calls the superseded 0.8.6–0.8.16 schedule the master while `DOC-INDEX.md` names 0.8.20–0.9.0. | Slice 7, preserve dynamic release-state lookup rather than hard-coding another version. |
| Test and experiment roots | `test/`, `tests/`, `experiments/`, `dev/experiments/`, `dev/perf-history/`, `dev/performance-benchmarking/` | **Keep.** The roots have distinct contracts: cross-cutting assets, executable tests, structured experiment index, facilitation experiments, machine-read perf history, and human benchmark evidence. | No change. |
| Legacy-looking release/prototype/watch paths | `dev/release/`, `dev/releases/`, `dev/release-policy.md`, `dev/prototypes/`, `dev/turso-watch/` | **Keep.** Each has an explicit canonicality, compatibility, reproducibility, or dated-snapshot role. | No change. |

## Additional draft product-document inputs

| ID | Draft need | Draft requirement | Draft acceptance criterion | Allocation |
| --- | --- | --- | --- | --- |
| N24-6 | A user needs current project links and release guidance, even after repository ownership changes. | R24-13: maintained public documentation, site configuration, and shipped package READMEs must name the canonical repository and an actually published current release. | AC24-13: a bounded scan of maintained `docs/**`, `mkdocs.yml`, and shipped package READMEs finds no former-owner URL or stale current-release assertion, except an explicitly historical record. | 7 |
| N24-7 | A maintainer needs one unambiguous route from engineering navigation to the active program and release state. | R24-14: active engineering navigation must point to the current program schedule and resolve release state through the release-state file, not contradictory hard-coded records. | AC24-14: `dev/README.md`, `dev/plans/README.md`, and `dev/DOC-INDEX.md` agree on the canonical schedule and release-state lookup contract. | 7 |
| N24-8 | A maintainer needs cleanup to preserve reproducible evidence and not silently break code or documentation references. | R24-15: any future archive/delete proposal must name a retention/replacement path and show no tracked runtime read or unresolved inbound reference. | AC24-15: each executed archive/delete item records its candidate paths, destination or replacement, tracked-code-read search, inbound-reference search, and targeted verification. | 7 or later, only if the owner accepts a concrete deletion/archive item. |

These are approved draft inputs for Slice 3. They do not alter the canonical
need, requirement, acceptance, or architecture documents in this slice.

## Design rules

1. Preserve the distinction between current guidance, historical evidence, and
   generated artifacts. A stale document is repaired or explicitly labeled; it
   is not silently discarded.
2. Limit current-link remediation to maintained user and package surfaces.
   Historical GitHub URLs remain evidence unless a specific reader-facing error
   justifies changing them.
3. Do not treat a public-document repair as a release-process gate. The proof
   is a scoped text/link/config check, not an intentionally triggered hosted
   CI cycle.
4. A dependency can be removed only after a direct-use search and a clean
   installation/tooling verification establish that its retained installation
   path is unnecessary.
5. Preserve append-only and code-read data, including perf history, experiment
   indexes, release evidence, and prototype regeneration inputs.

## Evidence

- [Slice 1 design review](slice-1-design-review.md) and [library sweep](slice-1-library-sweep.md).
- `dev/README.md`, `dev/plans/README.md`, `dev/DOC-INDEX.md`,
  `dev/archive/README.md`, and `dev/progress/README.md`.
- `mkdocs.yml`, `docs/**`, package README files, `package.json`,
  `scripts/bootstrap.sh`, and `scripts/agent-lint-md.sh`.
- `dev/perf-history/README.md`, `dev/performance-benchmarking/README.md`,
  `experiments/README.md`, `test/README.md`, `dev/release/README.md`,
  `dev/release-policy.md`, `dev/prototypes/l2-router/README.md`, and
  `dev/turso-watch/2026-06-21-sqlite-vs-turso-gap-analysis.md`.
