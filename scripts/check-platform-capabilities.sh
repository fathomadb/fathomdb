#!/usr/bin/env bash
# Verify that native-loader triples and release-ready package metadata have one
# source of truth. Public documentation is promoted only after registry smokes.
set -euo pipefail
# `git rev-parse` failing here used to degrade to `cd ""` — a bash no-op that
# leaves the script running in an arbitrary cwd. Bind and check it instead.
_repo_toplevel="$(git rev-parse --show-toplevel)" || exit 1
cd "$_repo_toplevel" || exit 1

python3 - <<'PY'
import json
import os
from pathlib import Path

root = Path('.')
manifest_path = Path(os.environ.get('MANIFEST_PATH', 'dev/platform-capabilities.json'))
manifest = json.loads((root / manifest_path).read_text())
platforms = manifest['platforms']
triples = {entry['triple']: entry for entry in platforms}
if manifest['schema'] != 1 or not triples:
    raise SystemExit('FAIL platform-capabilities: invalid or empty manifest')
if len(triples) != len(platforms):
    raise SystemExit('FAIL platform-capabilities: duplicate triple')

loader = (root / 'src/ts/src/platform.ts').read_text()
for triple, entry in triples.items():
    if triple not in loader:
        raise SystemExit(f'FAIL platform-capabilities: loader triple {triple} is absent from manifest coverage')
    if entry['status'] not in {'published', 'release-ready', 'planned'}:
        raise SystemExit(f'FAIL platform-capabilities: {triple} has invalid status')
    if entry['status'] in {'published', 'release-ready'}:
        package_dir = entry['package_dir']
        if not package_dir:
            raise SystemExit(f'FAIL platform-capabilities: published {triple} has no package directory')
        package = json.loads((root / package_dir / 'package.json').read_text())
        if package['name'] != entry['npm_package'] or triple not in package['main']:
            raise SystemExit(f'FAIL platform-capabilities: package metadata disagrees for {triple}')

published = [entry for entry in platforms if entry['status'] == 'published']
release_ready = [
    entry for entry in platforms if entry['status'] in {'published', 'release-ready'}
]
expected_ready = [
    'linux-x64-gnu', 'linux-arm64-gnu', 'darwin-x64', 'darwin-arm64',
    'win32-x64-msvc',
]
if [entry['triple'] for entry in release_ready] != expected_ready:
    raise SystemExit('FAIL platform-capabilities: release-ready matrix is incomplete or reordered')

published = [entry for entry in platforms if entry['status'] == 'published']
if [entry['triple'] for entry in published] != expected_ready:
    raise SystemExit(
        'FAIL platform-capabilities: published matrix must contain all five '
        '0.8.26 native targets'
    )

# The generic linux-arm64-gnu package keeps its hosted build and manylinux
# floor; the Jetson CUDA binary is a separately named variant of that triple
# distributed only through Tegra Pages
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decision 7).
TEGRA_TRIPLE = 'linux-arm64-gnu'
generic_arm64 = triples.get(TEGRA_TRIPLE, {})
expected_generic = {
    'rust_target': 'aarch64-unknown-linux-gnu', 'runner': 'ubuntu-24.04-arm',
    'smoke_job': 'post-publish-smoke-aarch64', 'npm_package': 'fathomdb-linux-arm64-gnu',
    'package_dir': 'src/ts/npm/linux-arm64-gnu', 'python_wheel': 'manylinux_2_28_aarch64',
}
for field, value in expected_generic.items():
    if generic_arm64.get(field) != value:
        raise SystemExit(f'FAIL platform-capabilities: generic {TEGRA_TRIPLE} {field} must stay {value!r}')
for triple, entry in triples.items():
    if triple != TEGRA_TRIPLE and 'variants' in entry:
        raise SystemExit(f'FAIL platform-capabilities: {triple} has no ruled variant')
variants = generic_arm64.get('variants')
if not isinstance(variants, list) or len(variants) != 1:
    raise SystemExit(f'FAIL platform-capabilities: {TEGRA_TRIPLE} must carry exactly the Tegra variant')
tegra = variants[0]
expected_tegra = {
    'variant': 'tegra', 'channel': 'tegra-pages', 'registry_publication': False,
    'optional_dependency': False, 'npm_package': 'fathomdb-linux-arm64-gnu-tegra',
    'package_dir': 'src/ts/npm-tegra/linux-arm64-gnu-tegra',
    'build_script': 'scripts/release/build-napi-cuda-tegra.sh',
    'features': ['embed-cuda', 'rerank-cuda'], 'glibc_floor_family': 'tegra',
}
for field, value in expected_tegra.items():
    if tegra.get(field) != value:
        raise SystemExit(f'FAIL platform-capabilities: Tegra variant {field} must be {value!r}')
if tegra.get('status') not in {'published', 'release-ready', 'planned'}:
    raise SystemExit('FAIL platform-capabilities: Tegra variant has invalid status')
if any(entry['npm_package'] == tegra['npm_package'] for entry in platforms):
    raise SystemExit('FAIL platform-capabilities: the Tegra package must not be a triple of its own')
tegra_package = json.loads((root / tegra['package_dir'] / 'package.json').read_text())
if (tegra_package.get('name') != tegra['npm_package']
        or tegra_package.get('main') != f'fathomdb.{TEGRA_TRIPLE}.node'
        or tegra_package.get('os') != ['linux'] or tegra_package.get('cpu') != ['arm64']
        or tegra_package.get('libc') != ['glibc']):
    raise SystemExit('FAIL platform-capabilities: Tegra package metadata disagrees with its variant')
if not (root / tegra['build_script']).is_file():
    raise SystemExit('FAIL platform-capabilities: Tegra variant build script is absent')
if tegra['npm_package'] not in loader:
    raise SystemExit('FAIL platform-capabilities: the loader does not name the Tegra variant')
main_package = json.loads((root / 'src/ts/package.json').read_text())
for block in ('dependencies', 'optionalDependencies', 'peerDependencies'):
    if tegra['npm_package'] in (main_package.get(block) or {}):
        raise SystemExit(f'FAIL platform-capabilities: the main package lists the Tegra variant in {block}')

for path in ('README.md', 'docs/compatibility/index.md', 'docs/install/python.md', 'docs/install/typescript.md'):
    text = (root / path).read_text()
    if 'Linux x86_64' not in text and 'x86_64-unknown-linux-gnu' not in text:
        raise SystemExit(f'FAIL platform-capabilities: {path} lacks the published-platform boundary')
    if 'Linux AArch64' not in text and 'aarch64-unknown-linux-gnu' not in text:
        raise SystemExit(f'FAIL platform-capabilities: {path} lacks the published ARM64-platform boundary')
print(f'ok    platform-capabilities: {len(triples)} loader triples, {len(release_ready)} in the release matrix, {len(published)} published')
PY
