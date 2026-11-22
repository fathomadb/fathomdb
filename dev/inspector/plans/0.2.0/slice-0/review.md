---
title: Inspector 0.2.0 Slice 0 execution review
date: 2026-11-19
status: REVIEW
---

# Environment and project infrastructure

Executed under the owner's new Slices 0–9 commission, after reading the plan,
design, filled prompt, workflow, scope and applicable memory. Their earlier
“no execution during drafting” statements describe the previous planning turn.
Source is final planning HEAD `aef5b2eac19dbcf5850713081ccd427896909eea`.
The draft's older source SHA is adjusted to this HEAD; source/manifest content
is retained unchanged. Latest singular commission serializes all reports under
one writer instead of the draft's separate executor per slice.

## Checkout result

S0-BOOTSTRAP-01 is performed: branch `inspector/dev/0.2.0`, checkout
`/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0`, initial HEAD above.
Branch/path were absent from the worktree/ref inventory; source was clean and
unchanged, and `git merge-base --is-ancestor 6edc1155f039f87cd0a37660cb27ae4373bbce71 aef5b2eac19dbcf5850713081ccd427896909eea`
succeeded. All subsequent commands explicitly select the release checkout.
Plain health preflight and post-entry `preflight.sh --landing --min-disk-gb 15`
passed. The Engine-specific `--worktree`/dependency ladder is not used to
pretend Inspector 0.2.0 is the current Engine release. No rebase or Engine change.

## Observed inventory and limits

| Surface | Actual read-only observation | Consequence |
| --- | --- | --- |
| OS/resources | Ubuntu 24.04.5 x86_64, glibc 2.39; 24 available CPUs; 51 GiB available RAM; 143 GiB disk free | Current capacity exceeds four-core/8-GiB minimum; scratch >=2x actual fixture remains a per-run check. Shared host, no quiet-host/cgroup prerequisite. |
| Rust/C | rustc/cargo 1.95.0; cc and strace on PATH | Toolchain matches `rust-toolchain.toml`; ptrace execution not tested here. |
| Python | system 3.12.3; no checkout `.venv/bin/python` | Full verify cannot qualify; no borrowed environment. |
| JS | Node 26.8.2/npm 12.0.2 on PATH; TS declares Node >=25 <26/npm 11.12.1 | Shared SDK/tool contract mismatch, no runtime dependency of installed Inspector. |
| Browser | Playwright package 1.62.1 and chromium-1248 executable exist at tool defaults | Filesystem presence only; no browser launch or installed pairing qualification. |
| Product | Inspector manifest 0.1.0 unpublished, empty defaults, Engine defaults off; Engine packages 0.8.28/current-format source 37 | Retain exact composition. Browser/search/graph future work unchanged. |
| Packaging | `package.sh` selects GNU target; package README records 24.04/glibc 2.39 build | This artifact cannot establish Ubuntu 22.04 support; 70 must qualify actual supported artifact. |
| Markdown | Fresh checkout has no root node_modules; normal gate requires it | No installation or symlink created in proposal-only prework. Existing read-only tool invocation may check changed documents; normal gate limit remains explicit. |

`bootstrap.sh` is unsuitable for blind execution here: it creates a venv, runs
editable native install, npm installs and cargo-installs lychee. Worktree native
editable installation is prohibited by applicable memory. A later approved
setup must use a checkout-owned environment and isolated wheel build/install,
with pinned developer tools, rather than this shared-state recipe.

## Individual proposals for Slice 9

