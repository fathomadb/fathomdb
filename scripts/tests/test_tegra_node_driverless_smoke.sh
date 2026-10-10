#!/usr/bin/env bash
# Contract for the driverless AArch64 import of the Jetson Tegra Node package
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decisions 5 and 6).
#
# The live smoke runs only on a hosted, non-Tegra Linux ARM64 runner without
# a CUDA driver: it installs the Jetson-built tarballs into fresh consumers
# inside a network-less container, imports and opens the Tegra addon directly,
# and proves the main package's loader ignores the Tegra package there. These
# checks run on any host: argument handling, the host refusal, the verdict
# over retained-evidence fixtures and the structural guarantees of the script.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SMOKE="$REPO_ROOT/scripts/release/smoke/smoke-tegra-node-driverless.sh"
PROBE="$REPO_ROOT/scripts/release/smoke/tegra_node_driverless_probe.sh"
ROW="$REPO_ROOT/scripts/release/smoke/tegra_node_driverless_row.mjs"
VERDICT="$REPO_ROOT/scripts/release/smoke/tegra_node_driverless_verdict.mjs"
CONTRACT="$REPO_ROOT/scripts/release/cuda-artifact-contract.sh"
CANDIDATE='0123456789abcdef0123456789abcdef01234567'
FAILED=0

WORK="$(mktemp -d)"
cleanup() {
  case "$WORK" in
    "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$WORK" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$WORK" >&2 ;;
  esac
}
trap cleanup EXIT
pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

for file in "$SMOKE" "$PROBE" "$ROW" "$VERDICT"; do
  if [ -f "$file" ]; then
    pass "driverless smoke component exists: ${file#"$REPO_ROOT"/}"
  else
    fail "driverless smoke component is absent: ${file#"$REPO_ROOT"/}"
  fi
done
if [ "$FAILED" -ne 0 ]; then
  printf '%s driverless Tegra Node smoke contract check(s) failed\n' "$FAILED" >&2
  exit 1
fi
if [ -x "$SMOKE" ]; then pass 'smoke is executable'; else fail 'smoke must be executable'; fi

# --- Argument handling ------------------------------------------------------
printf 'not a tarball\n' >"$WORK/main.tgz"
printf 'not a tarball\n' >"$WORK/tegra.tgz"
expect_usage_error() {
  local label="$1" status
  shift
  set +e
  bash "$SMOKE" "$@" >"$WORK/usage.out" 2>&1
  status=$?
  set -e
  if [ "$status" -eq 2 ]; then
    pass "$label"
  else
    fail "$label (exit $status)"
    cat "$WORK/usage.out" >&2
  fi
}
expect_usage_error 'no arguments is a usage error'
expect_usage_error 'an unknown flag is a usage error' --skip-loader
expect_usage_error 'both tarballs are required' \
  --main-tarball "$WORK/main.tgz" --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e1"
expect_usage_error 'candidate SHA must be 40 lowercase hex' \
  --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha ABC --evidence-dir "$WORK/e2"
expect_usage_error 'evidence directory is required' \
  --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" --candidate-sha "$CANDIDATE"

expect_failure() {
  local label="$1" needle="$2" status
  shift 2
  set +e
  "$@" >"$WORK/failure.out" 2>&1
  status=$?
  set -e
  if [ "$status" -eq 1 ] && grep -Fq -- "$needle" "$WORK/failure.out"; then
    pass "$label"
  else
    fail "$label (exit $status)"
    cat "$WORK/failure.out" >&2
  fi
}
mkdir -p "$WORK/existing-evidence"
expect_failure 'an existing evidence directory is refused' 'evidence path must be new' \
  bash "$SMOKE" --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/existing-evidence"
expect_failure 'an absent tarball is refused' 'tarball is absent' \
  bash "$SMOKE" --main-tarball "$WORK/absent.tgz" --tegra-tarball "$WORK/tegra.tgz" \
  --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e3"

# --- The host refusal -------------------------------------------------------
# The proof only means something on a non-Tegra AArch64 host without a CUDA
# driver, so every other host is refused before any container runs. The
# expectation is derived here independently of the script.
host_reason=''
host_os="$(uname -s)"
host_arch="$(uname -m)"
if [ "$host_os" != Linux ] || [ "$host_arch" != aarch64 ]; then
  host_reason='not a Linux AArch64 host'
elif grep -aqs 'nvidia,tegra' /proc/device-tree/compatible || [ -e /etc/nv_tegra_release ]; then
  host_reason='Tegra-family host'
elif ld_cache="$(ldconfig -p 2>/dev/null || true)" && [[ "$ld_cache" == *libcuda.so* ]]; then
  host_reason='CUDA driver library is present'
