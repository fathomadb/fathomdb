#!/usr/bin/env bash
# scripts/tests/test_idempotent_republish.sh — R-REL-4b/4c coordinated-publish
# resilience. The prerequisite for the 0.8.20 OPP-12 breaking-pair publish,
# where a partial-landing retry must NOT double-publish.
#
# THIS TEST USES REAL LOCAL REGISTRIES, not argv-recording shims (the prior
# revision only recorded argv, which validated branching but never the real
# publish -> query-sees-it -> re-run-no-ops -> install round-trip R-REL-4b
# requires). What is real vs simulated here:
#
#   • npm  — REAL. A minimal but faithful npm registry (implements the publish
#     PUT + packument GET + tarball GET HTTP protocol) is stood up in-process.
#     `npm publish` really uploads the thin main `fathomdb` package AND the
#     platform package for THIS host's architecture (resolved via the real
#     loader's tripleFor, e.g. `fathomdb-linux-arm64-gnu` on aarch64,
#     `fathomdb-linux-x64-gnu` on x86_64 — 0.8.23 Slice 80.2, AC80-4) over
#     HTTP through the real npm-publish-if-new.sh helper; the tarballs are
#     really stored and served back with the integrity npm computed. A
#     re-run NO-OPS via the helper's registry-query path (npm is NOT
#     re-invoked — asserted by a server-side PUT counter, so the no-op is
#     not vacuous). A real `npm install` then pulls the os/cpu-gated
#     platform package, and the REAL loader from src/ts/src/platform.ts
#     (loadPlatformBinding) resolves it on this host and throws
#     UnsupportedPlatformError for a foreign host.
#     Fix-1 safety (publish targets the queried registry, never prod) is
#     structurally guaranteed here: the whole round-trip is confined to the
#     local registry — a stray prod publish would escape it.
#
#   • crates.io — SIMULATED (documented residual). A real local crates registry
#     (sparse index + `cargo publish` /api/v1/crates/new endpoint + auth/token
#     + on-disk index) is infeasible inside this harness. The crates leg keeps
#     an offline http-fixture for the registry QUERY plus a shim for the publish
#     tool, exercising the helper's branch logic + query-registry routing only.
#     The REAL crates publish path is covered at real-tag time (cargo publish's
#     own idempotency/verify). See test_cargo_publish_if_new.sh for the version-
#     resolution unit coverage.
#
#   • PyPI — the REAL twine upload -> query-sees-it -> re-run-no-op round-trip
#     lives in test_pypi_publish_roundtrip.sh (it needs a `twine` client; kept
#     separate so this node-only test stays dependency-light and always runs).
#
# Fix-1 flag-routing (publish targets $BASE / --repository-url) has a dedicated
# fast unit guard in test_publish_registry_safety.sh.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CARGO_HELPER="$REPO_ROOT/scripts/release/cargo-publish-if-new.sh"
NPM_HELPER="$REPO_ROOT/scripts/release/npm-publish-if-new.sh"

FAILED=0
SERVE_DIR="$(mktemp -d)"
SHIM_DIR="$(mktemp -d)"
WORK_DIR="$(mktemp -d)"
TMP="$(mktemp -d)"
PORT=0
PID=0
NPM_REG_PID=0

cleanup() {
  if [ "$PID" -ne 0 ]; then kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true; fi
  if [ "$NPM_REG_PID" -ne 0 ]; then kill "$NPM_REG_PID" 2>/dev/null || true; wait "$NPM_REG_PID" 2>/dev/null || true; fi
  rm -rf "$SERVE_DIR" "$SHIM_DIR" "$WORK_DIR" "$TMP"
}
trap cleanup EXIT

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; FAILED=$((FAILED + 1)); }

# This whole suite (crates.io leg included, for a consistent diagnostic) needs
# node — both for the REAL npm round-trip below and for resolving the host
# triple next.
if ! command -v node >/dev/null 2>&1; then
  fail "node is required for the REAL npm round-trip but is not on PATH"
  exit 1
