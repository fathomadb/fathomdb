#!/usr/bin/env bash
# Consume locally built Python and N-API artifacts without contacting a registry.
set -euo pipefail

if [ "$#" -lt 4 ] || [ "$#" -gt 5 ]; then
  printf 'usage: %s <wheel-dir> <ts-dir> <platform-package-dir> <napi-label> [retained-test-manifest]\n' "$0" >&2
  exit 2
fi

WHEEL_DIR="$1"
TS_DIR="$2"
PLATFORM_PACKAGE_DIR="$3"
NAPI_LABEL="$4"
RETAINED_TEST_MANIFEST="${5:-}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
PYTHON_BASE="${FATHOMDB_SMOKE_PYTHON:-python3}"
NODE_CMD="${FATHOMDB_SMOKE_NODE:-node}"
LIGHTWEIGHT="${FATHOMDB_SMOKE_LIGHTWEIGHT:-0}"

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
if ! "$PYTHON_BASE" - "${wheel_paths[0]}" "$ABI3_TAG" <<'PY'
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
WORK="$(cd "$WORK" && pwd -P)"
unset PYTHONPATH

"$PYTHON_BASE" -m venv "$WORK/python-venv"
PYTHON="$WORK/python-venv/bin/python"
"$PYTHON" -m pip install --no-index --find-links "$WHEEL_DIR" fathomdb
"$PYTHON" - "$WORK/python-venv" <<'PY'
from pathlib import Path
import sys

import fathomdb
from fathomdb import _native

venv = Path(sys.argv[1]).resolve()
package = Path(fathomdb.__file__).resolve()
native = Path(_native.__file__).resolve()
if venv not in package.parents or venv not in native.parents:
    raise SystemExit(
        f"installed Python import escaped isolated environment: package={package} native={native}"
    )
print(f"slice75-native-python-paths: package={package} native={native} source_fallback=false")
PY
"$PYTHON" - "$WORK/python-runtime-performance.fathom" <<'PY'
import sys

from fathomdb import Engine, admin

configured = admin.configure_runtime(sqlite_mode="performance")
assert configured.sqlite_mode == "performance"
Engine.open(sys.argv[1], use_default_embedder=False).close()
print("slice85-runtime-python-performance: pass")
PY
"$PYTHON" - <<'PY'
from fathomdb import admin
from fathomdb.errors import RuntimeConfigurationError

admin.configure_runtime(sqlite_mode="diagnostics")
admin.configure_runtime(sqlite_mode="diagnostics")
try:
    admin.configure_runtime(sqlite_mode="performance")
except RuntimeConfigurationError as error:
    assert error.reason == "conflict"
    assert error.requested_mode == "performance"
    assert error.effective_mode == "diagnostics"
else:
    raise AssertionError("expected a typed runtime configuration conflict")
print("slice85-runtime-python-conflict: pass")
PY
"$PYTHON" - <<'PY'
from fathomdb import admin

try:
    admin.configure_runtime(sqlite_mode="fast")
except ValueError:
    pass
else:
    raise AssertionError("expected an invalid-mode ValueError")
assert admin.configure_runtime(sqlite_mode="performance").sqlite_mode == "performance"
print("slice85-runtime-python-invalid: pass")
PY
"$PYTHON" - "$REPO_ROOT/tests/fixtures/slice45_frozen_context_v3.json" \
  "$WORK/python-frozen-fixture.sqlite" "$WORK/python-frozen-token.txt" <<'PY'
import json
import sqlite3
import sys

from fathomdb import Engine
from fathomdb.types import ReadContextV1, ReadView, SearchFilter

