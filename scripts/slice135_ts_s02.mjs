#!/usr/bin/env node
/** Exercise one installed-package S02 whole-product sequence on a fresh database. */

import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { DatabaseSync } from "node:sqlite";

import { makeCorpus } from "./slice135_ts_s01.mjs";

export const GRAPH_SOURCE = "slice135-s02-graph";
export const SOURCE_BODY = '{"summary": "s02 canonical source evidence bytes"}';
const CLAIM_TOKEN = "slice135s02evidenceneedle";
const CORPUS_SOURCE = "slice135-s01-python";
const QUERIES = {
  text: "brightblue",
  vector: "boat schedule across water",
  hybrid: "harbor ferry",
};

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function requireCondition(condition, reason) {
  if (!condition) throw new Error(reason);
}

export function makeGraphRecords() {
  const canonical = {
    schemaVersion: 1,
    role: "canonical",
    artifactRevisionId: "s02-source-r1",
    sourceVersionId: "s02-source-v1",
  };
  const derived = (revision) => ({
    schemaVersion: 1,
    role: "derived",
    artifactRevisionId: revision,
    sourceVersionId: "s02-source-v1",
    sourceRevisionId: "s02-source-r1",
    sourceLocator: { kind: "whole_body" },
    canonicalSourceHash: { algorithm: "sha256", digestHex: sha256(SOURCE_BODY) },
  });
  return [
    {
      kind: "doc", body: SOURCE_BODY, sourceId: GRAPH_SOURCE,
      logicalId: "s02-source", provenance: canonical,
    },
    {
      kind: "doc", body: '{"summary": "s02 graph root"}', sourceId: GRAPH_SOURCE,
      logicalId: "s02-root", provenance: derived("s02-root-r1"),
    },
    {
      kind: "doc", body: `{"summary": "${CLAIM_TOKEN}"}`, sourceId: GRAPH_SOURCE,
      logicalId: "s02-claim", provenance: derived("s02-claim-r1"),
    },
    {
      edge: {
        kind: "supports", from: "s02-root", to: "s02-claim", sourceId: GRAPH_SOURCE,
        logicalId: "s02-edge", provenance: derived("s02-edge-r1"),
      },
    },
  ];
}

function checkHits(kind, result, eligible) {
  const ids = result.results.map((hit) => hit.id.value);
  const branches = result.results.map((hit) => hit.branch);
  requireCondition(ids.every((id) => eligible.has(id)), `unknown ${kind} retrieval id`);
  return { ids, branches };
}

export function validateObservations(observed, { finalCanonicalCounts = null } = {}) {
  requireCondition(observed.embedder === "fathomdb-bge-small-en-v1.5", "embedder identity");
  requireCondition(observed.readiness === "ready", "projection not ready");
  requireCondition(observed.unsupported_kinds.length === 0, "unsupported vector kind");
  requireCondition(observed.anchor_before === true, "anchor changed before queries");
  requireCondition(JSON.stringify(observed.text) === JSON.stringify({ ids: ["A"], branches: ["text"] }), "text anchor");
  requireCondition(observed.vector.ids.length > 0 && observed.vector.branches.includes("vector"), "vector branch");
  requireCondition(observed.hybrid.ids.includes("A"), "hybrid anchor");
  requireCondition(observed.hybrid_lexical_ids.includes("A"), "hybrid lexical eligibility");
  requireCondition(observed.hybrid.branches.every((branch) => ["text", "vector"].includes(branch)), "hybrid branch");
  requireCondition(
    observed.evidence.logical_id === "s02-claim" && observed.evidence.source_body === SOURCE_BODY,
    "canonical evidence",
  );
  requireCondition(
    JSON.stringify(observed.graph) === JSON.stringify({
      target_id: "s02-claim", target_revision: "s02-claim-r1", edge_revision: "s02-edge-r1",
      edge_from: "s02-root", edge_to: "s02-claim", source_body: SOURCE_BODY,
    }),
    "graph evidence",
  );
  requireCondition(
    JSON.stringify(observed.erasure) === JSON.stringify({
      source_ref: GRAPH_SOURCE, nodes_excised: 3, edges_excised: 1,
    }),
    "erasure count",
  );
  requireCondition(
    JSON.stringify(observed.second_erasure) === JSON.stringify({ nodes_excised: 0, edges_excised: 0 }),
    "erasure idempotence",
  );
  for (const name of ["after_erasure", "after_reopen"]) {
    const state = observed[name];
    requireCondition(
      ["source_absent", "root_absent", "claim_absent", "anchor_retained", "evidence_query_empty", "graph_empty"]
        .every((key) => state[key] === true),
      `${name} state`,
    );
    if (state.canonical_counts !== undefined || finalCanonicalCounts === null) {
      requireCondition(
        JSON.stringify(state.canonical_counts) === JSON.stringify({
          graph_nodes: 0, graph_edges: 0, corpus_nodes: 32,
        }),
        "canonical persistence",
      );
    }
  }
  if (finalCanonicalCounts !== null) {
    requireCondition(
      JSON.stringify(finalCanonicalCounts) === JSON.stringify({
        graph_nodes: 0, graph_edges: 0, corpus_nodes: 32,
      }),
      "canonical persistence",
    );
  }
  requireCondition(observed.after_reopen.readiness === "ready", "reopened readiness");
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
  const nativeSha256 = sha256(readFileSync(nativePath));
  requireCondition(nativeSha256 === expectedNativeSha256, "installed native artifact SHA-256 mismatch");
  return {
    module_path: modulePath, module_sha256: sha256(readFileSync(modulePath)),
    package_path: packagePath, package_sha256: sha256(readFileSync(packagePath)),
    package_version: JSON.parse(readFileSync(packagePath, "utf8")).version,
    native_path: nativePath, native_sha256: nativeSha256,
    node_executable: process.execPath, node_version: process.version,
  };
}

