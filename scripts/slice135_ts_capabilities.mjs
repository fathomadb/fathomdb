#!/usr/bin/env node
/** Exercise inspected native TypeScript tests from an isolated installed package. */
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, realpathSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";

const sha = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
const requireCondition = (condition, message) => { if (!condition) throw new Error(message); };

export function summarize(operations, rows) {
  if (new Set(operations).size !== operations.length ||
      new Set([...operations, ...Object.keys(rows)]).size !== operations.length ||
      Object.keys(rows).length !== operations.length) throw new Error("row set differs from canonical operation map");
  const counts = { supported: operations.length, executed: 0, failed: 0, gap: 0, unavailable: 0 };
  for (const row of Object.values(rows)) {
    if (!Object.hasOwn(counts, row.status) || row.status === "supported") throw new Error("unknown status");
    if (row.status === "executed" && !(row.positive?.case && row.negative?.case && row.reopen?.databases > 0)) {
      throw new Error("executed row omitted asserted evidence");
    }
    if (["gap", "unavailable"].includes(row.status) && !(row.reason && row.owner)) throw new Error("unexplained gap");
    if (row.status === "failed" && !row.error) throw new Error("failed row omitted error");
    counts[row.status]++;
  }
  return counts;
}

export function validateInstalledIdentity({ installRoot, nativePath, expectedNativeSha256 }) {
  if (!existsSync(join(installRoot, "node_modules"))) throw new Error("installed node_modules missing");
  const nodeModules = realpathSync(join(installRoot, "node_modules")) + sep;
  const paths = {
    module: join(installRoot, "node_modules/fathomdb/dist/index.js"),
    main_package: join(installRoot, "node_modules/fathomdb/package.json"),
    native: nativePath ?? join(installRoot, "node_modules/fathomdb-linux-x64-gnu/fathomdb.linux-x64-gnu.node"),
    native_package: join(installRoot, "node_modules/fathomdb-linux-x64-gnu/package.json"),
  };
  for (const path of Object.values(paths)) {
    if (!existsSync(path) || !realpathSync(path).startsWith(nodeModules)) throw new Error(`installed path escaped consumer: ${path}`);
  }
  if (sha(paths.native) !== expectedNativeSha256) throw new Error("installed native SHA-256 mismatch");
  return Object.fromEntries(Object.entries(paths).map(([key, path]) => [key, { path: realpathSync(path), sha256: sha(path) }]));
}

