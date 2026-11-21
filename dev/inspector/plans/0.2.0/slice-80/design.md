---
title: Inspector 0.2.0 Slice 80 — readiness design
status: APPROVED
desc: Reuse existing verification and package tools for a concrete local release handoff
---

# Slice80 readiness design

[Plan](plan.md) owns outcomes. [Owner decision](../release-decisions.md) removes
human recruitment and introduces80→90. No Engine publication lifecycle changes.

## Existing seams and proposed composition

- Canonical Inspector requirements/AC and the single matrix currently carry the
  former human-study gate. Reconcile current contracts and active docs once, with
  explicit historical pointers; keep user goals and actual technical evidence.
- Existing `src/inspector/tools/package.sh` builds a fixed0.2.0 cached-CPU GNU
  release tar with executable, license, README, build description and checksums.
  It does not itself establish an Ubuntu22 ABI floor. Reuse the qualified isolated
  Ubuntu22 image/target and native installed harnesses from70/75.
- Existing CI paths route Inspector changes to Ubuntu verify. One small
  Inspector-local entry point can compose existing source/unit/browser/package
  commands and document prerequisite/tool ownership; run it locally. Avoid new
  worker orchestration, duplicated206-suite registries or silent model/native skips.
- Existing package tests validate actual archive/extract/help/features. Extend
  only missing release-contract behavior and small negative fixtures. Keep the
  product's runtime requirements separate from Node/Python measurement tooling.

## Candidate and distribution contract

Use product/version/platform names independent of Engine's0.8.x publisher.
Candidate output stays in owned ignored staging/build storage until90. A compact
manifest binds actual candidate source/build, archive and executable SHA256,
features/schema/Engine and exact platform/browser/kernel limits. Inner checksums
cover bundle files; an outer checksum covers the archive without self-hashing.
Do not claim signed provenance or bit-reproducible builds merely from checksums.

A minimal tested local promotion step validates candidate identity/bytes and
uses owned same-filesystem staging before final `dist/inspector/0.2.0/`. Refuse
unrelated or mismatched destinations, preserve earlier candidates, never delete
arbitrary user paths and leave no partial success marker. Reuse existing ignore
rules if adequate. Future distribution documentation separates current local tar
from proposed public archive/registry channels, version/signing/credential/CI work;
no publisher/plugin framework or remote service is needed now.

## Verification and handoff

Review concrete command ownership, prerequisite/failure propagation, metadata
sources and destination handling before code. Test meaningful failures with small
fixtures; execute actual local CI and one real compatible package/installed workflow.
Sol-high reviews additions/code, Terra independently verifies documented commands
and artifact behavior. Record a small concrete handoff that90 can execute verbatim;
no repeated report packets or all-platform claims.

## Execution reconciliation — 2026-11-21

Since the draft, baseline395662894 added the owner ruling,80/90 plans and
release ladder.75 already supplies the seven UX fixes and a genuine206-suite
pass; retain those instead of reopening features. Existing package.sh has inner
checksums but no archive checksum/provenance or failure cleanup. Shared CI already
routes Inspector code/assets/package paths to Ubuntu verify; retain it unchanged.
Current docs/matrix still contain the removed study blocker and stale24-only
packaging language: adjust active contracts, mark old reports historical. No new
product behavior, Engine pin, test registry or study is needed.

### Concrete command contract

- `local-ci.sh NEW_CANDIDATE FIXTURES UBUNTU22_TARGET` preflights absolute paths,
  checkout tools and required real fixture stores. Default execution runs one typed
  full source gate, tool/JS unit tests, then an actual offline package and release
  examples build in the isolated Ubuntu22 userland/target. `package.sh` supports an
  explicit absolute CARGO_TARGET_DIR, emits inner/outer checksums and cleans only
  fresh failed output. Immediately after build, host `local-release.py seal` binds
  the actual executable/build/hashes to the same clean source HEAD tested above.
  Source stays unchanged throughout. Git is a host metadata prerequisite, not an
  extra container package. Existing installed harnesses consume that actual package
  and matching Ubuntu22 release fixtures. Every stage fails explicitly; statuses
  propagate. Existing shared hosted routing remains unchanged and fixture-tested.
  `--installed-only` verifies existing candidate bytes/source and installed behavior
  without a source-pass claim. It permits only a clean same-source or Inspector
  docs-only descendant, excluding packaged README and any scripts/tests/source.
- `local-release.py verify CANDIDATE` checks the exact allowlisted bundle/archive
  members, internal/outer hashes, build-description equality and production
  version/features/target/schema. It refuses symlinks and unexpected contents.
  `promote CANDIDATE NEW_DESTINATION` validates first, refuses any existing path,
  copies to a fresh sibling temporary directory, revalidates, and renames only
  after success. Exception/interrupt cleanup removes only its self-created staging.
  Destination parent must exist; caller chooses an owned rehearsal directory in80
  or `dist/inspector/0.2.0` in90. No replace/idempotent overwrite behavior is needed.

Use Python stdlib for release filesystem/JSON/tar checks, Bash only for existing
command composition. Tests exercise real small files/tars and actual subprocess
failure statuses; fake executable metadata is confined to those filesystem tests,
never qualification. Real package validation and installed CPU/person/UI checks
remain mandatory. Candidate manifest is unsigned local provenance, not a claim of
reproducible build or trusted external attestation. Metadata files are outside the
archive, avoiding self-hash cycles. After docs-only closure, preserve its true source
SHA rather than relabeling. Future registries are instructions/roadmap only.

Design review: directly spawned Sol-high approved this composition with source
binding and Ubuntu22 release-example clarifications before implementation.
The package source/build and later documentation closure SHA remain distinct.
