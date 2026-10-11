#!/usr/bin/env bash
# Runs inside the network-less driverless container started by
# smoke-tegra-node-driverless.sh. Records the container's driver and Tegra
# facts, installs fresh consumers from /inputs, runs one fresh Node process
# per row and leaves the verdict to tegra_node_driverless_verdict.mjs, which
# writes /evidence/summary.json.
set -euo pipefail

SMOKE_DIR=/smoke
EVIDENCE=/evidence
WORK=/tmp/work
TEGRA_PACKAGE='fathomdb-linux-arm64-gnu-tegra'
GENERIC_PACKAGE='fathomdb-linux-arm64-gnu'
NATIVE_NAME='fathomdb.linux-arm64-gnu.node'
export NPM_CONFIG_USERCONFIG=/dev/null NPM_CONFIG_UPDATE_NOTIFIER=false
mkdir -p "$HOME" "$WORK/consumers" "$WORK/db" "$EVIDENCE/rows" "$EVIDENCE/consumers"

# --- Host facts, recorded for the verdict rather than trusted here ----------
tegra_signal=false
if grep -aqs 'nvidia,tegra' /proc/device-tree/compatible || [ -e /etc/nv_tegra_release ]; then
  tegra_signal=true
fi
{ ldconfig -p 2>/dev/null || true; } | awk '/libcuda\.so/ { print $NF }' >"$WORK/libcuda.txt"
find /lib /usr/lib /usr/local -name 'libcuda.so*' 2>/dev/null >>"$WORK/libcuda.txt" || true
: >"$WORK/smi.txt"
for smi in /usr/bin/nvidia-smi /usr/sbin/nvidia-smi /usr/local/bin/nvidia-smi; do
  [ ! -e "$smi" ] || printf '%s\n' "$smi" >>"$WORK/smi.txt"
done
find /dev -maxdepth 1 \( -name 'nvidia*' -o -name nvgpu -o -name 'nvhost-ctrl-gpu' -o -name 'nvhost-gpu' \) \
  2>/dev/null >"$WORK/gpu-nodes.txt" || true

# --- Fresh consumers, offline, lifecycle scripts disabled -------------------
VERSION="$(tar -xzOf /inputs/main.tgz package/package.json | node -e \
  'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>process.stdout.write(JSON.parse(s).version))')"
new_consumer() {
  local dir="$WORK/consumers/$1"
  mkdir -p "$dir"
  printf '{"name":"fathomdb-driverless-%s","private":true,"type":"module"}\n' "$1" >"$dir/package.json"
  cp "$SMOKE_DIR/tegra_node_driverless_row.mjs" "$dir/row.mjs"
  printf '%s\n' "$dir"
}

# The unpublished generic package cannot be installed offline, so a stand-in
# with its name and version is co-installed. Loading it throws a sentinel, so
# the loader's error proves which package it chose.
DECOY="$WORK/decoy"
mkdir -p "$DECOY"
node -e '
  const [file, name, version] = process.argv.slice(1);
  require("fs").writeFileSync(file, JSON.stringify({
    name, version, main: "index.js", os: ["linux"], cpu: ["arm64"], libc: ["glibc"],
    description: "Driverless-smoke stand-in for the generic package.",
  }, null, 2) + "\n");
' "$DECOY/package.json" "$GENERIC_PACKAGE" "$VERSION"
printf 'throw new Error("the generic linux-arm64-gnu stand-in was selected");\n' >"$DECOY/index.js"
(cd "$DECOY" && npm pack --ignore-scripts --pack-destination "$WORK" >/dev/null)

tegra_only="$(new_consumer tegra-only)"
(cd "$tegra_only" && npm install --offline --ignore-scripts --no-audit --no-fund \
  /inputs/main.tgz /inputs/tegra.tgz >"$EVIDENCE/consumers/install-tegra-only.log" 2>&1)
coinstalled="$(new_consumer generic-coinstalled)"
(cd "$coinstalled" && npm install --offline --ignore-scripts --no-audit --no-fund \
  /inputs/main.tgz /inputs/tegra.tgz "$WORK/$GENERIC_PACKAGE-$VERSION.tgz" \
  >"$EVIDENCE/consumers/install-generic-coinstalled.log" 2>&1)

ADDON="$tegra_only/node_modules/$TEGRA_PACKAGE/$NATIVE_NAME"
tar -xzOf /inputs/tegra.tgz "package/$NATIVE_NAME" >"$WORK/packed.node"

node - "$EVIDENCE/context.json" "$VERSION" "$ADDON" "$tegra_signal" \
  "$WORK/libcuda.txt" "$WORK/smi.txt" "$WORK/gpu-nodes.txt" "$WORK/packed.node" <<'NODE'
const { createHash } = require("node:crypto");
const { readFileSync, writeFileSync } = require("node:fs");
const [out, version, addon, tegra, libcuda, smi, nodes, packed] = process.argv.slice(2);
const sha = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
const lines = (path) => [...new Set(readFileSync(path, "utf8").split("\n").filter(Boolean))].sort();
writeFileSync(out, JSON.stringify({
  schema: "fathomdb.tegra-node-driverless-context/v1",
  candidate_sha: process.env.CANDIDATE_SHA,
  version,
  main_tarball_sha256: sha("/inputs/main.tgz"),
  tegra_tarball_sha256: sha("/inputs/tegra.tgz"),
  tegra_native_sha256: sha(packed),
  tegra_native_path: addon,
  node: process.version,
  host: {
    arch: process.arch === "arm64" ? "aarch64" : process.arch,
    tegra_family_signal: tegra === "true",
    nvidia_smi: lines(smi),
    libcuda: lines(libcuda),
    gpu_device_nodes: lines(nodes),
  },
}, null, 2) + "\n");
NODE

# --- One fresh process per row; a failed row is still retained --------------
run_row() {
  local name="$1" consumer="$2" kind="$3"
  shift 3
  (cd "$consumer" && env -u LD_LIBRARY_PATH -u LD_PRELOAD "$@" \
    node row.mjs "$name" "$kind" "$EVIDENCE/rows/$name.json" "$WORK/db" "$ADDON") \
    >"$EVIDENCE/rows/$name.log" 2>&1 || true
}
run_row tegra-addon-early-init-default "$tegra_only" tegra-addon env -u FATHOMDB_CUDA_EARLY_INIT
run_row tegra-addon-early-init-off "$tegra_only" tegra-addon env FATHOMDB_CUDA_EARLY_INIT=off
run_row loader-generic-coinstalled "$coinstalled" loader env
run_row loader-tegra-only "$tegra_only" loader env

node "$SMOKE_DIR/tegra_node_driverless_verdict.mjs" "$EVIDENCE"
