# Slice 60 supplemental projected-metadata acceptance

This is post-code manual acceptance for AC29-60-FTS4/F13. F12 filter forwarding
already had observed RED75e7d5da9 and GREEN512015678. These operator assertions
were authored later; they are not regression tests, a new RED or automated
full-coverage evidence. A late compile-only test was archived externally and
removed without execution or landing. No product fixture/golden was regenerated.

## Observed execution

At product checkpoint2d8278c7290799afec4f9d15f06c5010017a9fd7, one real Default
CUDA producer projected exactly two authored documents and one edge, then closed.
The selected RTX3090 UUID was GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b, visible
ordinal0. Public OpenReport proved CUDA selection, three pinned cache hits,
no download/reranker/allocation witness;76 NVML samples observed5% utilization
and436MiB attributable to producerPID3418425. Driver580.178.04 and primary
CUDA12.6/nvcc12.6.68 remained unchanged. Genuine node/edge metadata was status
empty-string and created_at1794366027, with three vector rows. New empty-store
schema initialization used existing schema37; no migration was added.

Separate no-CUDA SDK None controls passed14 cases. The normal no-default-feature
MCP artifact passed structured14 cases and closed0, then compact orient stopped
on two wrong manual expected spellings. The original failure/transcripts remain
retained. Same Astra approved only those two spelling corrections and optional
format selection; text-only resumption passed all14 cases and closed0, without
another producer or structured replay. Raw-score structured comparison and
three-decimal compact comparison remain distinct. Each consumer uses the same
completed database and stored timestamps; sequential openers, no provider bypass
or manual projected-metadata writes. F11 remains held for the actual ref ruling.

`qualification.json` records actual artifact hashes, source, real exits, device
and timestamps. Raw logs/DB/process maps/transcripts remain under
`/mnt/data/tmp2/fathomdb-0829-01a1c5a5-s60/gpu-metadata/run-001/`; the original
attempt is in first-consumer-attempt. Resource/command receipts are under the
same task's receipts and logs directories. No model bytes or database are tracked.

## Operator sources and provenance

- `executed-sources.json` preserves the exact executed UTF8 sources and their
  SHA256 hashes, including original v2 checker and approved v3 correction. The
  executed final launcher uses explicitly reconstructable UTF8 fragments around
  a public tokenizer integrity checksum: concatenation is byte-identical to the
  frozen original36db38a40775b3e5a5cca2862cde96ee4412097d01c4c2c76f0a20528f627157.
  The normal hook initially misclassified that already-tracked public checksum
  as a credential. Portable pin rows use filename/digest tuples with complete
  unchanged values. No scanner exclusion, hook bypass or model rerun follows.
- `operator-sources.json` hashes the portable operator copies. They were adapted
  after the observed run: paths/selected UUID come from required environment
  parameters, Python/Rust formatting and unused imports are normalized, and the
  final launcher loads validation modules after descriptor sanitation. These
  copies were not substituted into the observed run, and were not model-run
  again for documentation. No assertion/predicate/record/budget was removed.
- `driver/` is the public SDK producer and no-CUDA None control. Its independent
  lock retains307 root-pinned package versions/sources/checksums; the sole added
  package is this publish-false manual operator. Its SDK path is relative to this
  checkout; candle-kernels keeps exact root patch182ceab0547348993b6dbec4b1aed610dcc8b23f.
- `mcp_check.py` runs real stdio structured/text sessions (optional format args),
  authored14-case selections, exact raw/display scores and format/fit checks.
- `netns_exec.sh` and `netns_final_exec.py` form the final runtime boundary:
  only lo DOWN/routes[]; private actual XDG loader cache; selected3090; pin and
  room verification; local non-socket stdio; every FD>=3 closed/verified before
  direct exec. There is no live-network fallback.
- `netns_descriptor_probes.py` and `netns_probe_target.py` are the model-free
  same-path isolation probes (seeded socket/file FDs close; socket stdio refuses;
  local streams/NVML identity succeed). No additional probe was run for this copy.
- `resource_guard.py` preserves exact root/task/RAM/footprint/swap start/stop
  floors, source identity, raw command output and real exits, stopping only its
  owned process group. `retain_spills.py` preserves canonical native logs across
  normal unlink using read-only owned-directory polling/retained descriptors.
  Their executed source is also archived; portable copies use required paths.
- `production_lifecycle.py` and `production_lifecycle.mjs` perform two sequential
  open/close cycles on fresh owned stores using the separately rebuilt actual
  production wheel/TypeScript package, capturing the loaded native identity.
  They are required supplemental artifact qualification, not new RED tests.
