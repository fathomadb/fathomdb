#!/usr/bin/env node
/** Exercise one installed TypeScript S02 shared-engine contention sequence. */

import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { DatabaseSync } from "node:sqlite";

import { makeCorpus } from "./slice135_ts_s01.mjs";
import { GRAPH_SOURCE, makeGraphRecords, SOURCE_BODY } from "./slice135_ts_s02.mjs";

const CONTENDED_SOURCE = "slice135-contended";
const CORPUS_SOURCE = "slice135-s01-python";
const WRITER_IDS = Array.from({ length: 8 }, (_, index) => `contend-${String(index).padStart(2, "0")}`);

function sha(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function requireCondition(condition, reason) {
  if (!condition) throw new Error(reason);
}

function artifactIdentity(installRoot, expectedNativeSha256) {
  const modulePath = resolve(installRoot, "node_modules/fathomdb/dist/index.js");
  const packagePath = resolve(installRoot, "node_modules/fathomdb/package.json");
  const nativePath = resolve(
    installRoot, "node_modules/fathomdb-linux-x64-gnu/fathomdb.linux-x64-gnu.node",
  );
  for (const path of [modulePath, packagePath, nativePath]) {
    requireCondition(existsSync(path), `installed artifact missing: ${path}`);
  }
  const nativeSha256 = sha(readFileSync(nativePath));
  requireCondition(nativeSha256 === expectedNativeSha256, "native module differs from pinned archive");
  return {
    module_path: modulePath,
    module_sha256: sha(readFileSync(modulePath)),
    package_sha256: sha(readFileSync(packagePath)),
    native_sha256: nativeSha256,
    node_version: process.version,
  };
}

function clock() {
  const origin = process.hrtime.bigint();
  return () => Number(process.hrtime.bigint() - origin);
}

function databaseState(database) {
  const connection = new DatabaseSync(database, { readOnly: true });
  try {
    const count = (table, source) => connection.prepare(
      `SELECT count(*) AS n FROM ${table} WHERE source_id = ?`,
    ).get(source).n;
    return {
      corpus_nodes: count("canonical_nodes", CORPUS_SOURCE),
      graph_nodes: count("canonical_nodes", GRAPH_SOURCE),
      graph_edges: count("canonical_edges", GRAPH_SOURCE),
      contended_nodes: count("canonical_nodes", CONTENDED_SOURCE),
      integrity_check: connection.prepare("PRAGMA integrity_check").get().integrity_check,
    };
  } finally {
    connection.close();
  }
}

async function sdkState(engine, read, graph, anchorBody) {
  return {
    anchor_retained: (await read.get(engine, "A"))?.body === anchorBody,
    erased_writer_absent: (await read.get(engine, WRITER_IDS[0])) === null,
    erased_graph_absent: (await read.get(engine, "s02-root")) === null,
    evidence_query_empty: (await engine.searchTextOnly("slice135s02evidenceneedle"))
      .results.length === 0,
    graph_empty: (await graph.neighbors(engine, "s02-root", 1)).length === 0,
  };
}

export function validateTimedState(observed) {
  const expectedSdk = {
    anchor_retained: true, erased_writer_absent: true, erased_graph_absent: true,
    evidence_query_empty: true, graph_empty: true,
  };
  for (const key of ["before_reopen", "after_reopen"]) {
    requireCondition(JSON.stringify(observed[key]) === JSON.stringify(expectedSdk),
      `${key} timer boundary or SDK state changed`);
  }
  const expectedDatabase = {
    corpus_nodes: 32, graph_nodes: 0, graph_edges: 0,
    contended_nodes: 0, integrity_check: "ok",
  };
  requireCondition(JSON.stringify(observed.final_state) === JSON.stringify(expectedDatabase),
    "direct SQLite state changed");
  requireCondition(Number.isSafeInteger(observed.elapsed_ns) && observed.elapsed_ns > 0
    && observed.elapsed_ns < 60_000_000_000,
  "product timer invalid");
  requireCondition(Number.isSafeInteger(observed.verification_ns)
    && observed.verification_ns > 0,
  "verification timer invalid");
  return true;
}

export function overlapCount(writer, reader) {
  return reader.filter((readCall) => writer.some((writeCall) =>
    writeCall.start_ns < readCall.end_ns && writeCall.end_ns > readCall.start_ns,
  )).length;
}

export function validateContention({ writer, reader, claimedOverlap, expectedCalls = 8 }) {
  requireCondition(writer.length === expectedCalls && reader.length === expectedCalls, "call count");
  for (const calls of [writer, reader]) {
    requireCondition(calls.every((call) =>
      Number.isSafeInteger(call.start_ns) && Number.isSafeInteger(call.end_ns)
      && call.start_ns < call.end_ns,
    ), "call interval invalid");
  }
  requireCondition(writer.every((call, index) =>
    call.logical_id === `contend-${String(index).padStart(2, "0")}`,
  ), "writer identity invalid");
  requireCondition(reader.every((call) => Number.isInteger(call.vector_hits) && call.vector_hits > 0),
    "reader vector result missing");
  const overlap = overlapCount(writer, reader);
  requireCondition(overlap > 0 && overlap === claimedOverlap, "call overlap mismatch");
  for (const calls of [writer, reader]) {
    requireCondition(calls.every((call, index) =>
      index === 0 || calls[index - 1].start_ns < call.start_ns,
    ), "call order invalid");
  }
  return overlap;
}

export async function runOnce({ database, installRoot, sourceSha, expectedNativeSha256 }) {
  requireCondition(!existsSync(database), "contention database must be fresh");
  requireCondition(/^[0-9a-f]{40}$/.test(sourceSha), "source SHA malformed");
  requireCondition(/^[0-9a-f]{64}$/.test(expectedNativeSha256), "native SHA malformed");
  const artifact = artifactIdentity(installRoot, expectedNativeSha256);
  const { Engine, graph, read } = await import(pathToFileURL(artifact.module_path).href);
  const corpus = makeCorpus(32);
  const records = [...corpus, ...makeGraphRecords()];
  const tick = clock();
  const engine = await Engine.open(database, { useDefaultEmbedder: true });
  const timeline = { writer_calls: [], reader_calls: [] };
  const observed = {
    status: "S02_TS_CONTENTION_FUNCTIONAL_OK",
    writer_ids: WRITER_IDS,
    bounded_seconds: 60,
  };
  try {
    await engine.write(records);
    await engine.configureProjections([
      { name: "summary", roles: ["searchable"], fts: true, vector: true },
    ]);
    await engine.drain(60_000);
    let release;
    const gate = new Promise((ready) => { release = ready; });
    const writer = async () => {
      await gate;
      for (const [index, logicalId] of WRITER_IDS.entries()) {
        const start = tick();
        await engine.write([{
          kind: "doc", body: JSON.stringify({ summary: `contended ${String(index).padStart(2, "0")}` }),
          sourceId: CONTENDED_SOURCE, logicalId,
        }]);
        timeline.writer_calls.push({ logical_id: logicalId, start_ns: start, end_ns: tick() });
        await new Promise((ready) => setTimeout(ready, 15));
      }
    };
    const reader = async () => {
      await gate;
      for (let index = 0; index < 8; index++) {
        const start = tick();
        const anchor = await read.get(engine, "A");
        requireCondition(anchor?.body === corpus[0].body, "anchor changed during contention");
        const text = await engine.searchTextOnly("brightblue");
        requireCondition(JSON.stringify(text.results.map((hit) => hit.id.value)) === '["A"]',
          "text result changed during contention");
        const vector = await engine.search("boat schedule across water");
        requireCondition(vector.results.some((hit) => hit.branch === "vector"),
          "vector result changed during contention");
        const neighbors = await graph.neighbors(engine, "s02-root", 1);
        requireCondition(neighbors.length > 0, "graph disappeared during contention");
        timeline.reader_calls.push({
          start_ns: start, end_ns: tick(), vector_hits: vector.results.length,
        });
      }
    };
    const pending = [writer(), reader()];
    release();
    const settled = await Promise.allSettled(pending);
    requireCondition(settled.every((item) => item.status === "fulfilled"),
      `contending task failed: ${settled.filter((item) => item.status === "rejected")
        .map((item) => String(item.reason)).join("; ")}`);
    observed.reader_cycles = timeline.reader_calls.length;
    observed.overlap_cycles = overlapCount(timeline.writer_calls, timeline.reader_calls);
    validateContention({
      writer: timeline.writer_calls, reader: timeline.reader_calls,
      claimedOverlap: observed.overlap_cycles,
    });
    await engine.drain(60_000);
    const projection = (await read.projections(engine)).find((item) => item.name === "summary");
    observed.readiness = projection?.vectorDenseReadiness;
    for (const id of WRITER_IDS) {
      requireCondition(await read.get(engine, id) !== null, `writer row missing: ${id}`);
    }
    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const found = await engine.searchWithEvidence({
      schemaVersion: 1, query: "slice135s02evidenceneedle", context: frozen, limit: 1,
    });
    requireCondition(found.evidence.length > 0, "evidence search empty");
    const resolved = await engine.resolveEvidence({
      schemaVersion: 1, evidenceRef: found.evidence[0].evidenceRef, context: frozen,
    });
    observed.evidence = {
      logical_id: resolved.logicalId, source_body: resolved.canonicalSourceBody,
    };
    const erasedGraph = await engine.eraseSource(GRAPH_SOURCE);
    const erasedContended = await engine.eraseSource(CONTENDED_SOURCE);
    observed.erasure = {
      graph_nodes: erasedGraph.nodesExcised,
      graph_edges: erasedGraph.edgesExcised,
      contended_nodes: erasedContended.nodesExcised,
    };
    observed.before_reopen = await sdkState(engine, read, graph, corpus[0].body);
  } finally {
    await engine.close();
  }
  const reopened = await Engine.open(database, { useDefaultEmbedder: true });
  try {
    requireCondition(await read.get(reopened, "A") !== null, "anchor missing after reopen");
    requireCondition(await read.get(reopened, WRITER_IDS[0]) === null,
      "erased writer visible after reopen");
    observed.after_reopen = await sdkState(reopened, read, graph, corpus[0].body);
    observed.reopened_readiness = (await read.projections(reopened))
      .find((item) => item.name === "summary")?.vectorDenseReadiness;
  } finally {
    await reopened.close();
  }
  observed.elapsed_ns = tick();
  const verifyStart = tick();
  observed.final_state = databaseState(database);
  observed.verification_ns = tick() - verifyStart;
  validateTimedState(observed);
  requireCondition(observed.readiness === "ready" && observed.reopened_readiness === "ready",
    "projection not ready");
  requireCondition(observed.evidence.logical_id === "s02-claim"
    && observed.evidence.source_body === SOURCE_BODY, "evidence changed");
  requireCondition(JSON.stringify(observed.erasure)
    === JSON.stringify({ graph_nodes: 3, graph_edges: 1, contended_nodes: 8 }),
  "erasure changed");
  return {
    schema_version: 1, source_sha: sourceSha,
    status: "S02_TS_CONTENTION_FUNCTIONAL_OK",
    runner_sha256: sha(readFileSync(fileURLToPath(import.meta.url))),
    s01_helper_sha256: sha(readFileSync(fileURLToPath(new URL("./slice135_ts_s01.mjs", import.meta.url)))),
    s02_helper_sha256: sha(readFileSync(fileURLToPath(new URL("./slice135_ts_s02.mjs", import.meta.url)))),
    artifact, observed, timeline,
  };
}

async function main() {
  const arguments_ = process.argv.slice(2);
  const options = {};
  for (let index = 0; index < arguments_.length; index += 2) {
    requireCondition(arguments_[index]?.startsWith("--") && arguments_[index + 1] !== undefined,
      "arguments must be --key value pairs");
    options[arguments_[index].slice(2)] = arguments_[index + 1];
  }
  for (const key of ["database", "install-root", "source-sha", "expected-native-sha256", "output"]) {
    requireCondition(options[key] !== undefined, `missing --${key}`);
  }
  const database = resolve(options.database);
  const output = resolve(options.output);
  requireCondition(!existsSync(output), "output must be fresh");
  const result = await runOnce({
    database, installRoot: options["install-root"], sourceSha: options["source-sha"],
    expectedNativeSha256: options["expected-native-sha256"],
  });
  writeFileSync(output, `${JSON.stringify(result, null, 2)}\n`);
  process.stdout.write("S02_TS_CONTENTION_FUNCTIONAL_OK\n");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`${error?.stack ?? String(error)}\n`);
    process.exitCode = 1;
  });
}
