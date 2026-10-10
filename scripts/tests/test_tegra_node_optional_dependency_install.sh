#!/usr/bin/env bash
# Cross-host install fixture for the documented Jetson Node install form
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decision 7): an
# application lists the exact-version Tegra Pages tarball as an
# optionalDependency so one package.json and lockfile serve Jetson and
# non-Jetson hosts.
#
# The documented package.json is read from docs/install/typescript.md. Its
# Pages URL and the main package are replaced by offline fixture tarballs that
# carry the committed os/cpu/libc metadata, and npm's --os/--cpu/--libc
# overrides stand in for each host. The fixture proves that the Tegra package
# installs only on linux/arm64/glibc, is skipped without failing the install
# everywhere else (also from a lockfile written on an AArch64 host), and that
# the real loader selects it only on a confirmed classic Jetson Orin.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOC="$REPO_ROOT/docs/install/typescript.md"
TEGRA_SOURCE="$REPO_ROOT/src/ts/npm-tegra/linux-arm64-gnu-tegra/package.json"
LOADER="$REPO_ROOT/src/ts/src/platform.ts"
TEGRA_PACKAGE='fathomdb-linux-arm64-gnu-tegra'
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
export NPM_CONFIG_USERCONFIG=/dev/null NPM_CONFIG_UPDATE_NOTIFIER=false
export NPM_CONFIG_CACHE="$WORK/npm-cache"

# --- The documented install form --------------------------------------------
if ! node - "$DOC" "$WORK/documented.json" <<'NODE'
const { readFileSync, writeFileSync } = require("node:fs");
const [doc, out] = process.argv.slice(2);
const blocks = [...readFileSync(doc, "utf8").matchAll(/```json\n([\s\S]*?)```/g)]
  .map((match) => match[1])
  .filter((body) => body.includes('"optionalDependencies"'));
if (blocks.length !== 1) {
  console.error(`expected exactly one json block with optionalDependencies, found ${blocks.length}`);
  process.exit(1);
}
const manifest = JSON.parse(blocks[0]);
const url = manifest.optionalDependencies?.["fathomdb-linux-arm64-gnu-tegra"];
const match = /^https:\/\/fathomadb\.github\.io\/fathomdb\/tegra\/packages\/fathomdb-linux-arm64-gnu-tegra-(\d+\.\d+\.\d+)\.tgz$/
  .exec(url ?? "");
if (!match) {
  console.error(`the Tegra optionalDependency is not an exact-version Pages tarball URL: ${url}`);
  process.exit(1);
}
if (manifest.dependencies?.fathomdb !== match[1]) {
  console.error(`fathomdb must be pinned to exactly ${match[1]}, got ${manifest.dependencies?.fathomdb}`);
  process.exit(1);
}
if (Object.keys(manifest.optionalDependencies).length !== 1) {
  console.error("the documented form lists only the Tegra package as an optionalDependency");
  process.exit(1);
}
writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
NODE
then
  fail 'docs/install/typescript.md documents the exact-version Pages tarball as an optionalDependency'
  printf '%s cross-host install check(s) failed\n' "$FAILED" >&2
  exit 1
fi
pass 'docs/install/typescript.md documents the exact-version Pages tarball as an optionalDependency'
VERSION="$(node -p 'require(process.argv[1]).dependencies.fathomdb' "$WORK/documented.json")"

# --- Offline fixture packages with the committed platform metadata ----------
stage_package() {
  local source_manifest="$1" dir="$2" main_file="$3"
  mkdir -p "$dir"
  node -e '
    const fs = require("fs");
    const [src, out, version] = process.argv.slice(1);
    const pkg = JSON.parse(fs.readFileSync(src, "utf8"));
    delete pkg["//version"];
    pkg.version = version;
    fs.writeFileSync(out, JSON.stringify(pkg, null, 2) + "\n");
  ' "$source_manifest" "$dir/package.json" "$VERSION"
  printf 'fixture, never loaded\n' >"$dir/$main_file"
  (cd "$dir" && npm pack --ignore-scripts --pack-destination "$WORK/tarballs" >/dev/null)
}
mkdir -p "$WORK/tarballs" "$WORK/sources"
stage_package "$TEGRA_SOURCE" "$WORK/sources/tegra" fathomdb.linux-arm64-gnu.node
for triple in linux-arm64-gnu linux-x64-gnu; do
  stage_package "$REPO_ROOT/src/ts/npm/$triple/package.json" "$WORK/sources/$triple" "fathomdb.$triple.node"
