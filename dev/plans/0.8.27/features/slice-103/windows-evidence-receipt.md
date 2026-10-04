# Slice 103 Track W Windows erasure evidence

Status on 2026-10-03: the Windows defect reproduced, the local-drive fix
is implemented, and an installed **0.8.27** Windows wheel from integrated code
SHA `f148aca86d4149e11b85d146b969064f4683677c` passed final platform
controls. The frozen and fixed branch wheels carry metadata `0.8.26` and are
identified only by hashes. Cross-platform Slice closeout and the integrated
repository gate remain the release orchestrator's responsibility.

## Identity and historical control

- Frozen release source: `8fb6fdac9d2f1655a62355e0ca8af51d42dd1fb7`.
  Exact transferred source archive SHA256:
  `cd54328dd89fd807f09e4708e1e6f339882e012c576ce5d5b4773df15065ebe1`.
  VM trial root: `gh-runner-wonl-win11:C:/ci/s103w/`.
- Official PyPI `fathomdb-0.8.26-cp310-abi3-win_amd64.whl` SHA256:
  `7a94e26f78b040ce95aa79ada69ffac6f6d369280f300ec15da0c713d7bd5a1c`,
  matching the [registry release record](https://pypi.org/pypi/fathomdb/0.8.26/json).
  Fresh installed native module SHA256:
  `afb2f4ff6ede39c2b46716be836b2ab3ca785c8c42a71e1c09ebbdd8263d1569`.
  Exact tracked Memex `d92610c5` reproducer SHA256:
  `fb3731ba86fb8b7560d5e349b9d5855b965def157f4f1e5157934462c267bb73`.
- Unchanged exact-script Windows control: first typed `wal_checkpoint`
  refusals **4/40 no-read, 1/40 read, 1/20 page = 6/100** independent fresh
  databases. VM logs `hist026-exact-{noread,read,page}.log` SHA256 respectively
  `2c02f07d8e22d19dada7966b14a4681530c69ace37d5784d0177439cee699c28`,
  `685003df4d1763e45abc927b58f8b367973f197194229e5f6933a251e1354e13`,
  `6bc14448f41f5ca02b8a7fdc9467e542e9c7796fc58d92f0162bf9f0fc783097`.
  A separately prepositioned script with the same executable flow but changed
  docstring/line endings yielded 8/100; it is not the exact-script control.
  Retained earlier Memex evidence supports 11 separate first failures; a
  proposed twelfth lacks a preserved trial. Log occurrences are not trials.

## Frozen branch-source comparison

The unmodified branch-source installed Windows wheel SHA256 was
`bc9bc2706a3d02bb08e4f88ed90a1ff506d6dff7e1dd3d404f62689b59ab885c`.
Three fresh 40/40/20 exact-script batches yielded first refusals:

| Batch | No-read | Read | Page | Total |
| --- | ---: | ---: | ---: | ---: |
| 1 | 1/40 | 2/40 | 3/20 | 6/100 |
| 2 | 0/40 | 4/40 | 1/20 | 5/100 |
| 3 | 3/40 | 0/40 | 1/20 | 4/100 |

Thus **15/300** branch-source Windows operations refused a completed
checkpoint; the symptom persisted. VM logs are `candidate-b{1,2,3}-{noread,read,page}.log`.
The reviewed, separately run retry arm found one read trial still BUSY after
five same-engine attempts and after close/reopen, with three WAL frames on the
reopened refusal. It also found ten post-success second-call BUSY outcomes.
These are distinct from first refusals and from write-fence findings. Its
`probe-retry-{noread,read,page}.jsonl` SHA256 values are
`62411d09a81752a0f6bfd01982d7c6b7bc9f23707fe569344e23ccde49597767`,
`929bdd4d9663dfbe4e7c6c36491a2efc6151a441988d7b9685f37eb864153be2`,
`b750e888559a65d7d4f31fe54f3b291529fa5ab304a82ebee2c81818d7d9c224`.
The diagnostic arm samples rows, owed closures, WAL, and write fence only after
the first operation and is excluded from rate comparison. No raw pre-erase
checkpoint was issued. The simple anonymous-Note fixture can legitimately
lack an owed physical closure; write permission there is not a fence breach.

Linux exact historical read/page controls were 0/40 and 0/40. The frozen
branch-source installed Linux wheel SHA256 was
`57c98d7698ff60654f3504f6e8476df62128f91b5622f693679c3177538eb477`;
reviewed retry probe read/page controls passed 40/40 each. Governed deleted-first
`purge` passed 20/20. Operator `recover --accept-data-loss --excise-source
src-a --json` returned exit 64/count 1, then exit 64/count 0; survivor only
and WAL zero.

## Cause, RED tests, and remedy

Engine canonicalization supplies local Windows `\\?\C:\...` paths to bundled
SQLite 3.53.2. Its
[`winIsUNCPath`](https://raw.githubusercontent.com/sqlite/sqlite/version-3.53.2/src/os_win.c)
misclassifies that syntax as UNC, selecting the shared WAL lock handle. The
[SQLite 3.53.3 correction](https://raw.githubusercontent.com/sqlite/sqlite/version-3.53.3/src/os_win.c)
excludes local verbatim drives. The Engine's eight idle-reader test failed on
Windows before the fix at cycle zero with `(busy=1, log=13, checkpointed=13)`;
`ab9a63f31` committed that RED test. It then passed on the vendored correction
for 3 databases × 50 cycles × 8 readers. The vendor preserves registry crate
0.38.1 and bundled SQLite 3.53.2, with only the upstream VFS hunk and a
Windows-only compile-option marker. Its original amalgamation SHA256 was
`0a409f1633283fa31a9126b11fbfd64a1991c5d30defad07e5745d4667f5e23d`;
final patched SHA256 is
`7e4799907bdc92cff8858db2c52f5a03e136d5989ead6e513c0ef64057baeb9e`.
The exact vendor-tree hash and upstream/license provenance are pinned in
`third_party/libsqlite3-sys-0.38.1/FATHOMDB-PATCH.md` and the pin checker.

The Engine simplifies only safe short local paths at the SQLite-facing seam.
The bundled correction retains long local path support. A published Rust
consumer does not inherit a workspace root Cargo patch: an isolated external
consumer resolved registry `libsqlite3-sys` 0.38.2 and its `Engine::open`
refused an unsimplifiable >260 local path before DB creation (exit 0,
`external-consumer.log` SHA256
`37d9c7af48b4b783bf39317aa08a0159f8b552bdfc7084b3b7491fd9acc1b14f`).
That downstream path can become supported when its SQLite VFS is at least
3.53.3 or carries the exact correction marker. The low-level WAL regression
test runs only with a corrected VFS. A separate real `Engine::open` test checks
trailing-dot refusal before DB or lock creation.

A focused installed-wheel check exposed a separate Windows filename split:
SQLite opened requested verbatim `trailing.` as physical `trailing`, while the
engine lock was `trailing..lock`. The final Engine guard refuses trailing-dot
and trailing-space components before file or lock creation. The RED
`red-trailing-path-test.log` SHA256 was
`120bd3f251d39a55b550157e9b5f9e547b702eceb68711eca0a44bcf30b0274b`;
the actual Engine GREEN test `green-engine-trailing-open-test.log` SHA256 was
`d54fe2e4de75075ddbfc4a7eed48463640728110bb34b465c7e9fb267ca1a656`.

## Installed fixed-wheel controls and scope

The vendor-only fixed wheel SHA256
`82b2f80e434877c5b09b7946eef0eabc3bc5222c236d0742a1ad38c5ec00062d`
passed the reviewed public retry probe **40/40 no-read, 40/40 read, 20/20
page**: no first refusals, independent-reopen erased-row absence, survivor
retention, zero-count identical completion, and successful post-completion
write. Focused Windows governed deleted-first `purge` passed 5/5 with WAL zero,
and operator `--excise-source src-a` returned exit 64/count 1 then exit
64/count 0, survivor retained and WAL zero (`fixed-focused.json` SHA256
`9c5b71f692982455c9a838d29e9d35c65290345630ea8c6682ac387de38ef251`).

The final guarded branch wheel SHA256
`49809f51592b77aedf99a7e156edb6244e32ee54508762a08e545a8cd66aee95`
was freshly installed. Its focused `guard2-wheel-path.log` exited 0: a >260
local path and verbatim `NUL.txt` opened/closed with schema 34; trailing-dot
and trailing-space names were refused before the requested DB, normalized
alias, or lock appeared. Log SHA256:
`21bdb6756f0769116d54582e714019cb9a3452d437d0b5143b0ffa7c949b192c`.
The earlier combined vendor+short-path wheel SHA256
`e35e869524d31e1f730f9c2791b12a6c8888f9e9f76a340bf83481cef210dfd5`
passed a bounded public probe 10/10 no-read, 10/10 read, 5/5 page; the
final guard only changes opening for unsafe path forms.

No accessible genuine UNC fixture existed on the reserved VM (`\\localhost\C$`
was inaccessible); mapped-share behavior was not qualified. SQLite's
[WAL documentation](https://www.sqlite.org/wal.html) says network filesystems
are unsupported because processes must share the WAL index. This local-drive
correction does not establish network-path support, and a per-engine gate
alone cannot establish it.

## Verification and remaining release step

Test chronology: `bc0e77632` froze the protocol; `21c911582` committed RED
probe-outcome contracts and `31a3d99ce` made them GREEN; `00e27eafa` put them
in the ordinary Python suite. Windows path RED commits were `ab9a63f31`,
`e2386cbc9`, `b5f32a0c8`, and `262a4a1af`; production commits were
`20324394c` and `0998cb0ef`; `75093d59e` added the actual Engine no-side-effect
regression. The final branch code HEAD is `75093d59e` before this receipt.
Independent review of the VFS hunk and guard passed, subject to final gate
and integrated same-SHA qualification.

The clean-HEAD full `./scripts/agent-verify.sh` at `75093d59e` reached the
normal test suite with security `0 violation(s), 0 blocker(s), 0 downgrade(s)`.
Two fast-suite tests failed. `test-slice90-root-reconciliation` found a stale
`lib_rs_blob` inventory after the W tests changed `lib.rs`; the release
orchestrator corrected only that Markdown hash at `9764dd6c1`, and its
focused 8-test suite then passed. The dynamic `test-preflight-release-state`
fixture also failed after the release branch advanced to integrated
`f148aca86`: isolated W HEAD was no longer a descendant of the current
release ref. The run was stopped with exit 130 after both recorded failures;
it is **unqualified**, not a green gate. The exact
partial log is
[agent-verify-75093d59e-unqualified.log](windows-evidence/agent-verify-75093d59e-unqualified.log),
SHA256 `7fd6bb2c7463ece7c0330349a3f114e806f2a64691be5dac4a899aac9bbdfb68`.
The separate release-orchestrator strict gate runs on clean integrated HEAD
`f148aca86` and supplies the authoritative final repository result. The
process-local W environment used `PYTHONPATH=src/python`,
`CARGO_TARGET_DIR=<Track W>/target`, `CARGO_INCREMENTAL=0`, and
`core.excludesFile=/tmp/fathomdb-s103w-git-excludes` containing only
`target/`; it ran unconfined for ptrace-dependent AC-036.

## Final integrated Windows wheel

A read-only `git archive` of release HEAD `f148aca86` has SHA256
`ff2a32fe02bd8d30e54c3768461d931567988d007f0b30d9a95e1228b1c478a4`.
In a disposable staging copy, the repository's
`scripts/set-version.sh --workspace 0.8.27` changed only 11 Axis-W metadata
files; `--check-files` passed (Axis W 0.8.27, Axis E 0.6.1). The staged archive
SHA256 was `758fe20d99e9b3268c804681e5c707eca0a4bffd87446f3121ee73b1012d9413`.
The exact staged `Cargo.toml`, `Cargo.lock`, and Python `pyproject.toml`
SHA256 values are respectively
`6951a118b7097bd28bbaf8508a6c6a514b929c16df1f525608be3489ead5ed05`,
`92a6397983c477fd0531eaa66aae23cf65b21cc4607c5c33b123b4715888f401`,
and `0a2e0af2b79c136c36c7e2058b563c9d3ea369bac2f50ce7154e8b8d8046c891`.
No tracked release checkout file was edited for the overlay.

The Windows VM verified both archive hashes, built and freshly installed
`fathomdb-0.8.27-cp310-abi3-win_amd64.whl` SHA256
`e152fb260a22bef8fb840d33351f96e98fdb360d02bcb1608c8a2f1dcfccfe1e`.
Installed `_fathomdb.pyd` SHA256 was
`34a639bddfffa5f406806b28e78a06e8f57cc56d533bdbf0c6583c7d98de864f`.
The reviewed public probe ran **40/40 no-read, 40/40 read, 20/20 page** with
all process exits 0 and **zero first refusals** in each mode, including
independent-reopen absence, survivor retention,
zero-count second completion, and post-completion write. VM logs
`f148-probe-{noread,read,page}.jsonl` SHA256 respectively:
`9b4dfae3112e29a1af3831d1093a515424ec5f580e1c868fa7af4c92f436920d`,
`6746ecb72a2e1807dabbe62e6b4f8b92ec5705fce340964333e7b7490bb1207d`,
`a4506e709c09e974bcc5eb9c71e016fcdb45d10d83dcf9f74156131bb0e51c41`.

The exact f148 CLI built from the same staged source. The installed wheel's
path fixture exited 0: >260 local path and `NUL.txt` opened/closed; trailing
dot/space refused before requested DB, alias, or lock. `f148-wheel-path.log`
SHA256 `21bdb6756f0769116d54582e714019cb9a3452d437d0b5143b0ffa7c949b192c`.
The governed deleted-first `purge` control passed 5/5 with WAL zero and the
survivor alone after reopen. Operator `recover --accept-data-loss
--excise-source src-a --json` returned exit 64/count 1, then exit 64/count 0;
WAL zero and survivor alone. `f148-focused.json` SHA256
`7a6b2df4e489d671e7311c3cbe4bafff1011ea2ff7c2dcc4de97b6cf54603e75`.

Final remaining work: the release orchestrator records the exact-f148 strict
repository gate, final Tegra wheel and cross-platform review, then closes
R27-103D/E and Slice 103 only when those combined receipts pass.
