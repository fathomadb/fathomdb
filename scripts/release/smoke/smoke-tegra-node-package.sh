#!/usr/bin/env bash
# Installed-package smoke for the Jetson Tegra Node addon
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decisions 6 and 8).
#
# It installs the main `fathomdb` package and the separately named
# `fathomdb-linux-arm64-gnu-tegra` package into fresh consumers outside the
# source tree, with lifecycle scripts disabled, and proves on a confirmed
# classic Jetson Orin that:
#   - the loaded addon is the installed Tegra binary, by path and SHA-256, also
#     when a generic linux-arm64-gnu package is installed beside it;
#   - `cpu`, `auto` and `cuda:0` each resolve as required for embedding and
#     for reranking, with forced CUDA never falling back to CPU;
#   - forced CUDA embedding yields an in-process GPU allocation witness that
#     verify-tegra-gpu-witness.py accepts against nvidia-smi;
#   - the heap-growth regression passes against the installed main package.
# Every row is retained, and tegra_node_package_verdict.py decides the result
# from that evidence: a missing, skipped or CPU-fallback row is a failure.
#
# Sources:
#   local  the tarballs build-napi-cuda-tegra.sh packed, installed offline.
#   pages  `fathomdb@<version>` from the npm registry plus the Tegra tarball
#          from its exact first-party Pages URL, checked against the SHA-256
#          retained from the Jetson run before it is installed.
#
# Nothing here publishes. Exit 2 is a usage error; exit 1 is any failure.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
ROW_SCRIPT="$SCRIPT_DIR/tegra_node_package_row.mjs"
VERDICT="$SCRIPT_DIR/tegra_node_package_verdict.py"
HEAP_TEST="$REPO_ROOT/scripts/tests/test_tegra_node_early_cuinit.sh"
WITNESS_VERIFIER="$REPO_ROOT/scripts/release/verify-tegra-gpu-witness.py"
TEGRA_PACKAGE='fathomdb-linux-arm64-gnu-tegra'
GENERIC_PACKAGE='fathomdb-linux-arm64-gnu'
NATIVE_NAME='fathomdb.linux-arm64-gnu.node'
PAGES_PACKAGES_URL='https://fathomadb.github.io/fathomdb/tegra/packages'
NPM_REGISTRY='https://registry.npmjs.org/'
# The pinned TinyBERT revision's cache directory; candle_reranker.rs derives
# it from sha256("<repo>@<revision>")[..12].
RERANKER_CACHE_DIR='fathomdb/reranker/0290849b0459'

usage() {
  cat >&2 <<USAGE
usage: $(basename "$0") --source local --main-tarball <tgz> --tegra-tarball <tgz>
         [--generic-tarball <tgz>] --candidate-sha <40-hex> --evidence-dir <new-dir>
       $(basename "$0") --source pages --version X.Y.Z
         --pages-url $PAGES_PACKAGES_URL/$TEGRA_PACKAGE-X.Y.Z.tgz
         --tegra-sha256 <64-hex> --pages-run <run-id>
         --candidate-sha <40-hex> --evidence-dir <new-dir>
USAGE
}
usage_error() {
  printf 'smoke-tegra-node-package: %s\n' "$1" >&2
  usage
  exit 2
}
fail() {
  printf 'smoke-tegra-node-package: %s\n' "$1" >&2
  exit 1
}

SOURCE='' MAIN_TARBALL='' TEGRA_TARBALL='' GENERIC_TARBALL='' CANDIDATE_SHA=''
EVIDENCE_DIR='' VERSION='' PAGES_URL='' EXPECTED_TEGRA_SHA256='' PAGES_RUN=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --source | --main-tarball | --tegra-tarball | --generic-tarball | --candidate-sha | \
      --evidence-dir | --version | --pages-url | --tegra-sha256 | --pages-run)
      [ "$#" -ge 2 ] || usage_error "$1 needs a value"
      case "$1" in
        --source) SOURCE="$2" ;;
        --main-tarball) MAIN_TARBALL="$2" ;;
        --tegra-tarball) TEGRA_TARBALL="$2" ;;
        --generic-tarball) GENERIC_TARBALL="$2" ;;
        --candidate-sha) CANDIDATE_SHA="$2" ;;
        --evidence-dir) EVIDENCE_DIR="$2" ;;
        --version) VERSION="$2" ;;
        --pages-url) PAGES_URL="$2" ;;
        --tegra-sha256) EXPECTED_TEGRA_SHA256="$2" ;;
        --pages-run) PAGES_RUN="$2" ;;
        *) usage_error "unhandled argument $1" ;;
      esac
      shift 2
      ;;
    *) usage_error "unknown argument: $1" ;;
  esac
