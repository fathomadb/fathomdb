# Contributing to Inspector

Inspector is a separate product area in the FathomDB repository:

- `dev/inspector/` contains needs, requirements, acceptance, design, reviews, roadmap and release plans.
- `docs/inspector/` contains public product documentation following the existing getting-started, concepts, guides, reference, operations and contributing layout.
- `src/inspector/` is the application source home; it contains the Rust application, embedded browser UI, tests and packaging.

The adopted 0.2.0 scope carries every assessed Need/Should and unfinished
view/qualification obligation from the original v0.1 scope, adding bounded
two-observation comparison. The Inspector package is0.2.0 with implemented views and installed technical
qualification.80 completed readiness and90 released locally to ignored
`dist/inspector/0.2.0/`; the unperformed
human study is removed by owner decision. Optional deeper
inspection and broader compatibility belong in [ROADMAP](../ROADMAP.md).
Source changes follow the repository's test-first and verification rules.
Tests must exercise real database behavior and actual UI interactions, with
independent identity, lexical, vector and graph facts.

Scoped 0.2.0 implementation work is Inspector-only. Engine, schema, SDK, MCP
and agent-access code/contracts remain unchanged dependencies and references. Contributions must not
create a second eligibility/ranking implementation or assume an SDK call is
side-effect-free merely because it returns data. Engine startup is allowed;
qualify normal Engine effects separately from Inspector-issued mutations and
retain cache/disclosure constraints. Public docs must clearly separate
planned capabilities from tested shipped behavior.

Use explicit `--features cached-cpu` for shipping binary/examples and package
qualification. Bare development/workspace defaults intentionally remain model-free;
never enable Engine defaults globally to satisfy an Inspector test. Build binary
and examples together to preserve their exact build pairing. Package with
`bash src/inspector/tools/package.sh /absolute/new/output`; this does not publish.
Ubuntu 22 compatibility requires a separate Ubuntu 22 target directory, final ELF
symbol inspection and actual installed runtime checks, not reused host objects.
Committed installed probes use production launch/bootstrap and native window close;
Node/Playwright/Python remain external test tools. No human-study pass is inferred from automation. Follow the source README for scoped and full source gates.

## Local CI and package/release work

Run from the target checkout with its own `.venv`, cached locked Rust dependencies,
Rust 1.95, Node/Playwright, existing Chromium156, Docker and the prepared Ubuntu22
image. Python/Node are development tools. No global installation or model fetch is
part of verification. `src/inspector/tools/README.md` gives exact mount/setup and
fixture commands, including closed real stores and their lock sentinels.

```sh
bash src/inspector/tools/local-ci.sh /absolute/new/candidate /absolute/fixtures /absolute/ubuntu22-target
```

The default command executes one strict typed full source gate, tool/unit tests,
compatible offline package/examples, source/build sealing, ELF check and fresh archive extraction and actual
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
[release notes](../releases/0.2.0.md), with no uploader in0.2.0.
