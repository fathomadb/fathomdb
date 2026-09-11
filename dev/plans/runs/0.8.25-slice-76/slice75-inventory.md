---
title: Slice 75 carry-forward inventory
status: COMPLETE
---

# Slice 75 carry-forward inventory

This is the bounded, repository-only inventory required by Slice 76. It reads
the checked-in Slice 75 manifest and status; it does not infer execution from
source, search outside documented receipt roots, or rerun any cell.

The declared receipt root `dev/plans/runs/0.8.25-slice-75/` has no tracked
files. Therefore every original Slice 75 cell remains unverified for Slice 85
unless an applicability audit accepts another retained receipt.

| Cell | Checked-in receipt | Disposition | Owner |
| --- | --- | --- | --- |
| `default-tree` | Missing | Unverified/unrun | Slice 85 |
| `final-interactions` | Missing | Unverified/unrun | Slice 85 |
| `schema26-upgrade` | Missing | Unverified/unrun | Slice 85 |
| `ac021` | Missing | Unverified/unrun | Slice 85 |
| `ac059b` | Missing | Unverified/unrun | Slice 85 |
| `ac034ab` | Missing | Unverified/unrun; AC-034c remains unavailable | Slice 85 |
| `performance` | Missing | AC-020 routes through Slices 77/80; remaining performance cells remain unverified | Slices 77/80/85 |
| `eu7-real` | Missing | Unverified/unrun | Slice 85 |
| `model-cache` | Missing | Unverified/unrun | Slice 85 |
| `model-ts` | Missing | Unverified/unrun | Slice 85 |
| `model-engine` | Missing | Unverified/unrun | Slice 85 |
| `model-python` | Missing | Unverified/unrun | Slice 85 |
| `model-cli` | Missing | Unverified/unrun | Slice 85 |
| `rust-release-build` | Missing | Unverified/unrun | Slice 85 |
| `rust-leaf-packages` | Missing | Unverified/unrun | Slice 85 |
| `cli-installed-smoke` | Missing | Unverified/unrun | Slice 85 |
| `linux-artifact-build` | Missing | Unverified/unrun | Slice 85 |
| `linux-artifact-current-smoke` | Missing | Unverified/unrun | Slice 85 |
| `linux-runtime-floor-smokes` | Missing | Unverified/unrun | Slice 85 |
| `linux-cuda-package` | Missing | Unverified/unrun | Slice 85 |
| `global-01-native` | Missing | Unverified/unrun | Slice 85 |
| `windows-runner-preflight` | Missing | Unverified/unrun | Slice 85 |
| `hosted-native-validation` | Missing | Unverified/unrun | Slice 85 |
| `jetson-tegra` | Missing | Unverified/unrun | Slice 85 |
| `mkdocs` | Missing | Unverified/unrun | Slice 85 |
| `hosted-ci` | Missing | Unverified/unrun | Slice 85 |

The four explicitly retained inputs are present and byte-identical to their
manifest hashes:

| Retained input | Receipt state | Applicability owner |
| --- | --- | --- |
| Slice 71 write recovery | Present, SHA-256 `c356a7f6...dabd5` | Slice 85 |
| Slice 71 AC-072 | Receipt and execution binding present, SHA-256 `4291cc2a...190` and `7e61a312...d22` | Slices 77/80/85 |
| Slice 72 CE CPU/CUDA | Present, SHA-256 `9cee6f67...7e14` | Slice 85 |
| Slice 73 Windows N-API | Present, SHA-256 `4020aad5...bbe` | Slice 85; current hosted cell replaces routing evidence |

No missing Slice 75 receipt is treated as a failed product result. It is
simply absent evidence to be resolved by the deduplicated Slice 85 matrix.
