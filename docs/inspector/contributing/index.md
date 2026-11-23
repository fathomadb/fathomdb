# Contributing to Inspector

Inspector is a separate product area in the FathomDB repository:

- `dev/inspector/` contains needs, requirements, acceptance, design, reviews, roadmap and release plans.
- `docs/inspector/` contains public product documentation following the existing getting-started, concepts, guides, reference, operations and contributing layout.
- `src/inspector/` is the application source home; it contains the Rust application, embedded browser UI, tests and packaging.

Inspector `0.3.0` (`R31-08`) upgrades Inspector onto FathomDB `0.8.31`
(schema `38`), adding outside-`body` node/edge `properties` and read-only
reserved `scope` inspection, historical `as_of` graph exploration subordinate
to the 8-flag `D-01` disclosure policy, `--scoped` default `local-ci.sh`,
bounded `P7-02` timeouts/cleanup, retained `D4 = B` native Ubuntu 24.04
(`glibc 2.39`) Chromium + bundled SQLite qualification, and GitHub Release
artifact delivery via `.github/workflows/release.yml` (`build-inspector`). The
unperformed five-person study remains removed by owner decision. Optional
deeper inspection and broader compatibility belong in [ROADMAP](../ROADMAP.md).
Source changes follow the repository's test-first and verification rules. Tests
must exercise real database behavior and actual UI interactions, with
independent identity, lexical, vector and graph facts.

Inspector work keeps Engine, schema, SDK, MCP and agent-access code/contracts
as unchanged dependencies and references. Contributions must not create a
second eligibility/ranking implementation or assume an SDK call is
side-effect-free merely because it returns data. Engine startup is allowed;
qualify normal Engine effects separately from Inspector-issued mutations and
retain cache/disclosure constraints. Public docs must clearly separate planned
capabilities from tested shipped behavior.

Use explicit `--features cached-cpu` for shipping binary/examples and package
qualification. Bare development/workspace defaults intentionally remain
model-free; never enable Engine defaults globally to satisfy an Inspector test.
Build binary and examples together to preserve their exact build pairing.
Package with `bash src/inspector/tools/package.sh /absolute/new/output`; this
does not publish. Native Ubuntu 24.04 (`glibc 2.39`) qualification (`D4 = B`)
uses an isolated build target directory, final ELF symbol inspection
(`GLIBC <= 2.39`) and actual installed runtime checks under host `Xvfb`,
`/proc` process discovery/reaping, bounded timeouts, and CDP `Browser.close`
(zero Docker, `openbox`, `xdotool`, or `sudo`). Committed installed probes use
production launch/bootstrap and native window close; Node/Playwright/Python
remain external test tools. No human-study pass is inferred from automation.
Follow the source README for scoped and full source gates.

## Local CI and package/release work

Run from the target checkout with its own `.venv` (Python >= 3.12), cached
locked Rust dependencies, Rust 1.95.0, Node/Playwright, existing Chromium 156,
`Xvfb`, `flock`, `readelf` and `strace`. Python/Node are development tools. No
global installation, Docker container or model fetch is part of verification.
`src/inspector/tools/README.md` gives exact setup and fixture commands,
including closed real stores and their lock sentinels.

```sh
# Default: ~5 min Inspector-scoped check (fails closed if changes cross outside Inspector)
bash src/inspector/tools/local-ci.sh [--base <ref>] [ABSOLUTE_CANDIDATE ABSOLUTE_FIXTURES ABSOLUTE_BUILD_TARGET]

# Full workspace verification (runs scripts/agent-verify.sh)
bash src/inspector/tools/local-ci.sh --full-workspace [ABSOLUTE_CANDIDATE ABSOLUTE_FIXTURES ABSOLUTE_BUILD_TARGET]

# Re-verify an already-sealed candidate after Inspector docs-only changes
bash src/inspector/tools/local-ci.sh --installed-only ABSOLUTE_CANDIDATE ABSOLUTE_FIXTURES ABSOLUTE_BUILD_TARGET
```

Mode flags (`--scoped`, `--full-workspace`, `--installed-only`) are mutually
exclusive (combining two or more exits `2`). The default `--scoped` mode runs
a fail-closed diff guard against `--base <ref>` (or `INSPECTOR_SCOPED_BASE`,
default `origin/main`) allowing only `src/inspector/`, `dev/inspector/`, and
`docs/inspector/`, runs `scripts/agent-lint.sh`, `scripts/agent-typecheck.sh`,
and default + `cached-cpu` `fathomdb-inspector` Rust tests, then executes
tooling, unit, native Ubuntu 24.04 offline packaging, clean-source candidate
sealing, ELF `GLIBC <= 2.39`, and freshly extracted installed browser/native
tests. A `--scoped` pass is **not** a full-workspace green claim; run
`--full-workspace` whenever changes touch shared/non-Inspector files or when a
release gate requires full verification. `--installed-only` makes no
source-gate claim.

Keep source/API boundaries in the canonical design: UI controls dispatch typed
owned requests; workers use qualified Engine or bounded stored reads. Extend an
existing route with real-database and browser oracles before adding generic
frameworks. Preserve build pairing, disclosure, typed identities and resource
limits. Changing the selected Engine/schema needs separate scope and contracts.
The local release protocol creates a new destination through verified sibling
staging and atomic no-overwrite rename, while `.github/workflows/release.yml`
(`build-inspector`) stages `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` for GitHub Release
attachment (see [0.3.0 release notes](../releases/0.3.0.md)).
