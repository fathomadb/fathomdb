---
title: FathomDB 0.8.25 post-release documentation update
status: COMPLETE
target_release: 0.8.25
---

# FathomDB 0.8.25 post-release documentation update

## Goal

Bring the maintained public and internal documentation into agreement with the
already-published FathomDB 0.8.25 artifacts. This is a documentation correction
for 0.8.25, not a new product release: the immutable `v0.8.25` tag, package
manifests, implementation, tests, and release-state ladder remain unchanged.

## Existing evidence

- `CHANGELOG.md` records the published 0.8.25 feature set and schema change.
- `dev/plans/runs/STATUS-0.8.25.md` records the tag, publication run, registries,
  and accepted release evidence.
- `dev/interfaces/{rust,python,typescript,cli,wire}.md` define the shipped
  contracts.
- `src/rust/crates/fathomdb-engine`, `src/python/fathomdb`, and `src/ts/src`
  provide the as-built API and SDK names.
- `src/rust/crates/fathomdb-schema/src/lib.rs` sets `SCHEMA_VERSION` to 33.

No new dry run, rehearsal, platform run, package build, or product test is
required to restate those accepted facts.

## Work

1. Correct maintained current-release, installation, compatibility, and
   release-note pages from 0.8.23 to 0.8.25.
2. Document the shipped 0.8.25 Python and TypeScript methods, data carriers,
   errors, runtime controls, and bounded graph/read surfaces using the locked
   interfaces and source as evidence.
3. Document the shipped `doctor data-plane-integrity` CLI verb and make the
   public reference indexes point to the new release notes.
4. Correct stale release metadata in the internal interface frontmatter while
   preserving the contracts themselves.
5. Run only documentation-proportionate verification: stale-current-version
   scans, Markdown/documentation lint, strict MkDocs build, and a clean Git
   diff/status check.

## Boundaries

- No source, schema, package, workflow, or test changes.
- No movement or recreation of `v0.8.25`.
- No reopening of the completed 0.8.25 release ladder.
- Historical release references remain historical; only maintained assertions
  that identify the current published release are updated.
- Any issue outside documentation is recorded rather than fixed unless it
  proves the published 0.8.25 surface is incorrectly described.

## Completion criteria

- Maintained public pages consistently identify 0.8.25 as current.
- Install examples select 0.8.25 and compatibility reports schema 33.
- Public Python, TypeScript, CLI, configuration, and error references cover the
  shipped 0.8.25 additions.
- A public 0.8.25 release-note page is reachable from the documentation.
- Documentation gates pass and the final diff contains documentation only.

## Result

Completed on 2026-09-12 as a documentation-only post-release correction. The
maintained public pages now identify 0.8.25, installation selects the published
artifacts, compatibility records schema 33 and the accepted performance-gate
disposition, and the Rust/Python/TypeScript/CLI/config/error references cover
the shipped additions. Historical 0.8.23 feature-origin references and the
separate exact `0.8.24+tegra` route remain intentionally unchanged.

Scoped verification passed:

- `bash scripts/agent-lint-docs.sh`
- `bash scripts/agent-lint-md.sh`
- `mkdocs build --strict`
- stale-current-version and governed-method-name scans
- `git diff --check`
