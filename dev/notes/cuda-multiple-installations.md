---
title: Multiple CUDA installations on one host — cautions and the safe recipe
status: OPERATIONAL_REFERENCE
source_date: 2026-10-11
---

# Multiple CUDA installations on one host

This note applies to any FathomDB host that carries more than one CUDA
toolkit or runtime-library set. It also covers the Python wheels that bring
CUDA libraries with them. The governing policy is
[ADR-0.8.28-cuda-version-support-policy](../adr/ADR-0.8.28-cuda-version-support-policy.md).
`windchill3` has its own hard rule: never change the driver (R580) or the
primary CUDA (12.6). That rule lives in the memory entry
`windchill3-never-change-driver-or-primary-cuda.md`.

## The three layers

1. **The driver.** It supplies `libcuda.so.1` and is the only part shared by
   the whole host. One driver serves every toolkit at or below its CUDA
   version. `nvidia-smi`'s "CUDA Version" is the newest CUDA that driver
   supports. It does not mean a toolkit is installed.
2. **A toolkit.** This means `nvcc`, the headers and the user-space libraries
   (`libcudart`, `libcublas`, `libnvrtc` and others), installed in a
   versioned directory. Several toolkits can coexist.
3. **Library-only copies.** Python wheels such as `nvidia-cuda-runtime` and
   `nvidia-cublas` put libraries in `site-packages/nvidia/cu13/lib/` or
   `.../cuda_runtime/lib/`. They have no compiler and are used only when a
   loader is pointed at them.

The loader picks a library in this order:

1. the executable's `RPATH`/`RUNPATH`;
2. `LD_LIBRARY_PATH`;
3. the `ldconfig` cache built from `/etc/ld.so.conf.d/`;
4. the default directories.

FathomDB links no CUDA library at load time (ADR-0.8.25). cudarc opens each
library by name at run time, so this order decides which CUDA FathomDB
actually uses.

## Cautions — the wrong ways