fixture = json.load(open(sys.argv[1], encoding="utf-8"))
database = sys.argv[2]
token_path = sys.argv[3]
Engine.open(database, use_default_embedder=False).close()
with sqlite3.connect(database) as connection:
    old_generation, declaration = connection.execute(
        "SELECT generation_id,declaration_sha256 "
        "FROM _fathomdb_projection_generations WHERE role='serving'"
    ).fetchone()
    assert declaration == fixture["declaration_sha256"]
    connection.execute(
        "UPDATE _fathomdb_projection_generations "
        "SET role='retired',retired_boundary=0 WHERE generation_id=?",
        (old_generation,),
    )
    connection.execute(
        "INSERT INTO _fathomdb_projection_generations("
        "schema_version,generation_id,declaration_sha256,transition_boundary,role,origin"
        ") VALUES(1,?,?,0,'serving','fresh')",
        (fixture["generation_id"], declaration),
    )
    connection.execute(
        "UPDATE _fathomdb_projection_generation_current SET generation_id=? WHERE singleton=1",
        (fixture["generation_id"],),
    )
    connection.execute(
        "UPDATE _fathomdb_open_state SET value=? WHERE key='_fathomdb_database_id'",
        (fixture["database_id"],),
    )
    connection.execute(
        "UPDATE _fathomdb_open_state SET value=? WHERE key='_fathomdb_read_context_key'",
        (fixture["read_context_key"],),
    )
    connection.execute(
        "UPDATE _fathomdb_read_visibility_state "
        "SET generation=0,state_nonce=? WHERE singleton=1",
        (fixture["state_nonce"],),
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
with open(token_path, "w", encoding="utf-8") as token_file:
    token_file.write(frozen.token)
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
printf 'slice75-native-python-result: markers=3 write_search=pass dependency=pass frozen_read=pass\n'

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

platform_name="$("$NODE_CMD" -p "require(process.argv[1]).name" "$PLATFORM/package.json")"
main_version="$("$NODE_CMD" -p "require(process.argv[1]).version" "$MAIN/package.json")"
injected="$("$NODE_CMD" -p "require(process.argv[1]).optionalDependencies[process.argv[2]] || ''" \
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
  "$NODE_CMD" --input-type=module - "$WORK/node-runtime-performance.fathom" <<'JS'
import { Engine, admin } from "fathomdb";

const configured = admin.configureRuntime({ sqliteMode: "performance" });
if (configured.sqliteMode !== "performance") throw new Error("wrong runtime mode");
const engine = await Engine.open(process.argv[2], { useDefaultEmbedder: false });
await engine.close();
console.log("slice85-runtime-node-performance: pass");
JS
  "$NODE_CMD" --input-type=module <<'JS'
import { RuntimeConfigurationError, admin } from "fathomdb";

admin.configureRuntime({ sqliteMode: "diagnostics" });
admin.configureRuntime({ sqliteMode: "diagnostics" });
try {
  admin.configureRuntime({ sqliteMode: "performance" });
  throw new Error("expected a typed runtime configuration conflict");
} catch (error) {
  if (!(error instanceof RuntimeConfigurationError)) throw error;
  if (error.reason !== "conflict" || error.requestedMode !== "performance" ||
      error.effectiveMode !== "diagnostics") throw error;
}
console.log("slice85-runtime-node-conflict: pass");
JS
  "$NODE_CMD" --input-type=module <<'JS'
import { admin } from "fathomdb";

try {
  admin.configureRuntime({ sqliteMode: "fast" });
  throw new Error("expected an invalid-mode RangeError");
} catch (error) {
  if (!(error instanceof RangeError)) throw error;
}
if (admin.configureRuntime({ sqliteMode: "performance" }).sqliteMode !== "performance") {
  throw new Error("invalid mode mutated the runtime");
}
console.log("slice85-runtime-node-invalid: pass");
JS
  "$NODE_CMD" --input-type=module - "$REPO_ROOT/tests/fixtures/slice45_frozen_context_v3.json" \
    "$WORK/python-frozen-fixture.sqlite" "$WORK/python-frozen-token.txt" <<'JS'
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { Engine } from "fathomdb";

const fixture = JSON.parse(readFileSync(process.argv[2], "utf8"));
const expectedToken = readFileSync(process.argv[4], "utf8");
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
    attributes: raw.attributes,
  },
});
assert.equal(frozen.schemaVersion, raw.schema_version);
assert.equal(frozen.effectiveValidAt, raw.valid_as_of);
assert.deepEqual(frozen.context, {
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
    createdAfter: undefined,
    status: undefined,
    attributes: raw.attributes,
  },
});
assert.equal(frozen.token, expectedToken);
await engine.close();
JS
  "$NODE_CMD" --input-type=module - "$WORK/npm-smoke.fdb" <<'JS'
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
  "$NODE_CMD" --input-type=module - "$WORK/npm-graph-evidence.fdb" <<'JS'
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { Engine, graph } from "fathomdb";

const declarations = readFileSync("node_modules/fathomdb/dist/index.d.ts", "utf8");
assert.match(declarations, /interface GraphEvidenceResolveRequestV1/);
assert.match(declarations, /resolveGraphEvidence/);

