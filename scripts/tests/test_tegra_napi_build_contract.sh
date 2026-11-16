#!/usr/bin/env bash
# Behavioral contract for the Jetson Tegra N-API build route
# (ADR-0.8.27-jetson-tegra-node-addon-distribution): the host-native build
# wrapper fails closed off its pinned host and never publishes, the separately
# named Tegra platform package stays out of the npm optionalDependencies set,
# and the generic linux-arm64-gnu package keeps its identity.
#
# The real build runs only on the Jetson. These checks run on any host.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WRAPPER="$REPO_ROOT/scripts/release/build-napi-cuda-tegra.sh"
INJECT="$REPO_ROOT/scripts/release/npm-inject-optional-deps.sh"
TEGRA_PACKAGE_DIR="$REPO_ROOT/src/ts/npm-tegra/linux-arm64-gnu-tegra"
GENERIC_PACKAGE="$REPO_ROOT/src/ts/npm/linux-arm64-gnu/package.json"
TEGRA_NAME='fathomdb-linux-arm64-gnu-tegra'

FAILED=0
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

# --- The wrapper exists, parses, and rejects malformed invocations ----------
if [ -f "$WRAPPER" ] && bash -n "$WRAPPER"; then
  pass 'Tegra N-API build wrapper exists and parses'
else
  fail 'scripts/release/build-napi-cuda-tegra.sh must exist and parse'
fi

expect_usage_error() {
  local label="$1" status
  shift
  set +e
  bash "$WRAPPER" "$@" >"$WORK/usage.out" 2>&1
  status=$?
  set -e
  if [ "$status" -eq 2 ]; then
    pass "$label"
  else
    fail "$label (exit $status)"
    cat "$WORK/usage.out" >&2
  fi
}
expect_usage_error 'rejects an unknown argument with a usage error' --publish
expect_usage_error 'rejects --base-version without a value' --base-version
expect_usage_error 'rejects --out without a value' --base-version 0.8.27 --out
expect_usage_error 'rejects a base version that is not a plain release version' \
  --base-version '0.8.27+tegra' --assert-only
expect_usage_error 'rejects a base version carrying shell metacharacters' \
  --base-version '0.8.27;true' --assert-only

# --- Assertions fail closed before anything is built ------------------------
# A stub `node` that reports the wrong version must stop the wrapper with its
# own diagnostic on every host: off-Jetson the architecture pin fires first,
# on the Jetson the Node pin does. Either way nothing is built or packed.
mkdir -p "$WORK/stub-bin"
cat >"$WORK/stub-bin/node" <<'STUB'
#!/usr/bin/env bash
printf 'v0.0.0-not-the-contract\n'
STUB
chmod +x "$WORK/stub-bin/node"
set +e
PATH="$WORK/stub-bin:$PATH" bash "$WRAPPER" --base-version 0.8.27 \
  --out "$WORK/never-built" >"$WORK/assert.out" 2>&1
status=$?
set -e
if [ "$status" -eq 1 ] && grep -q '^build-napi-cuda-tegra: ' "$WORK/assert.out" \
  && [ ! -e "$WORK/never-built" ]; then
  pass 'a host that misses a pin fails closed before building or packing'
else
  fail "a host that misses a pin must exit 1 with a wrapper diagnostic and no output (exit $status)"
  cat "$WORK/assert.out" >&2
fi

# --- No publication command, anywhere in the wrapper ------------------------
if [ -f "$WRAPPER" ]; then
  publication_hits="$(grep -nE 'npm[[:space:]]+publish|twine|cargo[[:space:]]+publish|maturin[[:space:]]+(publish|upload)|gh[[:space:]]+(workflow|release|api)|git[[:space:]]+push|gh-pages|deploy-pages|NODE_AUTH_TOKEN|NPM_TOKEN' "$WRAPPER" || true)"
  if [ -z "$publication_hits" ]; then
    pass 'wrapper contains no publication command'
  else
    fail "wrapper must not publish: $publication_hits"
  fi
fi

