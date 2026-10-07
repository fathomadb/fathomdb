#!/usr/bin/env node
/** Run a source-bound S01 pilot through an installed TypeScript package. */

import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const QUERIES = {
  text: "brightblue",
  vector: "boat schedule across water",
  hybrid: "harbor ferry",
};
const SOURCE_ID = "slice135-s01-python";
const MODEL = "fathomdb-bge-small-en-v1.5 default embedder";

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function makeCorpus(size) {
  if (size !== 32 && size !== 256) {
    throw new RangeError("S01 corpus size must be 32 or 256");
  }
  const summaries = [
    ["A", "harbor lantern ferry timetable brightblue"],
    ["B", "orchard apple harvest calendar redgreen"],
  ];
  for (let index = 0; index < size - 2; index += 1) {
    summaries.push([
      `F${String(index).padStart(4, "0")}`,
      `neutral archive ledger inventory entry ${index} papers`,
    ]);
  }
  return summaries.map(([logicalId, summary]) => ({
    body: JSON.stringify({ summary }).replace('":"', '": "'),
    kind: "doc",
    logical_id: logicalId,
    source_id: SOURCE_ID,
  }));
}

export function assertResult(kind, result, eligibleIds) {
  const hits = result?.results;
  if (!Array.isArray(hits) || hits.length === 0) {
    throw new Error("empty result");
  }
  if (hits.some((hit) => hit?.id?.space !== "logical" || !eligibleIds.has(hit.id.value))) {
    throw new Error("unknown id or id space");
  }
  if (kind === "text") {
    if (hits.length !== 1 || hits[0].id.value !== "A" || hits[0].branch !== "text") {
      throw new Error("text result is not the seeded exact match");
    }
  } else if (kind === "vector") {
    if (!hits.some((hit) => hit.branch === "vector")) {
      throw new Error("vector branch absent");
    }
  } else if (kind === "hybrid") {
    if (!hits.some((hit) => hit.id.value === "A")) {
      throw new Error("hybrid result lost lexical anchor");
    }
    if (hits.some((hit) => hit.branch !== "text" && hit.branch !== "vector")) {
      throw new Error("hybrid result has an unexpected branch");
    }
  } else {
    throw new Error(`unknown query kind ${kind}`);
  }
}

export function nearestRank(samples, quantile) {
  if (samples.length === 0 || !(quantile > 0 && quantile <= 1)) {
    throw new RangeError("nearest-rank input is empty or quantile invalid");
  }
  const ordered = [...samples].sort((left, right) => left - right);
  return ordered[Math.ceil(quantile * ordered.length) - 1];
}

async function timedQuery(engine, kind, eligibleIds) {
  const start = process.hrtime.bigint();
  let result;
  try {
    result = kind === "text"
      ? await engine.searchTextOnly(QUERIES[kind])
      : await engine.search(QUERIES[kind]);
  } catch (error) {
    return {
      latency_ns: Number(process.hrtime.bigint() - start),
      semantic_ok: false,
      reason: `query threw ${error?.name ?? "Error"}: ${error?.message ?? String(error)}`,
    };
  }
  const hits = Array.isArray(result?.results) ? result.results : [];
  const attempt = {
    latency_ns: Number(process.hrtime.bigint() - start),
    ids: hits.map((hit) => hit?.id?.value ?? null),
    branches: hits.map((hit) => hit?.branch ?? null),
  };
  try {
    assertResult(kind, result, eligibleIds);
    attempt.semantic_ok = true;
  } catch (error) {
    attempt.semantic_ok = false;
    attempt.reason = error.message;
  }
  return attempt;
}

function artifactIdentity(installRoot, expectedNativeSha256) {
  const modulePath = resolve(installRoot, "node_modules/fathomdb/dist/index.js");
  const packagePath = resolve(installRoot, "node_modules/fathomdb/package.json");
  const nativePath = resolve(
    installRoot,
    "node_modules/fathomdb-linux-x64-gnu/fathomdb.linux-x64-gnu.node",
  );
  for (const path of [modulePath, packagePath, nativePath]) {
    if (!existsSync(path)) {
      throw new Error(`installed artifact missing: ${path}`);
    }
  }
  const nativeSha256 = sha256(readFileSync(nativePath));
  if (nativeSha256 !== expectedNativeSha256) {
    throw new Error("installed native artifact SHA-256 mismatch");
  }
  return {
    module_path: modulePath,
    module_sha256: sha256(readFileSync(modulePath)),
    native_path: nativePath,
    native_sha256: nativeSha256,
    package_path: packagePath,
    package_sha256: sha256(readFileSync(packagePath)),
    package_version: JSON.parse(readFileSync(packagePath, "utf8")).version,
    node_executable: process.execPath,
    node_version: process.version,
  };
}