done

[[ "$CANDIDATE_SHA" =~ ^[0-9a-f]{40}$ ]] || usage_error 'candidate SHA must be 40 lowercase hexadecimal characters'
[ -n "$EVIDENCE_DIR" ] || usage_error '--evidence-dir is required'
case "$SOURCE" in
  local)
    [ -n "$MAIN_TARBALL" ] && [ -n "$TEGRA_TARBALL" ] ||
      usage_error 'local source needs --main-tarball and --tegra-tarball'
    [ -z "$VERSION$PAGES_URL$EXPECTED_TEGRA_SHA256$PAGES_RUN" ] ||
      usage_error 'local source does not accept --version, --pages-url, --tegra-sha256 or --pages-run'
    ;;
  pages)
    [ -z "$MAIN_TARBALL$TEGRA_TARBALL$GENERIC_TARBALL" ] ||
      usage_error 'pages source installs from npm and Pages, never from local tarballs'
    [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage_error 'pages source needs --version X.Y.Z'
    [ "$PAGES_URL" = "$PAGES_PACKAGES_URL/$TEGRA_PACKAGE-$VERSION.tgz" ] ||
      usage_error "pages URL must be exactly $PAGES_PACKAGES_URL/$TEGRA_PACKAGE-$VERSION.tgz"
    [[ "$EXPECTED_TEGRA_SHA256" =~ ^[0-9a-f]{64}$ ]] ||
      usage_error 'pages source needs --tegra-sha256 with 64 lowercase hexadecimal characters'
    [[ "$PAGES_RUN" =~ ^[0-9]+$ ]] || usage_error 'pages source needs a numeric --pages-run'
    ;;
  '') usage_error '--source is required' ;;
  *) usage_error "unknown source: $SOURCE (expected local or pages)" ;;
esac

# --- Inputs and offline prerequisites, before any host probe ----------------
[ ! -e "$EVIDENCE_DIR" ] || fail "evidence path must be new: $EVIDENCE_DIR"
if [ "$SOURCE" = local ]; then
  for tarball in "$MAIN_TARBALL" "$TEGRA_TARBALL" ${GENERIC_TARBALL:+"$GENERIC_TARBALL"}; do
    [ -f "$tarball" ] || fail "tarball is absent: $tarball"
  done
fi
HF_CACHE="${FATHOMDB_TEGRA_HF_HOME:-$HOME/.cache/huggingface}"
[ -d "$HF_CACHE" ] || fail "offline model cache is absent: $HF_CACHE"
RERANKER_CACHE="${FATHOMDB_TEGRA_RERANKER_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}}"
for file in config.json tokenizer.json model.safetensors; do
  [ -f "$RERANKER_CACHE/$RERANKER_CACHE_DIR/$file" ] ||
    fail "offline reranker cache is absent: $RERANKER_CACHE/$RERANKER_CACHE_DIR/$file"
done

# --- Toolchain and classic-Orin host ----------------------------------------
# shellcheck source=scripts/release/cuda-artifact-contract.sh
. "$REPO_ROOT/scripts/release/cuda-artifact-contract.sh"
command -v node >/dev/null 2>&1 || fail 'node is not on PATH'
command -v npm >/dev/null 2>&1 || fail 'npm is not on PATH'
NODE_VERSION="$(node --version)"
NPM_VERSION="$(npm --version)"
[ "$NODE_VERSION" = "v$CUDA_TEGRA_NAPI_NODE_VERSION" ] ||
  fail "node does not match the release contract (v$CUDA_TEGRA_NAPI_NODE_VERSION): $NODE_VERSION"
[ "$NPM_VERSION" = "$CUDA_TEGRA_NAPI_NPM_VERSION" ] ||
  fail "npm does not match the release contract ($CUDA_TEGRA_NAPI_NPM_VERSION): $NPM_VERSION"

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"
[ "$HOST_OS" = Linux ] && [ "$HOST_ARCH" = aarch64 ] || fail 'expected a Linux aarch64 Jetson host'
if ! grep -aq 'nvidia,tegra' /proc/device-tree/compatible 2>/dev/null &&
  [ ! -e /etc/nv_tegra_release ]; then
  fail 'classic Tegra platform signal is absent'
