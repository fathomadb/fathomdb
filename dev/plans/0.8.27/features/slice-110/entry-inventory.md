---
title: Slice 110 current-entry native inventory
status: FROZEN
target_release: 0.8.27
source_commit: 9340a8246e39127c255b55a3f97b674553eb6908
---

# Slice 110 entry inventory

This capture was taken from clean source `9340a824` before implementation.
The checked `npm run build:native` wrapper built production NAPI with pinned
Node 25.9.0, npm 11.12.1 and Rust 1.95.0. The direct sandbox run could not
spawn `/bin/sh`; the identical command passed in the permitted executor.
The native package version remains 0.8.26 until release staging.

| Oracle | Frozen evidence |
| --- | --- |
| Rust source items, attributes, impls and methods | [Entry source items](entry-source-items.txt), 465 lines, SHA-256 `02abcedf16c6aec006b2afcc7411e18b088b52b2278dc8cd55a9d0713774410b`. |
| Exact source-item owner assignment | [Entry source owner map](entry-source-owner.tsv), one row per captured source item; zero unassigned. Root keeps crate imports and module declarations only. |
| Generated production NAPI declaration | [Entry native declaration](entry-native.d.ts), 950 lines, SHA-256 `7188d6aa2fa249a7f6cf717c5573cf1e0877b3b59bbe01c5322d0632a8348f5f`. |
| Loaded native artifact | `src/ts/fathomdb.linux-x64-gnu.node`, SHA-256 `c53e326f75cc5c65317bc958eb7f50a32831e62bace5ae55b08e2e9636d42393`; [exports and Engine prototype](entry-runtime.json), SHA-256 `8d9677dacaa2a841cd27989d4debcaaa4120784af853e47b85a493f7f068c123`. |

There are 17 top-level runtime exports: one Engine constructor and 16 free
functions. The Engine prototype has 44 own names including `constructor`.
Generated declarations and runtime identity are separate oracles. Test hooks
are absent from this production build. The only approved intentional delta
before structural movement is the accepted TypeScript subscriber successor;
its signature/payload changes must be distinguished from moved registrations.

The source-reader scan covered `scripts/`, `dev/tools/` and `.github/workflows/`.
`dev/tools/surface_comparator.py` builds and reads generated NAPI declarations;
it does not parse `fathomdb-napi/src/lib.rs`. Release scripts and tests read
the crate manifest, binary or package metadata. No literal NAPI source-file
reader requires a path retarget at entry. A new source inventory helper must
fail a hidden-owner negative fixture before use as acceptance evidence.

The planned platform rows are Linux x64 GNU, Linux x64 CUDA, Linux arm64 GNU,
macOS x64, macOS arm64 and Windows x64 MSVC, with the existing release
workflow's feature selection. The current Linux production build qualifies
source/declaration/runtime entry only; it is not an installed package or
platform matrix receipt.