export async function runPilot({ rows, samples, installRoot, sourceSha, expectedNativeSha256 }) {
  if (rows !== 32 && rows !== 256) {
    throw new RangeError("S01 corpus size must be 32 or 256");
  }
  if (!Number.isInteger(samples) || samples < 10) {
    throw new RangeError("S01 pilot requires at least ten warm samples");
  }
  if (!/^[0-9a-f]{40}$/.test(sourceSha) || !/^[0-9a-f]{64}$/.test(expectedNativeSha256)) {
    throw new Error("source or native artifact hash malformed");
  }
  const artifact = artifactIdentity(installRoot, expectedNativeSha256);
  const { Engine, read } = await import(pathToFileURL(artifact.module_path).href);
  const corpus = makeCorpus(rows);
  const eligibleIds = new Set(corpus.map((item) => item.logical_id));
  const output = {
    schema_version: 1,
    status: "UNFROZEN_TS_S01_PILOT",
    started_utc: new Date().toISOString(),
    source_sha: sourceSha,
    artifact,
    corpus_size: rows,
    corpus_sha256: sha256(JSON.stringify(corpus)),
    model: MODEL,
    queries: QUERIES,
    warm_samples_per_cell: samples,
    cells: {},
  };

  const temporary = mkdtempSync(join(tmpdir(), "slice135-ts-s01-"));
  const database = join(temporary, "s01.sqlite");
  try {
    let engine = await Engine.open(database, { useDefaultEmbedder: true });
    try {
      if (engine.openReport().defaultEmbedder?.name !== "fathomdb-bge-small-en-v1.5") {
        throw new Error("default embedder identity changed");
      }
      await engine.write(corpus);
      await engine.configureProjections([
        { name: "summary", roles: ["searchable"], fts: true, vector: true },
      ]);
      await engine.drain(120_000);
      const projection = (await read.projections(engine)).find((item) => item.name === "summary");
      if (projection?.vectorDenseReadiness !== "ready") {
        throw new Error("vector projection is not ready");
      }
      for (const item of corpus.slice(0, 2)) {
        if ((await read.get(engine, item.logical_id))?.body !== item.body) {
          throw new Error(`anchor ${item.logical_id} changed before query`);
        }
      }
    } finally {
      await engine.close();
    }

    for (const kind of Object.keys(QUERIES)) {
      engine = await Engine.open(database, { useDefaultEmbedder: true });
      try {
        if (kind === "vector" && (await engine.searchTextOnly(QUERIES[kind])).results.length) {
          throw new Error("vector query has lexical matches");
        }
        if (kind === "hybrid" && !(await engine.searchTextOnly(QUERIES[kind])).results.length) {
          throw new Error("hybrid query has no lexical arm");
        }
        output.cells[kind] = {
          session_cold: await timedQuery(engine, kind, eligibleIds),
          warmup: await timedQuery(engine, kind, eligibleIds),
          warm: [],
        };
        for (let index = 0; index < samples; index += 1) {
          output.cells[kind].warm.push(await timedQuery(engine, kind, eligibleIds));
        }
      } finally {
        await engine.close();
      }
    }
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
  output.finished_utc = new Date().toISOString();
  output.semantic_failures = Object.values(output.cells).reduce(
    (total, cell) => total + [cell.session_cold, cell.warmup, ...cell.warm]
      .filter((attempt) => attempt.semantic_ok !== true).length,
    0,
  );
  return output;
}

function parseArguments(argv) {
  const argumentsByName = {};
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index];
    if (!name?.startsWith("--") || argv[index + 1] === undefined) {
      throw new Error("arguments must be --name value pairs");
    }
    argumentsByName[name.slice(2)] = argv[index + 1];
  }
  const required = ["rows", "samples", "install-root", "source-sha", "expected-native-sha256", "output"];
  if (required.some((name) => argumentsByName[name] === undefined)) {
    throw new Error(`required arguments: ${required.join(", ")}`);
  }
  return argumentsByName;
}

async function main() {
  const argumentsByName = parseArguments(process.argv.slice(2));
  const output = await runPilot({
    rows: Number(argumentsByName.rows),
    samples: Number(argumentsByName.samples),
    installRoot: argumentsByName["install-root"],
    sourceSha: argumentsByName["source-sha"],
    expectedNativeSha256: argumentsByName["expected-native-sha256"],
  });
  writeFileSync(resolve(argumentsByName.output), `${JSON.stringify(output, null, 2)}\n`);
  if (output.semantic_failures !== 0) {
    throw new Error(`${output.semantic_failures} semantic failures in S01 output`);
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  main().catch((error) => {
    process.stderr.write(`${error?.stack ?? String(error)}\n`);
    process.exitCode = 1;
  });
}
