# Slice 10 preparation design

Use the already recovered isolated checkout and branch for initial state bootstrap.
Final source/executable fixes use an isolated implementation writer, with strict
preflight after valid release activation. One writer owns each checkout; root
imports reviewed plan/state changes serially. Do not implement while9 is pending.

Candidate task-owned storage:
`/mnt/data/tmp2/fathomdb-0829-01a1c5a5`. Confirm access, noncollision and current
capacity before creation. Put Cargo target, wheel output and task scratch there;
keep source/venv ownership tied to this checkout. Preserve existing standing
`/mnt/data/tmp2/fathomdb-s95-publish-gpu-20261109`. No borrowed mutable caches.
Start with one native build/full gate at once and bounded Cargo jobs; inspect
peak usage before expanding. Existing >=10GiB root floor remains, not a claim
that16GiB root is enough for native/CUDA compilation.

Use command-local exact Node/npm pins and locked owned root/TS installs. Select
a verified developer Python>=3.11, create checkout-owned venv with pip and exact
Ruff/Pyright, build noneditable source-bound native wheel and install it there.
Resolve bootstrap/build editable behavior and tests before invoking those scripts
in a linked checkout. Record source/features/interpreter/package identity rather
than importing another checkout's venv. Retain reviewed Rust1.95.

Adapt only the approved Rustls/webpki dependency nodes into the current lock;
do not merge stale candidate ancestry. DOMPurify update stays in renderer lock;
assess actual config/compatibility. Retain declared Markdown cli2.23.3 rather
than opportunistically taking candidate.24 or sharing old cli2.23.0 modules.

Survey discovery/classification preserves historical locks/fixtures via explicit
rules. .29 first-tag routing uses the smallest exact authorized amendment or
existing permitted route, retaining selected workflows, nonempty environment
reviewers and held approval until normal route passes. No generic tag bypass.
