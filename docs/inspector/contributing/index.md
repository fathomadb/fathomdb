# Contributing to Inspector

Inspector is a separate product area in the FathomDB repository:

- `dev/inspector/` contains needs, requirements, acceptance, design, reviews, roadmap and release plans.
- `docs/inspector/` contains public product documentation following the existing getting-started, concepts, guides, reference, operations and contributing layout.
- `src/inspector/` is the application source home; it contains the Rust application, embedded browser UI, tests and packaging.

The adopted 0.2.1 scope upgrades Inspector onto FathomDB 0.8.30 and native
Ubuntu 24.04 (`glibc 2.39`) delivery while carrying forward every 0.2.0
capability and obligation. The Inspector package is `0.2.1` and is released
locally to ignored `dist/inspector/0.2.1/`; the unperformed five-person study
remains removed by owner decision. Optional deeper inspection and broader
compatibility belong in [ROADMAP](../ROADMAP.md). Source changes follow the
repository's test-first and verification rules. Tests must exercise real
database behavior and actual UI interactions, with independent identity,
lexical, vector and graph facts.

Scoped 0.2.1 implementation work is Inspector-only. Engine, schema, SDK, MCP
and agent-access code/contracts remain unchanged dependencies and references.
Contributions must not create a second eligibility/ranking implementation or
assume an SDK call is side-effect-free merely because it returns data. Engine
startup is allowed; qualify normal Engine effects separately from
Inspector-issued mutations and retain cache/disclosure constraints. Public docs
must clearly separate planned capabilities from tested shipped behavior.

Use explicit `--features cached-cpu` for shipping binary/examples and package
qualification. Bare development/workspace defaults intentionally remain
model-free; never enable Engine defaults globally to satisfy an Inspector test.
Build binary and examples together to preserve their exact build pairing.
Package with `bash src/inspector/tools/package.sh /absolute/new/output`; this
does not publish. Native Ubuntu 24.04 (`glibc 2.39`) qualification uses an
isolated build target directory, final ELF symbol inspection (`GLIBC <= 2.39`)
and actual installed runtime checks under host `Xvfb`, `/proc` process
discovery and CDP `Browser.close` (no Docker, no `openbox`, no `xdotool`).
Committed installed probes use production launch/bootstrap and native window
close; Node/Playwright/Python remain external test tools. No human-study pass
is inferred from automation. Follow the source README for scoped and full
source gates.

## Local CI and package/release work

Run from the target checkout with its own `.venv` (Python >= 3.12), cached
locked Rust dependencies, Rust 1.95, Node/Playwright, existing Chromium 156,
`Xvfb`, `flock`, `readelf` and `strace`. Python/Node are development tools. No
global installation, Docker container or model fetch is part of verification.
`src/inspector/tools/README.md` gives exact setup and fixture commands,
including closed real stores and their lock sentinels.

```sh
bash src/inspector/tools/local-ci.sh /absolute/new/candidate /absolute/fixtures /absolute/build-target
```

The default command executes one strict typed full source gate, tool/unit
tests, native Ubuntu 24.04 offline package/examples, clean-source candidate
sealing, ELF `GLIBC <= 2.39` check and fresh archive extraction with actual
installed browser/native tests. It fails on missing prerequisites or a failed
required stage. `--installed-only` rechecks an existing candidate after only
permitted documentation closure; it explicitly does not run or claim the source
gate. Run focused tests during development and the full command once per final
source phase. Existing hosted CI routes Inspector changes to Ubuntu verify;
local installed qualification is separate from that hosted job.

Keep source/API boundaries in the canonical design: UI controls dispatch typed
owned requests; workers use qualified Engine or bounded stored reads. Extend an
existing route with real-database and browser oracles before adding generic
frameworks. Preserve build pairing, disclosure, typed identities and resource
limits. Changing the selected Engine/schema needs separate scope and contracts.
The release protocol creates a new destination through verified sibling staging
and atomic no-overwrite rename. Future public distribution is described in the
[release notes](../releases/0.2.1.md), with no uploader in 0.2.1.
