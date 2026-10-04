---
title: FathomDB 0.8.27 Slice 103 — housekeeping and consistency
status: PLANNED
target_release: 0.8.27
planning_baseline: 9f8ee4509d2b3dee1667ae091119ceaa30bbca6e
---

# Slice 103 — housekeeping and consistency

Insert this bounded slice after completed Slice 100 and before Slice 110. It
preserves the late 0.8.26 Tegra build work in the 0.8.27 candidate and checks
whether historical Windows `erase_source` and `purge` failures persist. The
owner subsequently approved a doctor/recover route for owed physical erasures
within this slice. The Tegra, Windows, and recovery tracks share a closeout but
have separate behavioral tests and evidence. Publication is not authorized.

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
artifacts. Do not add the ignored data to this repository. The index records
1,154 `wal_checkpoint` error occurrences across FathomDB 0.8.22–0.8.26 logs:
1,024 for `erase_source` and 130 for `purge`. These are log occurrences, not independent
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
The tracked reproducer counts any `ErasureIncompleteError` and does not itself
assert the stage, row survivors, physical WAL state, retry outcome, or wheel
identity. Strengthen a separate 0.8.27 copy without changing the historical
script or treating its narrative as an oracle.

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
| R27-103F | The three tracks join into one verified candidate before the next refactor. | AC27-103F: each track has verified branch commits, a durable output witness, an evidence receipt, and independent review, including a reviewed no-production-code Windows disposition if warranted. After all reviewed branches are integrated, rebuild and install the Tegra and Windows wheels from one final 0.8.27 source SHA; bind their hashes, focused platform tests, relevant Linux/SDK regressions, the required repository gate, combined-diff review, and read-only verification to that SHA. No unresolved Slice 103 requirement is deferred into Slice 110 or the final release gate. |
| R27-103G | Operators can detect and complete owed physical erasures without knowing the original purge or erase argument. | AC27-103G: `doctor check-integrity` emits a read-only `E_ERASURE_INCOMPLETE` finding per owed `purged`/`source_erased` closure with opaque id, cause, phase, blocker, sequence, WAL frames, a documentation anchor, and a remediation argv that round-trips through the real parser with the original database path. Source identity is redacted. `recover --accept-data-loss --complete-erasures <db_path>` requires the existing acknowledgement, takes the canonical lock, uses the `--truncate-wal` admission rules and one recovery-only read-write connection, validates physical zero, completes telemetry redaction only with the correct attached sink, truncates the WAL, and marks only fully discharged closures complete. Missing sink or failed validation leaves the closure owed and exits 70; open-store or checkpoint BUSY exits 71; completed recovery exits 64. A successor ADR and `dev/interfaces/cli.md` plus `dev/design/recovery.md` updates land with the implementation. RED tests precede code, including a governed blocked-store fixture and command round-trip. |

`dev/acceptance.md` remains locked; these IDs are release-local.

## Parallel tracks and join

After step 1, steps 2 and 3 run concurrently as independent subagent briefs;
neither waits for the other. The owner-approved recovery addition in step 4
starts from the advanced release HEAD in a third worktree. Step 5 starts only
when all three tracks have returned reviewable results.

1. **Commission both tracks from one baseline.** The release coordinator
   records the live `release/0.8.27` HEAD at commissioning, verifies the
   release state, and prepares separate Tegra and Windows briefs and worktrees
   at that exact commit. Run
   `scripts/preflight.sh --worktree <path> --expect-closed 100 --plan dev/plans/plan-0.8.27.md`
   on each before its implementer starts. The two implementer subagents may
   work at the same time, each writing only in its own checkout and producing
   its own branch, track-local evidence receipt, and structured output witness. They do not
   edit the shared Slice 103 plan, release board, or release-state JSON. The
   release checkout has one integration and state writer. Keep build outputs
   and virtual environments isolated; do not run `pip install -e` or
   `maturin develop` from an implementer worktree and rebind the shared main
   environment. Coordinate use of any shared physical test host without
   serializing the two implementation tracks. After source, test, or script
   edits, each track runs the full `./scripts/agent-verify.sh` from its own
   prepared checkout with an isolated virtual environment and installed wheel;
   a missing toolchain or failed preflight is reported as a blocker, never a
   pass. Retain the exact exit and diagnostics for review.

2. **Track T — Tegra continuity (R27-103A–C).** This implementer inventories
   the exact five post-publication 0.8.26 commits against the frozen 0.8.27
   baseline and records every changed file's carry/adapt/exclude disposition.
   It owns the Candle/CUDA dependency and lockfile changes, Tegra artifact and
   installed-wheel checks, guarded Pages route, associated tests, and
   0.8.27-aware runbook. Review the existing driverless CPU contract and add
   RED tests before porting behavior. Reconcile all four Candle patches,
   x86_64 and AArch64 CUDA routes, and 0.8.27-only gate registrations. Build
   and inspect a branch-local `0.8.27+tegra` wheel on the classic Jetson route;
   record source SHA, toolchain, wheel hash, symbols/dependencies, clean-venv
   import outside the source tree, and CPU/auto/forced-CUDA allocation smokes.
   This receipt proves the track branch, not the later integrated candidate.
   The Tegra track leaves engine erasure behavior to Track W.