fi
if [ -n "$host_reason" ]; then
  expect_failure "this host is refused: $host_reason" "$host_reason" \
    bash "$SMOKE" --main-tarball "$WORK/main.tgz" --tegra-tarball "$WORK/tegra.tgz" \
    --candidate-sha "$CANDIDATE" --evidence-dir "$WORK/e4"
  if [ ! -e "$WORK/e4/rows" ]; then
    pass 'a refused host runs no row'
  else
    fail 'a refused host must not run any row'
  fi
else
  printf 'NOTE  this host is a driverless non-Tegra AArch64 host; the refusal case does not apply\n'
fi

# --- Structural guarantees of the live path ---------------------------------
contains() {
  local label="$1" needle="$2" file="$3"
  if grep -Fq -- "$needle" "$file"; then pass "$label"; else fail "$label (missing: $needle)"; fi
}
absent() {
  local label="$1" needle="$2" file="$3"
  if grep -Fq -- "$needle" "$file"; then fail "$label (unexpected: $needle)"; else pass "$label"; fi
}
contains 'the contract pins a separate ARM64 driverless Node image' \
  "export CUDA_TEGRA_DRIVERLESS_NODE_IMAGE='node:25-trixie-slim'" "$CONTRACT"
contains 'smoke sources the CUDA artifact contract' 'cuda-artifact-contract.sh' "$SMOKE"
# shellcheck disable=SC2016 # the needle is the smoke's literal, unexpanded text
contains 'smoke runs the ARM64 driverless image' '"$CUDA_TEGRA_DRIVERLESS_NODE_IMAGE"' "$SMOKE"
absent 'smoke does not reuse the x86_64 driverless image' 'CUDA_DRIVERLESS_NODE_IMAGE"' "$SMOKE"
contains 'container has no network' '--network none' "$SMOKE"
contains 'container is ARM64' '--platform linux/arm64' "$SMOKE"
absent 'container receives no GPU' '--gpus' "$SMOKE"
absent 'container receives no NVIDIA runtime' '--runtime' "$SMOKE"
contains 'tarballs are mounted read-only' 'readonly' "$SMOKE"
contains 'smoke hardens bash' 'set -euo pipefail' "$SMOKE"
contains 'probe hardens bash' 'set -euo pipefail' "$PROBE"
contains 'every probe install disables lifecycle scripts' '--ignore-scripts' "$PROBE"
npm_install_lines="$(grep -nE '(^|[^_[:alnum:]])npm[[:space:]]+(install|ci|i)([[:space:]]|$)' "$PROBE" || true)"
unscripted="$(grep -v -- '--ignore-scripts' <<<"$npm_install_lines" || true)"
if [ -n "$npm_install_lines" ] && [ -z "$unscripted" ]; then
  pass 'no probe npm install runs lifecycle scripts'
else
  fail "probe npm installs must all pass --ignore-scripts: ${npm_install_lines:-none}"
fi
contains 'probe installs offline' '--offline' "$PROBE"
contains 'probe records the driver-library search' 'libcuda' "$PROBE"
contains 'probe records the Tegra-family signal' 'nvidia,tegra' "$PROBE"
contains 'probe imports the addon with early CUDA init on' 'early-init-default' "$PROBE"
contains 'probe imports the addon with early CUDA init off' 'FATHOMDB_CUDA_EARLY_INIT=off' "$PROBE"
contains 'probe co-installs a generic stand-in' 'generic-coinstalled' "$PROBE"
contains 'probe checks the loader without a generic package' 'tegra-only' "$PROBE"
contains 'probe computes the verdict from the retained rows' 'tegra_node_driverless_verdict.mjs' "$PROBE"
for forbidden in 'npm publish' 'gh release' 'gh workflow' 'twine upload' 'cargo publish' 'deploy-pages' 'postinstall'; do
  absent "smoke cannot publish or fetch at install: $forbidden" "$forbidden" "$SMOKE"
  absent "probe cannot publish or fetch at install: $forbidden" "$forbidden" "$PROBE"
done