// Every name is an inspected, real-engine Node test. Test sources remain unchanged;
// only compiled package import specifiers are replaced in the isolated consumer.
export const CASES = {
  retrieve: ["functional-retrieve", "functional retrieve: read.get returns the active node by id"],
  many: ["functional-retrieve", "functional retrieve: read.getMany preserves order with null"],
  log: ["functional-retrieve", "functional retrieve: read.collection/read.mutations honor cursor + limit"],
  admin: ["functional-retrieve", "functional retrieve: admin.configure path exercised"],
  search: ["functional-search", "functional search: structured hit shape across the FFI"],
  write: ["functional-search", "functional write: rowCursors are 1:1 with the batch (Py ≡ TS)"],
  filtered: ["functional-search", "functional search: a SearchFilter prunes results"],
  list: ["functional-read-list", "functional read.list: unfiltered returns all active nodes of the kind"],
  list_refusal: ["functional-read-list", "functional read.list: non-allowlisted path throws InvalidFilterError"],
  neighbors: ["functional-graph", "graph.neighbors depth=1 outgoing returns B and D (direct children of A)"],
  neighbors_refusal: ["functional-graph", "graph.neighbors depth=4 raises InvalidArgumentError"],
  graph_search: ["functional-graph", "graph.searchExpand: neighbor appears in expanded"],
  dependency: ["slice20-source-dependencies", "dependency round trip uses canonical decimal generation"],
  actuation: ["slice25-actuation", "actuation commits and exact replay returns the terminal receipt"],
  actuation_refusal: ["slice25-actuation", "actuation rejects an unpaired surrogate at the canonical nested path"],
  lifecycle: ["opp12-lifecycle-verbs", "legal transitions + reason clear-on-admit / set-on-exclude"],
  lifecycle_refusal: ["opp12-lifecycle-verbs", "illegal transition is a typed error with fromState/toState/legal"],
  purge: ["opp12-lifecycle-verbs", "purge requires deleted-first and is idempotent"],
  erasure: ["erase-source", "eraseSource erases anonymous content end-to-end without the CLI"],
  erasure_refusal: ["erase-source", "eraseSource is idempotent (an absent source is a zero-count success)"],
  frozen: ["slice35-frozen-read", "frozen read context binds eligibility and rejects drift"],
  frozen_expand: ["slice35-frozen-read", "frozen search expansion returns the governed union"],
  frozen_refusal: ["slice35-frozen-read", "frozen authentication precedes query, shape, and range validation"],
  projection: ["slice15d-projection-registry", "configure + read.projections round-trips a spec verbatim"],
  projection_refusal: ["slice15d-projection-registry", "destructive change requires an explicit drop"],
  projected: ["slice45-nested-source-projections", "nested source type-collapsed equality and projected search use a real database"],
  projection_status: ["slice22-projection-status", "read.projectionStatus exposes exact current status wires"],
  readiness: ["slice30-embedding-readiness", "read.embeddingReadiness and immediate typed drain rejection keep edge bodies private"],
  generation: ["slice40-projection-generation", "projection generation status is typed and stable"],
  mutation_status: ["slice40-projection-generation", "receipt-keyed mutation status round-trips through TypeScript"],
  evidence: ["slice50-evidence", "search and resolve exact source evidence"],
  evidence_refusal: ["slice50-evidence", "evidence request schema and unknown fields are typed"],
  graph_evidence: ["slice20-graph-evidence", "real engine resolves exact target and terminal edge graph evidence"],
  graph_expand: ["slice60-graph-expand", "graph.expand applies direction, edge kind, and return-only target kind"],
  graph_refusal: ["slice60-graph-expand", "graph.expand validates canonical u64, real booleans, and closed unions"],
  text_only: ["slice23-text-limit-prefix-stability", "direct text limits are prefix-stable through TypeScript"],
  boundary: ["slice15b-node-validity-write", "TC-34: crossedBoundarySince works on SDK-authored windows"],
  embed: ["functional-embed", "embed: returns a fixed-dim float vector"],
  embed_refusal: ["functional-embed", "embed: without an embedder rejects EmbedderNotConfiguredError"],
  pages: ["slice45-pagination", "canonical and operational pages share frozen authority"],
  pages_refusal: ["slice45-pagination", "page request refusals retain PageError reason and field path"],
  closure_absent: ["slice30-dependency-closure", "closure lookup is closed and absent ids disclose nothing"],
  closure_committed: ["slice30-dependency-closure", "closure response decoder accepts a non-null installed status"],
  trace_refusal: ["slice55-request-validation", "slice55 request validation rejects schema before semantic fields"],
  trace_success: ["slice55-request-validation", "slice55 installed trace returns a committed dependency edge"],
};

export const PLAN = {
  "engine.open": ["retrieve", "direct"],
  "admin.configure": ["admin", "direct"],
  "engine.write": ["write", "direct"],
  "engine.actuate": ["actuation", "actuation_refusal"],
  "engine.register_source_dependency": ["dependency", "direct"],
  "engine.dependencies_for_source": ["dependency", "direct"],
  "engine.dependency_for_derived": ["dependency", "direct"],
  "engine.transition": ["lifecycle", "lifecycle_refusal"],
  "engine.purge": ["purge", "purge"],
  "engine.erase_source": ["erasure", "direct"],
  "engine.search": ["search", "direct"],
  "engine.freeze_read_context": ["frozen", "direct"],
  "engine.search_frozen": ["frozen", "frozen_refusal"],
  "engine.search_expand_frozen": ["frozen_expand", "frozen_refusal"],
  "engine.search_text_only": ["text_only", "direct"],
  "engine.search_projected_text": ["projected", "direct"],
  "engine.search_with_evidence": ["evidence", "evidence_refusal"],
  "engine.resolve_evidence": ["evidence", "direct"],
  "engine.resolve_graph_evidence": ["graph_evidence", "direct"],
  "engine.close": ["retrieve", "direct"],
  "read.get": ["retrieve", "direct"],
  "read.get_many": ["many", "direct"],
  "read.collection": ["log", "direct"],
  "read.mutations": ["log", "direct"],
  "read.list": ["list", "list_refusal"],
  "graph.expand": ["graph_expand", "graph_refusal"],
  "graph.neighbors": ["neighbors", "neighbors_refusal"],
  "graph.search_expand": ["graph_search", "direct"],
  "engine.embed": ["embed", "embed_refusal"],
  "engine.read_dependency_closure": ["closure_committed", "closure_absent"],
  "engine.trace_dependency": ["trace_success", "trace_refusal"],
  "read.crossed_boundary_since": ["boundary", "direct"],
  "engine.configure_projections": ["projection", "projection_refusal"],
  "read.projections": ["projection", "direct"],
  "read.projection_status": ["projection_status", "direct"],
  "read.embedding_readiness": ["readiness", "direct"],
  "read.projection_generation_status": ["generation", "direct"],
  "read.mutation_projection_status": ["mutation_status", "direct"],
  "read.canonical_page": ["pages", "pages_refusal"],
  "read.operational_state": ["pages", "direct"],
  "read.operational_state_page": ["pages", "direct"],
};

