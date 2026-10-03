---
title: FathomDB 0.8.27 Slice 103 — housekeeping and consistency
status: PLANNED
target_release: 0.8.27
planning_baseline: 9f8ee4509d2b3dee1667ae091119ceaa30bbca6e
---

# Slice 103 — housekeeping and consistency

Insert this bounded slice after completed Slice 100 and before Slice 110. It
preserves the late 0.8.26 Tegra build work in the 0.8.27 candidate and checks
whether historical Windows `erase_source` and `purge` failures persist. This
plan commissions neither a source change nor publication. The two tracks share
an intake and closeout but have separate behavioral tests and evidence.

## Entry evidence and change since the release plan

The published 0.8.26 branch was recorded at `b35212f678626e09ea7293e00ee337288ce74ff1`
on 2026-09-19. Five subsequent commits landed on
`origin/release/0.8.26` on 2026-10-03, through
`8c4fdfa9bb263552d7ffc1c019bc553169e2d266`:

| Commit | Reviewed change | 0.8.27 disposition to prove |
| --- | --- | --- |
| [`1a131e78`](https://github.com/fathomadb/fathomdb/commit/1a131e780de6d92041d0def46b4fbfb2865324fa) | Moves all four Candle patches and the CUDA contract to `1aefdd008ad1c994635b688b8e6f2ae5a5a920ae`; rejects unresolved CUDA runtime symbols and dynamic CUDA/NVIDIA dependencies; installs and imports the wheel without CUDA library search paths. | Carry the build, dependency, and artifact checks forward with a coherent lockfile and current 0.8.27 contracts. |
| [`600c598c`](https://github.com/fathomadb/fathomdb/commit/600c598ce8056bb99485fe4e9c3c7a2085d49c93) | Retains an independent review transcript of the first artifact change. | Use as historical review evidence only; it predates the later fixes and cannot qualify the 0.8.27 candidate. |
| [`9c7c3da7`](https://github.com/fathomadb/fathomdb/commit/9c7c3da71c20e660210effca95209dcf61583b71) | Changes to the isolated venv before import so the check loads the installed wheel rather than the staged source tree. | Preserve this provenance check and its negative fixture. |
| [`91b28587`](https://github.com/fathomadb/fathomdb/commit/91b28587ceb892279b348893ea587ad5006a8b89) | Adds the exact-head Tegra Pages publisher, operator test, and public runbook; also updates 0.8.26 release documentation. | Carry forward the reusable guarded route and adapt its tests/docs to the current published version and an eventual 0.8.27 candidate. Do not copy 0.8.26 publication claims into unreleased 0.8.27. |
| [`8c4fdfa9`](https://github.com/fathomadb/fathomdb/commit/8c4fdfa9bb263552d7ffc1c019bc553169e2d266) | Removes an unsupported `gh run list --event` filter and guards that interface. | Include this correction in the retained operator path. |

At the planning baseline, 0.8.27 still pins the earlier Candle revision
`cf02edbc2ade01b4da42715e9e2a8f0364e5dcee` and lacks the new Tegra
artifact/import checks and publisher. Its lockfile, test runner, and release
docs have diverged from 0.8.26. Port the reviewed behavior file by file;
do not merge or cherry-pick the whole post-publication branch. Preserve the
0.8.27-only gate registrations when adding the Tegra operator test. The source
diff is evidence of code, not proof that a Pages deployment or 0.8.27 wheel
has run.

The owner supplied the text of a
`https://claude.ai/artifact/RFa3nzD3CgS5cR545DuF3i` Windows WAL account
after the browser required sign-in. The account describes failures on the
**published 0.8.26 registry wheel**, not a measured 0.8.27 failure. The
separate Memex `windows-portability` worktree at `d92610c5` contains its
tracked FathomDB-only reproducer, draft upstream issue, and W12 verification
under `dev/plans/0.6.0/features/windows-portability/`. Those are available
read-only as consumer evidence, not a FathomDB fix or merge source. The
retained raw evidence was subsequently located in the ignored local directory
`~/projects/memex/data/windows-erase-wal-evidence/`. Its `README.md`,
`FINDINGS-0.8.26-windows-erase-wal-checkpoint.md`, `ei-digest.tsv`, and
`ei-failing-tests.tsv` index 507 retained `.log` paths, including 98 hard-link
duplicates, October Windows VM runs, and earlier FathomDB attribution
artifacts. Do not add the
ignored data to this repository. The index records 1,154 `wal_checkpoint`
error occurrences across FathomDB 0.8.22–0.8.26 logs: 1,024 for
`erase_source` and 130 for `purge`. These are log occurrences, not independent
trials or a measured failure rate. Some older per-attempt VM artifacts expired
or were removed; verify the retained receipts and exact wheel provenance before
binding historical measurements. Only `erase_source` has a focused
FathomDB-only reproducer. The retained consumer CI logs also show `purge`
failures; the clearest retained `purge` receipt is from 0.8.22, while the
focused 0.8.26 VM study exercised `erase_source`. Operator `excise_source`
has no incident-shaped spontaneous-failure reproducer yet. In the evidence
directory, `local-vm/2026-10-03-fixes-and-reopen/vm-full2.log` shows the
installed 0.8.26 package and a WAL BUSY failure at lines 63 and 335, followed
by repeated same-engine failures. The retained
`key-evidence/representative-ci-logs/github-actions_memex_run-32031624116-job-95393023450-Test-windows-amd64.log`
shows the 0.8.22 package and `purge` BUSY at lines 231 and 818–822. No exact
0.8.26 wheel SHA256 was found in the retained evidence.

The owner-reported 0.8.26 reproducer opened a fresh database for each trial,
wrote 50 `src-a` and one `src-b` Note, optionally did a materialized read or
canonical page, then erased `src-a`. It reported Windows `wal_checkpoint`
refusals in 2/40 no-read, 5/40 read, and 2/20 page trials, versus 0/40 Linux
read and 0/40 Linux page trials. The account reports 0/11 stuck erasures
recovered by same-engine retry over up to 40 seconds, 8/8 recovered by
close/reopen, and one later W12 run with renewed BUSY after reopening. The
five rows in the retained findings table sum to 11 first failures; a Memex
draft issue says 12 without a surviving raw trial output to substantiate the
extra case. Keep 0/11 as the documented count and flag 0/12 as unverified.
The tracked reproducer counts any `ErasureIncompleteError` and
does not itself assert the stage, row survivors, physical WAL state, retry
outcome, or wheel identity. Strengthen a separate 0.8.27 copy without changing
the historical script or treating its narrative as an oracle.

The historical [0.8.23 Slice 65 status](../../../runs/STATUS-0.8.23.md)
closed Windows WAL attribution as **UNATTRIBUTED / NO REMEDY** after both clean
erases and typed WAL-checkpoint refusals. Its
[investigation](../../../../design/0.8.23-wal-attribution-investigation.md)
found that raw checkpoints inserted before the erase changed the WAL state.
Observe the actual operation and its existing checkpoint attempts; a typed
post-commit refusal alone does not justify changing retries, reader lifetimes,
or production behavior.

The artifact proposes connection quiescence, pending-closure discovery or
automatic completion, narrower write fencing, a retry-completion report, and
corrected error guidance. The WAL holder, a busy prepared statement, Windows
`-shm` locking, and startup activity are **hypotheses**, not findings. Pending
closure identity, write fencing, and report shape are governed by the
[dependency closure ADR](../../../../adr/ADR-0.8.25-dependency-lifecycle-closure.md)
and public interfaces; changing them requires a successor ADR and interface
update.
Diagnostic wording may be a narrower correction, but it does not resolve the
underlying checkpoint behavior.

## Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-103A | The 0.8.26 post-publication Tegra work is accounted for. | AC27-103A: an exact five-commit inventory maps each changed file to carried behavior, version-specific adaptation, or justified historical-only exclusion; the four Candle patch revisions, lockfile sources, and CUDA contract agree. |
| R27-103B | A 0.8.27 Tegra wheel is buildable and installable on the supported classic Jetson target without losing the driverless CPU path. | AC27-103B: an exact-candidate host-native `0.8.27+tegra` wheel passes the declared glibc floor, no unresolved CUDA runtime symbols or dynamic CUDA/NVIDIA dependency, clean-venv installed import outside the source tree with CUDA library paths removed, and installed CPU/auto/forced-CUDA smokes with an in-process allocation witness. A source-tree import or a 0.8.26 artifact is not a pass. |
| R27-103C | The Tegra Pages operator route remains guarded and version truthful. | AC27-103C: local tests reject an unsupported `gh run list` flag, a mismatched remote SHA/version, and an unverified workflow result; the default route requires an installed Pages smoke, while an explicit skip cannot count as completed publication. The runbook distinguishes the 0.8.26 post-release route from an unpublished 0.8.27 candidate. Publication remains separately authorized. |
| R27-103D | The reported 0.8.22–0.8.26 failures are compared with the actual 0.8.27 candidate before they are called current defects. | AC27-103D: inventory the retained local logs and verify the 0.8.26 registry wheel identity; run the same real-database no-read/read/page `erase_source` scenarios on an exact 0.8.27 installed-wheel SHA/hash with bounded repeated Windows and Linux controls. Add focused `purge` and operator `excise_source` controls using the historical CI shapes without treating log occurrences as trial counts. Record each public verb/binding, typed stage/detail, checkpoint BUSY/frame counts, reader/connection state, committed rows and WAL bytes before/after, same-engine and reopen retry, exact write fence, and zero-count completion. Observe the real erase checkpoint without a raw pre-erase checkpoint. Classify persistence, nonreproduction, or inconclusive evidence; a clean 40-trial batch alone cannot prove absence. |
| R27-103E | Any failure that persists on 0.8.27 is attributed and handled without weakening erasure guarantees. | AC27-103E: if AC27-103D shows a current violation or sticky same-engine BUSY, isolate the holder or lock with controlled tests before selecting a remedy; commit a genuine failing Windows test before the smallest fix, then prove exact deletion/survivors, durable retry, independent-reopen physical absence, and truthful errors/reports. If 0.8.27 does not reproduce with adequate evidence, record a reviewed no-code disposition and its limits. An unresolved persistent store-wide write block prevents Slice 103 closeout. Any public closure-discovery, auto-finish, write-fence, or report change needs a separately accepted successor ADR and interface update. |
| R27-103F | The next refactor starts from a verified candidate. | AC27-103F: focused Tegra and Windows tests, relevant Linux/SDK regressions, the required repository gate, independent code review, and read-only verification bind to the final 0.8.27 source SHA; no unresolved Slice 103 requirement is deferred into Slice 110 or the final release gate. |

`dev/acceptance.md` remains locked; these IDs are release-local.

## Coordination

Run Tegra continuity and Windows erasure diagnosis as separate workstreams
from the same verified 0.8.27 baseline. Each file-mutating implementer gets
an isolated worktree and review; the release checkout remains the single
integration and release-state writer. The Windows workstream first examines
the retained 0.8.26-and-earlier evidence, then measures an exact 0.8.27
candidate before proposing a fix. The Tegra workstream ports the five
post-publication commits by behavior and proves a new 0.8.27 artifact. Merge
only reviewed changes, then rerun the combined platform and repository gates
at one final candidate SHA before closing Slice 103.

## Execution order

1. **Freeze intake.** Record the 0.8.27 entry SHA and fetch/verify the exact
   0.8.26 five-commit range. Save a changed-file disposition against the live
   0.8.27 tree, including lockfile and test-runner conflicts. Obtain the
   owner-pasted Windows account and Memex `windows-portability` repro and
   verification at `d92610c5`; inventory the ignored local evidence directory
   above, its retained VM/CI logs, wheel identities, and any off-repo patch,
   recording hashes and provenance. Retain the 11-versus-12 discrepancy as an
   explicit evidence limitation unless the missing trial receipt is recovered.
2. **Design and RED.** Review the Candle change and existing Tegra/driverless
   constraints before changing the pin. Add or adapt tests that fail for the
   missing static-runtime, installed-wheel, publisher, and Windows behavior.
   Run the unchanged 0.8.26 reproducer against its registry wheel as a
   historical control, then adapt a separate candidate test to assert the
   exact stage, physical/row effects, retry and store-wide write-fence state.
   Compare the current 0.8.27 installed wheel before selecting a correction.
   Freeze a bounded trial protocol before observing results: at least the
   original 40/40/20 Windows no-read/read/page matrix on the 0.8.26 wheel,
   followed by three such batches on the exact 0.8.27 wheel; retain the Linux
   read/page controls. Any candidate WAL refusal establishes persistence.
   Zero candidate refusals without a direct mechanism witness means "not
   reproduced within this sample," never "proved fixed"; if the 0.8.26
   control also fails to reproduce, classify the comparison inconclusive.
   Add `purge` and operator `excise_source` scenarios after the focused
   `erase_source` control, preserving their distinct argument and closure
   contracts. The Windows oracle uses a real database and the public operation,
   with outcome classes and post-commit effects specified before a fix is chosen.
   A five-attempt WAL BUSY refusal with durable deletion and retry obligation
   may be expected behavior; require a demonstrated contract violation before
   authorizing a remedy. Do not perturb the WAL with pre-erase raw checkpoints.
   Keep test files fixed during fix-to-spec.
3. **Integrate narrow changes.** Port Tegra checks and guarded publication
   tooling with 0.8.27-aware documentation. Reconcile all four Candle patches,
   lock sources, CUDA checker, pinned-override tests, and both x86_64 and
   AArch64 CUDA routes. Review any external Windows patch independently,
   then implement only behavior needed to make a reproduced 0.8.27 RED case
   green. If its changes overlap Slice 20/50 erasure owners, preserve their atomicity and
   proof-row exceptions and update accepted interfaces or ADRs for any public
   contract change.
4. **Prove on targets.** Run contract mutation/refusal tests and build the
   `0.8.27+tegra` wheel on the classic Jetson route; retain exact source,
   toolchain, wheel hash, symbol/dependency inspection, installed import,
   CPU/auto/CUDA and allocation-witness receipts. Run the Windows reproducer
   and affected Rust/Python/Node routes on the standing Windows VM or another
   named supported host, with exact candidate hash and unmodified failure
   diagnostics. Then run affected Linux regressions and
   `./scripts/agent-verify.sh`; use the full release gate where required by
   the changed dependency and packaging surface.
5. **Review and close.** Independently review the combined diff and verify
   the exact candidate. Record each carried/excluded 0.8.26 file, Windows
   diagnosis and disposition, test exits, platform receipts, and remaining
   risks in Slice 103 status. Advance release state to Slice 110 only when all
   acceptance rows pass. Keep any Pages dispatch, tag, push, registry write,
   or public deployment behind the release's separate authorization.

The already-published `v0.8.26` tag and generic artifacts remain historical
evidence. Slice 103 does not reissue them or declare `0.8.27+tegra` published.