1. **Never let a toolkit install bring a driver.** A runfile installs a driver
   unless told not to. The apt meta-packages `cuda` and `cuda-drivers` also
   pull one in. On `windchill3`, a driver newer than R580 drops the Quadro
   K620 (ADR decision 7). Install extra toolkits toolkit-only:
   [NVIDIA forum](https://forums.developer.nvidia.com/t/installing-additional-cuda-versions/268317).
2. **Never add a second toolkit through apt on a host whose `/usr/local/cuda`
   is managed by `update-alternatives`.** NVIDIA supports side-by-side apt
   installs and switches between them with `update-alternatives --config cuda`
   ([installation guide](https://docs.nvidia.com/cuda/cuda-installation-guide-linux/index.html)).
   On `windchill3`, `/etc/ld.so.conf.d/000_cuda.conf` (package
   `cuda-toolkit-config-common`) feeds `/usr/local/cuda/targets/x86_64-linux/lib`
   to `ldconfig`. If a new alternative took over the link, every process on
   the host would load the new major's libraries. Use a user-local runfile
   install instead.
3. **Never install into the shared `cuda` directory or over another
   version's directory.** Use one versioned directory per toolkit. One user
   who installed into the shared folder lost `nvidia-smi`'s connection to the
   driver
   ([Towards Data Science](https://towardsdatascience.com/managing-multiple-cuda-versions-on-a-single-machine-a-comprehensive-guide-97db1b22acdc/)).
4. **Never put two CUDA library directories in `LD_LIBRARY_PATH`, and never
   set it globally** (in `.bashrc` or `.profile`). Mixed paths, or paths that
   disagree with `ldconfig`, silently load the wrong `libcudart`
   ([NAMD list](https://tcbg.illinois.edu/Research/namd/mailing_list/namd-l.2013-2014/2644.html),
   [NVIDIA forum](https://forums.developer.nvidia.com/t/cuda-driver-and-runtime-version-mismatch-problem/17908)).
   Build tools can also find a system copy ahead of the one configured
   ([TVM forum](https://discuss.tvm.apache.org/t/solved-incorrect-libcuda-so-when-multiple-versions-of-cuda-exist-problem-and-solution/886)).
   Set it per command, with exactly one CUDA library directory.
5. **Never trust the version a tool reports. Check the library that actually
   loaded.** `nvcc --version` can report one release while a build still
   resolves another release's runtime
   ([NVIDIA forum](https://forums.developer.nvidia.com/t/compiling-cuda-looks-for-wrong-cuda-version/38654)).
   The bitsandbytes maintainers say to check `PATH`, `LD_LIBRARY_PATH` and
   `CUDA_HOME`, and to compare the loaded libraries with `nvidia-smi`
   ([bitsandbytes docs](https://huggingface.co/docs/bitsandbytes/en/errors)).
   FathomDB records the loaded path and version (`/proc/self/maps` or
   `dladdr`) in receipts and, under Slice 31 D-18, in the CUDA device report.
6. **Never assume a silent install is complete.** One `--silent --toolkit
   --toolkitpath` install came out with libraries missing
   ([NVIDIA forum](https://forums.developer.nvidia.com/t/silent-unattended-install-of-cuda-toolkit-and-sdk/21579)).
   Verify it afterwards (see the recipe below).
7. **Unversioned library names are a trap.** `ldconfig` registers names such
   as `libcublas.so` and `libcudart.so` for whatever directory it is fed. On
   `windchill3` they point at 12.6. Stock cudarc tries the unversioned name
   first (`get_lib_name_candidates`, cudarc `src/lib.rs`), so a CUDA 13 build
   run here silently loads cuBLAS 12. Load by the built major's soname. That
   is cudarc fork item 9 and the second proposal (`proposal/lib-major`).
8. **Never borrow another project's library-only copies.** A virtual
   environment's `nvidia/cu13/lib` belongs to that environment's PyTorch and
   moves when PyTorch moves. For example, FathomDB's own `.venv` carries
   cuBLAS 13.1.1, not the pinned 13.0.x. Build a scratch environment with
   exact pins instead (Slice 31 plan, W0 step 4).

## Safe recipe for an extra toolkit (no sudo)

1. Record the guard: the driver version from `nvidia-smi` and
   `readlink -f /usr/local/cuda`.
2. Read the runfile's `--help` for its exact flags. NVIDIA documents
   `--toolkit`, `--toolkitpath`, `--defaultroot`, `--override`,
   `--no-man-page` and `--tmpdir`. No documented flag skips the
   `/usr/local/cuda` link, but a non-root install cannot write it.
3. Run it as the user, never with sudo:
   `sh cuda_<ver>_linux.run --silent --toolkit --toolkitpath=$HOME/opt/cuda-<ver> --no-man-page --tmpdir=<scratch>`.
4. Do not edit shell profiles. Use the toolkit per command, for example
   `CUDA_PATH=$HOME/opt/cuda-<ver> PATH=$HOME/opt/cuda-<ver>/bin:$PATH LD_LIBRARY_PATH=$HOME/opt/cuda-<ver>/lib64 <command>`.
5. Verify:
   - `nvcc --version` from the new path;
   - the expected `lib64/libcudart.so.<major>`, `libcublas.so.<major>` and
     `libnvrtc.so.<major>` exist;
   - a test binary's loaded-library report names the new directory.
6. Re-check the guard. Both values must be unchanged.

## `windchill3` inventory (2026-10-11)

- Driver 580.178.04 (supports up to CUDA 13.0). GPUs: 2× RTX 3090 and a
  Quadro K620.
- Toolkit: 12.6 only, from apt packages (`cuda-*-12-6`). `/usr/local/cuda` →
  `/etc/alternatives/cuda` → `/usr/local/cuda-12.6`. No `nvcc` on `PATH`.
- `ldconfig`: `000_cuda.conf` → `/usr/local/cuda/targets/x86_64-linux/lib`
  and `988_cuda-12.conf` → `/usr/local/cuda-12/...`. Both the unversioned and
  the `.so.12` names resolve to 12.6.
- No CUDA 13 toolkit. CUDA 13 libraries exist only as wheels inside virtual
  environments: FathomDB's `.venv` (runtime 13.0.96, NVRTC 13.0.88, cuBLAS
  13.1.1.3), memex, nemo-asr and uv caches. None of them is on the loader
  path.
