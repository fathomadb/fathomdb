import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

import {
  GRAPH_SOURCE,
  SOURCE_BODY,
  makeGraphRecords,
  validateObservations,
} from "../slice135_ts_s02.mjs";

const goodState = () => ({
  source_absent: true,
  root_absent: true,
  claim_absent: true,
  anchor_retained: true,
  evidence_query_empty: true,
  graph_empty: true,
  canonical_counts: { graph_nodes: 0, graph_edges: 0, corpus_nodes: 32 },
});

const good = () => ({
  embedder: "fathomdb-bge-small-en-v1.5",
  readiness: "ready",
  unsupported_kinds: [],
  anchor_before: true,
  text: { ids: ["A"], branches: ["text"] },
  vector: { ids: ["B"], branches: ["vector"] },
  hybrid: { ids: ["A"], branches: ["text"] },
  hybrid_lexical_ids: ["A"],
  evidence: { logical_id: "s02-claim", source_body: SOURCE_BODY },
  graph: {
    target_id: "s02-claim",
    target_revision: "s02-claim-r1",
    edge_revision: "s02-edge-r1",
    edge_from: "s02-root",
    edge_to: "s02-claim",
    source_body: SOURCE_BODY,
  },
  erasure: { source_ref: GRAPH_SOURCE, nodes_excised: 3, edges_excised: 1 },
  second_erasure: { nodes_excised: 0, edges_excised: 0 },
  after_erasure: goodState(),
  after_reopen: { ...goodState(), readiness: "ready" },
});

test("S02 graph fixtures bind derived content to the canonical source", () => {
  const records = makeGraphRecords();
  assert.equal(records.length, 4);
  assert.deepEqual(records.map((item) => item.logicalId ?? item.edge?.logicalId), [
    "s02-source", "s02-root", "s02-claim", "s02-edge",
  ]);
  const digest = createHash("sha256").update(SOURCE_BODY).digest("hex");
  assert.equal(records[2].provenance.canonicalSourceHash.digestHex, digest);
  assert.equal(records[3].edge.provenance.canonicalSourceHash.digestHex, digest);
});

test("S02 state oracle catches missing evidence and incomplete erasure", () => {
  assert.doesNotThrow(() => validateObservations(good()));
  const missingEvidence = good();
  missingEvidence.evidence.source_body = "wrong";
  assert.throws(() => validateObservations(missingEvidence), /canonical evidence/);
  const retainedEdge = good();
  retainedEdge.after_reopen.canonical_counts.graph_edges = 1;
  assert.throws(() => validateObservations(retainedEdge), /canonical persistence/);
  const absentVector = good();
  absentVector.vector.branches = ["text"];
  assert.throws(() => validateObservations(absentVector), /vector branch/);
});

test("S02 timed mode accepts final independent counts after product timing", () => {
  const observed = good();
  delete observed.after_erasure.canonical_counts;
  delete observed.after_reopen.canonical_counts;
  const counts = { graph_nodes: 0, graph_edges: 0, corpus_nodes: 32 };
  assert.doesNotThrow(() => validateObservations(observed, { finalCanonicalCounts: counts }));
  assert.throws(() => validateObservations(observed), /canonical persistence/);
  assert.throws(
    () => validateObservations(observed, { finalCanonicalCounts: { ...counts, graph_edges: 1 } }),
    /canonical persistence/,
  );
});
