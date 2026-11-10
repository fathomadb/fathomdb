# Slice 0 workspace and setup design proposal

One writer per checkout. Mutable venv/native outputs/npm installs/build/scratch
belong to the selected checkout/task. Immutable model/download caches may be
reused only after input identity and availability are known. Never borrow a
venv or repoint another checkout's editable bindings.

The old planning worktree was archived during this session. Its snapshot
`a4fb5173a4bfa0bf83ff0d7bf1bcb4bec1abee5c` preserved all 35 changed/new planning
files. This chat now owns attached worktree
`/home/coreyt/.codex/worktrees/fathomdb-0829-work/dev-0.8.29`, on the original
`llm/0.8.29-chat` branch at `6edc1155f`. Recovery verified byte equality for all
35 files before prework edits. The snapshot remains; no force/reset or product
commit was made. Canonical integration stays `dev/0.8.29` in its other checkout.

Use noneditable source-bound wheels in an owned linked-worktree venv, subject
to reviewed preparation. Generic bootstrap/build currently run editable installs;
resolve that conflict before invoking them. Preserve R580 and primary CUDA 12.6.
No fault/reset action or GPU readiness claim belongs to environment inventory.
