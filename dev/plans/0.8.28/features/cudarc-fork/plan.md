---
title: FathomDB 0.8.28 — cudarc from the coreyt/cudarc fork, through crates.io (D28-13) plan
status: PROPOSED (2026-10-11; owner direction, not yet approved for execution)
target_release: 0.8.28
observed_on: 2026-10-11
---

# 0.8.28: cudarc from the coreyt/cudarc fork, through crates.io

This plan moves FathomDB's cudarc changes out of the vendored tree
`third_party/cudarc-0.19.7` and onto the owner's fork
[coreyt/cudarc](https://github.com/coreyt/cudarc), then publishes the fork
to crates.io so that crates.io consumers of `fathomdb` get the patched
cudarc too. It is the working plan for D28-13 in
`dev/plans/0.8.28-draft-scope.md` until the 0.8.28 plan, release-state
file and board exist; the slice number is assigned there.

## 1. Owner direction (2026-10-11)

- **Fork-first.** All cudarc work happens on `coreyt/cudarc`; all Candle
  work on `coreyt/candle-fathomdb`. Nothing is proposed to upstream
  (`chelsea0x3b/cudarc`, `huggingface/candle`) until the change has been
  used and tested further in the product, and only with owner sign-off.
  Ruling 38's "upstream-first in upstream style" still governs the
  *shape* of the commits, not where or when they are posted.
- **Consumption is a git dependency on the fork (choice 2b),** replacing
  the vendored tree. All six vendored items move to the fork, including the
  local-only aarch64 synchronous fallback (items 1–4).
- **End state: crates.io carries the patch, from the fork.** A crates.io
  consumer of `fathomdb` resolves the patched cudarc with no
  `[patch.crates-io]` entry of its own.
- The approved release task "drop the Candle git patch from the
  `tegra-pool` registry check" is folded into § 4 W4: once the fork crates
  are on crates.io, every Candle and cudarc git patch is dropped.

## 2. Current state (verified 2026-10-11 on `release/0.8.28` at `227d0b816`)

| Fact | Evidence |
| --- | --- |
| The workspace patches `cudarc` to the vendored 0.19.7 tree | root `Cargo.toml` `[patch.crates-io]` `cudarc = { path = "third_party/cudarc-0.19.7" }` |
| The vendored delta is two patch files: items 1–4 (598 lines) and items 5–6 with tests (676 lines) | `third_party/cudarc-0.19.7/FATHOMDB-PATCH.md`; `fathomdb-alloc-fallback.patch`, `fathomdb-private-pool.patch` |
| Candle (fork rev `859b8ea1`) depends on `cudarc = "0.19.4"` (caret) from crates.io | fork `Cargo.toml` workspace dependency |
| `fathomdb-embedder` depends directly on `cudarc = "=0.19.7"` on aarch64 Linux, with the marker feature `fathomdb-private-pool` so registry cudarc fails at resolution | `src/rust/crates/fathomdb-embedder/Cargo.toml` |
| `ug-cuda` pulls a separate `cudarc` 0.17.8 from crates.io; it is not patched | `Cargo.lock` |
| Published `candle-core-fathomdb` 0.10.3 depends on upstream `candle-kernels` 0.10.2 from crates.io; the reviewed fork `candle-kernels` (static CUDART boundary, `CUDA_PATH` rerun) reaches only workspace builds through a git patch | fork `Cargo.toml`; `cargo info candle-core-fathomdb@0.10.3` |
| `coreyt/cudarc` has only `main` = upstream `main` = tag `v0.19.10` (`ec1cac71`); upstream tag `v0.19.7` is `3e5d38b5` | `git ls-remote` |
| crates.io: `cudarc` latest 0.19.10; `candle-kernels` latest 0.11.0; the names `cudarc-fathomdb` and `candle-kernels-fathomdb` are free | `cargo search`, `cargo info` |

So today a crates.io consumer of `fathomdb` gets **unpatched** cudarc: no
pool primitive, no aarch64 fallback, and the embedder's marker feature makes
the `tegra-pool` feature fail to resolve for them.

## 3. Target state

```text
fathomdb-embedder ──(cudarc = { package = "cudarc-fathomdb" })──┐
candle-core-fathomdb 0.10.4 ──(cudarc = { package = "cudarc-fathomdb" })──┤
                                                             cudarc-fathomdb (crates.io,
                                                             published from coreyt/cudarc)
candle-core-fathomdb 0.10.4 ──(candle-kernels = { package = "candle-kernels-fathomdb" })   [if § 7 Q2 = yes]
```

- `cudarc-fathomdb` is the fork's crate under a new package name with
  `[lib] name = "cudarc"`, so source code still says `cudarc::…`.
- One cudarc 0.19 in the graph. A guard fails the build if any crate still
  resolves registry `cudarc` 0.19.x (two copies compile, but Candle's and
  FathomDB's `CudaContext` types would differ and `from_context` breaks).
- No `[patch.crates-io]` entry for cudarc or Candle remains after W4.
  libsqlite3-sys is out of scope.
- `third_party/cudarc-0.19.7` is deleted; its item table and upstream status
  live in the fork (`FATHOMDB.md` on the fork branch), and the pinned-override
  gate checks the fork rev, then the registry version.

## 4. Work, in order

Each package lists its repo, branch, and how it is verified. **Order:
W1 → W2 → W3 → W4 → W5.** W3's failing tests can be written while W1 and W2
are in progress; W3 cannot go green until W2 has a pinnable rev.

### W1. cudarc fork branch (repo `coreyt/cudarc`)

1. Push upstream tag `v0.19.7` (`3e5d38b5`) to the fork; cut branch
   `fathomdb/0.19.7` from it.
2. Re-apply the vendored delta as small upstream-style commits: items 1–4
   (fallback) first, then items 5–6 (pool primitive) with their tests. One
   item per commit where the patch allows.
3. Packaging commit: package name `cudarc-fathomdb`, `[lib] name = "cudarc"`,
   repository/README/licence attribution to upstream, `FATHOMDB.md` carrying
   the item table (upstream status, removal path) moved from
   `FATHOMDB-PATCH.md`.
4. **Equivalence check:** the branch's `src/` equals
   `third_party/cudarc-0.19.7/src/` byte for byte (the vendored tree is the
   reference until W3 deletes it). Any difference is a defect, not a cleanup.
5. Tests: the vendored crate tests (`fathomdb_alloc_fallback`,
   `fathomdb_private_pool`) on x86_64 and on the AGX Orin; the
   `cuda-*`/no-std/Windows `cargo check` matrix the upstream notes require.

Version (§ 7 Q1) is set here; nothing is published yet.

### W2. Candle fork (repo `coreyt/candle-fathomdb`)

1. Branch `fathomdb/0.8.28-cudarc-fork` from `859b8ea1`.
2. Workspace dependency `cudarc = { package = "cudarc-fathomdb", git =
   "https://github.com/coreyt/cudarc", rev = "<W1 rev>" }` for development;
   switched to the crates.io version in W4.
3. If § 7 Q2 is yes: rename `candle-kernels` to `candle-kernels-fathomdb`
   (keep `[lib] name`), and point `candle-core-fathomdb` at it.
4. Tests: the fork's CUDA tests and the four FathomDB private-pool GPU tests
   on the Orin against the new rev.

### W3. FathomDB slice (repo `fathomadb/fathomdb`, branch `slice/0.8.28-<N>-cudarc-fork` off `release/0.8.28`, own worktree)

Red first, per AGENTS.md § 5:

1. **Red:** the pinned-override checker
   (`scripts/check-pinned-override-rot.py`) and its test expect a git-sourced
   `cudarc-fathomdb` at an exact rev and no `third_party/cudarc-*`; a
   single-cudarc guard (`cargo tree -i cudarc` finds no registry
   `cudarc 0.19.*`); the CUDA release contract
   (`scripts/check-cuda-release-contract.py`) expects the new sources.
2. **Green:** `[patch.crates-io]` points `cudarc-fathomdb` and the Candle
   crates at the W1/W2 revs; `fathomdb-embedder`'s direct dependency becomes
   `cudarc = { package = "cudarc-fathomdb", … }` with the same marker
   feature; delete `third_party/cudarc-0.19.7`; update
   `scripts/check-tegra-pool-registry-build.sh` (its consumer manifest no
   longer strips a cudarc patch), `dev/design/release.md`,
   `dev/design/delivery-requirements-map-20260821.md`, the root
   `Cargo.toml` comments, and an amendment to the Slice 30 ADR (index row 62)
   recording 2b.
3. Full `./scripts/agent-verify.sh`; Orin GPU tests and
   `doctor cuda-allocator` (`private`, 3 GiB) unchanged from Slice 30.
4. Codex review gate (fallback: independent review subagent), then merge
   into `release/0.8.28`.

### W4. crates.io publication (owner publishes; dry-run first)

Publication order is forced by dependencies:

1. `cudarc-fathomdb` from the W1 branch tip.
2. `candle-kernels-fathomdb` (if Q2) and `candle-core/nn/transformers-fathomdb`
   0.10.4 from the W2 branch, now depending on the crates.io
   `cudarc-fathomdb`.
3. FathomDB follow-up commit on `release/0.8.28`: drop every cudarc and
   Candle `[patch.crates-io]` entry; depend on the registry versions; the
   pinned-override gate switches from rev pins to exact registry versions.
4. **Acceptance:** `scripts/check-tegra-pool-registry-build.sh` on the Orin
   with **no** patch table — a scratch crates.io consumer builds
   `tegra-pool` and passes the GPU smoke. This is the end state in § 1.

### W5. Later, not in 0.8.28

After more in-product use: rebase the pool items onto upstream `main` on a
separate fork branch, pass upstream's CI matrix, and, with owner sign-off,
post the prepared package (`dev/plans/0.8.28/features/slice-30/upstream-cudarc-package.md`).

## 5. Branch management

| Repo | Branch | Cut from | Writer | Lands by |
| --- | --- | --- | --- | --- |
| `coreyt/cudarc` | `fathomdb/0.19.7` | tag `v0.19.7` (`3e5d38b5`) | implementer, own clone | stays; FathomDB pins revs, later the published version |
| `coreyt/candle-fathomdb` | `fathomdb/0.8.28-cudarc-fork` | `859b8ea1` | implementer, own clone | stays; publishes 0.10.4 |
| `fathomadb/fathomdb` | `slice/0.8.28-<N>-cudarc-fork` | `origin/release/0.8.28` | implementer, own worktree | codex gate, merge to `release/0.8.28` |

- One writer per checkout; fork clones live outside the FathomDB checkout.
- Fork branches are never rewritten after FathomDB pins a rev; fixes are new
  commits and new pins.
- Pushes to the forks and to FathomDB branches need no per-push prompt
  (owner, 2026-10-11). Publishing to crates.io stays an owner action with a
  dry-run first.

## 6. Acceptance criteria

- AC-1: the W1 branch `src/` equals the vendored tree; the vendored tests pass
  on x86_64 and the Orin.
- AC-2: the workspace graph has exactly one cudarc 0.19 (the fork's); the
  guard is red when a registry `cudarc 0.19.*` is introduced.
- AC-3: `third_party/cudarc-0.19.7` is gone and the pinned-override gate
  checks the fork rev (W3), then the registry version (W4).
- AC-4: Slice 30's Orin results hold: the four private-pool GPU tests pass
  and `doctor cuda-allocator` reports `private`, 3 GiB.
- AC-5: a crates.io consumer with no patch table builds `tegra-pool` and
  passes the GPU smoke on the Orin (W4 step 4).
- AC-6: no upstream issue, comment or PR is opened.

## 7. Open questions for the owner

1. **Version of `cudarc-fathomdb`.** Options: (a) start at `0.19.7`, the
   upstream base, and bump the patch number for each FathomDB revision —
   the Candle precedent shipped `0.10.3` on a `0.10.2` base; (b) rebase
   onto upstream `v0.19.10` first. Recommendation: (a) for 0.8.28 — it keeps
   W1's byte-equivalence check against the qualified tree; a rebase is new
   qualification work.
2. **Publish `candle-kernels-fathomdb`?** Without it, crates.io consumers get
   upstream `candle-kernels` 0.10.2, not the reviewed fork's static CUDART
   boundary and `CUDA_PATH` fix. Recommendation: yes, in the same W4 batch.
3. **Slice number** in the 0.8.28 plan.

## 8. Risks

- **Two cudarc copies.** Anything left on registry `cudarc` 0.19 compiles
  and breaks only at the `from_context` boundary; AC-2's guard covers it.
- **`ug-cuda`'s cudarc 0.17.8** stays separate. It is reached only through
  Candle's optional `candle-ug`; confirm FathomDB never enables it.
- **Non-aarch64 consumers** get the fork crate too. Items 1–4 are cfg-gated
  to aarch64 Linux and items 5–6 are opt-in, so other targets compile
  upstream's logic unchanged (as the vendored tree does today).
- **Licensing.** cudarc is MIT/Apache-2.0; the republished crate keeps both
  licences and upstream attribution.

## 9. Ledger

On approval, record the 2026-10-11 directions (fork-first, 2b, crates.io end
state, push policy) with `ledgerwrite`, and update
`third_party/cudarc-0.19.7/FATHOMDB-PATCH.md` item 5's status to
"on fork, not posted" until W3 deletes the file.