fi
NVIDIA_SMI=''
for candidate in /usr/bin/nvidia-smi /usr/sbin/nvidia-smi /usr/local/bin/nvidia-smi; do
  if [ -x "$candidate" ]; then
    NVIDIA_SMI="$candidate"
    break
  fi
done
[ -n "$NVIDIA_SMI" ] || fail 'no approved absolute nvidia-smi is executable'
GPU_NAME="$(timeout 2s "$NVIDIA_SMI" --query-gpu=name --format=csv,noheader)"
case "$GPU_NAME" in
  *nvgpu*) ;;
  *) fail "host is not a confirmed classic Tegra CUDA device: $GPU_NAME" ;;
esac

# --- Evidence and scratch -----------------------------------------------------
mkdir -p "$EVIDENCE_DIR/rows" "$EVIDENCE_DIR/consumers"
EVIDENCE_DIR="$(cd "$EVIDENCE_DIR" && pwd -P)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-tegra-node-smoke.XXXXXX")"
WORK="$(cd "$WORK" && pwd -P)"
cleanup() {
  case "$WORK" in
    "${TMPDIR:-/tmp}"/* | /tmp/*) rm -rf "$WORK" ;;
    *) printf 'refusing to remove unexpected temp path: %s\n' "$WORK" >&2 ;;
  esac
}
trap cleanup EXIT
REPO_REAL="$(cd "$REPO_ROOT" && pwd -P)"
case "$WORK/" in
  "$REPO_REAL"/*) fail "consumer must be outside the source tree: $WORK" ;;
  *) ;;
esac
DB_ROOT="$WORK/databases"
mkdir -p "$DB_ROOT"

GPU_INFO="$(timeout 2s "$NVIDIA_SMI" --query-gpu=uuid,name,driver_version --format=csv,noheader)"
{
  printf 'uname='
  uname -srm
  printf 'nvidia_smi=%s\ngpu=%s\n' "$NVIDIA_SMI" "$GPU_INFO"
  cat /etc/nv_tegra_release 2>/dev/null || true
} >"$EVIDENCE_DIR/host-preflight.txt"
printf 'node=%s\nnpm=%s\n' "$NODE_VERSION" "$NPM_VERSION" >"$EVIDENCE_DIR/toolchain.txt"

# npm user/global configuration and npm_config_* variables could inject a
# registry, scripts or a cache; every npm call ignores them.
npm_clean() {
  local -a scrub=()
  local name names
  names="$(compgen -e)"
  while IFS= read -r name; do
    case "${name,,}" in
      npm_config_*) scrub+=(-u "$name") ;;
      *) ;;
    esac
  done <<<"$names"
  # npm refuses one file as both user and global config, hence a second
  # empty file for the global layer.
  : >"$WORK/npm-global-config"
  env "${scrub[@]}" NPM_CONFIG_USERCONFIG=/dev/null NPM_CONFIG_GLOBALCONFIG="$WORK/npm-global-config" \
    npm --cache "$WORK/npm-cache" "$@"
}

# Rows run without CUDA library search paths, preloads or caller policy, so
# the installed addon must load and resolve the device on its own.
runtime_env() {
  env -u LD_LIBRARY_PATH -u LD_PRELOAD -u LIBRARY_PATH -u CUDA_PATH -u CUDA_HOME \
    -u CUDA_VISIBLE_DEVICES -u NODE_OPTIONS -u NODE_PATH \
    -u FATHOMDB_EMBED_DEVICE -u FATHOMDB_RERANK_DEVICE -u FATHOMDB_GPU_ALLOCATION_WITNESS \
    -u FATHOMDB_CUDA_EARLY_INIT -u FATHOMDB_REQUIRE_LIVE \
    HF_HOME="$HF_CACHE" HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
    FATHOMDB_RERANKER_CACHE="$RERANKER_CACHE" "$@"
}

abs_path() {
  local dir
  dir="$(cd "$(dirname "$1")" && pwd -P)"
  printf '%s/%s\n' "$dir" "${1##*/}"
}

# Fails closed unless the packed manifest carries the expected field value.
require_manifest_field() {
  local tarball="$1" field="$2" expected="$3" actual
  actual="$(tarball_manifest_field "$tarball" "$field")"
  [ "$actual" = "$expected" ] || fail "$tarball: package $field is $actual, expected $expected"
}