fi

# ==========================================================================
# Host platform/arch (0.8.23 Slice 80.2, AC80-4): resolved through the REAL
# loader (src/ts/src/platform.ts's tripleFor/platformPackageName), the same
# single source of truth npm's optionalDependencies filtering and the loader
# itself both use — not a second, hand-maintained os/cpu table in this test.
# This makes the npm fixture's package name and os/cpu fields track whatever
# host actually runs this suite (x86_64 or aarch64) instead of assuming x64.
# ==========================================================================
LOADER_DIR="$TMP/loader"; mkdir -p "$LOADER_DIR"
TSC="$REPO_ROOT/src/ts/node_modules/.bin/tsc"
LOADER=""
if [ -x "$TSC" ]; then
  if "$TSC" "$REPO_ROOT/src/ts/src/platform.ts" \
        --types node --typeRoots "$REPO_ROOT/src/ts/node_modules/@types" \
        --module nodenext --moduleResolution nodenext --target es2022 \
        --outDir "$LOADER_DIR" >"$TMP/tsc.log" 2>&1; then
    LOADER="$LOADER_DIR/platform.js"
  fi
fi
if [ -z "$LOADER" ]; then
  # Fallback: run platform.ts directly via Node's type stripping (>=22.18/23+).
  cp "$REPO_ROOT/src/ts/src/platform.ts" "$LOADER_DIR/platform.ts"
  LOADER="$LOADER_DIR/platform.ts"
fi

cat >"$TMP/host-info.mjs" <<JS
import { tripleFor, platformPackageName } from ${LOADER@Q};
const triple = tripleFor(process.platform, process.arch, false);
if (!triple) {
  console.error("host platform/arch not mapped: " + process.platform + "/" + process.arch);
  process.exit(1);
}
console.log([process.platform, process.arch, triple, platformPackageName(triple)].join(" "));
JS
HOST_INFO="$(node "$TMP/host-info.mjs")" || { fail "could not resolve this host's triple via the real loader"; exit 1; }
read -r HOST_PLATFORM HOST_ARCH HOST_TRIPLE HOST_PKG_NAME <<<"$HOST_INFO"

# ==========================================================================
# crates.io leg — SIMULATED (documented above): offline http-fixture QUERY +
# publish-tool shim. Exercises branch logic + query-registry routing only.
# ==========================================================================
mkdir -p "$SERVE_DIR/api/v1/crates"
cat >"$SERVE_DIR/api/v1/crates/fathomdb-engine" <<'JSON'
{"crate":{"name":"fathomdb-engine"},"versions":[{"num":"9.9.9","yanked":false}]}
JSON
cat >"$SERVE_DIR/api/v1/crates/fathomdb-query" <<'JSON'
{"crate":{"name":"fathomdb-query"},"versions":[{"num":"1.0.0","yanked":false}]}
JSON

start_fixture_server() {
  ( cd "$SERVE_DIR" && python3 -u -m http.server 0 ) >"$SERVE_DIR/server.log" 2>&1 &
  PID=$!
  for _ in $(seq 1 50); do
    if grep -qE 'Serving HTTP on .* port [0-9]+' "$SERVE_DIR/server.log" 2>/dev/null; then
      PORT="$(sed -nE 's/.*port ([0-9]+).*/\1/p' "$SERVE_DIR/server.log" | head -1)"
      break
    fi
    sleep 0.05
  done
  [ "$PORT" -ne 0 ] || { fail "fixture http server failed to bind"; exit 1; }
}

export TOOL_LOG="$SHIM_DIR/tool.log"
cat >"$SHIM_DIR/cargo" <<'SHIM'
#!/usr/bin/env bash
printf 'cargo %s\n' "$*" >>"$TOOL_LOG"
exit 0
SHIM
chmod +x "$SHIM_DIR/cargo"
reset_log() { : >"$TOOL_LOG"; }

start_fixture_server
REG="http://127.0.0.1:${PORT}"