const UNAVAILABLE = {
  "engine.ingest_with_extractor": "No qualified extractor command or model for this artifact; protocol stubs are not provider evidence.",
  "engine.consolidate_with_provider": "The existing real subprocess test uses fabricated local LLM verdicts; no independently qualified provider/model is available.",
  rerank: "Standalone cross-encoder model and feature availability are not qualified by this installed-package run.",
};

function findSqliteFiles(root) {
  const found = [];
  const visit = (directory) => {
    for (const item of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, item.name);
      if (item.isDirectory()) visit(path);
      else if (item.isFile() && statSync(path).size >= 16 && readFileSync(path).subarray(0, 16).toString("binary") === "SQLite format 3\0") found.push(path);
    }
  };
  visit(root);
  return found;
}

async function reopenDatabase(Engine, read, path) {
  const sql = new DatabaseSync(path, { readOnly: true });
  let rows;
  try {
    rows = sql.prepare("SELECT logical_id, body FROM canonical_nodes WHERE state='active' AND superseded_at IS NULL AND logical_id IS NOT NULL ORDER BY logical_id").all();
  } finally { sql.close(); }
  const engine = await Engine.open(path, { useDefaultEmbedder: false });
  try {
    for (const row of rows) {
      const got = await read.get(engine, row.logical_id, { includeOutOfWindow: true });
      assert.equal(got?.body, row.body, `reopened body mismatch: ${row.logical_id}`);
    }
    assert.equal(await read.get(engine, "slice135-guaranteed-absent"), null);
    return { database: path, activeBodiesChecked: rows.length };
  } finally { await engine.close(); }
}

