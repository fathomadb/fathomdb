---
title: 0.8.24 Slice 3 — architecture draft CRUD
status: COMPLETE
target_release: 0.8.24
---

# Slice 3 — architecture draft CRUD

**Observed:** 2026-08-23. These are draft operations only. Accepted ADRs remain
authoritative; no architecture, ADR, interface, workflow, or source change is
made here.

## Disposition register

| ID | Current authority and evidence | Draft operation | Primary allocation |
| --- | --- | --- | --- |
| A24-1 release completion semantics | `dev/design/release.md` already documents idempotent partial-republish retries, while `REQ-050` / `dev/architecture.md` still describe atomic publish. ADR-0.8.18 explicitly says to replace atomicity with ordered, retry-safe completion. | **Update** `dev/design/release.md` and the release row in `dev/architecture.md`: publish artifacts are immutable; completion requires every required target to be present and smoked; retries skip existing valid versions and fail closed on uncertainty. No new ADR. | 60 |
| A24-2 Tegra public distribution | The current `+tegra` wheel is deliberately local proof only. ADR-0.8.20 covers generic Linux ARM64 npm packages, not a public Tegra CUDA Python distribution. | **Create, conditional, an ADR or design decision** that names the public distribution, resolver/CPU separation, trusted publisher, compatibility/unsupported contract, and target smoke. Do not amend ADR-0.8.20 by implication. | 30 |
| A24-3 Windows CUDA delivery | ADR-0.8.22 owns a Windows CPU npm package name. Current hosted Windows packaging is not CUDA proof; no remote executor or CUDA SDK matrix exists. | **Create, conditional, an ADR or design decision** for the chosen SDK surfaces, remote executor/trust boundary, CUDA/toolchain provenance, artifact naming/loader behavior, and unsupported-route policy. | 40 |
| A24-4 target-installed verification | `ADR-0.6.0-tier1-ci-platforms` requires installed smokes, while `dev/design/release.md` concentrates on a wheel/npm general statement. | **Update** the release design after A24-2/A24-3 decisions with an explicit capability matrix: CPU lanes preserved; each newly supported target has a named package identity, environment, installed smoke, and provenance. If it changes Tier-1 scope rather than adds a separately named distribution, propose an ADR successor. | 60 |
| A24-5 main CI interface | Slice 0 found proportional routing already on main and no uncovered target route yet. | **No architecture change.** Slice 10 starts from an evidence-backed no-change presumption. Any future selector/artifact route belongs in current main CI design, not the release branch. | 10 |
| A24-6 performance integration | SCALE-02 is a retained unmerged engine delta with an owner-selected no-rerun decision rule. Existing retrieval/performance architecture owns the execution path. | **No architecture CRUD now.** Slice 20 must write its own implementation design that links the branch evidence, constrained code delta, exact fidelity proof, and regressions before any integration. | 20 |
| A24-7 Windows WAL | Existing bindings design §7 documents cross-process lock behavior; external Memex evidence is unavailable due service limits. | **No architecture CRUD now.** Slice 50 compares actual client evidence with the installed Python path. A discrepancy may later require an architecture/interface amendment; unknown evidence does not. | 50 |
| A24-8 repository hygiene/dependencies | Slices 1–2 proposals affect root tooling, maintained links, navigation, and documentation controls only. | **No product architecture CRUD.** Retain as bounded Slice 7 maintenance. | 7 |

## Draft architecture changes by feature

### Slice 30 — Tegra

The Slice 30 draft-to-ready plan must carry A24-2 and the product trace from
`REQ-TARGET-TEGRA`. It must answer the owner-selected distribution name,
registry/trusted-publisher identity, pip selection behavior relative to generic
ARM64 CPU, CUDA and L4T compatibility, and exact installed-package evidence.
The local `+tegra` build remains an evidence tool and must not be reclassified
as a public package shape.

### Slice 40 — Windows CUDA

The Slice 40 draft-to-ready plan must carry A24-3 and
`REQ-TARGET-WINDOWS-CUDA`. It must bind the selected Python/npm surface to a
named remote executor, GPU/CUDA/toolchain facts, artifact transfer/provenance,
and unsupported behavior. The existing Windows CPU npm package name remains
governed by ADR-0.8.22; a CUDA route does not silently replace it.

### Slice 60 — release topology

The Slice 60 draft-to-ready plan must carry A24-1 and A24-4, including the
proposed `REQ-050`/`AC-054` and `REQ-052`/`AC-056` updates. It owns the matrix
of existing CPU lanes, named new target artifacts, registry query failure,
existing-version no-op, missing-version retry, and installed-package smoke.
It is not allowed to claim publication atomicity that the immutable registries
cannot provide.

### Slices 10, 20, 50, and 70

Slice 10 receives an explicit no-change architecture finding, not CI work by
default. Slice 20 receives a separately reviewed implementation design, not a
new generic performance architecture. Slice 50 receives evidence attribution
before any proposed WAL change. Slice 70 assembles the approved evidence and
does not create publication authority.

## Interfaces and ADR discipline

No current Rust, Python, TypeScript, or CLI runtime interface needs a draft
CRUD change merely because packaging changes. If Slice 30 or 40 exposes a new
user-visible installation/unsupported behavior through an SDK or loader, its
own plan must draft the matching interface document change. A decision that
changes the accepted platform or package-identity contract requires a new ADR
or successor; it cannot be recorded as a release-note-only exception.

## Evidence

- `dev/architecture.md` release subsystem row and `dev/design/release.md`.
- `ADR-0.6.0-tier1-ci-platforms`, `ADR-0.8.18-full-publish-pipeline`,
  `ADR-0.8.20-linux-aarch64-native-artifacts`,
  `ADR-0.8.20-unscoped-npm-platform-packages`, and
  `ADR-0.8.22-windows-native-npm-package`.
- `dev/design/bindings.md` §§7 and 9 and the current Python/TypeScript
  interface documents.
- Slice 0 benchmark, CI, publication, and owner-decision records.