| Key / owner | Proposed result and observable acceptance | Necessity; risk; effort |
| --- | --- | --- |
| S0-ENV / Inspector developer setup, conditional 10 | Prepare checkout-owned Python >=3.11, pip and pinned dev tools; build native wheel into owned scratch and install it into that venv without editable rebinding; run preflight before source gates. Exact setup commands require packaging-contract review first. | Required before full source gate; medium; medium |
| S0-ABI / Inspector packaging, 10 prepares and 70 qualifies | Select existing Ubuntu 22.04-compatible x86_64 build/test executor; inspect required ELF symbol versions and run installed/default artifact on 22.04, offline launch/read/close. No host provisioning performed. | Required inherited support target; high; medium/unknown executor |
| S0-TOOLS / shared tooling, 1/7 aliases | Reconcile Node/TS contract in owning scope; preserve browser/tool pair provenance and configurable existing paths. Do not silently use incompatible Node or bundle Node/Python into product. | Required for affected SDK gates; medium; small/medium; external for SDK declaration changes |
| S0-CUDA / release policy owner | Bounded delegated read-only review performed and summarized below; current usage shares remain UNASSESSED. Optional further Engine evidence can be considered at 9; no unchanged-review ratification rule is invented. | Partial coverage; no driver/toolkit/rating changes; optional further evidence |

The quality roadmap applies real databases, independent identity/adjacency/L2/
lexical facts, property boundaries, targeted faults/cancel/resource tests and
browser/keyboard/human qualification. Defer global MC/DC, Engine fuzzing and
exhaustive allocation programs as already scoped. New evidence is the real
format-37 port, queue-budget regression, missing venv and ABI floor mismatch.
No GPU setup, external compute connection, install, build or runtime campaign ran.

Next: 1 owns actual pin/advisory research; 4 owns unchanged public cache-only CPU
feasibility; 7 owns CI routing and gate/packaging remedies. All rows are proposals,
not approved implementation. Independent review and scoped-check disposition is in [the shared record](../slice-9/prework-verification.md).

## Delegated CUDA review integrated

Root directly commissioned read-only policy research, retrieved 2026-11-19.
This supplies the procedure's delegated research step; it is a **bounded partial
review**, not full completion of every usage signal. Minimal public source facts
and failed-access evidence are in [cuda-research-evidence.json](cuda-research-evidence.json).
No private data, whole pages or secrets were copied. ClickPy runtime shares,
pypistats/pepy corroboration and comparable llama.cpp/Ollama downloads were
inaccessible/unusable: current adoption trend remains **UNASSESSED**. October
baseline shares are not relabeled current measurements.

Current primary REPORTED evidence: [NVIDIA release notes](https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html)
list 13.5/R620; [compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/latest/minor-version-compatibility.html)
still separates generic major compatibility from PTX constraints. [Driver support](https://docs.nvidia.com/datacenter/tesla/drivers/latest/supported-drivers-and-cuda-toolkit-versions.html)
lists R580 LTS through June 2028; [architecture matrix](https://docs.nvidia.com/datacenter/tesla/drivers/latest/cuda-toolkit-driver-and-architecture-matrix.html)
retains R580 as last Maxwell/Pascal/Volta driver. [JetPack 6.2.3](https://developer.nvidia.com/embedded/jetpack-sdk-623)
includes CUDA 12.6.10; [JetPack 7.2.1](https://developer.nvidia.com/embedded/jetpack/downloads/archive-7.2.1)
reports 13.2.2. [PyTorch shipping-policy notice](https://dev-discuss.pytorch.org/t/notice-cuda-12-6-wheels-will-no-longer-be-published-from-pytorch-2-15-drops-maxwell-pascal-volta/3432)
describes dropping CUDA 12 after 2.14; [13.0 notice](https://dev-discuss.pytorch.org/t/removing-cuda-13-0-builds-from-pytorch-ci-cd-starting-the-week-of-sept-28/3447)
moves toward 13.2, while [ORT](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html)
and [JAX](https://docs.jax.dev/en/latest/installation.html) retain 13.0 build bases.
These are shipping/platform signals, not measured installed prevalence or a
FathomDB defect/qualification result.

Inference: retain accepted ratings/pins/artifact/host actions for this CPU-only
Inspector review; no changed importance/action is proposed. The ADR requires
human ratification **when changing** importance/actions; it has no CPU-only
exemption, but also no explicit new approval gate for unchanged partial research.
The researcher's recommendation to obtain owner sufficiency approval is advisory,
not an existing rule. Optional further current usage or PyTorch compiler-fix
relevance evidence belongs to separate Engine policy ownership. No GPU campaign,
driver/toolkit/support or policy mutation is authorized or performed here.