const sourceBody = "installed npm graph evidence bytes";
const digestHex = createHash("sha256").update(sourceBody).digest("hex");
const derived = (artifactRevisionId) => ({
  schemaVersion: 1,
  role: "derived",
  artifactRevisionId,
  sourceVersionId: "npm-graph-v1",
  sourceRevisionId: "npm-graph-source-r1",
  sourceLocator: { kind: "whole_body" },
  canonicalSourceHash: { algorithm: "sha256", digestHex },
});
const engine = await Engine.open(process.argv[2], { useDefaultEmbedder: false });
await engine.write([{
  kind: "document",
  body: sourceBody,
  sourceId: "npm-graph-owner",
  logicalId: "npm-graph-source",
  provenance: {
    schemaVersion: 1,
    role: "canonical",
    artifactRevisionId: "npm-graph-source-r1",
    sourceVersionId: "npm-graph-v1",
  },
}, {
  kind: "claim",
  body: "installed root",
  sourceId: "npm-graph-owner",
  logicalId: "npm-graph-root",
  provenance: derived("npm-graph-root-r1"),
}, {
  kind: "claim",
  body: "installed target",
  sourceId: "npm-graph-owner",
  logicalId: "npm-graph-target",
  provenance: derived("npm-graph-target-r1"),
}, {
  edge: {
    kind: "supports",
    from: "npm-graph-root",
    to: "npm-graph-target",
    sourceId: "npm-graph-owner",
    logicalId: "npm-graph-edge",
    provenance: derived("npm-graph-edge-r1"),
  },
}]);
await engine.drain(30_000);
const context = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
const expanded = await graph.expand(engine, {
  schemaVersion: 1,
  seed: {
    schemaVersion: 1,
    type: "explicit",
    logicalIds: [{ space: "logical", value: "npm-graph-root" }],
  },
  direction: "outgoing",
  edgeKinds: ["supports"],
  targetKinds: ["claim"],
  context: { schemaVersion: 1, type: "frozen", context },
  maxDepth: 1,
  resultLimit: 1,
  maxWorkUnits: "10",
  includeExplanation: false,
  includeEvidence: true,
});
assert.equal(expanded.targets[0]?.logicalId, "npm-graph-target");
assert.equal(expanded.evidence?.entries.length, 1);
const entry = expanded.evidence.entries[0];
const target = await engine.resolveGraphEvidence({
  schemaVersion: 1,
  evidenceRef: entry.targetEvidenceRef,
  context,
});
const edge = await engine.resolveGraphEvidence({
  schemaVersion: 1,
  evidenceRef: entry.terminalEdgeEvidenceRef,
  context,
});
assert.deepEqual(
  [target.artifact.artifactClass, target.artifact.logicalId, target.canonicalSourceBody],
  ["node", "npm-graph-target", sourceBody],
);
assert.equal(edge.artifact.artifactClass, "edge");
assert.equal(edge.artifact.from, "npm-graph-root");
assert.equal(edge.artifact.to, "npm-graph-target");
await engine.close();
console.log("slice20 installed N-API graph evidence: pass");
JS
)
printf 'slice85-runtime-configuration-result: python=3 node=3 skipped=0\n'

