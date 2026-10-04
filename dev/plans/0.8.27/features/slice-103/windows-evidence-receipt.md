# Slice 103 Track W preparatory evidence receipt

Status on 2026-10-03: **R27-103D and R27-103E remain OPEN.** The Windows
release-branch candidate has not been built or measured. Transfer of the exact
source archive and the tracked Memex reproducer to the Windows VM awaits
direct owner approval. This receipt preserves completed controls and the
reviewed test/probe work; it is not a Windows candidate qualification.

## Source and historical artifact identity

- Frozen release-branch source baseline:
  `8fb6fdac9d2f1655a62355e0ca8af51d42dd1fb7` on `release/0.8.27`.
  Track W's reviewed preparatory HEAD is
  `00e27eafa9da613a17c7922b822c814ba05f0e9c`. The track commits change
  the protocol, probe, and its tests, with no Rust production change. The
  baseline package metadata still declares `0.8.26`; a branch-local wheel
  must be identified by source and artifact hash, not called a released
  `0.8.27` wheel. The integrated final-SHA platform qualification is separate.
- Published historical registry wheel:
  `fathomdb-0.8.26-cp310-abi3-win_amd64.whl`, SHA256
  `7a94e26f78b040ce95aa79ada69ffac6f6d369280f300ec15da0c713d7bd5a1c`.
  The hash matches the [PyPI 0.8.26 release record](https://pypi.org/pypi/fathomdb/0.8.26/json).
  It was installed in `C:/ci/s103w/venv026` on Windows 11 10.0.26200 with
  Python 3.12.10. Import resolved to that environment's `Lib/site-packages`;
  installed `_fathomdb.pyd` SHA256 was
  `afb2f4ff6ede39c2b46716be836b2ab3ca785c8c42a71e1c09ebbdd8263d1569`.
- Exact tracked Memex `d92610c5` reproducer was materialized locally as
  `/tmp/fathomdb-s103-historical-repro.py`, SHA256
  `fb3731ba86fb8b7560d5e349b9d5855b965def157f4f1e5157934462c267bb73`.
  The already-present VM copy at
  `C:/ci/dev/memex/dev/plans/0.6.0/features/windows-portability/w12_fathomdb_erase_repro.py`
  has SHA256
  `f42de8f282d9d3fb919635a0e38eec3357aa779444d8b82d6c9d3a15b0fd2f2c`.
  Its executable body/control flow matches the tracked script; explanatory
  docstring and line endings differ. The fresh Windows control below used
  that prepositioned unchanged VM copy. The exact tracked-script Windows
  rerun remains pending transfer approval.

## Completed real-database controls

The prepositioned historical script used a fresh database per trial, 50
anonymous `src-a` Note rows and one `src-b` row, an optional fully
materialized read or page, then public Python `erase_source`. Its first
refusal counts were:

| Windows 0.8.26 mode | First typed WAL refusals | VM log and SHA256 |
| --- | ---: | --- |
| No read | 3/40 | `C:/ci/s103w/hist026-noread.log` · `443bcab0856f8ec33cfb978c9675ef6855a3cc3db2627c4bff141bc77c8f7d14` |
| Read | 3/40 | `C:/ci/s103w/hist026-read.log` · `71e5eed465ce49fa949e9b0577ce25dbe1fe649968233399fec1e23434d7f726` |
| Page | 2/20 | `C:/ci/s103w/hist026-page.log` · `bb8036187fe49888b37dff18f336959dc6d90f47866641d7c6eae8c963bf8b80` |

These are **8/100 independent trials**, not a count of repeated log lines.
The retained earlier findings support 11 first failures across their own
runs; a separate draft claims 12 but has no preserved twelfth raw trial. Its
1,154 log occurrences are not independent trials.

A separate timing-sensitive 0.8.26 diagnostic arm found 1/40 no-read
failures in `C:/ci/s103w/hist026-diagnostic-noread.jsonl` (SHA256
`72550e6036cb032f6bbba940f39657bd66263f75fdec8c20cf4a9af3430afee0`).
On that failure, `src-a` canonical rows were gone, `src-b` survived, WAL grew
from 1,273,112 to 1,421,432 bytes, and the typed stage was
`wal_checkpoint` with 345 frames reported. The same Engine refused retries
after 0.05, 0.25, 1, and 2 seconds; close/reopen then completed the exact
retry with `nodes_excised=0`, and a second completion also reported zero.
No physical dependency closure row was present in this simple fixture, so
an unrelated write was permitted; that is not a write-fence violation.
The diagnostic arm takes external read-only observations after the operation
and is excluded from rate comparisons.

On Linux, an installed wheel built from the frozen source baseline had SHA256
`57c98d7698ff60654f3504f6e8476df62128f91b5622f693679c3177538eb477`;
its installed native module SHA256 was
`2af33aaedcb66a84e50dcaa56fedab16ed6119233c7bd2eb1accde688a7bc8e0`.
The exact tracked Memex script gave 0/40 first refusals for read and 0/40
for page (`/tmp/fathomdb-s103w-linux-hist-read.log`, SHA256
`d8cc55820b73cf66d3632f1d0b3bbb5f8cc98af756cef5a53b3ff4d45b21e432`;
`/tmp/fathomdb-s103w-linux-hist-page.log`, SHA256
`fa83232672a2db98c74163995c87d3fd88cbe9f35fcd91cbee2c2bfc28bf03e5`).
The reviewed candidate probe's default retry arm gave 0/40 first refusals
and 0/40 assertion failures for each Linux mode:

| Linux branch-source mode | Result | Local JSONL SHA256 |
| --- | ---: | --- |
| Read | 40/40 passed | `/tmp/fathomdb-s103w-linux-probe-v2-read.jsonl` · `15b68f8bc594711e471075c28fd7161a723982508a933d422475c4fd7a380aef` |
| Page | 40/40 passed | `/tmp/fathomdb-s103w-linux-probe-v2-page.jsonl` · `b85d848a63bbcfdb8b4503bc9708c9de8e53ed28904b72b202b446d4ac61cca7` |

Focused Linux governed deleted-first `purge` completed 20/20 trials with
zero refusals and only the `src-b` survivor
(`/tmp/fathomdb-s103w-linux-purge.jsonl`, SHA256
`2779db5a035307622f6a73bd81f6dccd5831cc495f093ef2e01e2dcc1208bdc8`).
The branch-local Rust operator `recover --accept-data-loss --excise-source
src-a --json` returned exit 64 with one node excised; an exact repeat
returned exit 64 with zero nodes excised. The `src-b` row survived and WAL
was zero. Windows focused `purge` and operator controls remain pending.

## Probe, review, and verification

- `bc0e77632` froze the protocol before measurements; `f80fa16c3`
  committed the initial probe. Independent review identified timing and
  outcome-oracle defects. `21c911582` committed five genuinely RED contract
  tests (all failed because `record_passes` was absent); `31a3d99ce` made
  them GREEN and separated unperturbed retry from timing-sensitive
  diagnostics. `00e27eafa` moved the same five tests into the ordinary
  Python suite. Independent re-review passed this HEAD.
- Current probe SHA256:
  `9f9a5e39a9590f447df4406dc4ebc9d71c4f785ca98dd3d89fac496db113a129`.
  The default `retry` arm makes no SQLite read or write between the first
  refusal and same-engine/reopen retries; the `diagnostic` arm records rows,
  owed closures, and the write-fence result, explicitly changing timing.
  Neither arm issues a raw checkpoint. An unresolved completion, absent or
  nonzero second completion, missing survivor, retained `src-a` row,
  post-completion write failure, or pending physical closure after completion
  gives a failing process exit. A write refusal is asserted only when an owed
  physical closure actually exists.
- The required clean-HEAD full `./scripts/agent-verify.sh` on
  `00e27eafa9da613a17c7922b822c814ba05f0e9c` exited **0**. It reported
  `agent-test.sh: 178/180 suites passed (skipped=2 excluded=0)` and security
  `0 violation(s), 0 blocker(s), 0 downgrade(s)`, including strict AC-036/037.
  Full log: `/tmp/fathomdb-s103w-agent-verify-probe-v3.log`, SHA256
  `170278bb3b4818ae2217bcc06e23222252c2e4154171d2c081513c10b233ebec`.
  The process-local environment set `PYTHONPATH=src/python`,
  `CARGO_TARGET_DIR=<Track W worktree>/target`, `CARGO_INCREMENTAL=0`, and
  `core.excludesFile=/tmp/fathomdb-s103w-git-excludes` containing only
  `target/`; the checkout's own installed, non-editable wheel environment
  was used. The gate ran unconfined for ptrace-dependent AC-036. The
  process-local exclude is necessary because the 21 GB disposable target is
  untracked; tracked source was independently clean.

## Open attribution and next measurement

FathomDB canonicalizes local Windows database paths, yielding verbatim drive
syntax such as `\\?\C:\...`. Bundled SQLite 3.53.2's
[`winIsUNCPath`](https://raw.githubusercontent.com/sqlite/sqlite/version-3.53.2/src/os_win.c)
classifies any leading double backslash as UNC and selects shared WAL SHM
locking. [SQLite 3.53.3](https://www.sqlite.org/releaselog/3_53_3.html)
changes that classification for verbatim local-drive paths while retaining
genuine UNC behavior. The owner's standalone SQLite VM race supports this
mechanism, but the exact installed FathomDB Windows candidate and Engine lock
holder have **not** yet been measured. This remains a hypothesis, not an
attributed Track W finding or authority for a production change.

Pending approval is for the exact frozen-source archive SHA256
`cd54328dd89fd807f09e4708e1e6f339882e012c576ce5d5b4773df15065ebe1`,
tracked Memex reproducer SHA256
`fb3731ba86fb8b7560d5e349b9d5855b965def157f4f1e5157934462c267bb73`,
and reviewed probe SHA256
`9f9a5e39a9590f447df4406dc4ebc9d71c4f785ca98dd3d89fac496db113a129`
to `gh-runner-wonl-win11:C:/ci/s103w/` for local trials and cleanup. Once
authorized, verify received hashes, build/install the exact branch source
wheel, run three 40/40/20 Windows batches plus focused verbs, record the
operation's actual checkpoint observation and managed connection state in a
separate test-only arm, then decide R27-103D/E from those results. No
production-code disposition is made in this preparatory receipt.
