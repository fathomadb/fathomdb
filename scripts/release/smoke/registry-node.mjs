import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { closeSync, existsSync, fstatSync, openSync, readFileSync, readSync, realpathSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, isAbsolute, join, relative, sep } from "node:path";
import { pathToFileURL } from "node:url";

const [phase, work, expectedVersion, installLog, installExit] = process.argv.slice(2);
const sourceBody = "slice55 registry Node persisted café";
const platform = process.platform;
const arch = process.arch;
const glibc = platform === "linux" ? process.report.getReport().header.glibcVersionRuntime : undefined;
const triple = {
  "linux-x64": glibc ? "linux-x64-gnu" : undefined,
  "linux-arm64": glibc ? "linux-arm64-gnu" : undefined,
  "darwin-x64": "darwin-x64",
  "darwin-arm64": "darwin-arm64",
  "win32-x64": "win32-x64-msvc",
}[`${platform}-${arch}`];
const expectedOptional = triple === "win32-x64-msvc"
  ? "fathomdb-native-win32-x64-msvc"
  : triple && `fathomdb-${triple}`;
const versionFormat = /^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$/;

function resolvePath(require, specifier) {
  try {
    return realpathSync(require.resolve(specifier));
  } catch {
    return undefined;
  }
}

function manifest(path) {
  try {
    return path && JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return undefined;
  }
}

function knownVersion(value) {
  if (value === undefined) return "missing";
  return typeof value === "string" && value.length <= 64 && versionFormat.test(value)
    ? value
    : "invalid";
}

function inConsumer(path) {
  if (!path) return false;
  const fromModules = relative(join(realpathSync(work), "node_modules"), path);
  return fromModules !== ".." && !fromModules.startsWith(`..${sep}`) && !isAbsolute(fromModules);
}

