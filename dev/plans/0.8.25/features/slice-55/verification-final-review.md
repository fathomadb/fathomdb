---
title: 0.8.25 Slice 55 final independent verification
status: PASS
---

# Slice 55 final independent verification

## Verdict

**PASS.** Separate Linux and Windows verifiers completed the Slice 55 matrix
against exact candidate
`5a6942bcd34ef5211fc81d4f5a80241d66794c3f`, whose reviewed product state is
`3dd10ca888f50038431e8483999b80a7286f7e0a`. The release worktree remained
tracked-clean throughout verification.

No release package was staged, no registry was contacted for publication, and
no push, tag, upload, publication, or main-branch merge occurred.

## Startup, cancellation, and WAL regression evidence

- Projection-runtime startup suite: 4/4 passed.
- Exit-after-setup-report: 10/10 passed.
- Exact live-role startup: 10/10 passed.
- Eight concurrent startup-suite processes: 32/32 tests passed.
- Cancellation-safe projection pause: 2/2 passed.
- Exact WAL-attribution witness: 10/10 serial and 4/4 concurrent processes
  passed. Every retained run observed exactly five
  `owned_runtime_transaction` BUSY attempts, worker autocommit, complete idle
  native inventory, and a clean post-release sampler.
- Converted Slice 30 and Slice 40 callers: 3/3 passed.
- Native-state inventory: 2/2; WAL close boundary: 4/4; reader pool: 6/6;
  lifecycle public tests: 4/4 passed.

No startup, service-ready, projection-pause, or futex deadlock occurred.

## Linux verification

Focused routes passed:

- dependency trace: 18;
- data-plane integrity: 66;
- explanation: 17;
- governed facade: 2;
- CLI: 3;
- legacy integrity and trace: 3 + 3; and
- wire: 10.

The release performance witness passed at 50,000 hidden rows and 3,200,000 VM
steps in 52 ms with zero measured RSS delta. Formatting, Ruff, Pyright,
workspace Clippy with warnings denied, workspace all-target check, and release
engine check passed.

Broad repository gates passed without skips or exclusions:

- fast: 103/103;
- heavy: 3/3;
- all: 106/106; and
- security: zero violations, blockers, or downgrades.

The selected
`operator,test-hooks,default-embedder,default-reranker` serial route passed.
The canonical unconfined `scripts/test-rust-workspace.sh --serial` release gate
passed. The unconfined parallel reporter terminated normally. Its sole
diagnostic was a fixed-path CLI lock-holder collision; the unchanged exact
control passed 5/5. This is report-only evidence under TC-72/TC-74 and did not
show a product or liveness failure.

Fresh local/offline artifact evidence passed:

- wheel SHA-256:
  `8001ba68d2e533d1e1b9f1b0ee9c74e6cdd48c38a4cca80e136999548637c2f0`;
- installed native SHA-256:
  `7b83257f077f4bd1c0df37715af9e11fd8cf89346937f5eff554b82b1657b0ec`;
- N-API native SHA-256, recorded as first and last 32 hexadecimal characters:
  `9f8b1591b258b53569a23fefc9ee283c` / `53f8a96ad7980bb2e5d0215d70323f44`;
- installed Python smoke passed from site-packages, including typed trace and
  explanation behavior and immediate unlink after explicit close;
- four explicit TypeScript modules passed 31/31 with no skipped tests;
- the matched local wheel plus N-API offline harness passed; and
- the real-database CLI route passed 3/3.

The disposable artifact version remains 0.8.24 because Slice 55 does not own
the later release version cut. This local evidence was not release packaging.

## Windows verification

The exact Git-only source archive excluded `.git`, `target`, `node_modules`,
and virtual environments. Its local and Windows SHA-256 was
`b1ca449ffca2797b241bb1b00f565c615a06373f1a468f1875d2008c1a1d3c42`.

Fresh installed Python evidence passed:

- wheel SHA-256:
  `b0b7ef4698c0ea619661dc281740af1c82654f078e421a1d6c8441a0c93cc5f2`;
- installed `_fathomdb.pyd` SHA-256:
  `6de3922904db4d6c56ceb2ee945bd06f2304bc6d585f796e3db3a8c228aad48f`;
- both package origins resolved inside the fresh verifier environment; and
- installed smoke passed typed trace and explanation behavior plus immediate
  corruption-file unlink after explicit engine close.

Focused Windows Rust routes passed:

- startup and protocol-fault cleanup: 4/4;
- dependency trace: 18/18;
- data-plane integrity: 66/66;
- explanation: 17/17;
- wire: 10/10;
- legacy integrity and trace: 3/3 + 3/3;
- governed facade: 2/2; and
- CLI: 3/3.

The exact Windows N-API native SHA-256 was
`1cd7ec34f8c101366487aea02b3b14899b694725b8afcfe28a10761a053437f4`.
TypeScript compilation passed, four explicit compiled modules passed 31/31
with no skipped, cancelled, or todo tests, and the offline local wheel plus
matched N-API artifact harness passed without registry access.

## Environment-only retries

Unchanged candidate reruns distinguished these verifier-environment failures
from product behavior:

- restricted-sandbox process spawning denied N-API shell execution;
- the durable Python extension symlink pointed to a stale canonical native;
- a disposable-clone security check initially lacked its repository-local
  target path;
- source-directory shadowing hid the installed Windows package;
- an initial Windows Rust selector omitted `--lib` and filled the shared cache;
- Windows N-API compilation exhausted the remaining disk once; and
- the first manual TypeScript route omitted package-layout compilation.

Corrected unchanged routes passed. No candidate failure repeated.

## Cleanup and final state

- All verifier-owned Linux clones, archives, wheels, virtual environments,
  scripts, databases, locks, and artifact directories were removed.
- All verifier-owned Windows source, archive, wheel, virtual environment,
  native output, temporary databases, locks, and cache additions were removed.
- The Windows shared Cargo target returned to its pre-verification size with
  zero verifier-created files; Windows had 4,243,914,752 bytes free.
- Linux had 99,496,648,704 bytes free after artifact cleanup.
- No verifier Cargo, Rust, Python, Node, FathomDB, or test process remained.
- The durable release-worktree target remained an 85,719,167,522-byte
  rebuildable cache pending the explicit between-slice disk cleanup.
- Final release worktree: clean at
  `5a6942bcd34ef5211fc81d4f5a80241d66794c3f`.

The repository-state views, Markdown gate, and `git diff --check` passed before
closure. Slice 55 may advance to Slice 60 after this evidence and the
cycle-15 implementation-review PASS are committed to the release branch.
