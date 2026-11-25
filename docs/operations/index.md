# Operations

Operations docs cover the **0.8.32** documentation target (pre-publish; newest
published release: **0.8.31**).

## Single-scope default and multi-agent isolation (0.8.32, Schema 39)

**By default a FathomDB database is single-scope / single-agent** (one agent per
database, storing `scope = ""` under the default `ScopePolicy.unscoped()` /
`ScopePolicy::Unscoped`). Schema **39** `scope` is the **opt-in** isolation
layer for multi-agent or shared-store deployments (`ScopePolicy.single`,
`read_only` / `readOnly`, or `read_write` / `readWrite`). Because FathomDB
retains one-Engine-per-file exclusive locking (`<db-name>.sqlite.lock`), a
shared-store process shares its open handle across callers or opens sequential
scoped handles per phase, while single-agent deployments can run without
configuring `ScopePolicy`.

- [Tegra Pages publication](tegra-pages-publication.md) — the separately
  authorized, exact-commit route for the matching Tegra wheel and Node addon.
- [Erasure](erasure.md) — what `erase_source` / `purge` guarantee, what they
  do not, the erasure-audit record, the non-PII `source_id` rule, and
  `fathomdb doctor orphan-provenance`.
- [Immutable integrity inspection](integrity-inspection.md) — exact-version
  installation, quiescence, bounded invocation, and exit/reason handling for
  `doctor data-plane-integrity`.
- [Worktree and branch consolidation](worktree-consolidation.md) — the local,
  manifest-gated preservation workflow for safely reducing repository
  worktrees and local branches.

## Concurrency and durability limits

Use one Engine per database file, and share that handle across application
calls. The sidecar lock rejects a second Engine in the same process or another
process. Reader workers and projection workers have their own connections;
projection readiness can lag committed canonical writes. Use `drain` or the
receipt/generation readiness APIs when your next step requires derived state.
Close handles explicitly and wait for admitted work to finish.

A committed WAL can contain records absent from the main file. Quiesce and
close the application Engine before running `doctor safe-export` for a logical
snapshot. For offline inspection, follow the
[quiescence procedure](integrity-inspection.md). Do not treat a copied
main file alone as a complete WAL-mode backup. `doctor check-integrity --full`
and `doctor data-plane-integrity` inspect different layers; neither repairs a
store. Ordinary-open doctor commands may bootstrap a missing path or start
workers, while data-plane integrity uses the immutable inspection route.

Published `0.8.31` tests cover selected CPU process-kill/SIGKILL, WAL,
checkpoint, persistent SQLite-full/provider-fault, and reopen/recovery cases.
They do not prove behavior under torn/reordered device writes, sudden power
loss, or a general hardware fault model; expanded power-loss proof and
coverage-guided fuzzing remain staged within the `0.8.32` release program
rather than already-published `0.8.31` coverage claims. SDK/query admission has
bounded Proptest coverage; no general FFI, MC/DC or numerical line-coverage
claim follows.

See also [Reference — CLI](../reference/cli.md) for the full `doctor` /
`recover` / `migrate` verb tables, [Guide — MCP Server](../guides/mcp-server.md)
for `fathomdb-mcp` stdio lifecycle and opt-in scoped writes, and
[Positions — recovery surface](../positions/recovery-surface.md) for why
recovery is CLI-only while erasure is not.