# crates.io: version already present -> skip, cargo NOT invoked (simulated).
reset_log
if out="$(CARGO_PUBLISH_IF_NEW_REGISTRY="$REG" CARGO_REGISTRY_TOKEN=t \
          CARGO_PUBLISH_IF_NEW_LOCAL_VERSION=9.9.9 \
          PATH="$SHIM_DIR:$PATH" "$CARGO_HELPER" fathomdb-engine 2>&1)"; then
  if printf '%s' "$out" | grep -q 'already published; skipping' && ! grep -q '^cargo' "$TOOL_LOG" 2>/dev/null; then
    pass "crates.io re-run no-ops [SIMULATED] (cargo not invoked)"
  else
    fail "crates.io no-op: out='$out' tool='$(cat "$TOOL_LOG")'"
  fi
else
  fail "crates.io helper exited non-zero: $out"
fi

# crates.io positive control: absent version -> publish invoked (not vacuous).
# CARGO_PUBLISH_IF_NEW_PUBLISH_REGISTRY maps the redirected query to a publish
# alt-registry (Fix-2 split-brain guard) so the helper does not fail closed.
reset_log
if out="$(CARGO_PUBLISH_IF_NEW_REGISTRY="$REG" CARGO_REGISTRY_TOKEN=t \
          CARGO_PUBLISH_IF_NEW_PUBLISH_REGISTRY=staging-alt \
          CARGO_PUBLISH_IF_NEW_LOCAL_VERSION=9.9.9 \
          PATH="$SHIM_DIR:$PATH" "$CARGO_HELPER" fathomdb-query 2>&1)"; then
  grep -q '^cargo .*publish' "$TOOL_LOG" 2>/dev/null \
    && pass "crates.io absent version publishes [SIMULATED] (cargo invoked)" \
    || fail "crates.io absent: expected cargo publish; tool='$(cat "$TOOL_LOG")' out='$out'"
else
  fail "crates.io absent helper exited non-zero: $out"
fi

# ==========================================================================
# npm leg — REAL round-trip against a minimal in-process npm registry.
# ==========================================================================

NPM_REG_JS="$TMP/npm-registry.js"
cat >"$NPM_REG_JS" <<'JS'
// Minimal REAL npm registry: publish PUT + packument GET + tarball GET. Stores
// the tarball from the publish _attachments and serves it back with the exact
// dist.integrity npm computed, so `npm install` verifies + extracts a REAL
// tarball. Records every PUT to $PUT_LOG so the test can prove a no-op re-run
// did NOT publish again.
const http = require("http");
const fs = require("fs");
const putLog = process.env.PUT_LOG;
const store = new Map();
function pkgName(url) { return decodeURIComponent(url.replace(/^\//, "").split("/-/")[0]); }
const srv = http.createServer((req, res) => {
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", () => {
    const body = Buffer.concat(chunks);
    if (req.method === "PUT") {
      const doc = JSON.parse(body.toString("utf8"));
      const n = doc.name;
      if (putLog) fs.appendFileSync(putLog, n + "\n");
      const cur = store.get(n) || { name: n, "dist-tags": {}, versions: {}, _tarballs: {} };
      for (const [v, meta] of Object.entries(doc.versions || {})) cur.versions[v] = meta;
      Object.assign(cur["dist-tags"], doc["dist-tags"] || {});
      for (const [fn, att] of Object.entries(doc._attachments || {})) {
        cur._tarballs[fn] = Buffer.from(att.data, "base64");
      }
      store.set(n, cur);
      res.writeHead(201, { "content-type": "application/json" });
      res.end(JSON.stringify({ ok: true }));
      return;
    }
    const tgz = req.url.match(/\/-\/([^/]+\.tgz)$/);
    if (req.method === "GET" && tgz) {
      const doc = store.get(pkgName(req.url));
      const buf = doc && doc._tarballs[tgz[1]];
      if (!buf) { res.writeHead(404); res.end("no tarball"); return; }
      res.writeHead(200, { "content-type": "application/octet-stream" });
      res.end(buf);
      return;
    }
    if (req.method === "GET") {
      const n = pkgName(req.url);
      // A syntactically-valid error body must not be mistaken for an absent
      // package. This reproduces a rate-limited npm packument request.
      if (n === "fathomdb-rate-limited") {
        res.writeHead(429, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: "Too Many Requests" }));
        return;
      }
      const doc = store.get(n);
      if (!doc) { res.writeHead(404, { "content-type": "application/json" }); res.end(JSON.stringify({ error: "Not found" })); return; }
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ name: doc.name, "dist-tags": doc["dist-tags"], versions: doc.versions }));
      return;
    }
    res.writeHead(405); res.end("no");
  });
});
srv.listen(0, "127.0.0.1", () => { process.stdout.write("PORT=" + srv.address().port + "\n"); });
JS

