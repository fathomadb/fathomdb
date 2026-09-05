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
"$PYTHON" - "$REPO_ROOT/tests/fixtures/slice35_frozen_context_v1.json" \
  "$WORK/python-frozen-fixture.sqlite" <<'PY'
import json
import sqlite3
import sys

from fathomdb import Engine, ReadContextV1, ReadView, SearchFilter

fixture = json.load(open(sys.argv[1], encoding="utf-8"))
database = sys.argv[2]
Engine.open(database, use_default_embedder=False).close()
with sqlite3.connect(database) as connection:
    connection.execute(
        "UPDATE _fathomdb_open_state SET value=? WHERE key='_fathomdb_database_id'",
        (fixture["database_id"],),
    )
    connection.execute(
        "UPDATE _fathomdb_open_state SET value=? WHERE key='_fathomdb_read_context_key'",
        (fixture["read_context_key"],),
    )
raw = fixture["context"]
engine = Engine.open(database, use_default_embedder=False)
frozen = engine.freeze_read_context(ReadContextV1(
    schema_version=raw["schema_version"],
    view=ReadView(
        include_superseded=raw["include_superseded"],
        include_inactive=raw["include_inactive"],
        include_out_of_window=raw["include_out_of_window"],
        valid_as_of=raw["valid_as_of"],
    ),
    eligibility=SearchFilter(
        source_type=raw["source_type"],
        kind=raw["kind"],
        created_after=raw["created_after"],
        status=raw["status"],
        attributes=tuple(raw["attributes"]),
    ),
))
assert frozen.effective_valid_at == raw["valid_as_of"]
assert frozen.schema_version == raw["schema_version"]
assert frozen.context.schema_version == raw["schema_version"]
assert frozen.context.view.include_superseded is raw["include_superseded"]
assert frozen.context.view.include_inactive is raw["include_inactive"]
assert frozen.context.view.include_out_of_window is raw["include_out_of_window"]
assert frozen.context.view.valid_as_of == raw["valid_as_of"]
assert frozen.context.eligibility.source_type == raw["source_type"]
assert frozen.context.eligibility.kind == raw["kind"]
assert frozen.context.eligibility.created_after == raw["created_after"]
assert frozen.context.eligibility.status == raw["status"]
assert frozen.context.eligibility.attributes == tuple(raw["attributes"])
assert frozen.token == fixture["token"]
engine.close()
PY
"$PYTHON" - "$WORK/python-smoke.fdb" <<'PY'
import sys

from fathomdb import Engine, ReadContextV1
from fathomdb.errors import FrozenReadError

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
frozen = engine.freeze_read_context(ReadContextV1())
assert engine.search_frozen("runtime validation", frozen).results
engine.write([{
    "kind": "doc",
    "body": "visibility drift",
    "source_id": "smoke:local-native-wheel",
}])
try:
    engine.search_frozen("runtime validation", frozen)
except FrozenReadError as error:
    assert error.reason == "state_drifted"
else:
    raise AssertionError("stale frozen context was accepted")
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
  node --input-type=module - "$WORK/npm-frozen-fixture.sqlite" <<'JS'
import { Engine } from "fathomdb";
const engine = await Engine.open(process.argv[2], { useDefaultEmbedder: false });
await engine.close();
JS
  python3 - "$REPO_ROOT/tests/fixtures/slice35_frozen_context_v1.json" \
    "$WORK/npm-frozen-fixture.sqlite" <<'PY'
import json
import sqlite3
import sys

fixture = json.load(open(sys.argv[1], encoding="utf-8"))
with sqlite3.connect(sys.argv[2]) as connection:
    connection.execute(
        "UPDATE _fathomdb_open_state SET value=? WHERE key='_fathomdb_database_id'",
        (fixture["database_id"],),
    )
    connection.execute(
        "UPDATE _fathomdb_open_state SET value=? WHERE key='_fathomdb_read_context_key'",
        (fixture["read_context_key"],),
    )
PY
  node --input-type=module - "$REPO_ROOT/tests/fixtures/slice35_frozen_context_v1.json" \
    "$WORK/npm-frozen-fixture.sqlite" <<'JS'
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { Engine } from "fathomdb";

const fixture = JSON.parse(readFileSync(process.argv[2], "utf8"));
const raw = fixture.context;
const engine = await Engine.open(process.argv[3], { useDefaultEmbedder: false });
const frozen = await engine.freezeReadContext({
  schemaVersion: raw.schema_version,
  view: {
    includeSuperseded: raw.include_superseded,
    includeInactive: raw.include_inactive,
    includeOutOfWindow: raw.include_out_of_window,
    validAsOf: raw.valid_as_of,
  },
  eligibility: {
    sourceType: raw.source_type,
    kind: raw.kind,
    createdAfter: raw.created_after,
    status: raw.status,
    attributes: raw.attributes,
  },
});
assert.equal(frozen.schemaVersion, raw.schema_version);
assert.equal(frozen.effectiveValidAt, String(raw.valid_as_of));
assert.deepEqual(frozen.context, {
  schemaVersion: raw.schema_version,
  view: {
    includeSuperseded: raw.include_superseded,
    includeInactive: raw.include_inactive,
    includeOutOfWindow: raw.include_out_of_window,
    validAsOf: String(raw.valid_as_of),
  },
  eligibility: {
    sourceType: raw.source_type,
    kind: raw.kind,
    createdAfter: raw.created_after,
    status: raw.status,
    attributes: raw.attributes,
  },
});
assert.equal(frozen.token, fixture.token);
await engine.close();
JS
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
const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
const frozenResult = await engine.searchFrozen("runtime validation", frozen);
if (frozenResult.results.length === 0) throw new Error("frozen search returned no result");
await engine.write([{
  kind: "doc",
  body: "visibility drift",
  sourceId: "smoke:local-native-npm",
}]);
let drifted = false;
try {
  await engine.searchFrozen("runtime validation", frozen);
} catch (error) {
  drifted = error?.reason === "state_drifted";
}
if (!drifted) throw new Error("stale frozen context was accepted");
await engine.close();
console.log("local N-API package runtime validation: ok");
JS
)

printf 'smoke-local-native-artifacts: ok — local wheel + matched N-API package validated\n'
