---
title: FathomDB 0.8.28 — cudarc from the coreyt/cudarc fork, through crates.io (D28-13) plan
status: PROPOSED revision 2 (2026-10-11; base moved to cudarc v0.19.10; not yet approved for execution)
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
- **Base: upstream cudarc v0.19.10** (owner, 2026-10-11), because that is
  where an upstream memory-pool proposal has to land. The evaluation is in
  § 2.1.
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

### 2.1 Evaluation of v0.19.7 → v0.19.10 (2026-10-11)

Run in a scratch clone, nothing pushed; verified unless marked inference.

- **15 upstream commits, no merges.** Bug fixes: `70c5e46` (#591,
  `CudaSlice` drop/leak skip event waits unless multi-stream synchronization
  is managed) and `4af37b3` (#602, `alloc_pinned` leak on a failed assert).
  Additions: `alloc_pinned_with_flags`, `CudaEvent::try_is_complete`,
  `CudaContext::wrap_borrowed_stream`, `CudaFunction::cu_function`,
  `CudaViewMut::into_view`. One break: `CUgraphInstantiate_flags` became a
  bitmask newtype (#600). CUDA 13.3 and 13.4 support (features `cuda-13030`,
  `cuda-13040`). Default features and MSRV unchanged; nothing mentions
  Jetson, aarch64 or Tegra.
- **No upstream pool work landed.** `src/driver/result.rs` changed by 29
  lines, none pool-related; there is still no safe `CudaMemPool`, no
  allocation mode and no change to `has_async_alloc` or the default pool.
  Issue #536 is open. Open PR #594 (capture-scoped graph memory pool, cites
  #536) touches `CudaStream::alloc`, `leak` and drop; if it merges it would
  conflict with items 1–3 and 5 (inference).
- **The delta applies as text to v0.19.10.** Items 1–4: 11 hunks, offsets
  only. Items 5–6: the `src/` hunks apply; the `Cargo.toml` hunk fails on
  any git checkout (v0.19.7 too) because it was written against the
  normalized crates.io manifest — add the marker feature by hand. The one
  semantic contact is the `CudaSlice::drop` hunk now sitting under #591's
  gate; the two are independent (#591 gates event waits, item 1 gates the
  free).
- **Existing defect found on both bases.** `mem_pool.rs` does not compile
  with `cuda-13020` and later (`CUmemLocation_st` has no field `id`; it
  moved into `__bindgen_anon_1`), and `set_max_size`'s cfg list stops at
  `cuda-13020`, so the cap would be silently ignored on 13.3/13.4 once the
  first error is fixed. The Jetson's CUDA 12.6 is unaffected. Also a
  dead-code warning (`in_fresh_process`) on x86 that upstream's
  `clippy -D warnings` would reject.
- **Results on the rebased tree.** `cargo check` with `cuda-12060` passes;
  on an RTX 3090 all 62 non-ignored `driver::` tests pass, including the 19
  FathomDB tests (the 4 ignored peer/multi-GPU failures also fail on pristine
  v0.19.10). Candle fork `859b8ea1` (`candle-nn-fathomdb --features cuda`,
  nvcc 12.6) checks clean against it; none of the changed APIs are used by
  Candle. The aarch64-only paths of items 1–3 have not run on the rebased
  tree yet.

So today a crates.io consumer of `fathomdb` gets **unpatched** cudarc: no
pool primitive, no aarch64 fallback, and the embedder's marker feature makes
the `tegra-pool` feature fail to resolve for them.

## 3. Target state

```text
fathomdb-embedder ──(cudarc = { package = "cudarc-fathomdb" })──┐
candle-core-fathomdb 0.10.4 ──(cudarc = { package = "cudarc-fathomdb" })──┤
                                                             cudarc-fathomdb (crates.io,
                                                             published from coreyt/cudarc)
```

- `cudarc-fathomdb` is the fork's crate under a new package name with
  `[lib] name = "cudarc"`, so source code still says `cudarc::…`.
- One cudarc 0.19 in the graph. A guard fails the build if any crate still
  resolves registry `cudarc` 0.19.x (two copies compile, but Candle's and
  FathomDB's `CudaContext` types would differ and `from_context` breaks).
- No `[patch.crates-io]` entry for cudarc or the Candle core/nn/transformers
  crates remains after W4; `candle-kernels` keeps its fork git patch
  (§ 7 Q2). libsqlite3-sys is out of scope.
- `third_party/cudarc-0.19.7` is deleted; its item table and upstream status
  live in the fork (`FATHOMDB.md` on the fork branch), and the pinned-override
  gate checks the fork rev, then the registry version.

## 4. Work, in order

Each package lists its repo, branch, and how it is verified. **Order:
W1 → W2 → W3 → W4 → W5.** W3's failing tests can be written while W1 and W2
are in progress; W3 cannot go green until W2 has a pinnable rev.

### W1. cudarc fork branch (repo `coreyt/cudarc`)

1. Push upstream tag `v0.19.10` (`ec1cac71`, the fork's current `main`) to
   the fork; cut branch `fathomdb/0.19.10` from it.
2. Re-apply the vendored delta as small upstream-style commits: items 1–4
   (fallback) first, then items 5–6 (pool primitive) with their tests. One
   item per commit where the patch allows. The marker feature goes into the
   git `Cargo.toml` by hand (§ 2.1).
3. Fix commits, each red first: `mem_pool.rs` for `cuda-13020`+ (cfg-gate
   the `CUmemLocation` id, as upstream's `unified_memory.rs` does) and
   `cuda-13030`/`cuda-13040` in `set_max_size`; the `in_fresh_process`
   dead-code warning.
4. Packaging commit: package name `cudarc-fathomdb`, `[lib] name = "cudarc"`,
   repository/README/licence attribution to upstream, `FATHOMDB.md` carrying
   the item table (upstream status, removal path) moved from
   `FATHOMDB-PATCH.md`.
5. **Equivalence check:** the branch's diff from `v0.19.10` equals the
   vendored delta (`fathomdb-alloc-fallback.patch` +
   `fathomdb-private-pool.patch`) apart from hunk offsets, the hand-added
   marker feature and the step 3 fixes. Any other difference is a defect.
6. Tests: the vendored crate tests (`fathomdb_alloc_fallback`,
   `fathomdb_private_pool`) on x86_64 and on the AGX Orin; the
   `cuda-*`/no-std/Windows `cargo check` matrix the upstream notes require.

Version: `cudarc-fathomdb` `0.19.10` for the first publication; later
FathomDB revisions bump the patch number (the Candle precedent). Nothing is
published in W1.

### W2. Candle fork (repo `coreyt/candle-fathomdb`)

1. Branch `fathomdb/0.8.28-cudarc-fork` from `859b8ea1`.
2. Workspace dependency `cudarc = { package = "cudarc-fathomdb", git =
   "https://github.com/coreyt/cudarc", rev = "<W1 rev>" }` for development;
   switched to the crates.io version in W4.
3. Tests: the fork's CUDA tests and the four FathomDB private-pool GPU tests
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
3. Full `./scripts/agent-verify.sh`.
4. **Orin re-qualification for the new base** (Slice 30 qualified 0.19.7;
   about half a day to a day of Orin time, inference): the vendored-cudarc
   tests including the fresh-child-process table test
   (`scripts/tests/test_vendored_cudarc.sh` or its successor against the
   fork), the embedder `tegra-pool` tests, `doctor cuda-allocator`
   (`private`, 3 GiB), and one Node/V8-fragmented end-to-end embedding run
   confirming the private pool is selected, the synchronous fallback works
   and steady state is in Slice 30's stream-ordered band. Slice 110's
   driver-level teardown evidence is not repeated.
5. Codex review gate (fallback: independent review subagent), then merge
   into `release/0.8.28`.

### W4. crates.io publication (owner publishes; dry-run first)

Publication order is forced by dependencies:

1. `cudarc-fathomdb` from the W1 branch tip.
2. `candle-core/nn/transformers-fathomdb`
   0.10.4 from the W2 branch, now depending on the crates.io
   `cudarc-fathomdb`.
3. FathomDB follow-up commit on `release/0.8.28`: drop the cudarc and
   Candle core/nn/transformers `[patch.crates-io]` entries; depend on the registry versions; the
   pinned-override gate switches from rev pins to exact registry versions
   for those crates.
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
| `coreyt/cudarc` | `fathomdb/0.19.10` | tag `v0.19.10` (`ec1cac71`) | implementer, own clone | stays; FathomDB pins revs, later the published version |
| `coreyt/candle-fathomdb` | `fathomdb/0.8.28-cudarc-fork` | `859b8ea1` | implementer, own clone | stays; publishes 0.10.4 |
| `fathomadb/fathomdb` | `slice/0.8.28-<N>-cudarc-fork` | `origin/release/0.8.28` | implementer, own worktree | codex gate, merge to `release/0.8.28` |

- One writer per checkout; fork clones live outside the FathomDB checkout.
- Fork branches are never rewritten after FathomDB pins a rev; fixes are new
  commits and new pins.
- Pushes to the forks and to FathomDB branches need no per-push prompt
  (owner, 2026-10-11). Publishing to crates.io stays an owner action with a
  dry-run first.

## 6. Acceptance criteria

- AC-1: the W1 branch diff from `v0.19.10` equals the vendored delta plus the
  listed fixes; the crate tests pass on x86_64 and the Orin, and
  `cargo check` passes for `cuda-12060` through `cuda-13040`.
- AC-2: the workspace graph has exactly one cudarc 0.19 (the fork's); the
  guard is red when a registry `cudarc 0.19.*` is introduced.
- AC-3: `third_party/cudarc-0.19.7` is gone and the pinned-override gate
  checks the fork rev (W3), then the registry version (W4).
- AC-4: Slice 30's Orin results hold on the new base: the four private-pool GPU tests pass
  and `doctor cuda-allocator` reports `private`, 3 GiB.
- AC-5: a crates.io consumer with no patch table builds `tegra-pool` and
  passes the GPU smoke on the Orin (W4 step 4).
- AC-6: no upstream issue, comment or PR is opened.

## 7. Open questions for the owner

1. ~~Version of `cudarc-fathomdb`.~~ Ruled 2026-10-11: base v0.19.10,
   first publication `0.19.10` (§ 2.1, W1).
2. ~~Publish `candle-kernels-fathomdb`?~~ Ruled 2026-10-11: **no**, not now.
   Reasoning: Static CUDART linking is required for
   FathomDB-built artifacts (`ADR-0.8.25-driverless-cuda-runtime-linkage.md`,
   decision index row 52, REQ-068, after the 0.8.24 CPU-load defect), and
   those are always built in the workspace with the Candle git patch, so a
   published kernels crate changes nothing FathomDB ships. A crates.io
   consumer enabling `embed-cuda` gets upstream's dynamic `libcudart`, like
   any Candle CUDA user, and publishing the static variant would make them
   provide `libcudart_static.a` at build time. Revisit only if crates.io
   builds must load in CPU mode on CUDA-free hosts — a new requirement for
   an ADR first. The kernels stay pinned to the fork by git patch for
   workspace builds, and W4 keeps that one patch entry.
3. **Slice number** in the 0.8.28 plan.

## 8. Risks

- **Two cudarc copies.** Anything left on registry `cudarc` 0.19 compiles
  and breaks only at the `from_context` boundary; AC-2's guard covers it.
- **`ug-cuda`'s cudarc 0.17.8** stays separate. It is reached only through
  Candle's optional `candle-ug`; confirm FathomDB never enables it.
- **Non-aarch64 consumers** get the fork crate too. Items 1–4 are cfg-gated
  to aarch64 Linux and items 5–6 are opt-in, so other targets compile
  upstream's logic unchanged (as the vendored tree does today).
- **Upstream PR #594** (graph memory pool) overlaps items 1–3 and 5; the
  W5 proposal must be written to coexist with it.
- **The `cuda-13020`+ `mem_pool.rs` defect** is in today's vendored tree as
  well; it only bites a 13.2+ toolkit build with `tegra-pool`, which no
  shipped artifact uses. W1 step 3 fixes it.
- **Licensing.** cudarc is MIT/Apache-2.0; the republished crate keeps both
  licences and upstream attribution.

## 9. Ledger

On approval, record the 2026-10-11 directions (fork-first, 2b, crates.io end
state, push policy) with `ledgerwrite`, and update
`third_party/cudarc-0.19.7/FATHOMDB-PATCH.md` item 5's status to
"on fork, not posted" until W3 deletes the file.