export PUT_LOG="$TMP/put.log"
: >"$PUT_LOG"
node "$NPM_REG_JS" >"$TMP/npm-reg.log" 2>&1 &
NPM_REG_PID=$!
NPM_PORT=0
for _ in $(seq 1 50); do
  NPM_PORT="$(sed -nE 's/PORT=([0-9]+)/\1/p' "$TMP/npm-reg.log" | head -1)"
  [ -n "$NPM_PORT" ] && break
  sleep 0.05
done
[ -n "$NPM_PORT" ] || { fail "npm fixture registry failed to bind"; exit 1; }
NPM_REG="http://127.0.0.1:${NPM_PORT}"

# Package fixtures: thin main `fathomdb` (with an injected optionalDependency on
# the platform pkg, i.e. post-injection state) + the platform pkg. The platform
# pkg ships a small index.js sentinel as `main` so the loader's real require()
# resolves a loadable module (a real .node addon can't be dlopen'd in a script
# test; the loader RESOLUTION logic is what R-REL-4f/the loader owns).
PLAT_DIR="$WORK_DIR/plat"; mkdir -p "$PLAT_DIR"
cat >"$PLAT_DIR/package.json" <<PJ
{ "name": "$HOST_PKG_NAME", "version": "9.9.9",
  "os": ["$HOST_PLATFORM"], "cpu": ["$HOST_ARCH"], "main": "index.js", "files": ["index.js"] }
PJ
printf 'module.exports = { __fathomdb_native: "platform-pkg" };\n' >"$PLAT_DIR/index.js"
cat >"$PLAT_DIR/.npmrc" <<NPMRC
registry=${NPM_REG}
//127.0.0.1:${NPM_PORT}/:_authToken=fake-token
NPMRC

MAIN_DIR="$WORK_DIR/main"; mkdir -p "$MAIN_DIR/dist"
cat >"$MAIN_DIR/package.json" <<PJ
{ "name": "fathomdb", "version": "9.9.9", "main": "dist/index.js",
  "files": ["dist"],
  "optionalDependencies": { "$HOST_PKG_NAME": "9.9.9" } }
PJ
printf 'module.exports = {};\n' >"$MAIN_DIR/dist/index.js"
cat >"$MAIN_DIR/.npmrc" <<NPMRC
registry=${NPM_REG}
//127.0.0.1:${NPM_PORT}/:_authToken=fake-token
NPMRC

ABSENT_DRY_RUN_DIR="$WORK_DIR/absent-dry-run"; mkdir -p "$ABSENT_DRY_RUN_DIR"
cat >"$ABSENT_DRY_RUN_DIR/package.json" <<'PJ'
{ "name": "fathomdb-dry-run-absent", "version": "9.9.9" }
PJ

RATE_LIMIT_DIR="$WORK_DIR/rate-limited"; mkdir -p "$RATE_LIMIT_DIR"
cat >"$RATE_LIMIT_DIR/package.json" <<'PJ'
{ "name": "fathomdb-rate-limited", "version": "9.9.9" }
PJ

