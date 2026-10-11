---
title: FathomDB 0.8.27 Slice 150 — qualification design
status: REVIEWED_DESIGN
target_release: 0.8.27
planning_baseline: 3a69eef82e0381641e5bfbf7dcd8ed25141d97ff
---

# Slice 150 qualification design

## Identity and sequence

The working branch starts at local `release/0.8.27` commit `3a69eef82`; a
fetch on 2026-10-10 showed the remote branch was older, so local release Git
is the current planning source. No execution begins until the owner confirms
Slice 140 complete. Then fetch and compare local/remote ancestry, advance this
worktree to the completed release commit, and require the tracked Slice 140
state/board, exact dependency SHA and preflight to agree. Do not qualify an
uncommitted or mixed checkout. A reviewed candidate commit is immutable for
the run; work in separate scratch and clean consumer directories.

The candidate manifest in this slice records: source commit and tree hash;
reviewed Slice 140 SHA and closeout; host OS/architecture, compiler, Python,
Node, CUDA/driver versions where applicable; gate name, command, environment,
start/end, exit, count/skip/exclusion summary, log path and SHA-256; produced
artifact path, size and SHA-256; installed-consumer environment and artifact
provenance; comparator baseline/capture digests and row differences; explicit
`PASS`, `FAIL` or `UNAVAILABLE` per route. The manifest distinguishes product
source, native binary and package bytes. The full test gate must be one
uninterrupted run to claim an overall PASS; focused reruns alone cannot erase
a failed full-gate receipt.

## Qualification matrix

| Order | Evidence | Decision rule |
| --- | --- | --- |
| 1 | Git/preflight, free disk, toolchain and asset availability. | Candidate clean and dependency closed; provision missing prerequisites without weakening gates. The public comparator requires at least 100 decimal GB free, so do not launch it below that threshold. |
| 2 | Public comparator capture/compare, hidden capture/compare and per-feature test inventory; SDK parity checker. | First commit a RED test proving the missing `fathomdb-sdk` rustdoc row, then add that bounded `ROWS` entry and its test inventory in GREEN. Compare to immutable approved baselines; review the additive SDK row and list inherited/intentional deltas separately. Validate 44/44 parity. Never rewrite prior baseline entries or regenerate an oracle to hide differences. |
| 3 | Feature-complete gate, full `agent-verify`, full-workspace Clippy and check. | All mandatory gates exit zero; feature-complete shows every non-exempt target run with live prerequisites. Record exact counts. Follow repository lint → typecheck → test ordering, short-circuiting on failure. |
| 4 | Feature/platform matrix. | Distinct default, operator, test-hooks, slice72-test-hooks, migration/test and benchmark routes where the source declares them; debug/release, CPU/driverless and CUDA routes where applicable. Avoid mutually exclusive native backends in one misleading all-features run. Windows and Jetson use exact Git archive/package hashes and their own host receipts. |
| 5 | Package and consumer routes. | Fresh Python wheel and Node package/native addon from the candidate, with clean non-editable installs, resolved import/native paths and minimal open/write/read/close plus erasure. A candidate-bound Rust source archive includes the workspace manifest, lockfile, `src/rust` and patched `third_party`; an external consumer compiles `fathomdb-sdk` through a local path dependency and runs the same real-database smoke. Record archive members and hash, consumer manifest and lockfile hashes, resolved sibling and active patch paths, unused patches and result. The frozen Memex oracle uses the candidate native package, with its file and setup unchanged. Slice 160 owns registry-resolved Rust `.crate` packaging and registry-installed proof. |
| 6 | Strict security and live AC-037. | Human performs only the runbook's temporary AppArmor grant and revert. Agent captures baseline denial, exact candidate and three pass lines, then revert denial. Any downgrade or missing revert is not PASS. |
| 7 | Independent review, retirement and closeout. | `gpt-6-sol` high checks candidate/evidence and Terra independently reproduces selected decisive checks. Final hidden/test-inventory digests go to the hidden-surface status before deletion. After all pre-retirement qualification passes, prune owned oracle data, delete temporary tool/tests/fixtures/baselines and its fast-tier call; rewrite cadence step 4 and the master Slice 150 paragraph to cite those digests and state that the oracle ran and was retired. Retain permanent feature-complete, test-target coverage, feature matrix/allowlist, warning-free builds, dead-code and removal-changelog gates. Run applicable checks again, then repeat the complete human grant, strict security check and immediate revert on the final executable commit. |

The original Slice 30 baseline remains immutable. The Slice 140 reviewed
`f327c28c` hidden successor is the comparison authority unless the completed
Slice 140 closeout explicitly supplies a newer reviewed successor. The public
comparator's earlier-slice differences are not attributed to Slice 150. Use
the fixed Memex test at
`dev/plans/0.8.27/features/slice-20/memex-consumer-oracle.py`; record its file
hash before and after the run.

The pre-retirement live security run and the final post-retirement run are two
complete `dev/release/ac-037-live-netns-hitl-runbook.md` cycles. Each has its
own HITL grant, three required pass lines, immediate revert and denial
confirmation. After the second cycle, only receipt/status/release-state
commits may follow. They must preserve the tested `src/`, `scripts/`,
`dev/tools/`, workflow and package-manifest trees byte-for-byte; record their
Git tree IDs and an exact changed-path list against the tested commit. The
tested commit remains the final executable candidate. Any later change to a
gate input, source or executable script requires a new exact-candidate gate
receipt rather than a tree-equivalence assertion.

## Retention and large-artifact survey

Read Slice 90's artifact inventory, `runtime-performance-qualification.md`,
and audit hashes first. Identify the 77.92 MiB host-local checkpoint
revalidation set by exact member paths, sizes and receipt bindings; distinguish
it from other D27, performance, security, platform and published-0.8.26 audit
records. Measure existing files and directories without following symlinks,
using the same 100,000,000,000-byte strict threshold for both scopes:

1. 0.8.27 release checkout and its host-local `qualification-evidence` tree;
2. other repository checkouts and associated data directories on this host.

For every qualifying object, retain path, measured bytes, owning checkpoint,
receipt/digest binding and a reasoned retain or cleanup proposal. Record
`zero qualifying objects` for an empty scope. Avoid counting nested parents
and members as independent retained bytes. Survey is read-only; any cleanup
decision is separate from this qualification run.

## Failure and change rules

No new product code is planned. A failing behavior gets a RED reproducer
first, then an owner-scoped fix and `gpt-6-sol` high code review. A bad gate
gets a RED gate test or reproducible failing invocation before a narrowly
reviewed correction. Once the failure is fixed, re-run affected cells, then
the integrated final gate on the final source. If an environment prerequisite
is unavailable, record `UNAVAILABLE` with the exact attempted command and
host fact; do not relabel it PASS. A source change, artifact rebuild, or oracle
retirement invalidates only receipts whose inputs changed, but final claims
must still refer to the final commit and tree.

The Slice 150 status matrix maps each AC to receipt paths and hashes, review
verdicts, and remaining limits. Release state and generated board views are
updated only after qualification is genuinely complete. Neither this design
nor a green local gate authorizes publication.
