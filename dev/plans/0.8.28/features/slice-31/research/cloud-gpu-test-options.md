---
title: Cloud GPU test venues for the CUDA driver and library matrix
observed_on: 2026-10-11
status: research
---

# Cloud GPU test venues for the CUDA driver and library matrix

All retrievals happened on 2026-10-11. This was read-only web research:
no cloud account was created and nothing was run. Each claim carries one of
three labels:

- **REPORTED**: stated by the cited source.
- **INFERRED**: this author's reasoning or arithmetic from reported facts.
- **MEASURED**: nothing in this document is MEASURED. No cell was run.

Several Google pricing pages (`cloud.google.com/compute/gpus-pricing`,
`.../vm-instance-pricing`, `/spot-vms/pricing`) are rendered by script or are
too large, and the fetch tool returned no price tables from them. GCP prices
below therefore come from third-party trackers that quote Google's list
prices, and are labelled as such. Confirm them in the Google Cloud pricing
calculator before spending anything.

## Summary and recommendation

- **GCP can cover the whole x86 driver axis, cheaply.** On a plain Ubuntu
  GCE VM you install whichever NVIDIA driver you want (runfile, NVIDIA's apt
  repo, or Google's `cuda_installer.pyz --force-version`). On
  Container-Optimized OS (COS) you pin a branch with
  `cos-extensions install gpu -- -version=R535`. The driver lives on the VM,
  and the CUDA user-space libraries come from the `nvidia/cuda` container.
  So one VM per driver branch, running a container matrix, exactly matches
  the "driver from host, libraries from image" split. (REPORTED and
  INFERRED; sources in the GCP section.)
- **Use the T4 (`n1-standard-4` plus one `nvidia-tesla-t4`) as the workhorse.**
  It is compute capability 7.5, which is our floor. It is a data-center GPU,
  so `cuda-compat` works on it. It costs about $0.54/h on demand
  ($0.19 VM plus $0.35 GPU, us-central1). (REPORTED, third-party trackers.)
  V100 (7.0) and P4 (6.1) are below our floor, and Google says R580 is the
  last branch for them, so they cannot cover R590 and later. (REPORTED.)
- **Keep the L4 (`g2-standard-4`, CC 8.9) as the fallback for R590 and
  later.** R590 and later drivers ship only the open kernel modules, which
  need the GSP firmware. AWS tells T4 users to turn GSP off on drivers
  510 and later. If GSP is a problem on GCE T4s with R590+, run those rows on
  L4: $0.71/h on demand, about $0.40/h Spot. (REPORTED and INFERRED;
  unverified risk, see the risks section.)
- **Estimated cost of the full GCP driver matrix: about $5 to $15 in
  compute, and under $30 with reruns and a 2×T4 multi-device cell.** That is
  roughly ten driver branches at about one VM-hour each. (INFERRED; the
  workflow sketch has the arithmetic.)
- **windchill3 Docker already covers most of the library axis.** The driver
  is fixed at R580 there, but the cuBLAS and NVRTC majors come from the
  container, so the library-axis rows run locally for $0. The only thing it
  cannot do is forward compatibility: the RTX 3090 is a GeForce card, and
  NVIDIA limits `cuda-compat` to data-center GPUs. (REPORTED and INFERRED.)
- **arm64 with an NVIDIA GPU: GCP has nothing cheap.** The only Arm series
  with GPUs are A4X and A4X Max (Grace Blackwell, 4 GPUs per VM); C4A, N4A
  and T2A have no GPUs. The cheap arm64 option is AWS `g5g.xlarge` (Graviton2
  plus T4G, CC 7.5): $0.42/h on demand, Spot about $0.11 to $0.19/h. Lambda
  GH200 is about $2.29/h. (REPORTED.)
- **Jetson in the cloud is thin.** One provider, CloudJetson, lists an
  AGX Orin 64 GB at $1.70/h. It does not say which JetPack it runs or
  whether customers can reflash. No NVIDIA LaunchPad Jetson lab was found.
  For the Orin on JetPack 7.2 (CUDA 13.2.1) row, the realistic options are
  to reflash a second boot medium on our own Orin, buy a second dev kit, or
  ask CloudJetson whether they can provide JetPack 7.2. (REPORTED and
  INFERRED.)
- **MCP is available but not needed.** Google runs a Compute Engine remote
  MCP server (`https://compute.googleapis.com/mcp`, VM create and delete,
  OAuth plus IAM) and a remote gcloud MCP (`https://cloudcli.googleapis.com/mcp`,
  `run_gcloud_command`, which is not read-only). The open-source
  `googleapis/gcloud-mcp` is marked preview. None of them runs commands
  inside a VM. A startup script that self-deletes the VM is simpler and
  safer than an agent driving a billed account through MCP. (REPORTED and
  INFERRED.)
- **Honest assessment: this is straightforward.** The work is mostly a
  roughly 100-line startup script per OS flavour, plus a container matrix
  runner that we need anyway for windchill3. The hard parts are getting GPU
  quota on a new project (request it early) and not letting `cuda-compat`
  hide the 13-on-12 failure. The NVIDIA container toolkit injects the compat
  libraries automatically, which would turn the expected typed failure into
  a pass on data-center GPUs; see the cuda-compat section. (INFERRED.)

## The test matrix and which venue covers each row

Venue key: **W** = windchill3 Docker (R580 fixed, RTX 3090 sm_86 and K620
sm_50, GeForce and Quadro, so no forward compatibility). **O** = our Orin
(JetPack 6, CUDA 12.6, driver 540). **G** = GCP GCE (T4 or L4). **A** = AWS
`g5g` (arm64 plus T4G). **J** = Jetson on JetPack 7.2 (venue TBD). All
assignments are INFERRED from the facts cited in later sections.

| Row | Cell | W | O | G | Other | Notes |
|---|---|---|---|---|---|---|
| 1 | 12.6 build on R535 / R550 | no | no | **yes** (T4) | A | Expect PTX floor failure: 12.6 PTX needs ≥560.28.03 |
| 1 | 12.6 build on R555 / R560 | no | no | **yes** (T4) | | Confirms the PTX 8.5 floor (555 vs 560) |
| 1 | 12.6 and 13.0 builds on R570 / R575 | no | no | **yes** (T4) | | 13.0 here is the 13-on-12 row |
| 1 | both builds on R580 | **yes** | no | yes | | W is free; G only as a control |
| 1 | both builds on R590 / R595 | no | no | **yes** (T4; L4 if GSP blocks) | | V100/P4 cannot run R590+ |
| 1 | both builds on R610 / R615 | no | no | **yes** (T4 or L4) | | Needs a VM image whose kernel R615 supports |
| 2 | 12-on-13 (12.6 build on R580 to R615) | R580 only | no | **yes** | | Backward compatibility, expect pass |
| 2 | 13-on-12 without compat (13.0 on R570) | no | no | **yes**, compat hook disabled | | Expect typed failure |
| 2 | 13-on-12 with `cuda-compat-13-0` on R570 | no (GeForce) | no | **yes** (T4 is data-center) | | Expect pass |
| 3 | 12.6 build with cu12 cuBLAS 12.1 / 12.4 / 12.9 | **yes** | partly | yes | | Library comes from the container |
| 3 | 12.6 build in a cu13-only container | **yes** | no | yes | | Expect typed error, record which library loaded |
| 3 | 13.0 build with cu13 libs | **yes** | no | yes | | |
| 3 | 13.0 build with only cu12 libs | **yes** | no | yes | | Expect typed error (the cudarc loader defect) |
| 4 | cudarc 13.1 to 13.4 bindings, default features | no | no | **yes** on R590 to R615 | | Or R580 plus `cuda-compat-13-x` on G (not W) |
| 4 | cudarc driver-only tests, private pool, `maxSize` | R580 only | 12.x only | **yes** (13.x drivers) | | |
| 4 | cudarc multi-device | **yes** (2× 3090) | no | yes (2× T4 on `n1`) | | W already has 2 GPUs |
| 5 | Candle kernels with nvcc 12.6 and 13.0, CUDA tests | **yes** | 12.6 only | yes | | Building needs no GPU; W suffices |
| 5 | Candle device from a private-pool context | **yes** | yes (12.6) | yes | | |
| 6 | aarch64 non-Tegra, discrete GPU | no | no | no (A4X only, costly) | **A** (`g5g`) | Lambda GH200 for Grace |
| 6 | Jetson Orin on JetPack 7.2 / CUDA 13.2 | no | after reflash | no | **J** | See the Jetson section |
| — | below-floor device (sm_50) typed error | **yes** (K620) | no | no | | Unique to W |

Takeaways (INFERRED):

- **Rows 3 and 5 and most of 4 need no cloud spend.** windchill3 covers them.
- **Rows 1 and 2 are the cloud's job.** They are the only rows that need a
  driver other than R580, or a data-center GPU for `cuda-compat`.
- **Row 6 needs AWS (`g5g`) for generic aarch64.** For Jetson CUDA 13 it
  needs our own Orin reflashed or a second board.

## Google Cloud details

### GPU types, compute capability, shapes and prices

Prices are per hour in us-central1 (Iowa), retrieved 2026-10-11.

| GPU | CC | Floor ≥7.5? | Machine shape | GPU on-demand | VM on-demand | Spot (VM plus GPU) |
|---|---|---|---|---|---|---|
| T4 16 GB | 7.5 | yes, the floor | `n1-standard-4` + 1 T4 (1, 2 or 4 per VM) | $0.35 [a] | $0.19 [c] | VM $0.108 [c]; GPU spot not found |
| L4 24 GB | 8.9 | yes | `g2-standard-4` (GPU bundled) | $0.707 (bundled) [a], [b] | n/a | $0.4028 bundled [b] |
| V100 | 7.0 | **no** | `n1` | $2.48 [a] | | |
| P4 | 6.1 | **no** | `n1` | not listed by tracker | | |
| A100 40 GB | 8.0 | yes | `a2-highgpu-1g` | $3.67 [a] | (bundled) | |

- [a] REPORTED:
  [computeprices.com Google page](https://computeprices.com/providers/google)
  and [its T4 page](https://computeprices.com/providers/google/gpus/t4).
  Both say "last updated October 11, 2026" and give us-central1. They also
  list T4 1-year and 3-year commitments at $0.22 and $0.16.
- [b] REPORTED: [gcloud-compute.com g2-standard-4](https://gcloud-compute.com/g2-standard-4.html),
  "Last Update: Sun Oct 11 01:50:17 2026". It gives $0.7068 on demand and
  $0.4028 Spot, us-central1. Other trackers disagree about L4 Spot,
  between $0.20 and $0.62.
- [c] REPORTED: [gcloud-compute.com n1-standard-4](https://gcloud-compute.com/n1-standard-4.html):
  $0.19 on demand and $0.1082 Spot, us-central1, same update stamp.
- **Spot discount (REPORTED in search snippets of Google's GPU pricing
  page; the fetch of the page itself failed).** Spot discounts are "60-91%
  off" on-demand for most GPUs, and prices "can change up to once every 30
  days". A T4 Spot GPU would therefore cost about $0.03 to $0.14/h
  (INFERRED). NVIDIA Brev's page lists GCP T4 Spot at $0.08
  ([Brev pricing](https://www.nvidia.com/en-us/launchables/pricing),
  REPORTED via search snippet).
- **Typical short run (INFERRED).** A T4 with an `n1-standard-4` runs about
  $0.54/h on demand, or about $0.20 to $0.25/h Spot. Add a 50 GB
  pd-balanced boot disk (a few cents per day). A 1-hour cell costs well
  under $1. Google recommends at least 40 GB of boot disk for driver
  installs
  ([install-drivers-gpu](https://docs.cloud.google.com/compute/docs/gpus/install-drivers-gpu),
  REPORTED).
- **T4 and P4 lifetime.** "NVIDIA T4 and P4 GPUs reach end of support on
  August 1, 2027." T4 runs in about 23 regions, including us-central1,
  us-east1, us-west1 and europe-west4.
  ([GPU regions](https://docs.cloud.google.com/compute/docs/gpus/gpu-regions-zones),
  REPORTED.) P100 support ended on 2026-09-15
  ([COS GPU page](https://docs.cloud.google.com/container-optimized-os/docs/how-to/run-gpus),
  REPORTED).
- **Quota.** New projects get a global GPU quota (`GPUs (all regions)`)
  plus per-model regional quotas. Google's page does not give the default,
  which is commonly 0 (INFERRED; check the project's Quotas page). To get
  quota, request a per-model regional quota (for example
  `NVIDIA_T4_GPUS` in us-central1) and the global
  `GPUS_ALL_REGIONS` in the console's Quotas page. Spot can use a separate
  `PREEMPTIBLE_NVIDIA_T4_GPUS` quota; once that quota is granted, Spot VMs
  draw only from it. Free Trial billing accounts cannot change quota.
  ([Resource usage and quotas](https://docs.cloud.google.com/compute/resource-usage),
  REPORTED.) Ask for 2 T4s and 1 L4 in two regions, so the 2×T4 cell works
  and you can switch region when capacity runs short (INFERRED).

### Driver version control

- **Plain Ubuntu or Debian GCE image: any branch (REPORTED and INFERRED).**
  - Google's `cuda_installer.pyz` lists versions with `list_driver_versions`
    and pins one with `install_driver --force-version DRIVER_VERSION`. Pinned
    installs use binary mode. Branch shortcuts are `--installation-branch=prod|nfb|lts`.
  - Tested operating systems: Debian 12 and 13, RHEL and Rocky 8 to 10,
    Ubuntu 22, 24 and 26.
  - The script does not work with Secure Boot, so create VMs without
    Shielded VM Secure Boot, or use Ubuntu's pre-signed modules.
  - Google names R580 as the tested LTSB (support until June 2028) and R595
    as the latest tested production branch (support until March 2027). The
    Linux minimum for T4, L4 and A100 is 580.95.05 for *Google support*.
  - Source: [install-drivers-gpu](https://docs.cloud.google.com/compute/docs/gpus/install-drivers-gpu)
    (REPORTED).
  - The installer's catalogue was not visible, so whether it offers
    R535/R550/R555/R560 is unknown. For those, use NVIDIA's runfiles (every
    version is published under `download.nvidia.com`, for example the
    535.98, 550.76, 570.169 and 575.64 READMEs surfaced in search) or
    Ubuntu's `ubuntu-drivers --gpgpu install nvidia:535`
    ([Ubuntu on AWS docs](https://documentation.ubuntu.com/aws/en/latest/aws-how-to/instances/install-nvidia-drivers/),
    REPORTED). That a full arbitrary-version path exists is INFERRED.
  - Below Google's recommended minimum, Google will not support you, but
    nothing stops the install (INFERRED).
- **Container-Optimized OS: pinned branches, built for containers
  (REPORTED).**
  - `sudo cos-extensions list` shows the drivers available for that COS
    release.
  - `sudo cos-extensions install gpu -- -version=X` accepts `default`,
    `latest`, an exact version (`535.183.01`), or a branch (`R535`, needs
    `cos-gpu-installer` v2.2.1 or later).
  - Requires an x86 COS LTS milestone of 85 or later. The driver is rerun on
    every boot via startup script or cloud-init.
  - Supports T4, V100, P4, L4, A100 and newer.
  - Source: [COS run-gpus](https://docs.cloud.google.com/container-optimized-os/docs/how-to/run-gpus).
  - Which branches each COS milestone carries was not visible. Expect an
    older COS milestone for R535/R550 and a new one for R590+ (INFERRED).
- **Deep Learning VM images: R580 or R570 only (REPORTED).** Current
  families are `common-cu129-ubuntu-{2204,2404}-nvidia-580`. The previous
  family, `common-cu128-ubuntu-2204-nvidia-570`, reached end of support on
  2026-04-13. No cu130 or arm64 families were listed.
  ([DLVM images](https://docs.cloud.google.com/deep-learning-vm/docs/images).)
  These images are not useful for the driver axis.
- **GKE: tied to the GKE version (REPORTED).** `gpu-driver-version` accepts
  `default`, `latest` (COS only) or `disabled`. On COS, GKE 1.31 to 1.33
  offers R535 (default), R570, R575 or R580; Ubuntu nodes get R535. There is
  no arbitrary branch except by installing manually with
  `disabled`. ([GKE GPUs](https://docs.cloud.google.com/kubernetes-engine/docs/how-to/gpus).)
  The page's version table may lag current GKE. It is more machinery than
  this job needs.
- **Swapping drivers (INFERRED).**
  - **Option 1, a new VM per branch.** Clean state, and branches can run in
    parallel. Each boot plus driver install takes about 5 to 15 minutes.
    Recommended.
  - **Option 2, reinstall on one VM.** Purge, install the next runfile,
    reboot: about 5 to 10 minutes per swap. Cheaper on boot-disk churn, but
    it risks leftover libraries, for example a stale `libcuda.so.1`, which
    would corrupt exactly the "which libcuda loaded" receipts we want.
  - Kernel compatibility is the main snag. Old branches (R535, R550) may not
    build against the newest GCP kernels, and new branches (R610, R615) may
    need them. Use Ubuntu 22.04 for old branches and 24.04 for new ones, and
    record `uname -r` in every receipt.

### Containers

- **GCE plus containers.** COS ships Docker, and the NVIDIA libraries are
  bind-mounted from `/var/lib/nvidia`. On Ubuntu, install Docker plus the
  NVIDIA Container Toolkit. Batch states the toolkit is required for
  container runnables on any OS other than COS
  ([Batch GPUs](https://docs.cloud.google.com/batch/docs/create-run-job-gpus),
  REPORTED). The `nvidia/cuda:<ver>-runtime-ubuntu22.04` and `-devel` images
  then supply cuBLAS, NVRTC and the rest (INFERRED, standard practice).
  **Recommended:** Ubuntu plus the toolkit for uniformity with windchill3
  (CDI), or COS where the branch is offered.
- **Cloud Run GPU: driver fixed at R580 (REPORTED).**
  - GPUs are L4 (minimum 4 vCPU and 16 GiB) and RTX PRO 6000.
  - The driver is "580.x.x" (CUDA 13.0) and cannot be chosen.
  - For newer CUDA, Google suggests forward-compatibility packages on
    `LD_LIBRARY_PATH`.
  - Source: [Cloud Run GPU](https://docs.cloud.google.com/run/docs/configuring/services/gpu).
  - Use (INFERRED): one cheap data-center R580 cell for testing
    `cuda-compat-13-x` with 13.1 to 13.4 binaries. Not useful for the driver
    axis.
- **Cloud Batch (REPORTED).** `installGpuDrivers: true` installs a driver,
  and an optional `driverVersion` field selects one. The page lists no valid
  values, and the console supports only the default. A custom image with
  `installGpuDrivers: false` is the documented way to control the driver
  fully. ([Batch GPUs](https://docs.cloud.google.com/batch/docs/create-run-job-gpus).)
  Batch adds Spot retry handling and auto-cleanup, but `driverVersion`
  semantics are undocumented. Plain GCE plus a startup script is simpler to
  reason about (INFERRED).
- **Simplest for a driver × library matrix (INFERRED).** GCE VMs, one per
  driver branch, each running the same container matrix script we already
  use on windchill3.

### arm64 with NVIDIA on GCP

- **Only A4X and A4X Max have GPUs on Arm (REPORTED).** "To use GPUs with
  an Arm-based CPU, choose either the A4X Max or A4X machine series." These
  run NVIDIA Grace CPUs with Grace Blackwell superchips. N4A, C4A and T2A
  have no GPUs, and the SLA does not apply to A4X.
  ([Arm on Compute](https://docs.cloud.google.com/compute/docs/instances/arm-on-compute).)
- **A4X is a whole-node, high-end product (REPORTED).** `a4x-highgpu-4g`
  has 140 vCPU, 884 GB of RAM and 4 GB200 GPUs, in zones us-central1-a/b,
  us-east1-d, us-east4-b and europe-west4-b. No official per-hour price was
  found; a third-party cross-provider GB200 figure is about $10.50 per
  GPU-hour, so about $40/h per VM (INFERRED).
  ([GPU regions](https://docs.cloud.google.com/compute/docs/gpus/gpu-regions-zones),
  [getdeploying](https://getdeploying.com/gpus/nvidia-gb200-vs-nvidia-t4).)
  Likely gated behind reservations or capacity requests (INFERRED). Not
  appropriate for a smoke matrix.
- **COS on Arm does not support GPUs**
  ([COS run-gpus](https://docs.cloud.google.com/container-optimized-os/docs/how-to/run-gpus),
  REPORTED).

### MCP servers

| Server | Kind | What it does | Status | Auth |
|---|---|---|---|---|
| Compute Engine, `https://compute.googleapis.com/mcp` | Google-managed remote (HTTP) | About 29 tools: `create_instance` (defaults to e2-medium), `delete_instance`, start/stop/reset, many list/get tools. No in-VM command execution found. | Preview/GA label not found | OAuth 2.0 plus IAM; scopes `compute.read-only` / `compute.read-write`; permission `mcp.tools.call` |
| gcloud remote, `https://cloudcli.googleapis.com/mcp` | Google-managed remote | `run_gcloud_command`, `run_bq_command`. Google warns it "is NOT restricted to read-only commands". | not stated | "set up authentication" (Google auth) |
| `googleapis/gcloud-mcp` (`npx -y @google-cloud/gcloud-mcp`) | open-source, local | `run_gcloud_command` with a built-in deny list of interactive or arbitrary-input commands | "currently in preview and may see breaking changes"; not an officially supported product | inherits the active gcloud account; README suggests impersonating a least-privilege service account |
| Cloud Run remote MCP | remote | deploy and manage Cloud Run | GA | Google auth |

Sources (REPORTED): [google/mcp](https://github.com/google/mcp),
[gcloud-mcp README](https://github.com/googleapis/gcloud-mcp),
[cloudcli MCP reference](https://docs.cloud.google.com/sdk/reference/mcp),
[Compute Engine MCP guide](https://docs.cloud.google.com/compute/docs/use-compute-engine-mcp)
(its body did not render in the fetch tool; details came from search
snippets of Google's pages),
[glama connector listing](https://glama.ai/mcp/connectors/com.googleapis.compute/google-compute-engine)
(29 tools, "healthy" as of 2026-10-11), and
[MintMCP setup guide](https://www.mintmcp.com/docs/compute-engine-mcp-setup)
(roles: MCP Tool User, Compute Viewer, Compute Instance Admin v1, Service
Account User).

Risk notes (INFERRED):

- **The agent would act on a billed account.** Any of these servers lets
  an agent create GPU VMs.
- **Mitigations, if used:**
  - a dedicated project;
  - a service account holding only Compute Instance Admin on that project,
    with GPU quota capped at 2;
  - an org policy or quota limit as a hard ceiling;
  - `--max-run-duration` set on every VM;
  - the owner approving each create call (Claude Code permission prompts on
    the MCP tool).
- **Prefer the remote Compute Engine server over the gcloud tools.** It has
  a narrower verb set than `run_gcloud_command`.
- **None of these runs our tests inside the VM.** A startup script still
  has to do the work, so MCP adds little over a checked-in
  `gcloud compute instances create ...` script the owner runs by hand.

### Cost controls

- **Budgets alert; they do not cap (REPORTED).** "Setting an alerts-only
  budget doesn't automatically cap ... spending." Budgets can publish to
  Pub/Sub, and a function can disable billing.
  ([Budgets](https://docs.cloud.google.com/billing/docs/how-to/budgets).)
- **Spend-cap budgets entered Preview on 2026-07-27 (REPORTED).** They pause
  eligible services in a project until the cap is lifted by hand, using
  estimated costs, and enforcement is not instant. Whether Compute Engine is
  eligible was not confirmed
  ([billing release notes](https://docs.cloud.google.com/billing/docs/release-notes),
  [Medium summary](https://medium.com/@avnitbambah/gcp-finally-has-a-kill-switch-spend-cap-budgets-arrive-in-preview-002a8ceca318)).
- **VM run-time limits are the real safety net (REPORTED).**
  `--max-run-duration=2h --instance-termination-action=DELETE` deletes the
  VM after a limit of 30 seconds to 120 days. This works with Spot VMs,
  where the action defaults to STOP, so set DELETE explicitly. GPU VMs that
  auto-delete within 7 days can draw from preemptible quota.
  ([Limit VM runtime](https://docs.cloud.google.com/compute/docs/instances/limit-vm-runtime).)
- **Spot preemption (INFERRED).**
  - Make every cell idempotent, and upload each receipt to GCS as soon as
    the cell finishes, so a preemption loses at most one cell.
  - On restart, the runner skips cells whose receipt already exists.
  - Fall back to on-demand for a branch that gets preempted twice. On-demand
    is only about $0.30/h more for a T4.

### Workflow sketch and cost estimate

Sketch (INFERRED):

1. **One-time setup, done by the owner.**
   - Create a project, link billing, set a budget of $50 with alerts at 50 %,
     90 % and 100 %.
   - Request quota: 2× T4 and 1× L4 in us-central1 and one backup region,
     plus `GPUS_ALL_REGIONS=3`.
   - Create a GCS bucket for receipts, and a least-privilege service
     account for the VMs that can only write to that bucket.
2. **Per driver branch:** one command creates the VM.

   ```bash
   gcloud compute instances create "fdb-drv-$BRANCH" \
     --zone=us-central1-a --machine-type=n1-standard-4 \
     --accelerator=type=nvidia-tesla-t4,count=1 \
     --maintenance-policy=TERMINATE --provisioning-model=SPOT \
     --instance-termination-action=DELETE --max-run-duration=2h \
     --image-family=ubuntu-2204-lts --image-project=ubuntu-os-cloud \
     --boot-disk-size=60GB \
     --metadata=driver-version="$VER",receipt-bucket="$BUCKET" \
     --metadata-from-file=startup-script=ci/gpu-cell/startup.sh
   ```

3. **The startup script:**
   1. installs the pinned driver runfile (or runs
      `cos-extensions install gpu -- -version=R5xx` on COS);
   2. installs Docker and the NVIDIA Container Toolkit;
   3. pulls the prebuilt test images (12.6 build and 13.0 build × cu12.1,
      12.4, 12.9 and cu13 library images, from Artifact Registry);
   4. runs each cell and writes a JSON receipt. Each receipt records
      `nvidia-smi` output, `cuDriverGetVersion`, `/proc/driver/nvidia/version`,
      `uname -r`, the compat-hook setting, and each library resolved via
      `/proc/self/maps` or `dladdr` (the receipt shape for the cudarc loader
      fix);
   5. copies the receipts to GCS and runs `shutdown -h now`. The VM then
      deletes itself, because Spot plus DELETE is set.
4. **Locally,** pull the receipts and judge them against expected outcomes
   (pass, or a specific typed error).

Cost estimate (INFERRED, from the prices above):

| Item | Count | Unit | Est. |
|---|---|---|---|
| Driver branches (535, 550, 555, 560, 570, 575, 580, 590, 595, 610, 615) | 11 VMs × ~1 h | T4 Spot ≈ $0.25/h (on-demand $0.54/h) | $3 to $6 |
| Compat cells (13.0 to 13.4 compat on R535/R570/R580) | inside the VMs above | — | $0 |
| R590+ fallback on L4 if T4 GSP issues | 4 VMs × 1 h | $0.40 to $0.71/h | $2 to $3 |
| Multi-device 2× T4 on R580 and R595 | 2 VMs × 1 h | ≈ $0.70 to $0.90/h on demand | ~$2 |
| Disks, Artifact Registry storage, GCS | — | — | < $2 |
| Reruns and debugging (×2) | — | — | $10 to $15 |
| **Total** | | | **about $15 to $30** |

Image pulls into GCE from Artifact Registry in the same region incur no
egress charge (INFERRED). Keep the images in us-central1.

## Alternatives (brief)

- **AWS (REPORTED):**
  - `g4dn.xlarge` (T4, x86) costs $0.526/h on demand, Spot about $0.21 to
    $0.23/h, us-east-1
    ([DoiT](https://www.doit.com/compute/spot/us-east-1/g4dn.xlarge)).
  - **`g5g.xlarge` (Graviton2 arm64 plus T4G, CC 7.5)** costs $0.42/h on
    demand, Spot $0.11 to $0.19/h, us-east-1
    ([Holori](https://calculator.holori.com/aws/ec2/g5g.xlarge/us-east-1)).
    The minimum driver is 470.82.01, and AWS's arm64 DLAMIs ship 580.126.09
    (Ubuntu 24.04) or 570.211.01 (AL2)
    ([AWS NVIDIA drivers](https://docs.aws.amazon.com/ko_kr/AWSEC2/latest/UserGuide/install-nvidia-driver.html),
    [ARM64 DLAMI](https://docs.amazonaws.cn/dlami/latest/devguide/aws-deep-learning-ami-gpubaseossarm64-ul2404-2026-02-03.html)).
  - Driver selection: public NVIDIA drivers ("bring your own"), so any
    branch NVIDIA ships for the T4 or T4G.
  - AWS says: "If you are using NVIDIA driver version 510.x or greater on
    the G4dn, G5, or G5g instances, disable GSP"
    ([gaming driver page](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/nvidia-gaming-driver.html)).
  - **`g5g` is the recommended venue for the generic aarch64 non-Tegra
    row.**
- **Azure (REPORTED):** `Standard_NC4as_T4_v3` costs about $0.526/h on
  demand, Spot about $0.15 to $0.41/h by region
  ([Vantage](https://instances.vantage.sh/azure/vm/nc4ast4-v3)). The NVIDIA
  GPU driver extension installs the latest driver, with no documented
  version pinning on Linux
  ([N-series driver setup](https://learn.microsoft.com/azure/virtual-machines/linux/n-series-driver-setup)),
  so a manual install would be needed. No advantage over GCP.
- **Lambda (REPORTED):** GH200 (Grace arm64 plus Hopper) costs about $2.29
  per GPU-hour on demand per a 2026-08-04 tracker
  ([computeprices Lambda GH200](https://computeprices.com/providers/lambda/gpus/gh200)).
  Lambda's own page was not fetched. Driver choice on Lambda's image was not
  researched; root access normally allows a reinstall (INFERRED). Use it for
  a Grace-specific cell only if `g5g` is not enough.
- **RunPod and Vast.ai (REPORTED):** you rent containers on hosts whose
  driver you cannot set. RunPod filters with `allowedCudaVersions` (11.8 to
  13.0) ([RunPod create pod](https://docs.runpod.io/api-reference/pods/POST/pods.md)).
  Vast added "max CUDA and driver version" filters
  ([Vast October update](https://vast.ai/article/october-product-update)).
  You can *find* a host on a given driver but cannot choose it
  deterministically, and most hosts are GeForce, so no `cuda-compat`
  (INFERRED). These are useful only for opportunistic extra driver
  samples.
- **GitHub-hosted GPU runner (REPORTED):** one T4, 4 vCPU, 28 GB of RAM,
  $0.052/min on Linux ($3.12/h)
  ([larger runners](https://docs.github.com/en/actions/reference/runners/larger-runners),
  [runner pricing](https://docs.github.com/en/billing/reference/actions-runner-pricing)).
  No arm64 GPU runner exists. The preinstalled driver and whether it can be
  changed are not documented. The runner is a VM, so a reinstall plus
  reboot is likely impossible within a job (INFERRED). Not a fit for the
  driver axis, and about 6× GCP's on-demand T4 price.

## cuda-compat findings

- **Supported hardware is data-center GPUs, select NGC-ready RTX SKUs and
  Jetson boards; GeForce is not covered (REPORTED).** The package is
  `cuda-compat-<maj>-<min>`, installs to `/usr/local/cuda-X.Y/compat/`
  (`libcuda.so.*`, `libnvidia-ptxjitcompiler.so.*`, `libnvidia-nvvm*`), and
  needs `LD_LIBRARY_PATH` set. Error 803 `CUDA_ERROR_SYSTEM_DRIVER_MISMATCH`
  and error 804 `CUDA_ERROR_COMPAT_NOT_SUPPORTED_ON_DEVICE` are the failure
  codes.
  ([Forward compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/forward-compatibility.html).)
- **Compatibility matrix (REPORTED, same page):**
  - `cuda-compat-13-0` works on R535 and R570 (N/A on R580).
  - `cuda-compat-13-4` works on R535, R570, R580, R590, R595 and R610.
  - `cuda-compat-12-9` works on R535 and R570.
  - `cuda-compat-12-6` works on R535 only.
  - No package runs on a *newer* branch than its own: "X" for 13-0 on R590+.
  - Kernel-mode-dependent features and graphics interop are excluded.
- **What this means for 13-on-12 (REPORTED and INFERRED):**
  - Without compat, CUDA 13.x requires driver ≥580. Minor-version
    compatibility does not cross the major boundary
    ([Minor version compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/minor-version-compatibility.html)).
  - So a 13.0 build on R570 should fail at `cuInit` or at the version check.
    That failure is our typed-failure cell.
  - With `cuda-compat-13-0` on a T4 under R535 or R570, it should pass. The
    owner's assumption is confirmed.
- **PTX JIT (REPORTED):** "Applications that compile device code to PTX
  will not work on older drivers" under *minor*-version compatibility. The
  compat package bundles a newer PTX JIT compiler, so with compat PTX from a
  newer toolkit can JIT on an older kernel driver. Our 12.6 PTX on R535 or
  R550 should therefore fail without compat (row 1), and pass with
  `cuda-compat-12-6` on a T4 (INFERRED from the matrix plus the bundled
  `libnvidia-ptxjitcompiler`).
- **Hazard: the container toolkit injects compat automatically (REPORTED,
  important).**
  - The NVIDIA Container Toolkit has an `enable-cuda-compat` hook. If the
    container image carries `/usr/local/cuda/compat` libraries newer than
    the host driver, the hook puts them on the loader path. The `nvidia/cuda`
    images ship them.
  - Toolkit 1.17.7 added `cuda-compat-mode` (default `ldconfig`). 1.18 adds
    `nvidia-container-cli configure --cuda-compat-mode=disabled`, and legacy
    mode can set `disable-cuda-compat-lib-hook = true` in `config.toml`.
  - The hook had a CVE fixed in 1.17.8. A December 2025 change extended it
    to Tegra (`/usr/local/cuda/compat-orin`).
  - Sources: [toolkit 1.17.8 notes](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/1.17.8/release-notes.html),
    [1.18.1 notes](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/1.18.1/release-notes.html),
    [Isambard GPU guide](https://docs.isambard.ac.uk/user-documentation/guides/gpus_and_cuda/).
  - **Consequence (INFERRED):** on a T4 under R570, running the 13.0 build
    in an `nvidia/cuda:13.0` image may *silently pass* via compat, masking
    the expected typed failure. Every receipt must record the compat mode
    and the real path of the `libcuda.so.1` that loaded. Run the "without
    compat" cells with the hook disabled, or in an image with `compat/`
    removed. On windchill3 (GeForce) the hook should not take effect, but
    record it anyway.
- **Can compat reduce driver installs? Partly (INFERRED).**
  - On one R535 T4 VM, `cuda-compat-12-x` and `cuda-compat-13-0` to `13-4`
    exercise "new toolkit on old kernel driver" for every CUDA version. On
    one R580 T4 or L4 VM, `cuda-compat-13-1` to `13-4` exercise the 13.1 to
    13.4 cudarc bindings without installing R590 to R615.
  - That tests compat `libcuda`, *not* the real R590 to R615 drivers, so it
    supplements the 12-on-13 rows (12.6 build on a real R595 or R615) but
    cannot replace them.
  - Compat never works in the other direction, old toolkit on a new driver,
    because plain backward compatibility already covers it.
  - It does not help on windchill3, a GeForce card.
- **Jetson (REPORTED and INFERRED):** Jetson boards are in the supported
  list, and the container toolkit has an Orin compat root. Whether a CUDA 13
  compat package exists for JetPack 6 (L4T R36) Orin was **not found**.
  NVIDIA's JetPack 7.2 notes present CUDA 13 on Orin as arriving *with* the
  JetPack 7 BSP, which suggests a reflash is required (INFERRED). Open
  question for the owner and NVIDIA.

## Jetson in the cloud

- **CloudJetson (REPORTED, [cloudjetson.com](https://cloudjetson.com/)):**
  - AGX Orin 64 GB at $1.70/h, "Available Now". Thor at $2.98/h, Orin NX
    16 GB at $1.25/h and Orin Nano at $0.89/h are "Coming Soon".
  - Billed hourly, invoiced weekly. Access is "Full SSH/VNC access" with
    "JetPack SDK pre-installed".
  - Not stated: the JetPack version, reflash or JetPack choice, root access,
    location, and legal entity. The site also contradicts itself on price
    ($1.79 vs $1.70).
  - Treat as an unvetted vendor (INFERRED). Ask them, before any spend,
    whether they can provision Orin on JetPack 7.2 or 7.2.1. Upload no
    secrets to it.
- **NVIDIA LaunchPad:** free labs of up to two weeks on Equinix-hosted
  infrastructure. The labs found are data-center, AI software and Omniverse;
  **no Jetson lab was found**
  ([LaunchPad](https://nvidia.com/en-au/data-center/launchpad), REPORTED).
- **Allxon and similar:** remote *management* of Jetsons you own (OOB
  console, OTA), not rental
  ([Allxon Jetson](https://allxon.com/jetson), REPORTED).
- **AWS IoT Greengrass, NGC:** not remote device access (REPORTED via a
  forum thread and INFERRED).
- **JetPack 7.2 facts (REPORTED):** Jetson Linux 39.2, CUDA 13.2.1, Ubuntu
  24.04, kernel 6.8. It brings Orin into JetPack 7, with a unified ISO
  install. JetPack 7.2.1 (Jetson Linux 39.2.1, CUDA 13.2.1) is also listed.
  ([JetPack 7.2 archive](https://developer.nvidia.com/embedded/jetpack/downloads/archive-7.2),
  [JetsonHacks GTC 2026](https://jetsonhacks.com/2026/05/31/gtc-2026-taipei/).)
- **Recommendation (INFERRED):** the cheapest reliable route is to keep the
  JetPack 6 install intact and flash JetPack 7.2.1 to a *second* NVMe or USB
  boot medium on our Orin. The new ISO install may make this simpler; verify
  whether dual-boot is supported. Alternatively, buy a second Orin dev kit
  dedicated to JetPack 7.x. CloudJetson is a fallback only if they confirm
  JetPack 7.2.

## Risks and open questions for the owner

1. **Approval to open a GCP project with billing, and a budget ceiling.** A
   suggested budget is $50 with alerts. Expected spend is $15 to $30. GPU
   quota requests can take hours to days, so start them first.
2. **T4 on R590 and later (open modules, GSP required).** This is
   unverified on GCE. AWS's GSP-off advice for T4 conflicts with modules
   that require GSP. Plan an L4 fallback for R590 to R615 (about $3).
3. **Kernel versus driver build compatibility** for R535/R550 on current GCE
   Ubuntu kernels, and for R610/R615 on older ones. May force two base
   images. Record `uname -r`.
4. **The compat hook masks 13-on-12 failures** on data-center GPUs. The
   matrix runner must control and record `cuda-compat-mode`.
5. **Google's `cuda_installer` catalogue and COS branch availability** for
   R555, R560, R590 and R610 were not visible. Fall back to NVIDIA runfiles.
   The exact available set needs a 10-minute probe once a project exists.
6. **The T4 is end-of-support on GCP on 2027-08-01.** Fine for the 0.8.28
   timeframe, but plan L4 as the long-term floor GPU (CC 8.9, above our 7.5
   floor, so it no longer exercises sm_75 natively; PTX JIT still does).
7. **arm64:** approve an AWS account for `g5g` (about $1 to $3 for the
   aarch64 cells), or accept the aarch64 non-Tegra path as untested in
   0.8.28.
8. **Jetson CUDA 13:** choose between reflashing a second boot medium on our
   Orin, buying a second board, or querying CloudJetson. Also confirm with
   NVIDIA whether any CUDA 13 compat exists for JetPack 6. None was found.
9. **MCP:** decide whether an agent may create VMs at all. If yes, use the
   remote Compute Engine MCP with a least-privilege service account, quota
   capped at 2 GPUs, `--max-run-duration` mandatory, and per-call approval.
   The recommended default is no MCP: an owner-run checked-in script.
10. **Prices are third-party snapshots.** Google's own pricing pages did not
    render in the research tool. Re-check T4 Spot in the calculator.