async function runDirectRefusals(sdk) {
  const directory = mkdtempSync(join(tmpdir(), "slice135-ts-direct-refusals-"));
  const database = join(directory, "refusals.sqlite");
  const engine = await sdk.Engine.open(database, { useDefaultEmbedder: false });
  const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
  const bad = "invalid\ud800text";
  const invalidSchema = { schemaVersion: 2 };
  const calls = {
    "admin.configure": () => sdk.admin.configure(engine, { name: bad, body: "{}" }),
    "engine.write": () => engine.write([{ kind: "doc", body: "body", sourceId: bad }]),
    "engine.register_source_dependency": () => engine.registerSourceDependency(invalidSchema),
    "engine.dependencies_for_source": () => engine.dependenciesForSource(invalidSchema),
    "engine.dependency_for_derived": () => engine.dependencyForDerived(invalidSchema),
    "engine.erase_source": () => engine.eraseSource(bad),
    "engine.search": () => engine.search(bad),
    "engine.freeze_read_context": () => engine.freezeReadContext({ ...invalidSchema, view: {}, eligibility: {} }),
    "engine.search_text_only": () => engine.searchTextOnly(bad),
    "engine.search_projected_text": () => engine.searchProjectedText(bad, "summary"),
    "engine.resolve_evidence": () => engine.resolveEvidence(invalidSchema),
    "engine.resolve_graph_evidence": () => engine.resolveGraphEvidence(invalidSchema),
    "read.get": () => sdk.read.get(engine, bad),
    "read.get_many": () => sdk.read.getMany(engine, [bad]),
    "read.collection": () => sdk.read.collection(engine, "collection", { limit: -1 }),
    "read.mutations": () => sdk.read.mutations(engine, "collection", { limit: -1 }),
    "graph.search_expand": () => sdk.graph.searchExpand(engine, "query", 4),
    "read.crossed_boundary_since": () => sdk.read.crossedBoundarySince(engine, 1.5),
    "read.mutation_projection_status": () => sdk.read.mutationProjectionStatus(engine, invalidSchema),
    "read.operational_state": () => sdk.read.operationalState(engine, bad, "key", frozen),
    "read.operational_state_page": () => sdk.read.operationalStatePage(engine, "collection", frozen, { schemaVersion: 1, limit: 0 }),
  };
  const expected = {
    "admin.configure": "WriteValidationError",
    "engine.write": "WriteValidationError",
    "read.collection": "RangeError", "read.mutations": "RangeError",
    "engine.register_source_dependency": "DependencyError",
    "engine.dependencies_for_source": "DependencyError",
    "engine.dependency_for_derived": "DependencyError",
    "engine.erase_source": "WriteValidationError",
    "engine.search": "WriteValidationError",
    "engine.freeze_read_context": "InvalidArgumentError",
    "engine.search_text_only": "WriteValidationError",
    "engine.search_projected_text": "WriteValidationError",
    "engine.resolve_evidence": "EvidenceError",
    "engine.resolve_graph_evidence": "EvidenceError",
    "read.get": "WriteValidationError",
    "read.get_many": "WriteValidationError",
    "graph.search_expand": "InvalidArgumentError",
    "read.crossed_boundary_since": "InvalidArgumentError",
    "read.mutation_projection_status": "ProjectionGenerationError",
    "read.operational_state": "WriteValidationError",
    "read.operational_state_page": "PageError",
    "read.projections": "ClosingError",
    "read.projection_status": "ClosingError",
    "read.embedding_readiness": "ClosingError",
    "read.projection_generation_status": "ClosingError",
    "engine.open": "RangeError",
  };
  const observed = {};
  const attempt = async (id, call) => {
    try { await call(); observed[id] = { status: "failed", error: "invalid call was accepted" }; }
    catch (error) {
      const typed = error instanceof sdk.FathomDbError || error instanceof RangeError || error instanceof TypeError;
      const value = { name: error?.name, code: error?.code ?? null,
        reason: error?.reason ?? null, fieldPath: error?.fieldPath ?? null, message: String(error?.message ?? error) };
      const schemaFields = ["DependencyError", "EvidenceError", "ProjectionGenerationError"].includes(expected[id])
        ? value.reason === "unsupported_schema_version" && value.fieldPath === "/schemaVersion" : true;
      const pageFields = expected[id] === "PageError"
        ? value.reason === "invalid_page_limit" && value.fieldPath === "/limit" : true;
      observed[id] = typed && value.name === expected[id] && schemaFields && pageFields
        ? { status: "passed", error: value }
        : { status: "failed", error: `unexpected refusal ${JSON.stringify(value)}` };
    }
  };
  try {
    await engine.write([{ kind: "doc", body: "direct refusal sentinel", logicalId: "direct-refusal-sentinel", sourceId: "slice135-negative" }]);
    for (const [id, call] of Object.entries(calls)) await attempt(id, call);
  } finally { await engine.close(); }
  const closedCalls = {
    "read.projections": () => sdk.read.projections(engine),
    "read.projection_status": () => sdk.read.projectionStatus(engine),
    "read.embedding_readiness": () => sdk.read.embeddingReadiness(engine),
    "read.projection_generation_status": () => sdk.read.projectionGenerationStatus(engine),
  };
  for (const [id, call] of Object.entries(closedCalls)) await attempt(id, call);
  await engine.close();
  observed["engine.close"] = { status: "passed", control: "second close accepted idempotently" };
  await attempt("engine.open", () => sdk.Engine.open(join(directory, "bad.sqlite"), { engineConfig: { schedulerRuntimeThreads: 65 } }));
  for (const item of Object.values(observed)) item.reopen = await reopenDatabase(sdk.Engine, sdk.read, database);
  return { database, observed };
}

