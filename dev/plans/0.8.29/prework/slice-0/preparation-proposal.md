# Slice 0 decision-bound preparation proposal

Workspace recovery/switch was explicitly authorized November 10 and is already
done. All remaining preparation is proposed for Slice 9, to execute in 10.

1. Confirm owned build/scratch location and phase capacity allowance; recommend
   a new task-owned /mnt/data path if approved/access confirmed. Preserve other
   worktrees, data and the standing .28 GPU install. No implicit cleanup.
2. Select contained dependency/tool changes from Slice 1 and exact Node/npm pins.
   Retain Rust 1.95, independent E and qualified fork/SQLite/MCP baselines.
3. Activate valid .29 state/board/views from this existing checkout after final
   Slice 9 acceptance. Validate views and strict preflight before dependent
   worktree creation. Plain health is not an activation substitute.
4. Prepare command-local Node 25.9.0/npm 11.12.1 and owned locked npm installs.
5. Resolve editable-install instructions; prepare owned Python developer venv,
   pinned tools and source/features/interpreter-bound noneditable wheel/native
   artifacts. Never invoke worktree editable setup or borrow another venv.
6. Run relevant setup/version/identity checks; then begin 20 tests and 30
   measurements. Recheck datasets, feature selection, resource and venue access
   before each affected campaign.

Stop on insufficient approved capacity, unknown ownership/native identity,
missing data or strict preflight failure. Missing GPU access blocks only affected
GPU claims. Proposed concurrency starts with one native build/full gate at once;
adjust Cargo jobs from measured peaks, not an invented performance budget.
Do not install/provision tools in prework to make its records appear green.
