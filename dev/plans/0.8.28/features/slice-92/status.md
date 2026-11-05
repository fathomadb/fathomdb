---
title: FathomDB 0.8.28 Slice 92 — completion status
status: COMPLETE
---

# Slice 92 status

**COMPLETE.** Documentation baseline `7b24e4c0913ec6585781b1148b81da0853f9dd67`; Slice 95 is the remaining
release-closing work. Owner placement: ledger seq-365. No publication or final
release-readiness claim is made.

## Outcome and acceptance

The [plan](plan.md), [design](design.md) and [inventory](inventory.md) record
reconciliation from `77b6f0b4d`, all maintained-owner dispositions and source/test
witnesses. Astra design and code/content reviews APPROVE; a separate Astra
verifier approves semantic/example correctness and unchanged-input reuse.

| Acceptance | Result |
| --- | --- |
| S92-1 audit coverage | PASS: user/contributor/project cohorts and all 27 maintained design owners reviewed; historical cohorts preserved. |
| S92-2 user truth | PASS: pending .28 guidance, format36/refusal, provider admission, retrieval/filter/view/context/maintenance/configuration/errors, install prerequisites and release notes reconciled. |
| S92-3 contributor truth | PASS: current architecture/design/interface owners and code documentation match accepted decisions and source; taxonomy and cursor/identity claims corrected. No runtime/API/policy change. |
| S92-4 authority/hygiene | PASS: lifecycle/authority/index/navigation checks; three existing next-state renderers registered, 13 generated blocks current. Historical note bodies, decisions and frozen baselines unchanged. |
| S92-5 generated/release truth | PASS: existing generator freshness, pending source-bound schema36 vs published schema34 controls, exact install/platform/publication guards and local strict MkDocs build. Published .27 receipt and manifests unchanged. |
| S92-6 executable guidance | PASS: five isolated Python processes, two compiled external Node programs, external Rust SDK consumer, nine SDK doctests and immutable CLI inspection (clean, nine checks, exit0). |
| S92-7 reviews/testing | PASS with assembled provenance below: Astra findings resolved; all nine affected suites pass unchanged scoped rechecks. |
| S92-8 handoff | PASS: release state/board/plan point to 95; its plan/status consume this baseline and preserve final qualification/publication boundaries. |

## Verification provenance

- Checker RED `4e9e0c4db`: pending format36 failed the old schema34 assertion.
  GREEN `a0c0ccd46`: focused controls pass, including independent historical
  receipt guards and wrong/missing/ambiguous source/schema rejection.
- Content review baseline `082bb9eba`; final example candidate
  `ddd5337b25a5354ada207edb19be21b7c843698b`. Final navigation/taxonomy/state-view
  repairs at the baseline above change only documentation and release records;
  executable source/build inputs and exercised examples are unchanged.
- Full local command on clean ddd5337b2:
  `PATH=/home/coreyt/.nvm/versions/node/v25.9.0/bin:$PATH CARGO_BUILD_JOBS=2 ./scripts/agent-verify.sh`.
  Lint/typecheck/security pass; security records 0 violations, 0 blockers,
  0 downgrades. All 205 registered suites ran: 196 initially passed, 9 failed,
  0 skipped/excluded. Rust and language/package suites passed.
- The nine failures were historical-banner/index drift, missing existing
  generated next-state regions (including the shell guard's nested check), and
  six preflight families below the 10 GB disk requirement after artifact builds.
  Only task-owned disposable installs/caches/duplicate unpacked addons were
  removed; compressed packages, sources, lockfiles and logs remain. Headroom
  restored; no threshold, test or skip was weakened.
- All nine unchanged affected suites pass scoped rechecks: slice10 preparation,
  preflight landing, board currency, ledgers, governed-surface pin, C1,
  transcript hygiene, release-state views and shell pipefail guards.
  This is assembled/resumed verification, **not an uninterrupted full-gate pass**.
  No broad regression was repeated for the prose/record corrections.

Evidence: `/tmp/fathomdb-s92-full-verify.log`,
`/tmp/fathomdb-s92-rechecks.json` and named recheck logs;
`/tmp/fathomdb-s92-doc-gate.log`, `/tmp/fathomdb-s92-mkdocs.log`;
`/tmp/fathomdb-s92-verification/verification.json` and
`/tmp/fathomdb-s92-verification/ddd5337b2/` (repack, input comparison, examples,
consumer sources/lockfiles and artifact identities). These are reproducible
local evidence locators, not published-artifact receipts.

## Candidate consumers and limits

Rust 1.95.0, Python 3.12.3/maturin 1.15.0 and Node 25.9.0. Fresh CPU dev-profile
wheel/addon and packed packages carry intentionally unchanged version 0.8.27;
they prove candidate source behavior, not registry, optimized-release, CUDA or
five-platform qualification. Targeted repack reran both Node programs after
native README corrections; addon bytes stayed identical. Earlier Python/Rust/CLI
proofs remain valid by exact relevant-input comparison.

| Artifact | SHA256 |
| --- | --- |
| Wheel | `2a2edfedca8e27b7187f4624ccfc35a5e9eed006392464122f3ea04ebab5d9e4` |
| npm main | `90ae63b5d438dbb89ccd3b731e019617d63bbe20258ba29437bff78b7de7ce4a` |
| npm native | `db173f579d81114b556ab544632d74076f8ebcfe5ff6df1e60e3e0730ec978a3` |
| Addon | `e824a37e0179cc1c8787a5cc36a34952eff9074d9590ee8c44f497dba34caffe` |
| CLI | `f9c40ece23c9707b145ec78b95de071026d7de8157178cfed9adc48550cfdceb` |

## Slice 95 handoff

Consume the reviewed documentation baseline; recheck only documentation affected
by later CI/package repairs. Slice 95 retains its seven CI repair areas, final
feature/security/performance/platform/package matrix and non-publishing rehearsal.
Existing .29 hand-over and owner deferrals remain unchanged. Physical erasure is
complete except for defects; publication remains separately held. No hosted CI,
registry/Pages publication or .29-start action occurred here.

No task worktree or branch was created. The existing dev/release checkouts are
the landing route; the records-only closeout preserves the tested source/build
inputs. Disposable verification installs were cleaned without touching shared
caches or other worktrees; retained package evidence is listed above.