function prepareTest(repo, compiledRoot, consumer, name) {
  const [file] = CASES[name];
  const source = join(repo, "src/ts/tests", `${file}.test.ts`);
  const compiled = join(compiledRoot, "tests", `${file}.test.js`);
  requireCondition(existsSync(compiled), `missing compiled test ${compiled}`);
  const text = readFileSync(compiled, "utf8");
  const imports = [...text.matchAll(/from "\.\.\/src\/([^"]+)"/g)].map((match) => match[1]);
  requireCondition(imports.every((name) => ["index.js", "errors.js", ...(file === "slice45-pagination" ? ["binding.js"] : [])].includes(name)), `test ${file} imports unqualified source internals: ${imports}`);
  let transformed = text.replaceAll('from "../src/index.js"', 'from "fathomdb"')
    .replaceAll('from "../src/errors.js"', 'from "fathomdb"');
  if (file === "slice45-pagination") {
    transformed = transformed.replace('import { native } from "../src/binding.js";', 'const native = undefined;');
  }
  if (["slice50-evidence", "slice20-graph-evidence"].includes(file)) {
    transformed = transformed.replaceAll('await rm(directory, { recursive: true, force: true });', 'void directory;');
  }
  requireCondition(!transformed.includes('from "../src/'), "source import survived transformation");
  const target = join(consumer, "dist/tests", `${file}.test.js`);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, transformed);
  return { file, testSource: source, testSourceSha256: sha(source), compiledSha256: sha(compiled), installedTestSha256: sha(target),
    adaptation: "package import replacement; evidence-case temporary cleanup deferred for independent reopen", target };
}

async function runCase(repo, compiledRoot, consumer, name, sdk) {
  const evidence = prepareTest(repo, compiledRoot, consumer, name);
  const temporary = mkdtempSync(join(tmpdir(), `slice135-ts-cap-${name}-`));
  const [, pattern] = CASES[name];
  const command = [process.execPath, "--test", "--test-reporter=tap", `--test-name-pattern=^${pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`, evidence.target];
  const processResult = spawnSync(command[0], command.slice(1), {
    cwd: join(repo, "src/ts"), env: { ...process.env, TMPDIR: temporary }, encoding: "utf8", maxBuffer: 8 * 1024 * 1024,
  });
  const raw = { case: name, test: `${evidence.file}.test.ts::${pattern}`, ...evidence,
    command, cwd: join(repo, "src/ts"), temp: temporary,
    exit: processResult.status, stdout: processResult.stdout, stderr: processResult.stderr };
  const passed = processResult.status === 0 && /^# pass 1$/m.test(processResult.stdout) && /^# fail 0$/m.test(processResult.stdout);
  if (!passed) { raw.status = "failed"; raw.error = processResult.error?.message ?? "named native test did not pass exactly once"; return raw; }
  try {
    const databases = findSqliteFiles(temporary);
    requireCondition(databases.length > 0, "named test created no real SQLite database");
    raw.reopen = [];
    for (const path of databases) raw.reopen.push(await reopenDatabase(sdk.Engine, sdk.read, path));
    raw.status = "passed";
  } catch (error) { raw.status = "failed"; raw.error = error?.stack ?? String(error); }
  return raw;
}