# --- The Tegra platform package metadata ------------------------------------
if [ -f "$TEGRA_PACKAGE_DIR/package.json" ]; then
  if node -e '
    const pkg = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
    const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
    const problems = [];
    if (pkg.name !== process.argv[2]) problems.push(`name ${pkg.name}`);
    if (!same(pkg.os, ["linux"])) problems.push("os");
    if (!same(pkg.cpu, ["arm64"])) problems.push("cpu");
    if (!same(pkg.libc, ["glibc"])) problems.push("libc");
    if (pkg.main !== "fathomdb.linux-arm64-gnu.node") problems.push(`main ${pkg.main}`);
    if (!same(pkg.files, ["fathomdb.linux-arm64-gnu.node"])) problems.push("files");
    if (pkg.license !== "MIT") problems.push("license");
    // The version is stamped from the main package at build time so the pair
    // cannot drift; a committed version could.
    if ("version" in pkg) problems.push("committed version");
    for (const field of ["scripts", "dependencies", "optionalDependencies", "bin"]) {
      if (field in pkg) problems.push(`unexpected ${field}`);
    }
    if (problems.length) { console.error(problems.join(", ")); process.exit(1); }
  ' "$TEGRA_PACKAGE_DIR/package.json" "$TEGRA_NAME"; then
    pass 'Tegra platform package declares its own name, linux/arm64/glibc and the triple binary'
  else
    fail 'Tegra platform package metadata does not match the distribution ADR'
  fi
else
  fail "missing Tegra platform package metadata at src/ts/npm-tegra/linux-arm64-gnu-tegra/package.json"
fi

tegra_under_platform_dir="$(grep -rl "\"$TEGRA_NAME\"" "$REPO_ROOT/src/ts/npm" || true)"
if [ -z "$tegra_under_platform_dir" ]; then
  pass 'Tegra package is kept out of the optionalDependencies source directory'
else
  fail "Tegra package must not live under src/ts/npm/: $tegra_under_platform_dir"
fi

# --- Generic linux-arm64-gnu package keeps its identity ---------------------
if node -e '
  const pkg = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
  const ok = pkg.name === "fathomdb-linux-arm64-gnu"
    && pkg.main === "fathomdb.linux-arm64-gnu.node"
    && JSON.stringify(pkg.files) === JSON.stringify(["fathomdb.linux-arm64-gnu.node"])
    && JSON.stringify(pkg.libc) === JSON.stringify(["glibc"])
    && !JSON.stringify(pkg).toLowerCase().includes("tegra")
    && !JSON.stringify(pkg).toLowerCase().includes("cuda");
  process.exit(ok ? 0 : 1);
' "$GENERIC_PACKAGE"; then
  pass 'generic fathomdb-linux-arm64-gnu package is unchanged and carries no Tegra or CUDA identity'
else
  fail 'generic fathomdb-linux-arm64-gnu package must keep its name, binary and non-CUDA identity'
fi
if grep -q '"default-embedder",' "$REPO_ROOT/src/ts/scripts/build-native.mjs" \
  && ! grep -qE 'embed-cuda|rerank-cuda|tegra' "$REPO_ROOT/src/ts/scripts/build-native.mjs"; then
  pass 'generic build-native keeps the default-embedder-only feature set'
else
  fail 'generic build-native must keep default-embedder only'
fi

# --- npm-inject-optional-deps never injects the Tegra package ---------------
mkdir -p "$WORK/main"
cp "$REPO_ROOT/src/ts/package.json" "$WORK/main/package.json"
if bash "$INJECT" "$WORK/main" "$REPO_ROOT/src/ts/npm" >/dev/null \
  && ! grep -q "$TEGRA_NAME" "$WORK/main/package.json"; then
  pass 'injection over the committed platform set never adds the Tegra package'
else
  fail 'injection over the committed platform set must not add the Tegra package'
fi

# Even if a Tegra package directory is copied under the platform root by
# mistake, injection must refuse rather than make npm select it automatically.
mkdir -p "$WORK/main-misplaced" "$WORK/npm-misplaced/linux-arm64-gnu" \
  "$WORK/npm-misplaced/linux-arm64-gnu-tegra"
printf '{"name":"fathomdb","version":"1.2.3"}\n' >"$WORK/main-misplaced/package.json"
printf '{"name":"fathomdb-linux-arm64-gnu","version":"1.2.3"}\n' \
  >"$WORK/npm-misplaced/linux-arm64-gnu/package.json"
printf '{"name":"%s","version":"1.2.3"}\n' "$TEGRA_NAME" \
  >"$WORK/npm-misplaced/linux-arm64-gnu-tegra/package.json"
before="$(sha256sum "$WORK/main-misplaced/package.json")"
set +e
bash "$INJECT" "$WORK/main-misplaced" "$WORK/npm-misplaced" >"$WORK/inject.out" 2>&1
status=$?
set -e
after="$(sha256sum "$WORK/main-misplaced/package.json")"
if [ "$status" -ne 0 ] && [ "$before" = "$after" ] && grep -q "$TEGRA_NAME" "$WORK/inject.out"; then
  pass 'injection refuses a misplaced Tegra package and leaves the main manifest untouched'
