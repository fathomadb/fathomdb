# Contributing

Start with the [source map](../generated/contributor-map.md) for workspace
Rust workspace owners, Python/TypeScript package roots, and typed developer
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

After the documented repository setup:

- Documentation and plan changes: `./scripts/agent-lint-md.sh`.
- Public documentation: also `./scripts/agent-lint-docs.sh`.
- Release-state changes: `./scripts/check-release-state-views.sh`.
- Source, tests, or executable scripts: `./scripts/agent-verify.sh` runs the
  full developer gate. Resolve specific findings with focused checks, then
  run the required full gate on the integrated candidate.
- `./scripts/check.sh` is the broader CI gate, including the MkDocs build.

A scoped pass does not establish a full-workspace pass. Keep the command,
candidate commit, result, and any environment limits in the review record.

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
