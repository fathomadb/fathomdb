# Hardware, CUDA, and Self-Hosted Runner Notes

Operational constraints for release hosts and self-hosted runners.

## Release GPU workstation (`windchill3`)

- **Never alter the NVIDIA driver or primary `/usr/local/cuda` (12.6).**
  The primary driver and CUDA 12.6 toolkit are shared, stability-pinned host
  dependencies (`dev/adr/ADR-0.8.28-cuda-version-support-policy.md`).
- **GPU0 is the canonical release CUDA GPU** (`FATHOMDB_CUDA_GPU_UUID` stored
  in the `cuda-unmerged-preflight` GitHub environment).
- **Temporary org GPU runner lifecycle:** Spin up temporary runners for org
  runner group `3` (`fathomdb-gpu-release`, labels `gpu,cuda-12`) in an
  ephemeral `/tmp/` directory copied from `/home/coreyt/actions-runner/fathomdb`
  binaries without mutating existing runner directories. Stop, deregister via
  `gh api -X DELETE orgs/fathomadb/actions/runners/<id>`, and remove `/tmp/`
  immediately after `build-cuda-linux-x64-gnu` succeeds.

## Jetson Orin Tegra runner

- **Host reachability:** Use the current Orin LAN address (`10.83.10.13`; do not
  rely on stale `/etc/hosts` entries if SSH times out).
- **Pre-registered runner hygiene:** `~/actions-runner/fathomdb`
  (`jetson-fathomdb`, agentId `22`) is pre-registered and intentionally stopped
  at idle. Start it only for a `jetson-tegra-cuda-evidence.yml` dispatch and
  stop **only** that listener afterward, leaving its registration/credentials
  and any unrelated runner services on the host untouched.