else
  fail "injection must refuse a misplaced Tegra package (exit $status)"
  cat "$WORK/inject.out" >&2
fi

if [ "$FAILED" -gt 0 ]; then
  printf '\n%d Tegra N-API build contract test(s) failed\n' "$FAILED" >&2
  exit 1
fi
printf '\nTegra N-API build contract tests passed\n'

# An explicit instrumented request still performs host assertions; it never
# silently falls back to a shipping feature tuple or bypasses the host gate.
set +e
observer_output="$(bash "$WRAPPER" --qualification-observer --assert-only 2>&1)"
observer_status=$?
set -e
if [ "$observer_status" -ne 1 ] || ! grep -Fq 'features=embed-cuda,rerank-cuda,tegra-pool,cuda-pool-observer' <<<"$observer_output"; then
  printf 'FAIL explicit qualification features were not selected before host refusal: exit=%s\n%s\n' "$observer_status" "$observer_output" >&2
  exit 1
fi

# Exercise the installed CLI parser and its downstream Cargo invocation. Host
# assertions are covered above; this stops at Cargo build before native work.
python3 - "$REPO_ROOT" "$WORK" <<'PY'
import json
import os
from pathlib import Path
import subprocess
import sys

root, work = map(Path, sys.argv[1:])
ts = root / "src/ts"
cli_package = ts / "node_modules/@napi-rs/cli/package.json"
assert cli_package.is_file(), "run npm ci in src/ts to test the pinned NAPI CLI"
locked = json.loads((ts / "package-lock.json").read_text())["packages"]
version = json.loads(cli_package.read_text())["version"]
assert version == locked["node_modules/@napi-rs/cli"]["version"]
source = (root / "scripts/release/build-napi-cuda-tegra.sh").read_text()
selection = source[source.index("BUILD_CARGO_FLAGS=("):source.index("# --- Host and toolchain")]
command_start = source.index('npm exec --offline -- napi build')
command = source[command_start:source.index('\nNATIVE_BINARY=', command_start)]

cargo = work / "cargo-bin/cargo"
cargo.parent.mkdir()
cargo.write_text('''#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys

if sys.argv[1] == "metadata":
    print(json.dumps({
        "resolve": {"root": "fathomdb-napi"},
        "packages": [{"id": "fathomdb-napi", "name": "fathomdb-napi",
                      "targets": [{"kind": ["cdylib"], "crate_types": ["cdylib"]}]}],
    }))
    sys.exit(0)
if sys.argv[1] == "build":
    Path(os.environ["CARGO_CAPTURE"]).write_text(json.dumps({
        "args": sys.argv[1:], "offline": os.environ.get("CARGO_NET_OFFLINE"),
    }))
    print("test: reached Cargo build; native compilation intentionally stopped", file=sys.stderr)
    sys.exit(73)
sys.exit("unexpected Cargo invocation")
''')
cargo.chmod(0o755)

for observer in (False, True):
    capture = work / f"cargo-{observer}.json"
    environment = dict(os.environ)
    environment.pop("CARGO", None)
    environment.pop("CARGO_NET_OFFLINE", None)
    environment.update({
        "PATH": f"{cargo.parent}:{environment['PATH']}",
        "CARGO_CAPTURE": str(capture),
        "TS_DIR": str(ts),
        "NATIVE_DEST": str(work / "native"),
        "QUALIFICATION_OBSERVER": str(int(observer)),
    })
    result = subprocess.run(
        ["bash", "-ec", '. "$1"\n' + selection + '\ncd "$TS_DIR"\n' + command,
         "test-napi-cli", str(root / "scripts/release/cuda-artifact-contract.sh")],
        cwd=ts, env=environment, capture_output=True, text=True, timeout=30,
    )
    assert capture.is_file(), (
        f"NAPI CLI {version} must pass {'observer' if observer else 'shipping'} flags "
        f"to Cargo build (exit {result.returncode}):\n{result.stdout}{result.stderr}"
    )
    observed = json.loads(capture.read_text())
    features = "embed-cuda,rerank-cuda,tegra-pool"
    expected = ["build", "--release", "--features",
                features + (",cuda-pool-observer" if observer else ""), "--locked"]
    if observer:
        expected.append("--offline")
    assert observed["args"] == expected, observed
    assert observed["offline"] == ("true" if observer else None), observed
    assert result.returncode != 0, "the Cargo sentinel must stop native compilation"
    print(f"PASS  pinned NAPI CLI {version}: {observed['args']}")
PY
