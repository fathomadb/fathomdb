---
title: Slice 135 raw archive transfer — 2026-10-08
status: LOCAL_COPIES_VERIFIED
target_release: 0.8.27
---

# Slice 135 raw archive transfer — 2026-10-08

The three compressed archives below are in the established local evaluation
store outside the Slice 135 worktree. SHA-256 and `zstd -t` were checked on
2026-10-08. The Phase 1 copy has the same digest as the separately audited
self-contained bundle in the [Phase 1 retention review](results/2026-10-08-raw-retention-review/README.md).

| Archive in `/home/coreyt/projects/fathomdb/data/corpus-data/eval/` | SHA-256 | Contents |
| --- | --- | --- |
| `slice135-phase1-raw-2026-10-08.tar.zst` | `9674227303546aa9a7bffb9152ebc2c25fa4dfeaad6ec89469d455ad2c15ea8e` | Eight exact-source engine, Python and TypeScript campaign archives with self-contained model links and manifests. |
| `slice135-phase2-raw-2026-10-08.tar.zst` | `1817c5a961e9a4e419b78eaef63a9680e7ba2bbf821600f4322054b6b343bfdb` | Eleven Phase 2 result directories, including invalid attempts and SQLite databases. |
| `slice135-gpu-query-sample-raw-2026-10-08.tar.zst` | `f61c7c958cc97af1ec252cd788076dc544de9b781da0f79c70e92f44c5c0ef33` | Frozen GPU sample, installed artifact, CPU/GPU results, device receipts and invalid first probe. |

These are local copies on the same host, not an off-host backup or a public
publication. The Phase 2 source payload store is separately retained at
`/home/coreyt/projects/fathomdb/data/corpus-data/raw`; the archive does not
contain those source payloads. Keep the licensed corpus data and derived
verbatim text outside Git and use the pinned protocol hashes to identify it.