tarball_manifest_field() {
  tar -xzOf "$1" package/package.json | python3 -c \
    'import json, sys; print(json.load(sys.stdin)[sys.argv[1]])' "$2"
}

new_consumer() {
  local consumer="$WORK/consumer-$1"
  mkdir -p "$consumer"
  printf '{"name":"fathomdb-tegra-node-smoke-%s","private":true,"type":"module"}\n' "$1" \
    >"$consumer/package.json"
  cp "$ROW_SCRIPT" "$consumer/tegra_node_package_row.mjs"
  printf '%s\n' "$consumer"
}

retain_consumer() {
  local name="$1" consumer="$2"
  mkdir -p "$EVIDENCE_DIR/consumers/$name"
  cp "$consumer/package.json" "$consumer/package-lock.json" "$EVIDENCE_DIR/consumers/$name/"
}

# --- Acquire the packages -------------------------------------------------------
declare -A CONSUMER_ROOTS=() CONSUMER_GENERIC=()
if [ "$SOURCE" = local ]; then
  MAIN_TARBALL="$(abs_path "$MAIN_TARBALL")"
  TEGRA_TARBALL="$(abs_path "$TEGRA_TARBALL")"
  VERSION="$(tarball_manifest_field "$MAIN_TARBALL" version)"
  require_manifest_field "$MAIN_TARBALL" name fathomdb
  require_manifest_field "$TEGRA_TARBALL" name "$TEGRA_PACKAGE"
  # The loader refuses a mismatched pair; refuse it here first, by name.
  require_manifest_field "$TEGRA_TARBALL" version "$VERSION"

  if [ -n "$GENERIC_TARBALL" ]; then
    GENERIC_KIND=tarball
    GENERIC_TARBALL="$(abs_path "$GENERIC_TARBALL")"
    require_manifest_field "$GENERIC_TARBALL" name "$GENERIC_PACKAGE"
  else
    # No unpublished generic package can be resolved offline, so a decoy with
    # the generic name stands in. Loading it throws, so a pass proves the
    # loader chose the Tegra package although the generic one was installed.
    GENERIC_KIND=decoy
    decoy="$WORK/decoy-generic"
    mkdir -p "$decoy"
    python3 - "$decoy/package.json" "$GENERIC_PACKAGE" "$VERSION" <<'PY'
import json
import sys

path, name, version = sys.argv[1:]
manifest = {
    "name": name,
    "version": version,
    "description": "Smoke-only stand-in for the generic package; loading it is a selection failure.",
    "os": ["linux"],
    "cpu": ["arm64"],
    "libc": ["glibc"],
    "main": "index.js",
}
open(path, "w").write(json.dumps(manifest, indent=2) + "\n")
PY
    printf 'throw new Error("the generic linux-arm64-gnu stand-in was loaded instead of the Tegra package");\n' \
      >"$decoy/index.js"
    (cd "$decoy" && npm_clean pack --ignore-scripts --pack-destination "$WORK" >/dev/null) ||
      fail 'could not pack the generic stand-in package'
    GENERIC_TARBALL="$WORK/$GENERIC_PACKAGE-$VERSION.tgz"
    [ -f "$GENERIC_TARBALL" ] || fail 'could not pack the generic stand-in package'
  fi

  CONSUMER="$(new_consumer tegra-only)"
  (cd "$CONSUMER" && npm_clean install --offline --ignore-scripts --no-audit --no-fund \
    "$MAIN_TARBALL" "$TEGRA_TARBALL" >"$EVIDENCE_DIR/consumers/install-tegra-only.log" 2>&1) ||
    fail 'offline install of the main and Tegra tarballs failed'
  CONSUMER_ROOTS[tegra-only]="$CONSUMER"
  CONSUMER_GENERIC[tegra-only]=absent

  coinstalled="$(new_consumer generic-coinstalled)"
  (cd "$coinstalled" && npm_clean install --offline --ignore-scripts --no-audit --no-fund \
    "$MAIN_TARBALL" "$TEGRA_TARBALL" "$GENERIC_TARBALL" \
    >"$EVIDENCE_DIR/consumers/install-generic-coinstalled.log" 2>&1) ||
    fail 'offline install with the generic package co-installed failed'
  CONSUMER_ROOTS[generic-coinstalled]="$coinstalled"
  CONSUMER_GENERIC[generic-coinstalled]="$GENERIC_KIND"
  PRIMARY=tegra-only
