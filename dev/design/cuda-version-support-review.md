---
title: CUDA version usage and importance review (every release)
status: ACTIVE
owner_decision: ADR-0.8.28-cuda-version-support-policy.md (decision 6)
first_run: 0.8.28 Slice 31, 2026-10-11
---

# CUDA version usage and importance review

Every release plan runs this review once, while the plan is drafted. It
keeps [ADR-0.8.28-cuda-version-support-policy.md](../adr/ADR-0.8.28-cuda-version-support-policy.md)
true to how CUDA is actually used. The first run is the baseline:
[cuda-version-adoption.md](../plans/0.8.28/features/slice-31/research/cuda-version-adoption.md)
(data) and
[cuda-version-support-proposal.md](../plans/0.8.28/features/slice-31/research/cuda-version-support-proposal.md)
(importance and actions).

## 1. Collect usage data

Delegate collection to a research subagent. Collection is read-only network
access; nothing is installed. Every number carries its source URL, its
retrieval date, and a label: MEASURED, REPORTED or INFERRED. Raw data is
saved next to the write-up so the numbers can be re-checked. Saved web pages
are scanned for embedded tokens before commit.

| Signal | Source | What it shows |
| --- | --- | --- |
| CUDA runtime downloads by version and month | ClickPy (`sql-clickhouse.clickhouse.com`, `pypi.pypi_downloads_per_day_by_version`); queries in the baseline's `data/clickpy-queries.sql` | shares by major and minor, and the trend since the last review |
| Cross-checks | pypistats.org (180 days), pepy.tech (per version) | that ClickPy's counts agree |
| PyTorch default CUDA per release | PyPI JSON for each `torch` release (`requires_dist`) | what new Python environments install |
| Desktop end users | GitHub release-asset downloads for llama.cpp CUDA builds | the CUDA 12 / 13 split outside CI |
| Jetson | Ollama JetPack asset downloads; NVIDIA JetPack archive | JetPack 5 / 6 / 7 fleet and which modules each supports |
| Drivers | NVIDIA toolkit release notes (driver table), the minor-version compatibility page, driver branch lifecycle | the driver floor of each toolkit; branch end of life; architecture drops |
| Peers | candle, cudarc (crates.io), ONNX Runtime GPU, vLLM, JAX, CuPy | which CUDA versions comparable projects ship |
| Distros and clouds | Ubuntu driver packages; cloud GPU base images | defaults new hosts get |

Write the result to `dev/plans/<version>/research/cuda-version-adoption.md`
with raw data under `research/data/`. Use shares, not absolute counts.
Download volumes swing with collection artifacts (September 2026 dropped
30–40 % for every package).

## 2. Review importance and actions

Compare the new data with the ADR's importance table and the previous
review:

1. **Each version's importance.** Does any version cross a threshold? For
   example: 13.x becomes most of the end-user signal; the JetPack 6 share
   falls; a driver branch reaches end of life; a peer drops 12.x.
2. **Artifacts.** Which toolkit each artifact builds with (the lowest that
   the target platforms share), its driver floor, and its GPU floor (PTX
   target).
3. **Tests.** Whether the library compatibility cells still match the
   versions users have (older and newer cuBLAS within each major; a
   one-major-only environment).
4. **Test hosts.** Whether `windchill3`'s R580 constraint (Quadro K620) or
   the Orin's JetPack still lets the release measure what it ships.

## 3. Record and act

- **The release plan records the review.** Its quality section gets a
  short "CUDA version review" paragraph: the data file, what changed, and
  the importance table (unchanged, or the changes).
- **Changes need owner ratification.** A change to an importance rating or
  action is proposed to the owner, who ratifies it. It is then recorded in
  a successor or amendment of the policy ADR, and in the release's scope.
- **New work is filed.** Work the review creates (for example, a new
  artifact) becomes a slice or a ledger todo with `ledgerwrite`.
