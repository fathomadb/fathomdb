# Slice 10 owned resources and setup recipe

## Fresh read-only observations

2026-11-10T18:21:20Z, planning checkout at2d5041795f0cbbe49d55d81a2710f09e2043b8a9.
These replace inherited13:35UTC capacity observations, and expire before setup.
`df -B1`: root/tmp available118356549632 bytes; /mnt/data980659458048 bytes.
`free -b`: MemAvailable38728978432 bytes; swap free9560064 bytes. High occupied
swap alone is not proof of thrashing. /mnt/data and /mnt/data/tmp2 owned by
coreyt; tmp2 mode775. The exact proposed s10 task root below was separately confirmed absent at
18:30:11Z. No directory,
env/module install or native import was created. Standing.28 install exists.

Exact Node binary `/home/coreyt/.nvm/versions/node/v25.9.0/bin/node` reports
v25.9.0; its bundled npm reports11.19.0, so it is not the approved npm. Selected
Python `/usr/bin/python3.12` exists; Rust toolchain declares1.95.0. Available
PATH includes actionlint, shellcheck, lychee and strace, but exact versions/
capability must be checked before use. Checkout has no authoritative .venv or
root/TS modules. Release.29 local ref absent at this inspection. Later4/5 closure
52a85c38cb50af220b418a4d05bc4c6e31f23fe2 changes planning documents only.

## Proposed reservation and hard stops

Task root `/mnt/data/tmp2/fathomdb-0829-01a1c5a5-s10` is created only after
fresh noncollision/owner/access checks, using mode700. Source worktree and real
.venv/modules live under a new `/home/coreyt/projects/fathomdb-worktrees/`
checkout; Cargo targets, wheel outputs, TMPDIR/TMP/TEMP, npm/pip/XDG/Puppeteer
caches and logs live under that task root. Preserve
`/mnt/data/tmp2/fathomdb-s95-publish-gpu-20261109` and every other session's paths.
No cleanup/global installer/driver/toolkit change is authorized.

These are proposed conservative numeric operating limits, not measured build
peaks or a ready reservation: one native build/full gate at once; Cargo jobs2
and CMake parallel2, no CUDA build in10. Before each heavy phase require root/tmp
free>=32GiB, task filesystem free>=250GiB and MemAvailable>=16GiB. Budget at most
200GiB new task storage; stop launching work and terminate only owned active
workload if task growth reaches200GiB, task filesystem free<50GiB, root free<20GiB
or MemAvailable<8GiB. Keep existing strict preflight10GiB floor unchanged.
Sample every30s during owned native/render/full phases, recording free bytes,
task growth, MemAvailable, swap-in/out delta, phase/process identity and real exit.
Sustained swap-in/out>64MiB/s for three samples also stops the phase. Missing
measurement is not clearance; pause affected work before resource exhaustion.
No deletion follows a stop. Reassess budget from actual restore/build/test/
package/cache-post peak receipts before expansion; no cache upload is scheduled.

## Owned setup commands to execute only after review/bootstrap

Use explicit checkout cwd and command-local PATH; never change HOME or global
nvm/npm/rustup state. Existing exact immutable Node may be used read-only. Use
its bundled npm only to install `npm@11.12.1` non-globally into the owned
`$task_root/tools/npm` prefix with owned npm cache. Then invoke that installed
npm CLI through the exact Node binary, verify11.12.1, and put its bin directory
before the Node distribution on command-local PATH. npm ci at checkout root,
src/ts and dev/tools/mermaid creates distinct owned modules from current locks;
verify resolved paths stay within the checkout. No node_modules symlink.

Create actual checkout `.venv` via selected Python>=3.11 with pip; reject any
foreign activated env or .venv symlink. Parse pyproject dev requirements using
stdlib tomllib into owned scratch and install with this venv's pip; record actual
resolved package list, interpreter path/version/ABI and Ruff/Pyright exact pins.
Use this venv's maturin build --locked --release --interpreter pointing to the
same owned Python, explicit production features in design.md and unique wheel
output. Install that wheel noneditable with the same pip; from an external cwd
with PYTHONPATH cleared verify imports/version/native digest inside this venv.
No `pip install -e`, maturin develop or manual native copy. Existing test-hooks
fresh-wheel gate remains separate and owns its source-module receipt.

Provision a real separate `scripts/sbom-survey/.venv` with the same selected
interpreter and noneditable `pip install 'scripts/sbom-survey[dev]'`; run its
pytest suite explicitly with no configfile and independent schema-validator
prerequisites. Do not install exporter requirements or run model export. Mermaid
uses owned browser/cache outputs under task root; browser access failures stop
only renderer qualification, not a speculative sandbox/host reconfiguration.

Before source tests verify actionlint1.7.12, shellcheck0.11.0, required scanner
policy and owned Python/root tools with existing dev-environment checks. Reuse
matching immutable binaries read-only; missing/mismatching global tool stops and
needs an owned supported install path, never silent global replacement. Obtain
ptrace-capable unchanged strict full verification; existing strace presence does
not prove sandbox permission. Never borrow another checkout's native install.
