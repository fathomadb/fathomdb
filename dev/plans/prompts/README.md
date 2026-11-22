# `dev/plans/prompts/` — execution-prompt archive

This directory holds the **execution prompts** used to drive per-slice / per-packet /
per-release implementation work across every FathomDB release line (0.6.0 → 0.8.20+).
As of DOC-HYGIENE-1 (2026-07-24) it holds **~216 files / ~2.8 MB**.

## Archive-in-place convention (and why)

Prompts are **archived in place, never relocated or renamed**, per the standing rule in
`dev/plans/README.md` § "Archive convention": completed prompts are cross-referenced
**by path** from ADRs, design docs, implementation docs, `dev/DOC-INDEX.md`, and prior
run logs — roughly **120 distinct prompt paths are referenced from ~140 files**.
Physically moving a completed prompt would break those references or force rewrites of
immutable historical run logs (`dev/plans/runs/*`). A prompt's status lives in its
release's `STATUS-*.md` / `*-implementation.md` ledger, not in its filesystem location.

**Do not `git mv` or rename anything in this directory.** If a path genuinely must
change, update every inbound reference in the same change and leave a note in the
ledger — this is the rare exception, not the normal path.

## What's actually in here

The great majority of this directory — well over 90% of the ~216 files — is
**closed-line, one-shot execution prompts**: per-slice prompts (`0.8.0-slice-*.md`),
per-packet/per-fix handoffs (`0.7.0-PVQ-P1-DESIGN-fix-1.md`), and per-release launch/GA
prompts (`0.6.1-GA.md`, `0.7.0-RC1.md`) for release lines that have **already shipped or
closed**. Each was written to be read exactly once, by the subagent it was addressed to,
for a slice/packet that is now done. Reading them at cold-start is almost never useful —
treat them as historical record, not as live guidance (see the staleness index in
`dev/plans/README.md`).

## Live prompts an agent should actually read

Only the entry point below is **live** for new release and slice planning:

| Prompt | Role |
|---|---|
| `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md` | **Canonical** cross-harness / cross-model release scaffolding, prework, slice planning, and Slices 20+ execution template. |

## Deprecated / legacy entry points (retained in place)

These former entry points are **deprecated — do not use for new releases or slices**. They remain on disk under the archive-in-place convention (and `scripts/commission-manifest.sh` compatibility) only:

| Prompt | Former Role / Status |
|---|---|
| `PLAN-TEMPLATE.md` | **DEPRECATED** — superseded by `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md`. |
| `0.8.0-SLICE-TEMPLATE.md` | **DEPRECATED** — superseded by `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md` (Section 2). |
| `LIBRARY-BUMP-ORCHESTRATOR-TEMPLATE.md` | **DEPRECATED** — LBO workflow retired; superseded by `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md` (Slice 1 / Slice 10). |
| `LIBRARY-BUMP-STEWARD.md` | **DEPRECATED** — LBS workflow retired; superseded by `RELEASE-AND-SLICE-PLANNING-TEMPLATE.md` (Slice 1 / Slice 9 / Slice 10). |
| `0.8.x-STEWARD-HANDOFF.md` | **DEPRECATED** — Program Steward workflow and skills removed. |
| `0.8.x-RELEASE-ORCHESTRATOR-HANDOFF.md` | **DEPRECATED** — Release Orchestrator workflow and skills removed. |
| `0.8.0-SESSION-SCAFFOLD-GENERATOR.md` | **DEPRECATED** — closed `0.8.0` campaign. |
| `DOC-HYGIENE-1-HANDOFF.md` | Historical DOC-HYGIENE-1 commissioning brief. |

Everything else in this directory is closed-line and archived-in-place. For
current program state, read the single live
`dev/plans/release-state-*.json` file selected by `scripts/release-current.py`,
then the board named by its `board` key, not a scan of this directory.