- `build_env.sh` holds command-local existing CUDA12.6/GCC13/cap75/jobs2 flags.
- `run_producer.py` executes one fresh fixture, samples NVML50ms, binds the actual
  PID and refuses unobserved engagement. It never repeats encoding to manufacture
  a witness; room/driver/foreign-process violations stop only its owned child.

## Reproduction procedure

Run only with the required GPU/resource authority, fresh task-owned resources and
checkout-owned tools. This is an opt-in manual procedure, not a full-gate target.
Do not run while another native owner holds the window. Keep root/task free space
at least32/250GiB, MemAvailable16GiB and total owned footprint below200GiB before
starting; stop owned workloads below20/50GiB or MemAvailable8GiB, at200GiB owned,
or swap exceeding64MiB/s for three30s samples. One native phase, cargo jobs2.

```bash
export S60_SOURCE=/absolute/owned/source-checkout
export S60_TASK=/absolute/private/task
export S60_GPU_UUID=GPU-of-an-authorized-idle-3090
cd "$S60_SOURCE"
source "$S60_TASK/env.sh" # owned base Cargo/npm/Python/cache/temp environment
export CARGO_BUILD_JOBS=2 CMAKE_BUILD_PARALLEL_LEVEL=2
ops="$S60_SOURCE/dev/experiments/0.8.29-slice60-projected-metadata"
```

Check driver/primary CUDA/nvcc/room before and after; never change them. Require
an idle3090 with at least20480MiB free and no foreign compute process. K620 must
remain hidden from CUDA. Independently copy only current pinned config.json,
tokenizer.json and model.safetensors into
`$S60_TASK/gpu-metadata/model-cache/fathomdb/embedders/0b2926f8a9b1/`;
verify all three source/destination SHA256 values from the final launcher,
regular non-symlink files and distinct inodes. Never download on a miss.
Normal CPU/full-gate environment/cache/features/targets stay unchanged.

```bash
# GPU build in a subshell: no global environment change and no model execution.
(source "$ops/build_env.sh"; cargo build --manifest-path "$ops/driver/Cargo.toml"   --features cuda-producer --locked)
# One fresh producer; runner verifies hashes/room/namespace and records real exits.
python "$ops/resource_guard.py" manual-producer python "$ops/run_producer.py" run-001
# Once producer has closed, separate no-CUDA SDK control artifact/session.
CARGO_TARGET_DIR="$S60_TASK/gpu-metadata/cpu-control-target" cargo build   --manifest-path "$ops/driver/Cargo.toml" --no-default-features --locked
"$S60_TASK/gpu-metadata/cpu-control-target/debug/s60-manual-metadata-acceptance"   controls "$S60_TASK/gpu-metadata/run-001"   > "$S60_TASK/gpu-metadata/run-001/controls.jsonl"
# Normal actual candidate shipping consumer, separate from CUDA producer target.
cargo build -p fathomdb-mcp --bin fathomdb-mcp --no-default-features --locked
python "$ops/mcp_check.py" "$CARGO_TARGET_DIR/debug/fathomdb-mcp"   "$S60_TASK/gpu-metadata/run-001"
```

The final exec boundary is essential: offline flags alone do not stop the loader
from downloading. If namespace/device/FD/pin/room/engagement checks fail, preserve
the real failure and stop/hold, without CPU/live-network fallback or extra encode.
The optional model-free probe is `python "$ops/netns_descriptor_probes.py"`;
it requires a fresh fd-probes output directory and performs no model access.

Actual execution used the archived reviewed sources through
`unshare -rUn -- bash <netns-exec.v3.sh> run <host-netns> <producer> <run-root>`.
The exact source/build/run commands and private-lock hashes are retained in task
receipts manual-cuda-producer-build, manual-gpu-producer, manual-cpu-consumers and
manual-text-consumer-resume. Private lock SHA256 was
99110175134552542ddc4200d9bfb8cee892706ef760cfa375efbf923c63e543;
root lock SHA256 was bf2c9122b50fe841f2407c625755ebc910491699e41872db4690b09c66c19525.

## Separate final production binding lifecycle

After a clean committed checkpoint, rebuild the owned noneditable Python wheel
with `bash scripts/setup-python-owned.sh --build` and TypeScript production
package with `npm run build` from src/ts. Capture source/lock/features/artifact
hashes and real exits. With the base owned environment, execute these operator
scripts without PYTHONPATH and before the canonical test-hook gate replaces
any test artifact:

```bash
env -u PYTHONPATH "$S60_SOURCE/.venv/bin/python" "$ops/production_lifecycle.py" "$S60_TASK/production-python.sqlite"
node "$ops/production_lifecycle.mjs" "$S60_TASK/production-typescript.sqlite"
```

The canonical full gate's nonce-bound Python candidate/child native receipt is
separate evidence, not a substitute for these production open/close/exit checks.
