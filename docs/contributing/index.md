# Contributing

Start with the [source map](../generated/contributor-map.md) for Rust
workspace owners, Python/TypeScript package roots, and typed developer
commands. The [SDK operation map](../generated/sdk-operation-map.md)
connects governed operations across Python, TypeScript, and Rust; detailed
contracts remain in the language references.

## Change a behavior

Read the repository's [agent and contributor instructions](https://github.com/fathomadb/fathomdb/blob/main/AGENTS.md),
the relevant accepted ADR and interface contract before editing. Use TDD:
write a test that fails for the required behavior, commit or stage that RED
test for review, then implement the GREEN change without weakening the test.
Use real databases for integration tests. Property tests cover codec,
projection, recovery, and round-trip invariants.

Give each concurrent writer an isolated worktree. Keep one writer per
checkout; read-only reviews may share it. Commit with the normal hooks.
Public API changes need the matching interface or ADR update.

## Verify the change

Use Node 26.13.1 from `.nvmrc`, npm 11.20.0 from the root and TypeScript
manifests, and Rust 1.95.0 from `rust-toolchain.toml`. Python package runtime
support starts at 3.10; repository developer gates require Python 3.11 or later.
Published TypeScript compatibility is documented in the
[install guide](../install/typescript.md).

Each checkout owns its root and TypeScript `node_modules` from `npm ci` (both
are preflighted upfront by `scripts/tests/test_dev_environment_tools.sh`) and
its Python `.venv`. On Ubuntu 24.04 hosts, `scripts/bootstrap.sh` and
`scripts/setup-python-owned.sh` prefer bare PATH Python `>=3.11` (such as
`/usr/bin/python3.12`) over `uv` standalone Python builds before falling back
to `uv python find`. Run
`bash scripts/setup-python-owned.sh --provision` from that checkout to install developer tools
and its noneditable, source-bound Python wheel. Never share mutable environments or editable
bindings between worktrees. `--tools-only` provisions just developer tools for
fast/scoped work; it does not install or verify the native binding and cannot
replace `--provision` for the full gate. Use `--build` after native source changes
to rebuild the same checkout's wheel. Do not run `maturin develop` or editable
installs from a worktree. After this setup:

- Documentation and plan changes: `./scripts/agent-lint-md.sh`.
- Public documentation: also `./scripts/agent-lint-docs.sh`,
  `python3 scripts/generate-public-docs.py --check`, and `mkdocs build --strict`.
- Release-state changes: `./scripts/check-release-state-views.sh`.
- Source, tests, or executable scripts: `./scripts/agent-verify.sh` runs the
  full developer gate. Resolve specific findings with focused checks, then
  run the required full gate on the integrated candidate. Its strict AC-036
  check needs a ptrace-capable executor. If the sandbox denies
  `PTRACE_TRACEME`, rerun the same strict gate where ptrace is available.
- `./scripts/check.sh` is the broader CI gate, including the MkDocs build.
  Use `AGENT_LONG=1 ./scripts/check.sh` for the long AC-021 and AC-059b
  cases. The default developer gate skips those variants.

Use `npm run format:md` for guarded Markdown fixes. Never use Prettier on
Markdown or invoke `markdownlint-cli2 --fix` directly.

A scoped pass does not establish a full-workspace pass. Keep the command,
candidate commit, result, and any environment limits in the review record.

## Inspector local CI

Inspector changes use `bash src/inspector/tools/local-ci.sh` from a clean
checkout (see [Inspector contributing](../inspector/contributing/index.md) for
prerequisites and fixture setup):

- **Default (`--scoped`, ~5 min target):** Enforces a fail-closed diff guard
  allowing only changes under `src/inspector/`, `dev/inspector/`, or
  `docs/inspector/` relative to `--base <ref>` (or `INSPECTOR_SCOPED_BASE`,
  default `origin/main`), runs `scripts/agent-lint.sh`,
  `scripts/agent-typecheck.sh`, and default + `cached-cpu`
  `fathomdb-inspector` Rust tests, then executes Inspector tooling, unit,
  native Ubuntu 24.04 packaging, sealing, ABI (`GLIBC <= 2.39`), and installed
  browser checks. A `--scoped` pass is **not** a full-workspace green claim.
- **`--full-workspace`:** Runs the full `./scripts/agent-verify.sh` gate
  before the Inspector packaging and installed stages. Required whenever edits
  cross shared or non-Inspector workspace files or when a release gate requires
  full verification.
- **`--installed-only`:** Re-verifies an already-sealed candidate after
  Inspector documentation-only changes (requires explicit candidate, fixtures,
  and build-target paths; makes no source-gate claim).

Mode flags are mutually exclusive (combining two or more exits `2`).

## Maintain generated documentation

The tracked generated pages index source, rather than inventing API
signatures. Refresh them after changing the governed parity manifest,
Rust workspace or Python/TypeScript package manifests, or typed developer
entry points. The generator
requires Python 3.11 or later for its standard-library TOML parser. On Jetson
hosts with OS Python 3.10, use the configured `.venv/bin/python` interpreter
from repository setup. With the appropriate interpreter active:

```bash
python3 scripts/generate-public-docs.py --write
python3 scripts/generate-public-docs.py --check
mkdocs build --strict
```

Commit the generated changes alongside their source changes. Both complete
Pages build routes reject stale generated pages; they also reject a
publication candidate that does not include current main.

## Release records

Find the live release with `python3 scripts/release-current.py`.
The release JSON owns generated board/plan regions; edit the state JSON and
refresh its views rather than editing those generated regions directly.
Use the [engineering index](https://github.com/fathomadb/fathomdb/blob/main/dev/README.md)
for plans, interfaces and decisions. Slice status and verification receipts
record actual release progress; public release documentation describes the
release surface.
