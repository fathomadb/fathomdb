# Slice 0 current environment register

Read-only probes November 10, 2026, approximately 13:35 UTC; these observations
expire as resource use changes. No installation, build or product test ran.

| Area | Declared/source | Observed / proposal |
| --- | --- | --- |
| Git/base | .29 `6edc1155f`; completed .28 `f43435241` | Branch `llm/0.8.29-chat` and origin/dev equal; origin/main/.28 ancestor. New attached checkout is the planning writer. Remote refs not freshly fetched. |
| Versions | W 0.8.28; independent E 0.8.0 | Retain until approved coordinated W bump; no lockstep E bump. |
| Rust | [Toolchain](../../../../../rust-toolchain.toml) 1.95.0; MSRV 1.95 | Active rustc/cargo 1.95.0; clippy/rustfmt installed. Retain. |
| Node/npm | [.nvmrc](../../../../../.nvmrc) 25.9.0; TS >=25 <26; npm 11.12.1 | Active 26.8.2/12.0.2 mismatch. Propose command-local pins and owned locked installs. |
| Python | Package >=3.10; developer >=3.11 path; Ruff 0.15.17/Pyright 1.1.410 | System 3.12.3, uv 0.11.6. No owned venv or native package; no product Python readiness. |
| Mutable dependency/build directories | Checkout ownership required | No `.venv`, root `node_modules`, TS `node_modules`, or `target`. Old checkout's shared npm symlink observation no longer applies here. |
| Host | windchill3 Linux x64 | Linux 7.0.0-42; 24 logical CPUs; RAM 62 GiB, 37 available; swap 7.4/8 GiB occupied. No current thrashing claim from occupancy alone. |
| Capacity | Existing plain preflight floor >=10 GiB | Root/tmp about 16 GiB free, 99% used; /mnt/data 914 GiB free. Plain health passes, but no measured native/CUDA peak budget. |
| Toolkit | Primary 12.6 fixed | `/usr/local/cuda` resolves 12.6; direct nvcc 12.6.68 works, not on PATH. Select per command only. |
| GPU | R580/3090 venues; never K620 | nvidia-smi exit 9 cannot communicate in this executor. Driver/UUID/ordinal/readiness unknown; no repair inferred. |
| Inputs | Actual corpora/model digests required | This checkout has data READMEs only; primary cold-data symlink target unavailable here. No dataset readiness. |
| Release state | Activate .29 only after 9 acceptance, in 10 | release-current.py exits 0 empty; no .29 state/board. Plain preflight passes; strict worktree/dependency preflight cannot yet authorize dependent work. |

Commands: `git rev-parse`, `git merge-base --is-ancestor`, `rustc --version`,
`cargo --version`, `rustup component list --installed`, `node --version`,
`npm --version`, `python3 --version`, `uv --version`, `df -h`, `free -h`,
`readlink /usr/local/cuda`, direct nvcc, `nvidia-smi`, `release-current.py`,
plain `preflight.sh`, and path existence checks. Recheck before execution.