function showInstallLog(log) {
  if (!log || !existsSync(log)) return;
  const fd = openSync(log, "r");
  let content;
  try {
    const size = fstatSync(fd).size;
    const edge = 16 * 1024;
    function readAt(length, position) {
      const buffer = Buffer.alloc(length);
      return buffer.subarray(0, readSync(fd, buffer, 0, length, position)).toString("utf8");
    }
    content = size > edge * 2
      ? `${readAt(edge, 0)}\n[npm install output truncated]\n${readAt(edge, size - edge)}`
      : readAt(size, 0);
  } finally {
    closeSync(fd);
  }
  for (const line of content.split(/\r?\n/)) {
    if (!line) continue;
    const redacted = /(?:^|[^\w])(?:PATH|PYTHONPATH|NODE_PATH|HOME|GITHUB_\w+|RUNNER_\w+|ACTIONS_\w+|\w*(?:TOKEN|SECRET|PASSWORD|KEY)\w*)["']?\s*[:=]/i.test(line)
      ? "[REDACTED environment transcript line]"
      : line.replace(/(https?:\/\/)[^\s/@]+@/gi, "$1[REDACTED]@")
        .replace(/\b(?:npm_[A-Za-z0-9]+|(?:Bearer|Basic)\s+\S+)/gi, "[REDACTED]")
        .replace(/##\[/g, "# [");
    console.error(`npm install: ${JSON.stringify(redacted)}`);
  }
}

function installedPackage() {
  const consumerRequire = createRequire(join(work, "package.json"));
  const mainPath = resolvePath(consumerRequire, "fathomdb/package.json");
  const main = manifest(mainPath);
  const mainRequire = mainPath ? createRequire(mainPath) : consumerRequire;
  const optionalPath = expectedOptional && resolvePath(mainRequire, `${expectedOptional}/package.json`);
  const optional = manifest(optionalPath);
  const expectedPath = expectedOptional && join(
    optionalPath ? dirname(optionalPath) : join(work, "node_modules", expectedOptional),
    `fathomdb.${triple}.node`,
  );
  const observed = expectedOptional && resolvePath(mainRequire, expectedOptional);
  const resolved = resolvePath(consumerRequire, "fathomdb");
  const valid = main?.name === "fathomdb" && main.version === expectedVersion && inConsumer(mainPath)
    && inConsumer(resolved)
    && expectedOptional && main.optionalDependencies?.[expectedOptional] === expectedVersion
    && optional?.name === expectedOptional && optional.version === expectedVersion
    && inConsumer(optionalPath) && observed === expectedPath && inConsumer(observed);

  if (phase === "inspect" || !valid) {
    const libc = platform === "linux" ? (glibc ? `glibc-${glibc}` : "non-glibc") : "n/a";
    console.error(`npm smoke: requested=${expectedVersion} resolved=${knownVersion(main?.version)} platform=${platform} arch=${arch} libc=${libc}`);
    console.error(`npm smoke: optional=${expectedOptional ?? "unsupported"} expected_version=${expectedVersion} installed_version=${knownVersion(optional?.version)} expected_path=${expectedPath ?? "unsupported"} observed_native=${observed ?? "missing"}`);
  }
  if ((installExit !== undefined && installExit !== "0") || !valid) {
    showInstallLog(installLog);
    throw new Error(`npm install exit=${installExit ?? "already-installed"}; expected resolved package and platform addon are required`);
  }
  return { mainRequire, resolved, nativePath: observed };
}

async function writeAndVerifyTrace(e, DependencyTraceError) {
  await e.write([
    {
      kind: "doc",
      body: sourceBody,
      sourceId: "smoke:npm-package",
      logicalId: "source",
      provenance: {
        schemaVersion: 1,
        role: "canonical",
        artifactRevisionId: "source-r1",
        sourceVersionId: "v1",
      },
    },
    {
      kind: "fact",
      body: "slice55 npm derived",
      sourceId: "smoke:npm-package",
      logicalId: "derived",
      provenance: {
        schemaVersion: 1,
        role: "derived",
        artifactRevisionId: "derived-r1",
        sourceVersionId: "v1",
        sourceRevisionId: "source-r1",
        sourceLocator: { kind: "whole_body" },
        canonicalSourceHash: {
          algorithm: "sha256",
          digestHex: createHash("sha256").update(sourceBody).digest("hex"),
        },
      },
    },
  ]);
  await e.registerSourceDependency({
    schemaVersion: 1,
    dependencyId: "dep-1",
    sourceRevisionId: "source-r1",
    derivedRevisionId: "derived-r1",
  });
  const context = await e.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
  const trace = await e.traceDependency({
    schemaVersion: 1,
    rootRevisionId: "source-r1",
    direction: "to_dependents",
    context,
  });
  assert.equal(trace.complete, true);
  assert.equal(trace.dependencyEdges.length, 1);
  const explained = await e.search("slice55", undefined, 0, false, 0.3, 0, true);
  assert.ok(explained.explanation?.correlationId);
  assert.ok(explained.explanation.perHit.every((item) => item.structural !== undefined));
  await assert.rejects(
    e.traceDependency({
      schemaVersion: 1,
      rootRevisionId: "source-r1",
      direction: "invalid",
      context,
    }),
    (error) =>
      error instanceof DependencyTraceError &&
      error.reason === "trace_direction_invalid" &&
      error.fieldPath === "/direction",
  );
  assert.equal("checkDataPlaneIntegrity" in e, false);
  assert.equal("doctor" in e, false);
}

async function main() {
  if (!["inspect", "write", "reopen"].includes(phase) || !work
      || !versionFormat.test(expectedVersion ?? "")) {
    throw new Error("usage: registry-node.mjs inspect|write|reopen <consumer-directory> <exact-version>");
  }
  const { mainRequire, resolved, nativePath } = installedPackage();
  if (phase === "inspect") {
    console.log(nativePath);
    return;
  }
  const store = join(work, "smoke.fdb");
  assert.ok(phase !== "reopen" || existsSync(store), "persisted store is missing");
  const { DependencyTraceError, Engine, read } = await import(pathToFileURL(resolved).href);
  const engine = await Engine.open(store, { useDefaultEmbedder: false });
  try {
    assert.ok(mainRequire.cache[nativePath], `the expected native module was not loaded: ${nativePath}`);
    if (phase === "write") {
      await writeAndVerifyTrace(engine, DependencyTraceError);
    }
    const found = await read.get(engine, "source");
    assert.equal(found?.body, sourceBody, "persisted body is absent or wrong");
    assert.equal(await read.get(engine, "registry-smoke:missing"), null);
  } finally {
    await engine.close();
  }
  console.log(`npm ${phase}: pid=${process.pid} exact body and missing record verified; native=${nativePath}`);
}

try {
  await main();
} catch (error) {
  console.error(`npm smoke: ${error.message}`);
  process.exitCode = 1;
}
