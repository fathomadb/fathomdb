# ADR-0.8.22 — Windows native npm package name

- **Status: ACCEPTED (HITL approved 2026-08-09).** Partially supersedes
  ADR-0.8.20-unscoped-npm-platform-packages for Windows x64 MSVC only.

## Decision

The Windows x64 MSVC native npm package is
`fathomdb-native-win32-x64-msvc`, not `fathomdb-win32-x64-msvc`.

All non-Windows native packages retain the unscoped
`fathomdb-<triple>` naming convention from ADR-0.8.20.

## Rationale

The attempted trusted-publishing bootstrap for `fathomdb-win32-x64-msvc`
received npm registry `E404`, while the bootstrap package
`fathomdb-native-win32-x64-msvc@0.8.22-bootstrap.1` published successfully.
The package name is therefore a release prerequisite, not a compatibility
alias.

## Consequences

The runtime loader, publish-time optional dependency injection, platform
capability manifest, Windows platform package metadata, release workflow, and
trusted-publishing procedure use the new Windows name. No compatibility shim
or publication is made for the unavailable former name.