3. **Track W — Windows WAL and erasure (R27-103D–E).** This implementer reads
   the Memex `windows-portability` reproducer at `d92610c5`, owner account,
   and ignored local VM/CI evidence read-only. Inventory hashes, provenance,
   and missing receipts; keep the 11-versus-12 discrepancy explicit unless
   the extra trial is recovered. Freeze the outcome classes and bounded trial
   protocol before running it. Use the unchanged 0.8.26 registry wheel as a
   historical control, then build and install a hashed 0.8.27 Windows wheel
   from this track's frozen-baseline branch. Run at least the original
   40/40/20 no-read/read/page matrix on 0.8.26, followed by three such batches
   on 0.8.27, with Linux read/page controls. Add focused `purge` and operator
   `excise_source` scenarios with their distinct argument and closure contracts.
   Any candidate WAL refusal establishes persistence; zero refusals without a
   mechanism witness means only "not reproduced within this sample." If the
   historical control also fails to reproduce, classify the comparison
   inconclusive.

   The Windows oracle uses a real database and public operations, recording
   typed stage/detail, BUSY/frame counts, reader/connection state, committed
   rows and WAL bytes before/after, exact write fence, same-engine and reopen
   retry, and zero-count completion. Do not perturb the WAL with a raw
   pre-erase checkpoint. A typed five-attempt BUSY refusal can be correct
   fail-closed behavior; demonstrate the contract violation before changing
   production code. If it persists, isolate the holder or lock, commit a RED
   Windows test, review any external patch, then make the smallest fix while
   preserving Slice 20/50 atomicity and proof-row exceptions. Keep test files
   fixed during fix-to-spec. A public contract change requires a successor ADR
   and interface update. If no production fix is supported, commit the
   candidate reproducer/tests, return a findings receipt and no-production-code
   disposition in the output witness, and have that disposition reviewed.
   The Windows track leaves CUDA dependencies, the lockfile, and Pages
   publication tooling to Track T; it flags any shared test-runner edit for
   integration review.

4. **Track R — owed-erasure diagnosis and recovery (R27-103G).** Use a
   worktree from the advanced `release/0.8.27` HEAD, separate from Track W's
   frozen baseline. Record the owner's approval in a successor dependency
   closure ADR before changing the public contract. First commit failing
   real-database and parser round-trip tests. Implement the doctor finding and
   offline recover action in the existing CLI roots, sharing physical-zero
   validation and erasure-at-rest logic without opening an Engine reader pool
   or projection runtime. Keep telemetry obligations owed when the correct
   sink is unavailable. Update the CLI interface and recovery design with the
   exact finding, report, and exit behavior. Produce a branch receipt and
   witness, run the full repository verifier, and obtain independent review.

5. **Join and close (R27-103F).** Gate each branch with its own independent
   code review and verify its commit(s), output witness, tests, and evidence
   from git. Cherry-pick reviewed commits into the release checkout one track
   at a time. If a later track conflicts with the advanced release HEAD,
   commission an implementer fix branch from that HEAD for source/test changes;
   the coordinator does not edit an implementer's worktree or resolve its code
   by hand. Review the resolution and combined diff. Branch-local platform
   receipts remain provisional. From one final integrated code commit SHA,
   rebuild and install the Tegra and Windows wheels and repeat their required
   platform checks with exact wheel hashes. Run affected Linux/SDK regressions,
   `./scripts/agent-verify.sh`, and the full release gate required by the
   dependency and packaging changes. Independently review the combined diff
   and verify the same code SHA read-only. The coordinator records all track
   dispositions, target receipts, test exits, and remaining risks in Slice 103
   status, then advances release state to Slice 110 only when every acceptance
   row passes. Record both the qualified code SHA and the later status/state
   bookkeeping SHA; verify that the latter changes no source, tests, scripts,
   dependencies, or build inputs. Any such change requires requalification.
   Pages dispatch, tag, push, registry write, and public deployment retain
   their separate release authorization.

The owner's approved recovery addition is implemented in its own worktree from
the advanced release HEAD. It must pass independent review and the full
repository verifier before joining the release checkout. `R27-103F` closes
only after this third track and the Windows path result are integrated and the
final platform artifacts are rebuilt from one code SHA. Automatic completion
at `Engine::open` is not part of `R27-103G`; it requires its own failing test
and contract decision if the Windows evidence makes it necessary.

The already-published `v0.8.26` tag and generic artifacts remain historical
evidence. Slice 103 does not reissue them or declare `0.8.27+tegra` published.