# --- Verdict over retained evidence -----------------------------------------
make_fixture() {
  local dir="$1"
  python3 - "$dir" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
rows = root / "rows"
rows.mkdir(parents=True)
addon = "/work/consumers/tegra-only/node_modules/fathomdb-linux-arm64-gnu-tegra/fathomdb.linux-arm64-gnu.node"
context = {
    "schema": "fathomdb.tegra-node-driverless-context/v1",
    "candidate_sha": "0123456789abcdef0123456789abcdef01234567",
    "version": "0.8.27",
    "main_tarball_sha256": "b" * 64,
    "tegra_tarball_sha256": "c" * 64,
    "tegra_native_sha256": "a" * 64,
    "tegra_native_path": addon,
    "host": {
        "arch": "aarch64",
        "tegra_family_signal": False,
        "nvidia_smi": [],
        "libcuda": [],
        "gpu_device_nodes": [],
    },
}
(root / "context.json").write_text(json.dumps(context, indent=2) + "\n")
for name, early_init in (("tegra-addon-early-init-default", None), ("tegra-addon-early-init-off", "off")):
    row = {
        "schema": "fathomdb.tegra-node-driverless-row/v1",
        "name": name,
        "kind": "tegra-addon",
        "early_init_env": early_init,
        "outcome": "completed",
        "error": None,
        "engine_open_type": "function",
        "opened": True,
        "closed": True,
        "loaded_native_addons": [addon],
        "loaded_native_sha256": {addon: "a" * 64},
        "libcuda_mapped": False,
    }
    (rows / f"{name}.json").write_text(json.dumps(row) + "\n")
messages = {
    "loader-generic-coinstalled": (
        'Unsupported platform: no prebuilt native binary for linux/arm64 (linux-arm64-gnu). '
        'The optional platform package "fathomdb-linux-arm64-gnu" is not installed. '
        'Cause: the generic linux-arm64-gnu stand-in was selected'
    ),
    "loader-tegra-only": (
        'Unsupported platform: no prebuilt native binary for linux/arm64 (linux-arm64-gnu). '
        'The optional platform package "fathomdb-linux-arm64-gnu" is not installed. '
        "Cause: Cannot find module 'fathomdb-linux-arm64-gnu'"
    ),
}
for name, message in messages.items():
    row = {
        "schema": "fathomdb.tegra-node-driverless-row/v1",
        "name": name,
        "kind": "loader",
        "outcome": "completed",
        "imported": False,
        "error_name": "UnsupportedPlatformError",
        "error_message": message,
        "loaded_native_addons": [],
    }
    (rows / f"{name}.json").write_text(json.dumps(row) + "\n")
PY
}

mutate_fixture() {
  local dir="$1" case_name="$2"
  python3 - "$dir" "$case_name" <<'PY'
import json
import sys
from pathlib import Path

root, case = Path(sys.argv[1]), sys.argv[2]
rows = root / "rows"

def load(name):
    return json.loads((rows / f"{name}.json").read_text())

def save(name, value):
    (rows / f"{name}.json").write_text(json.dumps(value) + "\n")

context = json.loads((root / "context.json").read_text())
if case == "missing-off-row":
    (rows / "tegra-addon-early-init-off.json").unlink()
elif case == "missing-loader-row":
    (rows / "loader-tegra-only.json").unlink()
elif case == "tegra-host":
    context["host"]["tegra_family_signal"] = True
elif case == "driver-present":
    context["host"]["libcuda"] = ["/usr/lib/aarch64-linux-gnu/libcuda.so.1"]
elif case == "gpu-node":
    context["host"]["gpu_device_nodes"] = ["/dev/nvidia0"]
elif case == "nvidia-smi":
    context["host"]["nvidia_smi"] = ["/usr/bin/nvidia-smi"]
elif case == "wrong-arch":
    context["host"]["arch"] = "x86_64"
elif case == "bad-candidate":
    context["candidate_sha"] = "HEAD"
elif case == "import-error":
    row = load("tegra-addon-early-init-default")
    row.update(outcome="error", error="libcuda.so.1: cannot open shared object file", opened=False)
    save("tegra-addon-early-init-default", row)
elif case == "not-opened":
    row = load("tegra-addon-early-init-off")
    row["opened"] = False
    save("tegra-addon-early-init-off", row)
elif case == "other-addon":
    row = load("tegra-addon-early-init-default")
    row["loaded_native_addons"] = ["/tmp/other.node"]
    row["loaded_native_sha256"] = {"/tmp/other.node": "a" * 64}
    save("tegra-addon-early-init-default", row)
elif case == "wrong-sha":
    row = load("tegra-addon-early-init-default")
    addon = context["tegra_native_path"]
    row["loaded_native_sha256"] = {addon: "d" * 64}
    save("tegra-addon-early-init-default", row)
elif case == "libcuda-mapped":
    row = load("tegra-addon-early-init-off")
    row["libcuda_mapped"] = True
    save("tegra-addon-early-init-off", row)
elif case == "early-init-not-off":
    row = load("tegra-addon-early-init-off")
    row["early_init_env"] = None
    save("tegra-addon-early-init-off", row)
elif case == "loader-selected-tegra":
    row = load("loader-generic-coinstalled")
    row["loaded_native_addons"] = [context["tegra_native_path"]]
    save("loader-generic-coinstalled", row)
elif case == "loader-imported":
    row = load("loader-tegra-only")
    row.update(imported=True, error_name=None, error_message=None)
    save("loader-tegra-only", row)
elif case == "loader-jetson-error":
    row = load("loader-tegra-only")
    row["error_message"] = (
        'The installed Jetson package "fathomdb-linux-arm64-gnu-tegra" was selected for this '
        "classic Jetson Orin, but its native binary failed to load."
    )
    save("loader-tegra-only", row)
elif case == "loader-mismatch-error":
    row = load("loader-generic-coinstalled")
    row["error_name"] = "PlatformPackageVersionMismatchError"
    save("loader-generic-coinstalled", row)
elif case == "loader-generic-not-selected":
    row = load("loader-generic-coinstalled")
    row["error_message"] = row["error_message"].replace("stand-in was selected", "something else")
    save("loader-generic-coinstalled", row)
else:
    sys.exit(f"unknown mutation {case}")
(root / "context.json").write_text(json.dumps(context, indent=2) + "\n")
PY
}