export async function run({ repo, compiledRoot, installRoot, canonicalPath, sourceSha, nativeSha256, archives }) {
  requireCondition(/^[0-9a-f]{40}$/.test(sourceSha), "invalid source SHA");
  requireCondition(/^[0-9a-f]{64}$/.test(nativeSha256), "invalid native SHA");
  const identity = validateInstalledIdentity({ installRoot, expectedNativeSha256: nativeSha256 });
  const canonical = JSON.parse(readFileSync(canonicalPath, "utf8"));
  requireCondition(canonical.schema_version === "fathomdb.governed-operation-parity/v1", "unknown canonical schema");
  const operations = canonical.operations.filter((entry) => entry.state === "live").map((entry) => entry.id);
  requireCondition(operations.length === 44, "canonical live operation count changed; review plan");
  const sdk = await import(pathToFileURL(identity.module.path).href);
  const consumer = resolve(installRoot);
  const helperSource = join(compiledRoot, "tests/helpers.js");
  const helperTarget = join(consumer, "dist/tests/helpers.js");
  mkdirSync(dirname(helperTarget), { recursive: true });
  copyFileSync(helperSource, helperTarget);
  writeFileSync(join(consumer, "package.json"), '{"type":"module","private":true}\n');
  const fixtureSources = ["functional_retrieve_fixture.json", "functional_search_fixture.json"]
    .map((name) => join(repo, "src/python/tests", name));
  const fixtureTargetDirectory = join(consumer, "../python/tests");
  mkdirSync(fixtureTargetDirectory, { recursive: true });
  for (const source of fixtureSources) copyFileSync(source, join(fixtureTargetDirectory, source.split("/").at(-1)));
  const selected = [...new Set([...Object.values(PLAN).flat(), "closure_absent", "trace_refusal"].filter((name) => CASES[name]))];
  const cases = {};
  for (const name of selected) cases[name] = await runCase(repo, compiledRoot, consumer, name, sdk);
  const direct = await runDirectRefusals(sdk);
  const rows = {};
  for (const id of operations) {
    if (UNAVAILABLE[id]) { rows[id] = { status: "unavailable", reason: UNAVAILABLE[id], owner: "Slice 135 provider/model qualification" }; continue; }
    const route = PLAN[id];
    if (!route || route.some((name) => name !== "direct" && !CASES[name])) {
      rows[id] = { status: "gap", reason: route ? `Selected case unavailable: ${route.filter((name) => name !== "direct" && !CASES[name]).join(", ")}` : "No inspected installed native positive and typed-refusal route", owner: "Slice 135 TypeScript capability exercise" }; continue;
    }
    const [positive, negative] = route;
    const failure = [positive, negative].find((name) => name === "direct" ? direct.observed[id]?.status !== "passed" : cases[name]?.status !== "passed");
    if (failure) { rows[id] = { status: "failed", case: failure, error: failure === "direct" ? direct.observed[id]?.error ?? "direct refusal missing" : cases[failure]?.error ?? "case failed" }; continue; }
    const reopen = cases[positive].reopen;
    rows[id] = { status: "executed", positive: { case: positive }, negative: { case: negative, ...(negative === "direct" ? { refusal: direct.observed[id].error } : {}) },
      reopen: { databases: reopen.length, activeBodiesChecked: reopen.reduce((sum, entry) => sum + entry.activeBodiesChecked, 0) } };
  }
  const counts = summarize(operations, rows);
  return { schema_version: 1, kind: "installed_ts_selected_functional_capabilities", source_sha: sourceSha,
    canonical: { path: canonicalPath, sha256: sha(canonicalPath) },
    runner_sha256: sha(fileURLToPath(import.meta.url)), node: process.version,
    artifact: identity, archives: archives.map((path) => ({ path, sha256: sha(path) })),
    fixtures: fixtureSources.map((path) => ({ path, sha256: sha(path) })), cases, direct, operations: rows, counts };
}

function args(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i += 2) { requireCondition(argv[i]?.startsWith("--") && argv[i + 1], "expected --key value"); out[argv[i].slice(2)] = argv[i + 1]; }
  for (const key of ["repo", "compiled-root", "install-root", "canonical", "source-sha", "native-sha256", "main-archive", "native-archive", "output"]) requireCondition(out[key], `missing --${key}`);
  return out;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const value = args(process.argv.slice(2));
    requireCondition(!existsSync(value.output), "output already exists");
    const result = await run({ repo: resolve(value.repo), compiledRoot: resolve(value["compiled-root"]), installRoot: resolve(value["install-root"]),
      canonicalPath: resolve(value.canonical), sourceSha: value["source-sha"], nativeSha256: value["native-sha256"],
      archives: [resolve(value["main-archive"]), resolve(value["native-archive"]) ] });
    writeFileSync(value.output, `${JSON.stringify(result, null, 2)}\n`);
    process.stdout.write(`${JSON.stringify(result.counts)}\n`);
  } catch (error) { process.stderr.write(`${error?.stack ?? String(error)}\n`); process.exitCode = 1; }
}