function canonicalCounts(database) {
  const connection = new DatabaseSync(database, { readOnly: true });
  try {
    const count = (table, source) => connection.prepare(
      `SELECT count(*) AS n FROM ${table} WHERE source_id = ?`,
    ).get(source).n;
    return {
      graph_nodes: count("canonical_nodes", GRAPH_SOURCE),
      graph_edges: count("canonical_edges", GRAPH_SOURCE),
      corpus_nodes: count("canonical_nodes", CORPUS_SOURCE),
    };
  } finally {
    connection.close();
  }
}

async function postState(engine, read, graph, database, anchorBody, includeCanonicalCounts) {
  const state = {
    source_absent: (await read.get(engine, "s02-source")) === null,
    root_absent: (await read.get(engine, "s02-root")) === null,
    claim_absent: (await read.get(engine, "s02-claim")) === null,
    anchor_retained: (await read.get(engine, "A"))?.body === anchorBody,
    evidence_query_empty: (await engine.searchTextOnly(CLAIM_TOKEN)).results.length === 0,
    graph_empty: (await graph.neighbors(engine, "s02-root", 1)).length === 0,
  };
  if (includeCanonicalCounts) state.canonical_counts = canonicalCounts(database);
  return state;
}

export async function runOnce({ installRoot, sourceSha, expectedNativeSha256, timingMode = "functional" }) {
  requireCondition(/^[0-9a-f]{40}$/.test(sourceSha), "source SHA malformed");
  requireCondition(/^[0-9a-f]{64}$/.test(expectedNativeSha256), "native SHA malformed");
  requireCondition(["functional", "product"].includes(timingMode), "timing mode invalid");
  const productTiming = timingMode === "product";
  const artifact = artifactIdentity(installRoot, expectedNativeSha256);
  const { Engine, graph, read } = await import(pathToFileURL(artifact.module_path).href);
  const corpus = makeCorpus(32);
  const records = [...corpus, ...makeGraphRecords()];
  const eligible = new Set(records.filter((item) => item.logical_id ?? item.logicalId)
    .map((item) => item.logical_id ?? item.logicalId));
  const temporary = mkdtempSync(join(tmpdir(), "slice135-ts-s02-"));
  const database = join(temporary, "s02.sqlite");
  const stages = {};
  const timed = async (name, call) => {
    const start = process.hrtime.bigint();
    try { return await call(); } finally { stages[name] = Number(process.hrtime.bigint() - start); }
  };
  try {
    const wholeStart = process.hrtime.bigint();
    const engine = await timed("open", () => Engine.open(database, { useDefaultEmbedder: true }));
    let observed;
    try {
      const embedder = engine.openReport().defaultEmbedder?.name;
      await timed("write", () => engine.write(records));
      const delta = await timed("configure_projection", () => engine.configureProjections([
        { name: "summary", roles: ["searchable"], fts: true, vector: true },
      ]));
      await timed("drain", () => engine.drain(120_000));
      const projection = (await read.projections(engine)).find((item) => item.name === "summary");
      const anchorBefore = (await read.get(engine, "A"))?.body === corpus[0].body
        && (await read.get(engine, "B"))?.body === corpus[1].body;
      const text = checkHits("text", await timed("text", () => engine.searchTextOnly(QUERIES.text)), eligible);
      const vector = checkHits("vector", await timed("vector", () => engine.search(QUERIES.vector)), eligible);
      const hybrid = checkHits("hybrid", await timed("hybrid", () => engine.search(QUERIES.hybrid)), eligible);
      const hybridLexicalIds = (await timed("hybrid_lexical_control", () =>
        engine.searchTextOnly(QUERIES.hybrid))).results.map((hit) => hit.id.value);
      const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
      const evidenceSearch = await timed("evidence_search", () => engine.searchWithEvidence({
        schemaVersion: 1, query: CLAIM_TOKEN, context: frozen, limit: 1,
      }));
      requireCondition(evidenceSearch.evidence.length > 0, "evidence sidecar empty");
      const resolved = await timed("evidence_resolve", () => engine.resolveEvidence({
        schemaVersion: 1, evidenceRef: evidenceSearch.evidence[0].evidenceRef, context: frozen,
      }));
      const expanded = await timed("graph_expand", () => graph.expand(engine, {
        schemaVersion: 1,
        seed: { schemaVersion: 1, type: "explicit", logicalIds: [{ space: "logical", value: "s02-root" }] },
        direction: "outgoing", edgeKinds: ["supports"], targetKinds: ["doc"],
        context: { schemaVersion: 1, type: "frozen", context: frozen },
        maxDepth: 1, resultLimit: 1, maxWorkUnits: "10",
        includeExplanation: false, includeEvidence: true,
      }));
      requireCondition(expanded.targets.length > 0 && expanded.evidence?.entries.length > 0, "graph empty");
      const entry = expanded.evidence.entries[0];
      const target = await timed("graph_target_resolve", () => engine.resolveGraphEvidence({
        schemaVersion: 1, evidenceRef: entry.targetEvidenceRef, context: frozen,
      }));
      const edge = await timed("graph_edge_resolve", () => engine.resolveGraphEvidence({
        schemaVersion: 1, evidenceRef: entry.terminalEdgeEvidenceRef, context: frozen,
      }));
      const report = await timed("erase", () => engine.eraseSource(GRAPH_SOURCE));
      const second = await timed("erase_again", () => engine.eraseSource(GRAPH_SOURCE));
      observed = {
        embedder, readiness: projection?.vectorDenseReadiness,
        unsupported_kinds: delta.vectorUnsupportedKinds, anchor_before: anchorBefore,
        text, vector, hybrid, hybrid_lexical_ids: hybridLexicalIds,
        evidence: { logical_id: resolved.logicalId, source_body: resolved.canonicalSourceBody },
        graph: {
          target_id: target.artifact.logicalId, target_revision: target.artifactRevisionId,
          edge_revision: edge.artifactRevisionId, edge_from: edge.artifact.from,
          edge_to: edge.artifact.to, source_body: target.canonicalSourceBody,
        },
        erasure: {
          source_ref: report.sourceRef, nodes_excised: report.nodesExcised,
          edges_excised: report.edgesExcised,
        },
        second_erasure: { nodes_excised: second.nodesExcised, edges_excised: second.edgesExcised },
        after_erasure: await postState(engine, read, graph, database, corpus[0].body, !productTiming),
      };
    } finally {
      await timed("close", () => engine.close());
    }
    const reopened = await timed("reopen", () => Engine.open(database, { useDefaultEmbedder: true }));
    try {
      const projection = (await read.projections(reopened)).find((item) => item.name === "summary");
      observed.after_reopen = {
        ...await postState(reopened, read, graph, database, corpus[0].body, !productTiming),
        readiness: projection?.vectorDenseReadiness,
      };
    } finally {
      await timed("reopened_close", () => reopened.close());
    }
    const wholeNs = Number(process.hrtime.bigint() - wholeStart);
    const verificationStart = process.hrtime.bigint();
    const finalCanonicalCounts = productTiming ? canonicalCounts(database) : null;
    validateObservations(observed, { finalCanonicalCounts });
    const verificationNs = Number(process.hrtime.bigint() - verificationStart);
    return {
      schema_version: 1,
      status: productTiming ? "UNFROZEN_TS_S02_PRODUCT_TIMING_FEASIBILITY" : "UNFROZEN_TS_S02_FUNCTIONAL_FEASIBILITY",
      finished_utc: new Date().toISOString(), source_sha: sourceSha,
      runner_sha256: sha256(readFileSync(fileURLToPath(import.meta.url))),
      s01_helper_sha256: sha256(readFileSync(fileURLToPath(new URL("./slice135_ts_s01.mjs", import.meta.url)))),
      artifact, corpus_sha256: sha256(JSON.stringify(corpus)),
      graph_records_sha256: sha256(JSON.stringify(makeGraphRecords())),
      ...(productTiming
        ? { whole_product_ns: wholeNs, verification_ns: verificationNs, final_canonical_counts: finalCanonicalCounts }
        : { whole_sequence_including_checks_ns: wholeNs }),
      stage_ns: stages, observed, semantic_ok: true,
    };
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

function parseArguments(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 2) {
    requireCondition(argv[index]?.startsWith("--") && argv[index + 1] !== undefined, "arguments must be --name value pairs");
    values[argv[index].slice(2)] = argv[index + 1];
  }
  for (const name of ["install-root", "source-sha", "expected-native-sha256", "output"]) {
    requireCondition(values[name] !== undefined, `missing --${name}`);
  }
  return values;
}

async function main() {
  const values = parseArguments(process.argv.slice(2));
  const outputPath = resolve(values.output);
  requireCondition(!existsSync(outputPath), "output already exists");
  const output = await runOnce({
    installRoot: values["install-root"], sourceSha: values["source-sha"],
    expectedNativeSha256: values["expected-native-sha256"],
    timingMode: values["timing-mode"] ?? "functional",
  });
  writeFileSync(outputPath, `${JSON.stringify(output, null, 2)}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`${error?.stack ?? String(error)}\n`);
    process.exitCode = 1;
  });
}