else
  mkdir -p "$WORK/download"
  TEGRA_TARBALL="$WORK/download/$TEGRA_PACKAGE-$VERSION.tgz"
  curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
    --retry 4 --retry-all-errors "$PAGES_URL" --output "$TEGRA_TARBALL" ||
    fail "could not download $PAGES_URL"
  ACTUAL_TEGRA_SHA256="$(sha256sum "$TEGRA_TARBALL" | awk '{print $1}')"
  [ "$ACTUAL_TEGRA_SHA256" = "$EXPECTED_TEGRA_SHA256" ] ||
    fail "downloaded tarball SHA-256 differs from retained evidence: expected $EXPECTED_TEGRA_SHA256, got $ACTUAL_TEGRA_SHA256"
  require_manifest_field "$TEGRA_TARBALL" name "$TEGRA_PACKAGE"
  require_manifest_field "$TEGRA_TARBALL" version "$VERSION"

  CONSUMER="$(new_consumer pages)"
  (cd "$CONSUMER" && npm_clean install --ignore-scripts --no-audit --no-fund \
    --registry "$NPM_REGISTRY" "fathomdb@$VERSION" "$TEGRA_TARBALL" \
    >"$EVIDENCE_DIR/consumers/install-pages.log" 2>&1) ||
    fail "install of fathomdb@$VERSION from npm with the Pages tarball failed"
  CONSUMER_ROOTS[pages]="$CONSUMER"
  if [ -f "$CONSUMER/node_modules/$GENERIC_PACKAGE/package.json" ]; then
    CONSUMER_GENERIC[pages]=registry
  else
    CONSUMER_GENERIC[pages]=absent
  fi
  PRIMARY=pages
fi

TEGRA_TARBALL_SHA256="$(sha256sum "$TEGRA_TARBALL" | awk '{print $1}')"
TEGRA_NODE_SHA256="$(tar -xzOf "$TEGRA_TARBALL" "package/$NATIVE_NAME" | sha256sum | awk '{print $1}')"
for name in "${!CONSUMER_ROOTS[@]}"; do
  retain_consumer "$name" "${CONSUMER_ROOTS[$name]}"
done

consumers_json="$(
  for name in "${!CONSUMER_ROOTS[@]}"; do
    printf '%s\t%s\t%s\n' "$name" "${CONSUMER_ROOTS[$name]}" "${CONSUMER_GENERIC[$name]}"
  done
)"
CONTEXT_CONSUMERS="$consumers_json" CONTEXT_SOURCE="$SOURCE" CONTEXT_CANDIDATE_SHA="$CANDIDATE_SHA" \
  CONTEXT_VERSION="$VERSION" CONTEXT_TEGRA_TARBALL_SHA256="$TEGRA_TARBALL_SHA256" \
  CONTEXT_TEGRA_NODE_SHA256="$TEGRA_NODE_SHA256" CONTEXT_PRIMARY="$PRIMARY" \
  CONTEXT_NODE="$NODE_VERSION" CONTEXT_NPM="$NPM_VERSION" \
  CONTEXT_MAIN_TARBALL="$MAIN_TARBALL" CONTEXT_TEGRA_TARBALL="$TEGRA_TARBALL" \
  CONTEXT_PAGES_URL="$PAGES_URL" CONTEXT_EXPECTED_TEGRA_SHA256="$EXPECTED_TEGRA_SHA256" \
  CONTEXT_PAGES_RUN="$PAGES_RUN" CONTEXT_NPM_REGISTRY="$NPM_REGISTRY" \
  python3 - "$EVIDENCE_DIR/context.json" <<'PY'
import json
import os
import sys

env = os.environ
consumers = {}
for line in env["CONTEXT_CONSUMERS"].splitlines():
    name, root, generic = line.split("\t")
    consumers[name] = {"root": root, "generic": generic}
context = {
    "schema": "fathomdb.tegra-node-package-smoke-context/v1",
    "source": env["CONTEXT_SOURCE"],
    "candidate_sha": env["CONTEXT_CANDIDATE_SHA"],
    "version": env["CONTEXT_VERSION"],
    "tegra_tarball_sha256": env["CONTEXT_TEGRA_TARBALL_SHA256"],
    "tegra_node_sha256": env["CONTEXT_TEGRA_NODE_SHA256"],
    "primary_consumer": env["CONTEXT_PRIMARY"],
    "consumers": consumers,
    "node": env["CONTEXT_NODE"],
    "npm": env["CONTEXT_NPM"],
}
if context["source"] == "local":
    context["main_tarball"] = env["CONTEXT_MAIN_TARBALL"]
    context["tegra_tarball"] = env["CONTEXT_TEGRA_TARBALL"]
