---
title: 0.8.24 Slice 60 — evidence matrix
status: COMPLETE
target_release: 0.8.24
---

# Slice 60 evidence matrix

| Target | Exact identity | Source | Evidence required | State |
| --- | --- | --- | --- | --- |
| Jetson Tegra | `fathomdb==0.8.24+tegra` | first-party Pages simple index | retained artifact `wheel.sha256` and Pages link both report `652ad6926b17c9580365b012ec9cb925fa1aabc6fe83047874c718dc5c5e5897`; clean venv lifecycle and forced CUDA witness | complete: owner-authorized direct smoke on Jetson `10.83.10.13`; durable records in `evidence/` |
| Python CPU | `fathomdb==0.8.24` | PyPI | existing publisher/idempotency fixtures; post-tag public smoke | owner-only post-publication |
| Cargo/npm/CLI CPU | exact 0.8.24 packages | crates.io/npm/GitHub release | existing publisher/idempotency fixtures; post-tag smokes | owner-only post-publication |
| Existing Windows CPU artifacts | inherited release workflow paths | existing release routes | existing preservation contracts only | preserved; no new work by `seq-258` |

No row is marked complete solely from an Actions artifact, an index listing, or
a source-tree test.