done
printf '{"name":"fathomdb","version":"%s","main":"index.js"}\n' "$VERSION" >"$WORK/sources/main.json"
stage_package "$WORK/sources/main.json" "$WORK/sources/main" index.js

# The documented manifest with its URL and registry pins swapped for the
# fixture tarballs. The generic packages stand in for the platform packages the
# published main package lists as optionalDependencies.
node - "$WORK/documented.json" "$WORK/consumer.json" "$WORK/tarballs" "$VERSION" <<'NODE'
const { readFileSync, writeFileSync } = require("node:fs");
const [documented, out, tarballs, version] = process.argv.slice(2);
const manifest = JSON.parse(readFileSync(documented, "utf8"));
manifest.name = "fathomdb-cross-host-consumer";
manifest.private = true;
manifest.dependencies.fathomdb = `file:${tarballs}/fathomdb-${version}.tgz`;
manifest.optionalDependencies["fathomdb-linux-arm64-gnu-tegra"] =
  `file:${tarballs}/fathomdb-linux-arm64-gnu-tegra-${version}.tgz`;
for (const triple of ["linux-arm64-gnu", "linux-x64-gnu"]) {
  manifest.optionalDependencies[`fathomdb-${triple}`] = `file:${tarballs}/fathomdb-${triple}-${version}.tgz`;
}
writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
NODE

installed() { [ -f "$1/node_modules/$2/package.json" ]; }
expect_install() {
  local label="$1" verb="$2" dir="$3" os="$4" cpu="$5" libc="$6" tegra="$7" generic="$8"
  shift 8
  if ! (cd "$dir" && npm "$verb" --offline --ignore-scripts --no-audit --no-fund \
    --os="$os" --cpu="$cpu" --libc="$libc" >"$dir.log" 2>&1); then
    fail "$label: npm $verb failed"
    cat "$dir.log" >&2
    return
  fi
  local problems=''
  if ! installed "$dir" fathomdb; then problems+=' fathomdb-missing'; fi
  if [ "$tegra" = yes ] && ! installed "$dir" "$TEGRA_PACKAGE"; then problems+=' tegra-missing'; fi
  if [ "$tegra" = no ] && installed "$dir" "$TEGRA_PACKAGE"; then problems+=' tegra-installed'; fi
  if [ "$generic" != none ] && ! installed "$dir" "fathomdb-$generic"; then problems+=" $generic-missing"; fi
  for other in linux-arm64-gnu linux-x64-gnu; do
    if [ "$other" != "$generic" ] && installed "$dir" "fathomdb-$other"; then problems+=" $other-installed"; fi
  done
  if [ -z "$problems" ]; then pass "$label"; else fail "$label:$problems"; fi
}
new_consumer() {
  mkdir -p "$WORK/$1"
  cp "$WORK/consumer.json" "$WORK/$1/package.json"
  printf '%s\n' "$WORK/$1"
}

jetson="$(new_consumer jetson)"
expect_install 'linux/arm64/glibc installs the Tegra and generic packages' install "$jetson" linux arm64 glibc yes linux-arm64-gnu
x64="$(new_consumer x64)"
expect_install 'linux/x64/glibc skips the Tegra package and still installs' install "$x64" linux x64 glibc no linux-x64-gnu
musl="$(new_consumer musl)"
expect_install 'linux/arm64/musl skips the Tegra package and still installs' install "$musl" linux arm64 musl no none
darwin="$(new_consumer darwin)"
expect_install 'darwin/arm64 skips the Tegra package and still installs' install "$darwin" darwin arm64 '' no none