else:
    context["pages_url"] = env["CONTEXT_PAGES_URL"]
    context["expected_tegra_tarball_sha256"] = env["CONTEXT_EXPECTED_TEGRA_SHA256"]
    context["pages_deployment_run"] = env["CONTEXT_PAGES_RUN"]
    context["npm_registry"] = env["CONTEXT_NPM_REGISTRY"]
with open(sys.argv[1], "w", encoding="utf-8") as stream:
    stream.write(json.dumps(context, indent=2, sort_keys=True) + "\n")
PY

# --- Rows ------------------------------------------------------------------------
run_in_consumer() {
  local name="$1" label="$2" status
  shift 2
  set +e
  (cd "${CONSUMER_ROOTS[$name]}" && runtime_env "$@" node tegra_node_package_row.mjs \
    "$name" "${ROW_ARGS[@]}" "$EVIDENCE_DIR/rows/$label.json" "$DB_ROOT") \
    >"$EVIDENCE_DIR/rows/$label.log" 2>&1
  status=$?
  set -e
  printf '%s\n' "$status" >"$EVIDENCE_DIR/rows/$label.rc"
  printf 'smoke-tegra-node-package: row %s exited %s\n' "$label" "$status"
}

run_select() {
  ROW_ARGS=(select -)
  run_in_consumer "$1" "$1-select"
}

run_row() {
  local component="$1" policy="$2"
  local -a policy_env=()
  case "$component" in
    embed) policy_env=(FATHOMDB_EMBED_DEVICE="$policy") ;;
    rerank) policy_env=(FATHOMDB_RERANK_DEVICE="$policy") ;;
    *) fail "unknown row component: $component" ;;
  esac
  if [ "$component" = embed ] && [ "$policy" = cuda:0 ]; then
    policy_env+=(FATHOMDB_GPU_ALLOCATION_WITNESS=1)
  fi
  ROW_ARGS=("$component" "$policy")
  run_in_consumer "$PRIMARY" "$component-${policy//:/}" "${policy_env[@]}"
}

for name in "${!CONSUMER_ROOTS[@]}"; do
  run_select "$name"
done
run_row embed cpu
run_row embed auto
run_row embed cuda:0
run_row rerank cpu
run_row rerank auto
run_row rerank cuda:0

# --- Witness -------------------------------------------------------------------------
WITNESS="$EVIDENCE_DIR/tegra-gpu-allocation-witness.json"
set +e
{
  python3 "$VERDICT" witness --evidence-dir "$EVIDENCE_DIR" --out "$WITNESS" &&
    PATH="/usr/sbin:$PATH" python3 "$WITNESS_VERIFIER" --witness "$WITNESS" --nvidia-smi
} >"$EVIDENCE_DIR/witness-verification.log" 2>&1
witness_status=$?
set -e
printf '%s\n' "$witness_status" >"$EVIDENCE_DIR/witness-verification.rc"

# --- Heap-growth regression against the installed main package ------------------
CONSUMER="${CONSUMER_ROOTS[$PRIMARY]}"
set +e
runtime_env FATHOMDB_REQUIRE_LIVE=1 \
  FATHOMDB_TEGRA_NODE_PACKAGE="$CONSUMER/node_modules/fathomdb" \
  bash "$HEAP_TEST" >"$EVIDENCE_DIR/heap-growth.log" 2>&1
heap_status=$?
set -e
printf '%s\n' "$heap_status" >"$EVIDENCE_DIR/heap-growth.rc"

# --- Verdict ---------------------------------------------------------------------------
if python3 "$VERDICT" verdict --evidence-dir "$EVIDENCE_DIR"; then
  printf 'smoke-tegra-node-package: PASS — %s %s@%s on %s (evidence: %s)\n' \
    "$SOURCE" "$TEGRA_PACKAGE" "$VERSION" "$GPU_NAME" "$EVIDENCE_DIR"
else
  fail "FAIL — see $EVIDENCE_DIR/summary.json"
fi
