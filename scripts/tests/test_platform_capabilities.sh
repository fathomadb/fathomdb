#!/usr/bin/env bash
set -euo pipefail
# `git rev-parse` failing here used to degrade to `cd ""` — a bash no-op that
# leaves the script running in an arbitrary cwd. Bind and check it instead.
_repo_toplevel="$(git rev-parse --show-toplevel)" || exit 1
cd "$_repo_toplevel" || exit 1

scripts/check-platform-capabilities.sh

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
cp dev/platform-capabilities.json "$tmpdir/manifest.json"
python3 - "$tmpdir/manifest.json" <<'PY'
import json, sys
path = sys.argv[1]
data = json.load(open(path))
data['platforms'][0]['npm_package'] = 'wrong'
json.dump(data, open(path, 'w'))
PY
if MANIFEST_PATH="$tmpdir/manifest.json" scripts/check-platform-capabilities.sh >/dev/null 2>&1; then
  echo 'FAIL test-platform-capabilities: mismatched package metadata passed' >&2
  exit 1
fi

cp dev/platform-capabilities.json "$tmpdir/manifest.json"
python3 - "$tmpdir/manifest.json" <<'PY'
import json, sys
path = sys.argv[1]
data = json.load(open(path))
data['platforms'][1]['status'] = 'release-ready'
json.dump(data, open(path, 'w'))
PY
if MANIFEST_PATH="$tmpdir/manifest.json" scripts/check-platform-capabilities.sh >/dev/null 2>&1; then
  echo 'FAIL test-platform-capabilities: unadvertised ARM64 publication passed' >&2
  exit 1
fi
# The Jetson Tegra Pages package is a variant of linux-arm64-gnu, never a
# triple of its own and never a change to the generic record
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decision 7).
python3 - dev/platform-capabilities.json <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
arm64 = [p for p in data['platforms'] if p['triple'] == 'linux-arm64-gnu']
variants = arm64[0].get('variants') if arm64 else None
names = [v.get('npm_package') for v in variants or []]
if names != ['fathomdb-linux-arm64-gnu-tegra']:
    sys.exit(f'FAIL test-platform-capabilities: linux-arm64-gnu must carry exactly the Tegra variant, got {names}')
PY

reject_mutation() {
  local case_name="$1" label="$2"
  cp dev/platform-capabilities.json "$tmpdir/manifest.json"
  python3 - "$tmpdir/manifest.json" "$case_name" <<'PY'
import json, sys
path, case = sys.argv[1], sys.argv[2]
data = json.load(open(path))
arm64 = next(p for p in data['platforms'] if p['triple'] == 'linux-arm64-gnu')
tegra = (arm64.get('variants') or [{}])[0]
if case == 'no-variant':
    arm64.pop('variants', None)
elif case == 'second-variant':
    arm64['variants'].append(dict(tegra, variant='thor'))
elif case == 'other-triple':
    data['platforms'][0]['variants'] = arm64.pop('variants')
elif case == 'wrong-name':
    tegra['npm_package'] = 'fathomdb-linux-arm64-gnu-cuda'
elif case == 'injected-dir':
    tegra['package_dir'] = 'src/ts/npm/linux-arm64-gnu'
elif case == 'registry':
    tegra['registry_publication'] = True
elif case == 'channel':
    tegra['channel'] = 'npm'
elif case == 'features':
    tegra['features'] = ['default-embedder']
elif case == 'floor':
    tegra['glibc_floor_family'] = 'manylinux'
elif case == 'generic-runner':
    arm64['runner'] = 'self-hosted-jetson'
elif case == 'generic-name':
    arm64['npm_package'] = 'fathomdb-linux-arm64-gnu-tegra'
elif case == 'generic-wheel':
    arm64['python_wheel'] = 'linux_aarch64'
elif case == 'promoted-triple':
    data['platforms'].append(dict(tegra, triple='linux-arm64-gnu-tegra', status='published',
                                  rust_target='aarch64-unknown-linux-gnu'))
else:
    sys.exit(f'unknown mutation {case}')
json.dump(data, open(path, 'w'))
PY
  if MANIFEST_PATH="$tmpdir/manifest.json" scripts/check-platform-capabilities.sh >"$tmpdir/out" 2>&1; then
    echo "FAIL test-platform-capabilities: $label passed" >&2
    exit 1
  fi
  if ! grep -q 'FAIL platform-capabilities' "$tmpdir/out"; then
    echo "FAIL test-platform-capabilities: $label failed without a checker diagnostic" >&2
    cat "$tmpdir/out" >&2
    exit 1
  fi
}
reject_mutation no-variant 'a manifest without the Tegra variant'
reject_mutation second-variant 'a second linux-arm64-gnu variant'
reject_mutation other-triple 'a Tegra variant on another triple'
reject_mutation wrong-name 'a Tegra variant with the wrong package name'
reject_mutation injected-dir 'a Tegra variant inside the injected platform directory'
reject_mutation registry 'a Tegra variant published to the npm registry'
reject_mutation channel 'a Tegra variant outside the Tegra Pages channel'
reject_mutation features 'a Tegra variant without the CUDA features'
reject_mutation floor 'a Tegra variant with the generic glibc floor'
reject_mutation generic-runner 'a generic record moved off its hosted runner'
reject_mutation generic-name 'a generic record renamed to the Tegra package'
reject_mutation generic-wheel 'a generic record with a different wheel floor'
reject_mutation promoted-triple 'the Tegra package promoted to a triple'
echo 'PASS test-platform-capabilities'
