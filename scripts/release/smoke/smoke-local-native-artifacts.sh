#!/usr/bin/env bash
# Consume locally built Python and N-API artifacts without contacting a registry.
set -euo pipefail

if [ "$#" -ne 4 ]; then
  printf 'usage: %s <wheel-dir> <ts-dir> <platform-package-dir> <napi-label>\n' "$0" >&2
  exit 2
fi

WHEEL_DIR="$1"
TS_DIR="$2"
PLATFORM_PACKAGE_DIR="$3"
NAPI_LABEL="$4"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

wheel_paths=("$WHEEL_DIR"/*.whl)
if [ "${#wheel_paths[@]}" -ne 1 ] || [ ! -f "${wheel_paths[0]}" ]; then
  printf 'smoke-local-native-artifacts: expected exactly one wheel in %s\n' "$WHEEL_DIR" >&2
  exit 1
fi
if [ ! -f "$TS_DIR/fathomdb.$NAPI_LABEL.node" ]; then
  printf 'smoke-local-native-artifacts: missing native N-API artifact %s\n' \
    "$TS_DIR/fathomdb.$NAPI_LABEL.node" >&2
  exit 1
fi

# The shipped wheel is a stable-ABI build (pyo3 abi3-py310), so one artifact
# serves every CPython >= 3.10. Assert that tag structurally — filename and
# WHEEL metadata — because a single-interpreter maturin build would otherwise
# silently produce a version-specific wheel that still installs here.
ABI3_TAG="cp310-abi3"
wheel_name="$(basename "${wheel_paths[0]}")"
case "$wheel_name" in
  *-"$ABI3_TAG"-*.whl) ;;
  *)
    printf 'smoke-local-native-artifacts: wheel %s is not tagged %s\n' \
      "$wheel_name" "$ABI3_TAG" >&2
    exit 1
    ;;
esac
if ! python3 - "${wheel_paths[0]}" "$ABI3_TAG" <<'PY'
import sys, zipfile

path, abi3_tag = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(path) as wheel:
    names = [n for n in wheel.namelist() if n.endswith(".dist-info/WHEEL")]
    if len(names) != 1:
        sys.exit(f"expected exactly one dist-info/WHEEL entry, found {names}")
    tags = [
        line.split(":", 1)[1].strip()
        for line in wheel.read(names[0]).decode().splitlines()
        if line.startswith("Tag:")
    ]
if not tags or any(not tag.startswith(f"{abi3_tag}-") for tag in tags):
    sys.exit(f"WHEEL metadata is not tagged {abi3_tag} (tags: {tags})")
PY
then
  printf 'smoke-local-native-artifacts: wheel %s is not tagged %s in its WHEEL metadata\n' \
    "$wheel_name" "$ABI3_TAG" >&2
  exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

python3 -m venv "$WORK/python-venv"
PYTHON="$WORK/python-venv/bin/python"
"$PYTHON" -m pip install --no-index --find-links "$WHEEL_DIR" fathomdb
"$PYTHON" - "$WORK/python-smoke.fdb" <<'PY'
import sys

from fathomdb import Engine

engine = Engine.open(sys.argv[1])
engine.write([
    {
        "kind": "doc",
        "body": "AéB",
        "source_id": "smoke:local-native-wheel",
        "provenance": {
            "schema_version": 1,
            "role": "canonical",
            "artifact_revision_id": "wheel-source-revision",
            "source_version_id": "wheel-source-version",
        },
    },
    {
        "kind": "entity",
        "body": "local native wheel runtime validation",
        "source_id": "smoke:local-native-wheel",
        "provenance": {
            "schema_version": 1,
            "role": "derived",
            "artifact_revision_id": "wheel-derived-revision",
            "source_version_id": "wheel-source-version",
            "source_revision_id": "wheel-source-revision",
            "source_locator": {"kind": "whole_body"},
            "canonical_source_hash": {
                "algorithm": "sha256",
                "digest_hex": "0290cc0c54e573ce8b5150fcdaa22ee7506e99ede078ce66d012eaa901b6edbb",
            },
        },
    },
])
dependency = engine.register_source_dependency({
    "schema_version": 1,
    "dependency_id": "wheel-dependency",
    "source_revision_id": "wheel-source-revision",
    "derived_revision_id": "wheel-derived-revision",
})
assert dependency.registered_dependency_generation == "1"
assert engine.dependencies_for_source({
    "schema_version": 1,
    "source_revision_id": "wheel-source-revision",
}).items == (dependency,)
assert engine.dependency_for_derived({
    "schema_version": 1,
    "derived_revision_id": "wheel-derived-revision",
}) == dependency
engine.search("runtime validation")
engine.close()
print("local Python wheel runtime validation: ok")
PY

MAIN="$WORK/main"
NPM_ROOT="$WORK/npm"
PLATFORM="$NPM_ROOT/$NAPI_LABEL"
CONSUMER="$WORK/consumer"
mkdir -p "$MAIN" "$PLATFORM" "$CONSUMER"
cp "$TS_DIR/package.json" "$TS_DIR/LICENSE" "$MAIN/"
cp -R "$TS_DIR/dist" "$MAIN/dist"
cp "$PLATFORM_PACKAGE_DIR/package.json" "$PLATFORM_PACKAGE_DIR/LICENSE" "$PLATFORM/"
cp "$TS_DIR/fathomdb.$NAPI_LABEL.node" "$PLATFORM/fathomdb.$NAPI_LABEL.node"

# This is the same publish-time injection used by the release workflow. The
# local fixture contains only its matched platform package, so npm never needs
# a registry to resolve unrelated native packages.
bash "$REPO_ROOT/scripts/release/npm-inject-optional-deps.sh" "$MAIN" "$NPM_ROOT"

platform_name="$(node -p "require(process.argv[1]).name" "$PLATFORM/package.json")"
main_version="$(node -p "require(process.argv[1]).version" "$MAIN/package.json")"
injected="$(node -p "require(process.argv[1]).optionalDependencies[process.argv[2]] || ''" \
  "$MAIN/package.json" "$platform_name")"
if [ "$injected" != "$main_version" ]; then
  printf 'smoke-local-native-artifacts: %s optionalDependency is %s, expected %s\n' \
    "$platform_name" "${injected:-<missing>}" "$main_version" >&2
  exit 1
fi

platform_tarball="$(cd "$PLATFORM" && npm pack --silent)"
main_tarball="$(cd "$MAIN" && npm pack --silent)"
cat > "$CONSUMER/package.json" <<EOF
{
  "private": true,
  "type": "module",
  "dependencies": {
    "fathomdb": "file:$MAIN/$main_tarball",
    "$platform_name": "file:$PLATFORM/$platform_tarball"
  }
}
EOF

(
  cd "$CONSUMER"
  npm install --offline --ignore-scripts
  node --input-type=module - "$WORK/npm-smoke.fdb" <<'JS'
import { Engine } from "fathomdb";

const engine = await Engine.open(process.argv[2]);
await engine.write([{
  kind: "doc",
  body: "AéB",
  sourceId: "smoke:local-native-npm",
  provenance: {
    schemaVersion: 1,
    role: "canonical",
    artifactRevisionId: "npm-source-revision",
    sourceVersionId: "npm-source-version",
  },
}, {
  kind: "entity",
  body: "local native npm runtime validation",
  sourceId: "smoke:local-native-npm",
  provenance: {
    schemaVersion: 1,
    role: "derived",
    artifactRevisionId: "npm-derived-revision",
    sourceVersionId: "npm-source-version",
    sourceRevisionId: "npm-source-revision",
    sourceLocator: { kind: "whole_body" },
    canonicalSourceHash: {
      algorithm: "sha256",
      digestHex: "0290cc0c54e573ce8b5150fcdaa22ee7506e99ede078ce66d012eaa901b6edbb",
    },
  },
}]);
const dependency = await engine.registerSourceDependency({
  schemaVersion: 1,
  dependencyId: "npm-dependency",
  sourceRevisionId: "npm-source-revision",
  derivedRevisionId: "npm-derived-revision",
});
if (dependency.registeredDependencyGeneration !== "1") throw new Error("bad generation");
const bySource = await engine.dependenciesForSource({
  schemaVersion: 1,
  sourceRevisionId: "npm-source-revision",
});
if (bySource.items.length !== 1 || bySource.items[0].dependencyId !== "npm-dependency") {
  throw new Error("source dependency lookup failed");
}
const byDerived = await engine.dependencyForDerived({
  schemaVersion: 1,
  derivedRevisionId: "npm-derived-revision",
});
if (byDerived?.dependencyId !== "npm-dependency") {
  throw new Error("derived dependency lookup failed");
}
await engine.search("runtime validation");
await engine.close();
console.log("local N-API package runtime validation: ok");
JS
)

printf 'smoke-local-native-artifacts: ok — local wheel + matched N-API package validated\n'