run_verdict() {
  node "$VERDICT" "$1" >"$WORK/verdict.out" 2>&1
}

make_fixture "$WORK/good"
if run_verdict "$WORK/good" \
  && node -e 'const s=require(process.argv[1]); if (s.outcome !== "PASS") process.exit(1)' "$WORK/good/summary.json"; then
  pass 'complete driverless evidence yields a PASS summary'
else
  fail 'complete driverless evidence must pass the verdict with outcome PASS'
  cat "$WORK/verdict.out" >&2
fi

expect_verdict_failure() {
  local case_name="$1" needle="$2" label="$3" dir="$WORK/mut-$1"
  make_fixture "$dir"
  mutate_fixture "$dir" "$case_name"
  if run_verdict "$dir"; then
    fail "$label (verdict passed)"
  elif ! grep -Fq -- "$needle" "$WORK/verdict.out"; then
    fail "$label (diagnostic lacks: $needle)"
    cat "$WORK/verdict.out" >&2
  elif node -e 'const s=require(process.argv[1]); if (s.outcome !== "FAIL") process.exit(1)' "$dir/summary.json" 2>/dev/null; then
    pass "$label"
  else
    fail "$label (summary.json must record outcome FAIL)"
  fi
}
expect_verdict_failure missing-off-row 'tegra-addon-early-init-off: row is missing' 'a missing early-init-off row fails'
expect_verdict_failure missing-loader-row 'loader-tegra-only: row is missing' 'a missing loader row fails'
expect_verdict_failure tegra-host 'host: Tegra-family signal' 'a Tegra-family host is not driverless proof'
expect_verdict_failure driver-present 'host: CUDA driver library' 'a host with libcuda is not driverless proof'
expect_verdict_failure gpu-node 'host: GPU device node' 'a host with a GPU device node is not driverless proof'
expect_verdict_failure nvidia-smi 'host: nvidia-smi' 'a host with nvidia-smi is not driverless proof'
expect_verdict_failure wrong-arch 'host: architecture x86_64' 'a non-AArch64 host is not proof'
expect_verdict_failure bad-candidate 'context: candidate SHA' 'evidence must bind a 40-hex candidate'
expect_verdict_failure import-error 'tegra-addon-early-init-default: outcome error' 'a failed driverless import fails'
expect_verdict_failure not-opened 'tegra-addon-early-init-off: engine did not open and close' 'an unopened engine fails'
expect_verdict_failure other-addon 'tegra-addon-early-init-default: loaded' 'loading another addon fails'
expect_verdict_failure wrong-sha 'tegra-addon-early-init-default: loaded addon SHA-256' 'loading different bytes fails'
expect_verdict_failure libcuda-mapped 'tegra-addon-early-init-off: libcuda was mapped' 'a mapped driver library fails'
expect_verdict_failure early-init-not-off 'tegra-addon-early-init-off: FATHOMDB_CUDA_EARLY_INIT' 'the off row must set early init off'
expect_verdict_failure loader-selected-tegra 'loader-generic-coinstalled: a native addon was loaded' 'the loader must not load the Tegra addon off-Tegra'
expect_verdict_failure loader-imported 'loader-tegra-only: fathomdb imported' 'the loader must not import through the Tegra package off-Tegra'
expect_verdict_failure loader-jetson-error 'loader-tegra-only: the loader selected the Jetson package' 'a Jetson selection error fails'
expect_verdict_failure loader-mismatch-error 'loader-generic-coinstalled: error PlatformPackageVersionMismatchError' 'a version-mismatch error off-Tegra fails'
expect_verdict_failure loader-generic-not-selected 'loader-generic-coinstalled: the generic stand-in was not selected' 'the co-installed generic package must be the selected one'

if [ "$FAILED" -ne 0 ]; then
  printf '%s driverless Tegra Node smoke contract check(s) failed\n' "$FAILED" >&2
  exit 1
fi
printf 'driverless Tegra Node smoke contract: all checks passed\n'
