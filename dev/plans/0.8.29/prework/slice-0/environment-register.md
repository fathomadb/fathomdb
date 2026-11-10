# Slice 0 current environment register

Inherited read-only probes November 10, 2026, approximately 13:35 UTC; these
observations expire as resource use changes and were not rerun for this
reconciliation. Earlier source/Git inspection used product-source baseline
`6edc1155f`; a November10 follow-up recorded planning-only baseline `40d82c2e7`
on the same branch, with no intervening product source/configuration change.
These are dated observations: derive actual HEAD/branch from Git before execution.
Remote refs were not fetched; live tools/resources were not refreshed.
No installation, build or product test ran.

| Area | Declared/source | Observed / proposal |
| --- | --- | --- |
| Git/base | Earlier product-source baseline .29 `6edc1155f`; completed .28 `f43435241`; November10 planning baseline `40d82c2e7` | Earlier inspection found branch `llm/0.8.29-chat` equal to local origin/dev refs, with origin/main/.28 ancestral. Follow-up recorded the same branch at the planning baseline. Ref equality/ancestry is historical, not a live remote claim; no fetch ran. Attached checkout is the planning writer. Recheck actual HEAD/branch/refs from Git before use. |
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
| Release state | Activate .29 in10 after required9 dispositions and10 design review | release-current.py exits 0 empty; no .29 state/board. Plain preflight passes; strict worktree/dependency preflight cannot yet authorize dependent work. |

Commands: `git rev-parse`, `git merge-base --is-ancestor`, `rustc --version`,
`cargo --version`, `rustup component list --installed`, `node --version`,
`npm --version`, `python3 --version`, `uv --version`, `df -h`, `free -h`,
`readlink /usr/local/cuda`, direct nvcc, `nvidia-smi`, `release-current.py`,
plain `preflight.sh`, and path existence checks. Recheck before execution.

Preparation ownership and disposition:

- 10 owns command-local Node25.9.0/npm11.12.1 and checkout-owned Python/native,
  root/TS modules and build/scratch preparation; scope is approved by batches1/2.
  Missing owned tools are preparation prerequisites, not evidence of product
  failure. 1 supplies selected contained Rustls/DOMPurify and dependency decisions.
- Retain Rust1.95 and Python package floor>=3.10;10 selects an owned developer
  interpreter>=3.11 with pinned Ruff/Pyright. Current generic bootstrap/build
  editable installs must be reconciled before linked-worktree invocation.
- 9 retains the coordinated W bump decision;10 performs any approved bump.
  E0.8.0 remains independent; format37 remains unchanged, with no .29 migration.
- 10 owns exact /mnt/data path/access and measured build/output peak budget,
  including root/tmp spill space; approved use does not imply current capacity.
  Keep the existing >=10GiB preflight floor. No heavy build starts until10 has
  set a phase-specific reservation and numeric stop threshold from actual needs.
- 30 and affected qualification owners resolve exact corpus/license/hash and model
  revision/digest;120/130 select artifact/venue coverage; actual faults require a
  permitted venue. Incomplete evidence goes to9, not an invented ready status.

The archived checkout's root-module symlink and18GiB figure are superseded as
checkout observations. The inherited replacement inventory above reports no
modules and16GiB; both tool and capacity facts must be rechecked before use.
