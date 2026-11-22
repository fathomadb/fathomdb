---
title: Inspector 0.2.1 Slice 30 status
date: 2026-11-21
product: FathomDB Inspector
target_release: 0.2.1
status: COMPLETE
desc: Native Ubuntu 24.04 tooling, 0.2.1 release bump, dual published 0.8.29/0.8.30 qualification, full local-ci gate, and atomic local promotion
---

# Slice 30 status

**COMPLETE.** Removed Docker, Ubuntu 22.04 (`glibc 2.35`), `openbox`, and
`xdotool` from Inspector's build and qualification tooling (D4 = B), bumped
package/binary/test release identity to `0.2.1`, passed the full native
Ubuntu 24.04 `local-ci.sh` gate (including full `scripts/agent-verify.sh` and
all 81 installed browser qualification tests), atomically promoted the sealed
candidate to gitignored `dist/inspector/0.2.1/` (D2 = A), and verified a fresh
tar extraction independently against fresh closed databases from both
published FathomDB `0.8.29` and published FathomDB `0.8.30` (D3 = A).

## Changes landed (`73cb67b47`..`c6e1a0f09`)

- **Native Ubuntu 24.04 tooling & Docker / X11 WM removal:**
  - Retired `src/inspector/tools/ubuntu22.Dockerfile`.
  - Updated `src/inspector/tools/local-ci.sh`: builds natively on Ubuntu 24.04
    (`glibc 2.39`), verifies ELF `GLIBC <= 2.39`, removes the `docker`
    prerequisite and container stage, and executes the `0.2.1` tar and
    installed suite directly on the host.
  - Updated `src/inspector/tools/run-installed.sh` &
    `src/inspector/tools/installed-browser.sh`: runs directly on host under
    `Xvfb -displayfd`, isolates temporary `HOME` and unsets host GUI/D-Bus
    variables (`WAYLAND_DISPLAY`, `XAUTHORITY`, `DBUS_SESSION_BUS_ADDRESS`,
    `XDG_RUNTIME_DIR`), probes Landlock ABI ≥ 3, verifies pinned cache
    readability, and preserves hostile parent env vars for worker-isolation
    tests.
  - Updated `src/inspector/tests/browser/installed-harness.mjs` &
    `src/inspector/tests/browser/installed.mjs`: replaced `xdotool` window PID
    lookup with `/proc` cmdline discovery, replaced `xdotool key alt+F4` with
    guarded CDP `Browser.close` / `page.close()`, added parent-wrapped
    `strace -ff` support for `INSPECTOR_TRACE_OUTPUT`, and removed hardcoded
    `/cache/` container paths.
  - Updated `src/inspector/tools/qualify-installed-network.sh`: parent-wrapped
    `strace -ff -e trace=network,%file,process` proving 0 Internet-family
    socket/connect calls in the `--session-worker` tree, 0 outbound
    connect/send/UDP/IPv6 calls in the supervisor tree, `O_RDONLY`-only pinned
    asset opens (`config.json`, `tokenizer.json`, `model.safetensors`), and 0
    cache mutations without `sudo` or Docker root.
- **Release identity `0.2.1` & closure allowlist:**
  - Updated `src/inspector/Cargo.toml`, `Cargo.lock` (strictly `+17/-0` lines
    against `release/0.8.30`), `src/inspector/tools/package.sh`,
    `src/inspector/tools/local-release.py` (`0.2.1`, Engine
    `72fe93634321616214352a7e07066ce0f93bcf4c`, Ubuntu 24.04 / `glibc 2.39`,
    plus `mkdocs.yml` permitted in `--source` docs-only closure),
    `src/inspector/package/README.md`, `src/inspector/tests/browser/package.mjs`,
    `src/inspector/tests/tools/test_local_ci.py`,
    `src/inspector/tests/tools/test_local_release.py`, and
    `src/inspector/tests/tools/test_package.py`.
- **Post-tag 0.8.30 doc-truth alignment:**
  - Updated `dev/DOC-INDEX.md` historical archive banner rows (`0.8.29` →
    `0.8.30`) and `scripts/tests/test_tegra_publication_operator_path.sh`
    (`0.8.29+tegra` → `0.8.30+tegra`) so full `scripts/agent-verify.sh` passes
    cleanly on `release/0.8.30` after `v0.8.30` was tagged.