resolved_main="$(cd "$CONSUMER" && "$NODE_CMD" -e 'process.stdout.write(require.resolve(process.argv[1]))' fathomdb)"
resolved_native="$(cd "$CONSUMER" && "$NODE_CMD" -e 'process.stdout.write(require.resolve(process.argv[1]))' "$platform_name")"
resolved_main="$(realpath "$resolved_main")"
resolved_native="$(realpath "$resolved_native")"
case "$resolved_main" in
  "$CONSUMER"/*) ;;
  *) printf 'smoke-local-native-artifacts: main module escaped isolated consumer\n' >&2; exit 1 ;;
esac
case "$resolved_native" in
  "$CONSUMER"/*) ;;
  *) printf 'smoke-local-native-artifacts: native module escaped isolated consumer\n' >&2; exit 1 ;;
esac
built_native_sha256="$("$PYTHON_BASE" - "$TS_DIR/fathomdb.$NAPI_LABEL.node" <<'PY'
import hashlib
from pathlib import Path
import sys

print(hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest())
PY
)"
installed_native_sha256="$("$PYTHON_BASE" - "$resolved_native" <<'PY'
import hashlib
from pathlib import Path
import sys

print(hashlib.sha256(Path(sys.argv[1]).read_bytes()).hexdigest())
PY
)"
if [ "$built_native_sha256" != "$installed_native_sha256" ]; then
  printf 'smoke-local-native-artifacts: installed native artifact digest mismatch\n' >&2
  exit 1
fi

if [ "$LIGHTWEIGHT" != 1 ] && [ -n "$RETAINED_TEST_MANIFEST" ]; then
  COMPILED="$WORK/compiled"
  MIRROR="$WORK/mirror"
  mkdir -p "$COMPILED" "$MIRROR/src/ts"
  "$PYTHON_BASE" - "$REPO_ROOT" "$TS_DIR" "$RETAINED_TEST_MANIFEST" \
    "$WORK/modules.txt" "$WORK/fixtures.txt" <<'PY'
import json
import pathlib
import re
import sys

repo = pathlib.Path(sys.argv[1]).resolve()
ts = pathlib.Path(sys.argv[2]).resolve()
manifest_path = pathlib.Path(sys.argv[3]).resolve()
if repo not in manifest_path.parents or not manifest_path.is_file():
    raise SystemExit("retained test manifest escaped the source checkout")
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if manifest.get("schema_version") != "fathomdb.slice73.windows-napi/v1":
    raise SystemExit("unsupported retained test manifest schema")
modules = manifest.get("modules", [])
fixtures = manifest.get("fixtures", [])
if not modules or not fixtures or len(modules) != len(set(modules)) or len(fixtures) != len(set(fixtures)):
    raise SystemExit("retained test manifest must contain unique modules and fixtures")
for module in modules:
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]*\.test\.js", module):
        raise SystemExit(f"unsafe retained module {module}")
    if not (ts / "tests" / module.replace(".js", ".ts")).is_file():
        raise SystemExit(f"missing retained test source {module}")
for fixture in fixtures:
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._/-]*", fixture) or ".." in fixture:
        raise SystemExit(f"unsafe retained fixture {fixture}")
    source = (repo / fixture).resolve()
    if repo not in source.parents or not source.is_file():
        raise SystemExit(f"missing retained fixture {fixture}")
pathlib.Path(sys.argv[4]).write_text("\n".join(modules) + "\n", encoding="utf-8")
pathlib.Path(sys.argv[5]).write_text("\n".join(fixtures) + "\n", encoding="utf-8")
PY
  "$NODE_CMD" "$TS_DIR/node_modules/typescript/bin/tsc" \
    -p "$TS_DIR/tsconfig.json" --outDir "$COMPILED"
  rm -rf "$COMPILED/src"
  mkdir -p "$COMPILED/src"
  cp -R "$CONSUMER/node_modules/fathomdb/dist/." "$COMPILED/src/"
  while IFS= read -r fixture; do
    mkdir -p "$MIRROR/$(dirname "$fixture")"
    cp "$REPO_ROOT/$fixture" "$MIRROR/$fixture"
  done < "$WORK/fixtures.txt"
  mkdir -p "$MIRROR/src/ts/dist"
  cp -R "$COMPILED/." "$MIRROR/src/ts/dist/"
  cp -R "$CONSUMER/node_modules" "$MIRROR/src/ts/node_modules"
  total_modules=0
  total_tests=0
  while IFS= read -r module; do
    module_output="$(cd "$MIRROR/src/ts" && "$NODE_CMD" --test --test-reporter=tap "dist/tests/$module")"
    printf '%s\n' "$module_output"
    counts="$(printf '%s\n' "$module_output" | "$PYTHON_BASE" -c '
import re, sys
text=sys.stdin.read()
def one(name):
    found=re.findall(rf"^# {name} (\d+)\s*$", text, re.M)
    if len(found) != 1: raise SystemExit(f"missing unique TAP count: {name}")
    return int(found[0])
tests=one("tests"); passed=one("pass")
bad=sum(one(name) for name in ("fail", "cancelled", "skipped", "todo"))
if tests <= 0 or passed != tests or bad: raise SystemExit("retained module did not pass without skips")
print(tests)
')"
    total_modules=$((total_modules + 1))
    total_tests=$((total_tests + counts))
  done < "$WORK/modules.txt"
  printf 'slice75-native-napi-result: modules=%s tests=%s pass=%s skipped=0\n' \
    "$total_modules" "$total_tests" "$total_tests"
fi

printf 'smoke-local-native-artifacts: ok — local wheel + matched N-API package validated\n'