# --- Dry-run command contract and HTTP error fail-closed behavior ----------
NPM_DRY_RUN_LOG="$TMP/npm-dry-run-command.log"
cat >"$SHIM_DIR/npm-dry-run-capture" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$NPM_DRY_RUN_LOG"
exit 0
SHIM
chmod +x "$SHIM_DIR/npm-dry-run-capture"

# An absent package still needs the exact npm publish --dry-run rehearsal used
# by the workflow: same registry, dist-tag, and publishing extras.
: >"$NPM_DRY_RUN_LOG"
if out="$(cd "$ABSENT_DRY_RUN_DIR" && NPM_PUBLISH_IF_NEW_REGISTRY="$NPM_REG" \
          NPM_DRY_RUN_LOG="$NPM_DRY_RUN_LOG" NPM_BIN="$SHIM_DIR/npm-dry-run-capture" \
          "$NPM_HELPER" --dry-run --tag next -- --access public --provenance 2>&1)"; then
  NPM_DRY_RUN_COMMAND="$(<"$NPM_DRY_RUN_LOG")"
  if printf '%s' "$out" | grep -q 'fathomdb-dry-run-absent@9.9.9 — dry-run' \
     && printf '%s\n' "$NPM_DRY_RUN_COMMAND" | grep -Fx \
        "publish --registry $NPM_REG --dry-run --tag next --access public --provenance" >/dev/null; then
    pass "npm absent-version dry-run preserves publish registry/tag/extras"
  else
    fail "npm absent-version dry-run: out='$out' npm='$NPM_DRY_RUN_COMMAND'"
  fi
else
  fail "npm absent-version dry-run exited non-zero: $out"
fi

# A valid JSON error response such as npm's 429 rate-limit envelope is not an
# absent packument. The helper must stop before attempting any npm command.
: >"$NPM_DRY_RUN_LOG"
if out="$(cd "$RATE_LIMIT_DIR" && NPM_PUBLISH_IF_NEW_REGISTRY="$NPM_REG" \
          NPM_DRY_RUN_LOG="$NPM_DRY_RUN_LOG" NPM_BIN="$SHIM_DIR/npm-dry-run-capture" \
          "$NPM_HELPER" --dry-run --tag next -- --access public 2>&1)"; then
  fail "npm dry-run HTTP 429 unexpectedly succeeded: $out"
elif printf '%s' "$out" | grep -q 'registry query failed' \
   && [ ! -s "$NPM_DRY_RUN_LOG" ]; then
  pass "npm dry-run HTTP 429 fails closed (no npm command invoked)"
else
  NPM_DRY_RUN_COMMAND="$(<"$NPM_DRY_RUN_LOG")"
  fail "npm dry-run HTTP 429: out='$out' npm='$NPM_DRY_RUN_COMMAND'"
fi

# --- REAL publish through the helper (absent -> publishes) -----------------
if out="$(cd "$PLAT_DIR" && NPM_PUBLISH_IF_NEW_REGISTRY="$NPM_REG" NPM_BIN=npm \
          "$NPM_HELPER" --tag next -- --registry "$NPM_REG" 2>&1)"; then
  if printf '%s' "$out" | grep -Fq "$HOST_TRIPLE@9.9.9" \
     && grep -Fq "$HOST_PKG_NAME" "$PUT_LOG"; then
    pass "npm platform pkg REAL publish (real HTTP PUT landed)"
  else
    fail "npm platform publish: out='$out' put='$(cat "$PUT_LOG")'"
  fi
else
  fail "npm platform helper exited non-zero: $out"
fi

if out="$(cd "$MAIN_DIR" && NPM_PUBLISH_IF_NEW_REGISTRY="$NPM_REG" NPM_BIN=npm \
          "$NPM_HELPER" --tag next -- --registry "$NPM_REG" 2>&1)"; then
  grep -q '^fathomdb$' "$PUT_LOG" \
    && pass "npm main pkg REAL publish (real HTTP PUT landed)" \
    || fail "npm main publish: out='$out' put='$(cat "$PUT_LOG")'"