# One lockfile, written on the AArch64 host, shared with other hosts.
if [ -f "$jetson/package-lock.json" ]; then
  for host in x64-ci arm64-ci; do
    mkdir -p "$WORK/$host"
    cp "$jetson/package.json" "$jetson/package-lock.json" "$WORK/$host/"
  done
  expect_install 'npm ci from the shared lockfile skips the Tegra package on linux/x64' ci "$WORK/x64-ci" linux x64 glibc no linux-x64-gnu
  expect_install 'npm ci from the shared lockfile installs the Tegra package on linux/arm64' ci "$WORK/arm64-ci" linux arm64 glibc yes linux-arm64-gnu
else
  fail 'the AArch64 install must write a shareable package-lock.json'
fi

# --- The real loader against the installed layouts --------------------------
# Host signals are seams; package resolution reads each consumer's real
# node_modules. Node strips the loader's TypeScript types at import.
if node --input-type=module - "$LOADER" "$jetson" "$x64" "$VERSION" >"$WORK/loader.out" 2>&1 <<'NODE'
import { createRequire } from "node:module";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
const [loader, jetson, x64, version] = process.argv.slice(2);
const { loadPlatformBinding, PlatformPackageVersionMismatchError } = await import(pathToFileURL(loader).href);
function select(consumer, { family, gpu, main = version }) {
  const require = createRequire(join(consumer, "package.json"));
  let chosen = null;
  loadPlatformBinding({
    platform: "linux",
    arch: "arm64",
    isMusl: false,
    loadLocal: () => null,
    requirePackage: (pkg) => { chosen = pkg; return {}; },
    installedPackageVersion: (pkg) => {
      try { return require(`${pkg}/package.json`).version; } catch { return null; }
    },
    mainPackageVersion: () => main,
    tegraFamilySignal: () => family,
    tegraGpuName: () => gpu,
  });
  return chosen;
}
const cases = [
  ["non-Tegra AArch64 host with the Tegra package installed", jetson, { family: false, gpu: null }, "fathomdb-linux-arm64-gnu"],
  ["Thor-like host with the Tegra package installed", jetson, { family: true, gpu: "NVIDIA Thor" }, "fathomdb-linux-arm64-gnu"],
  ["confirmed classic Jetson Orin", jetson, { family: true, gpu: "Orin (nvgpu)" }, "fathomdb-linux-arm64-gnu-tegra"],
  ["confirmed Jetson with a lockfile install that skipped the package", x64, { family: true, gpu: "Orin (nvgpu)" }, "fathomdb-linux-arm64-gnu"],
];
let failed = 0;
for (const [label, consumer, signals, expected] of cases) {
  const chosen = select(consumer, signals);
  if (chosen === expected) console.log(`PASS  loader: ${label} selects ${expected}`);
  else { console.log(`FAIL  loader: ${label} selected ${chosen}, expected ${expected}`); failed++; }
}
try {
  select(jetson, { family: true, gpu: "Orin (nvgpu)", main: "0.0.1" });
  console.log("FAIL  loader: a mismatched fathomdb version must be refused on a confirmed Jetson");
  failed++;
} catch (error) {
  if (error instanceof PlatformPackageVersionMismatchError) console.log("PASS  loader: a mismatched fathomdb version is refused");
  else { console.log(`FAIL  loader: unexpected error ${error}`); failed++; }
}
process.exit(failed ? 1 : 0);
NODE
then
  cat "$WORK/loader.out"
else
  cat "$WORK/loader.out"
  fail 'the loader selects the Tegra package only on a confirmed classic Jetson Orin'
fi

if [ "$FAILED" -ne 0 ]; then
  printf '%s cross-host install check(s) failed\n' "$FAILED" >&2
  exit 1
fi
printf 'cross-host Tegra optional-dependency install: all checks passed\n'