## Promoted local release (`dist/inspector/0.2.1/`, gitignored)

- **Source SHA:** `c6e1a0f09e292fa5b1e9f65551b813ed945c7b31`
- **Engine commit:** `72fe93634321616214352a7e07066ce0f93bcf4c` (byte-identical
  Rust/Cargo source to peeled `v0.8.30` `e3d31f5cd`)
- **Build ID:** `a1aa0ec6052d7c642716ea5840b41a8eae11d470183ac03d0e451752f1a9f4a4`
- **Archive SHA-256 (`fathomdb-inspector-0.2.1-linux-x86_64.tar.gz`):**
  `dc3336c89e6f084765fe46109e17de1081cf91002b3d50b65d8bdaa028dc0332`
- **Executable SHA-256 (`fathomdb-inspector`):**
  `b81fd41280c34619d443b4793c4f43019fa4b742fa43450449cdc4a7d18af5e9`
- **ELF maximum GLIBC:** `2.39` (`Ubuntu 24.04 / glibc 2.39`, kernel
  `7.0.0-43-generic`, Landlock ABI `8`, Chromium `156.0.8078.4`)
- **Boundaries:** `git diff release/0.8.30 -- src/rust/ .cargo/config.toml` is
  empty; `git check-ignore dist/inspector/0.2.1/candidate.json` confirmed
  gitignored.

## Verification & review results

| Check | Result | Notes |
| --- | --- | --- |
| Full `scripts/agent-verify.sh` (inside `local-ci.sh` on `c6e1a0f09`) | PASS | `agent-lint`, `agent-typecheck`, `agent-security` (0 violations/blockers/downgrades, AC-036/AC-037 pass), `agent-test.sh` (240/243 passed, 3 expected platform skips, 0 failures) |
| `local-ci.sh` native pipeline (`tooling`, `unit`, `native-build`, `seal`, `candidate`, `abi`, `extract`, `installed`) | PASS | 12/12 tool tests, 11/11 JS unit tests, ELF `GLIBC 2.39 <= 2.39`, **81/81** installed browser tests pass natively without Docker/`openbox`/`xdotool` |
| `local-release.py promote` & `verify "$PWD/dist/inspector/0.2.1" --source` | PASS | Atomic `renameat2` promotion verified byte-for-byte |
| Independent final-dist `qualify-installed-network.sh` (fresh tar extract) | PASS | `traced_files=330`, `supervisor_traced_pids=46`, `worker_traced_pids=13`, `worker_internet_family_attempts=0`, `supervisor_outbound_attempts=0`, `source_asset_readonly_opens=3`, `source_cache_mutations=0`, `close_ms=348.35` |
| Independent final-dist dual published `0.8.29` verification (fresh tar extract) | PASS | Published PyPI `fathomdb==0.8.29` DB (`sha256: 499607eb...`, preserved `0444`): `vector_hits=6`, `equal_body_hits=3` (incl. edge + `record_keyed_not_merged`), graph, A/B compare, queued erasure + `--no-show-inactive` / `--show-erased`, `close_ms=166.9–170.3` |
| Independent final-dist dual published `0.8.30` verification (fresh tar extract) | PASS | Published PyPI `fathomdb==0.8.30` DB (`sha256: b1710a93...`, preserved `0444`): `vector_hits=6`, `equal_body_hits=3` (incl. edge + `record_keyed_not_merged`), graph, A/B compare, queued erasure + `--no-show-inactive` / `--show-erased`, `close_ms=150.2–190.6` |
| Adversarial Slice 30 code review | APPROVE | First pass `REQUEST_CHANGES` (3 P2 + 3 P3 findings); all fixed in `c6e1a0f09` and re-reviewed to **APPROVE** |

## Handoff to Slice 40

Slice 40 starts on clean `inspector/dev/0.2.1` HEAD in
`/mnt/data/worktrees/inspector-0.2.1` for strictly documentation-only
reconciliation (`docs/inspector/releases/0.2.1.md`, `docs/inspector/`,
`mkdocs.yml`, `src/inspector/{README,tools/README}.md`, `dev/inspector/`),
preserving `local-release.py verify "$PWD/dist/inspector/0.2.1" --source` and
passing `agent-lint-md.sh`, `agent-lint-docs.sh`, and `mkdocs build --strict`.