else
  fail "npm main helper exited non-zero: $out"
fi

# --- REAL idempotency: re-run no-ops, npm NOT re-invoked -------------------
PUTS_BEFORE="$(wc -l <"$PUT_LOG")"
if out="$(cd "$PLAT_DIR" && NPM_PUBLISH_IF_NEW_REGISTRY="$NPM_REG" NPM_BIN=npm \
          "$NPM_HELPER" --tag next -- --registry "$NPM_REG" 2>&1)"; then
  PUTS_AFTER="$(wc -l <"$PUT_LOG")"
  if printf '%s' "$out" | grep -q 'already published; skipping' \
     && [ "$PUTS_AFTER" -eq "$PUTS_BEFORE" ]; then
    pass "npm re-run NO-OPS (query-sees-it; zero new PUTs -> not vacuous)"
  else
    fail "npm no-op: out='$out' puts_before=$PUTS_BEFORE puts_after=$PUTS_AFTER"
  fi
else
  fail "npm no-op helper exited non-zero: $out"
fi

# --- REAL dry-run recovery: a published version still rehearses packing -----
#
# A release dry-run may follow a partial real release, where a platform package
# is already immutable on npm. `npm publish --dry-run` rejects that state with
# the same overwrite error as a real publish. The helper must instead use
# `npm pack --dry-run`: it exercises the local package construction without
# publishing or asking npm to overwrite the immutable version.
PUTS_BEFORE_DRY_RUN="$(wc -l <"$PUT_LOG")"
if out="$(cd "$PLAT_DIR" && NPM_PUBLISH_IF_NEW_REGISTRY="$NPM_REG" NPM_BIN=npm \
          "$NPM_HELPER" --dry-run --tag next -- --access public --provenance 2>&1)"; then
  PUTS_AFTER_DRY_RUN="$(wc -l <"$PUT_LOG")"
  if printf '%s' "$out" | grep -q 'already published; rehearsing with npm pack --dry-run' \
     && printf '%s' "$out" | grep -Fq "$HOST_PKG_NAME-9.9.9.tgz" \
     && [ "$PUTS_AFTER_DRY_RUN" -eq "$PUTS_BEFORE_DRY_RUN" ]; then
    pass "npm dry-run for a published version packs locally (zero new PUTs)"
  else
    fail "npm published-version dry-run: out='$out' puts_before=$PUTS_BEFORE_DRY_RUN puts_after=$PUTS_AFTER_DRY_RUN"
  fi
else
  fail "npm published-version dry-run exited non-zero: $out"
fi

# A dry-run must not turn an unavailable registry into a blind local pack or
# publish attempt. The publish command is a sentinel: the helper must stop at
# the failed registry query before invoking it.
NPM_DRY_RUN_LOG="$TMP/npm-dry-run-command.log"
cat >"$SHIM_DIR/npm-dry-run-sentinel" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$NPM_DRY_RUN_LOG"
exit 99
SHIM
chmod +x "$SHIM_DIR/npm-dry-run-sentinel"
: >"$NPM_DRY_RUN_LOG"
if out="$(cd "$PLAT_DIR" && NPM_PUBLISH_IF_NEW_REGISTRY="http://127.0.0.1:1" \
          NPM_DRY_RUN_LOG="$NPM_DRY_RUN_LOG" NPM_BIN="$SHIM_DIR/npm-dry-run-sentinel" \
          "$NPM_HELPER" --dry-run --tag next -- --access public 2>&1)"; then
  fail "npm dry-run registry error unexpectedly succeeded: $out"
elif printf '%s' "$out" | grep -q 'registry query failed' \
   && [ ! -s "$NPM_DRY_RUN_LOG" ]; then
  pass "npm dry-run registry error fails closed (no npm command invoked)"
