---
title: 0.8.25 Slice 40 design review — cycle 4
status: PASS
reviewed_commit: 9cf6f146
verified_commit: 465eb0a5
---

# Slice 40 design review — cycle 4

## Verdict

PASS after the documented corrections. The independent reviewer initially
accepted generation identity, retention, stale-job fencing, digest ordering,
trigger coverage, and platform routes, but found the remaining
implementation-shaping gaps below. Exact-commit verification at `465eb0a5`
confirmed that all P1/P2 findings are closed.

## Findings and dispositions

| Finding | Severity | Disposition in design v7 |
|---|---:|---|
| DR40-46 receipt semantics departed from Slice 25 | P1 | Preserve the exact terminal-absence definition; correlate only existing pending cursors; use a dispatcher-latched usable embedder in the measurement fixture. |
| DR40-47 strict source eligibility missing from physical membership | P1 | Add one effective-time eligibility predicate shared by membership, scheduling, pending probes, and publication. |
| DR40-48 future edge-expiry residue incorrectly called corruption | P1 | Treat retained expiry artifacts as legal dormant rows and preserve dense soft fallback; reserve corruption for transitions promising pruning. |
| DR40-49 coarse readiness mapping changed accepted semantics | P1 | Leave existing runtime-gated readiness unchanged and make the new generation status authoritative. |
| DR40-50 completion classifier was not total | P2 | Enumerate omitted cases and make every unlisted state a tested typed corruption. |
| DR40-51 exact config replay can legitimately repair state | P2 | Separate quiescent no-op from same-generation enrolment/unstranding repair. |
| DR40-52 boot rederive always changed frozen visibility | P1 | Require an exact differential comparison and no-write healthy restart, including a nonempty projection test. |
| DR40-53 mean recomputation paths omitted | P1 | Classify automatic pin, boot recovery, and doctor recompute as same-generation physical maintenance with visibility/race coverage. |
| DR40-54 public and wire shapes incomplete | P2 | Specify existing operation-ID string grammar, additive receipt field in all SDKs, exact error payload, paths, order, and redaction. |
| DR40-55 fresh-versus-legacy predicate undefined | P2 | Define a closed bootstrap manifest covering content, registry, receipts, enrolment, terminals, physical rows, and cursors. |

Implementation did not begin until the cycle-4 reviewer verified these
corrections.

## Correction verification follow-up

Verification of the first correction at `f1261481` closed DR40-46, DR40-48,
DR40-49, and DR40-51 through DR40-54. It kept DR40-47/50 open because source
restoration must not implicitly re-admit a dependency-closed derived node, and
kept DR40-55 open because the bootstrap manifest omitted receipt-source
references and did not dispose every system/config table.

Design v7 now makes registered-derived membership lifecycle-aware, defines one
complete explicit-reactivation projection path shared by ordinary and actuation
transitions, adds both restoration sequences to tests, includes receipt-source
references and the remaining content-bearing tables, and gives open-state,
read-visibility, default-profile, and seeded collection rows exact baseline
dispositions. A final exact-commit verification of this same cycle remains
required.

The next verification accepted the lifecycle correction but found that a fresh
database opened with a supported caller-provided embedder would be mislabeled
legacy. The final correction excludes exactly the single validated supplied
default-profile identity when no mean is pinned, binds that identity in the
declaration digest, adds caller-embedder fresh/restart tests, and limits the
special FTS/dense repair helper explicitly to registered derived-node
reactivation. Final exact-commit verification at `465eb0a5` passed.
