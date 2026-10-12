---
title: ADR-0.8.28-cuda-version-support-policy
date: 2026-10-11
target_release: 0.8.28
desc: FathomDB GPU artifacts support CUDA 12.x and 13.x libraries on Turing or newer GPUs; 12.6 is the shipping floor, 13.0 the next artifact, 11.x unsupported; CUDA version usage and importance are re-reviewed at every release
blast_radius: CUDA release artifacts (x86_64 and Jetson); release plans; scripts/release/cuda-artifact-contract.sh; GPU documentation; windchill3 test host
status: accepted (owner, 2026-10-11)
---

# ADR-0.8.28 — CUDA version support policy

## Context

Which CUDA versions FathomDB builds and supports had never been decided:
12.6 was what `windchill3` had installed, and the 0.8.23 release contract
froze it for x86 and Jetson alike (Slice 31 plan § 15 Q6). The 2026-10-11
usage research measured how CUDA versions are actually used
([cuda-version-adoption.md](../plans/0.8.28/features/slice-31/research/cuda-version-adoption.md))
and proposed an importance per version
([cuda-version-support-proposal.md](../plans/0.8.28/features/slice-31/research/cuda-version-support-proposal.md)).
The owner accepted the proposal's importance ratings and recommendations.

Facts that bound the choice:

- FathomDB artifacts link no CUDA library at load time
  (`ADR-0.8.25-driverless-cuda-runtime-linkage.md`); cudarc loads the
  driver and cuBLAS lazily. A host therefore needs a driver new enough for
  the build's PTX and a cuBLAS of the build's major.
- The x86 kernels are PTX for compute capability 7.5, so only Turing or
  newer GPUs can run FathomDB on CUDA; the Orin build targets 8.7.
- A 12.6 build emits PTX ISA 8.5 (`.version 8.5`, `.target sm_75`, measured
  on a release build's `candle-kernels` output, 2026-10-11).
- Driver R580 is the last branch that supports Maxwell, Pascal and Volta,
  and CUDA 13.x applications run on R580 or later.

## Decision

1. **Library floor: CUDA 12.x.** FathomDB GPU artifacts support CUDA 12.x
   and 13.x libraries. CUDA 11.x is unsupported.
2. **Importance per version** (owner-accepted, 2026-10-11):

   | Version | Importance | Action |
   | --- | --- | --- |
   | 11.x | none | unsupported; covered only by cudarc's own compile matrix |
   | 12.6 | tier 1, shipping floor | the 0.8.x artifact for x86 and Jetson (JetPack 6) |
   | other 12.x (12.0–12.9) | covered by the 12.6 build | no separate build; library compatibility with older and newer 12.x cuBLAS is tested |
   | 13.0 | tier 1 next | qualified in 0.8.28; a second artifact beside 12.6 in the release after 0.8.28 |
   | 13.2 | low | no separate build; the 13.0 build runs on its drivers (JetPack 7.2) |
   | 13.4 and later minors | compile-check only | never build artifacts with them (their PTX needs a newer driver) |

3. **Build with the lowest toolkit of each supported major** that the target
   platforms share (12.6 today; 13.0 for the 13.x artifact), so the PTX
   driver floor stays as low as the platforms allow.
4. **Driver floor documented per artifact.** The 12.6 artifact documents
   NVIDIA's driver for CUDA 12.6 (Linux 560.28.03 or later). PTX ISA 8.5
   appears to load from the CUDA 12.5 driver (555.42.02) on secondary
   evidence; that is untested and not promised.
5. **GPUs: Turing or newer on x86** (GeForce RTX 20 and GTX 16 series,
   Quadro RTX and T series, Tesla T4, and every later architecture).
6. **Every release re-reviews CUDA usage and importance** by the procedure
   in [cuda-version-support-review.md](../design/cuda-version-support-review.md),
   records the review in its release plan, and amends this ADR (or a
   successor) when an importance rating or action changes.
7. **`windchill3` stays on the R580 driver branch.** Its Quadro K620
   (Maxwell, sm_50) loses support in every later branch, and the host's
   driver is never changed. CUDA 13.1+ PTX and R590+ behaviour are
   therefore never measured on `windchill3`. The K620 is below the x86
   kernel target, so FathomDB GPU tests on `windchill3` use the RTX 3090s.

## Consequences

- Release artifacts stay on 12.6 in 0.8.28; a 13.0 artifact is planned
  work for the next release (artifact naming and `auto` selection between
  the two are designed there).
- Users whose environments carry only CUDA 13 libraries (for example, with
  PyTorch 2.11 or later) need a CUDA 12 cuBLAS for the 12.6 artifact until
  the 13.0 artifact ships; the failure must be typed and plain.
- cudarc must load only libraries of the major it was built for (Slice 31,
  cudarc fork plan); a wrong-major cuBLAS is never loaded silently.
- A host that carries a second CUDA toolkit or library set follows
  [the multiple-installation cautions](../notes/cuda-multiple-installations.md):
  toolkit-only, user-local, no global `LD_LIBRARY_PATH`, and the loaded
  library checked rather than assumed.

## Alternatives rejected

- **Support 11.x:** about 3 % of downloads and falling; the JetPack 5 users
  it would serve can move to JetPack 6.
- **Build with the newest 13.x toolkit:** its PTX would need R615-class
  drivers and lock out R580–R610, including the LTS branch.
- **Move the single artifact to 13.0 now:** drops JetPack 6 Orin and hosts
  on drivers 555–575.