else
  NPM_DRY_RUN_COMMAND="$(<"$NPM_DRY_RUN_LOG")"
  fail "npm dry-run registry error: out='$out' npm='$NPM_DRY_RUN_COMMAND'"
fi

# --- REAL install from the registry + REAL loader exercise -----------------
CONSUMER="$WORK_DIR/consumer"; mkdir -p "$CONSUMER"
cat >"$CONSUMER/.npmrc" <<NPMRC
registry=${NPM_REG}
//127.0.0.1:${NPM_PORT}/:_authToken=fake-token
NPMRC
printf '{ "name": "consumer", "version": "1.0.0", "private": true }\n' >"$CONSUMER/package.json"
if ( cd "$CONSUMER" && npm install "fathomdb@9.9.9" --registry "$NPM_REG" >"$TMP/install.log" 2>&1 ); then
  if [ -f "$CONSUMER/node_modules/$HOST_PKG_NAME/index.js" ]; then
    pass "npm install pulls the os/cpu-gated platform package (real extract)"
  else
    fail "npm install: platform package not present after install; log=$(cat "$TMP/install.log")"
  fi
else
  fail "npm install failed: $(cat "$TMP/install.log")"
fi

# Drive loadPlatformBinding (the same transpiled/type-stripped REAL loader
# resolved as $LOADER near the top) against the REAL installed node_modules.
cat >"$TMP/loader-harness.mjs" <<JS
import { loadPlatformBinding, UnsupportedPlatformError } from ${LOADER@Q};
import { createRequire } from "node:module";
const require = createRequire(${CONSUMER@Q} + "/package.json");
let failed = 0;
// Host: loader resolves the REAL installed platform package, using the
// REAL process.platform/process.arch rather than a hardcoded pair, so this
// arm passes on whichever architecture actually runs the suite.
try {
  const mod = loadPlatformBinding({
    platform: process.platform, arch: process.arch, isMusl: false,
    loadLocal: () => null, requirePackage: (p) => require(p),
  });
  if (mod && mod.__fathomdb_native === "platform-pkg") console.log("HOST_RESOLVES_OK");
  else { console.log("HOST_WRONG", JSON.stringify(mod)); failed = 1; }
} catch (e) { console.log("HOST_THREW", String(e)); failed = 1; }
// Foreign host: aix/ppc64 is outside the ENTIRE supported matrix (linux-x64,
// linux-arm64, darwin-x64, darwin-arm64, win32-x64) — unlike darwin/arm64,
// this stays foreign even if this suite is ever run on a darwin/arm64 host.
// The optional dep was never installed -> require throws MODULE_NOT_FOUND ->
// loader throws UnsupportedPlatformError (no segfault).
try {
  loadPlatformBinding({
    platform: "aix", arch: "ppc64", isMusl: false,
    loadLocal: () => null, requirePackage: (p) => require(p),
  });
  console.log("FOREIGN_DID_NOT_THROW"); failed = 1;
} catch (e) {
  if (e instanceof UnsupportedPlatformError) console.log("FOREIGN_UNSUPPORTED_OK");
  else { console.log("FOREIGN_WRONG", String(e)); failed = 1; }
}
process.exit(failed);
JS

if hout="$(node "$TMP/loader-harness.mjs" 2>&1)"; then
  if printf '%s' "$hout" | grep -q 'HOST_RESOLVES_OK'; then
    pass "loader resolves the installed platform package on this host (real require)"
  else
    fail "loader host resolve: $hout"
  fi
  if printf '%s' "$hout" | grep -q 'FOREIGN_UNSUPPORTED_OK'; then
    pass "loader throws UnsupportedPlatformError for a foreign host (real require)"
  else
    fail "loader foreign: $hout"
  fi
else
  fail "loader harness failed: $hout"
fi

if [ "$FAILED" -gt 0 ]; then
  printf '\n%d test(s) failed\n' "$FAILED" >&2
  exit 1
fi
printf '\nAll idempotent-republish tests passed (npm REAL round-trip; crates SIMULATED)\n'
