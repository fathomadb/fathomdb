---
title: FathomDB 0.8.28 Slice 42 — goal
status: PLANNED (phase 1 docs rewritten 2026-10-31; awaiting design review)
target_release: 0.8.28
observed_on: 2026-10-31
---

# Slice 42 goal

Two units, placed by owner rulings on 2026-10-16 and re-scoped on
2026-10-31:

- **Unit A (todos ledger `dev/todos-and-considerations-ledger.jsonl`
  seq 343, `TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`).** Make the
  descriptor check in
  `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts`
  independent of the other tests in its binary. Today it counts every
  open descriptor in the process, so it fails in 9 of 10 parallel runs.
- **Unit B (todos ledger seq 344,
  `TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`; re-scoped 2026-10-31).**
  Adopt `rusqlite` 0.41.1 and `libsqlite3-sys` 0.39.1 (SQLite 3.54.1,
  no `SQLITE_ENABLE_MEMORY_MANAGEMENT`, the Windows WAL path fix
  included). Delete the vendored `libsqlite3-sys` 0.38.1 and its patch,
  make the Windows verbatim-path guard a SQLite version check, and update
  the records. Published crates and `cargo install` then get one page
  cache per connection without a forked SQLite.

The slice is complete when every AC42 in
[requirements.md](requirements.md) passes or is named as carried to
final verification, the design review and the codex review approve, and
the slice lands on `dev/0.8.28`. Steps are in [plan.md](plan.md); state
is in [status.md](status.md).

Paste the block below to continue the slice in a fresh session.

```text
Continue FathomDB 0.8.28 Slice 42 (test hygiene and the SQLite bump) in
worktree /home/coreyt/projects/fathomdb-worktrees/slice-0.8.28-42 on
branch slice/0.8.28-42-test-hygiene-and-published-cache. Read AGENTS.md,
the memory index, and
dev/plans/0.8.28/features/slice-42/{requirements,design,test-plan,plan,status}.md.
Resume from status.md. Unit A edits only the test lines the owner ruled
on (Q-A1). Unit B follows design § 2.6: one RED commit per behaviour,
then its GREEN. BGE embedding runs only on the RTX 3090 cuda:0.
```
