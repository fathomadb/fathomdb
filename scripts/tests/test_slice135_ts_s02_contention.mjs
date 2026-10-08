import assert from "node:assert/strict";
import test from "node:test";

import {
  overlapCount, validateContention, validateTimedState,
} from "../slice135_ts_s02_contention.mjs";

test("reader and writer intervals must actually overlap", () => {
  const writer = [
    { logical_id: "contend-00", start_ns: 10, end_ns: 20 },
    { logical_id: "contend-01", start_ns: 30, end_ns: 40 },
  ];
  const reader = [
    { start_ns: 15, end_ns: 25, vector_hits: 1 },
    { start_ns: 41, end_ns: 50, vector_hits: 1 },
  ];
  assert.equal(overlapCount(writer, reader), 1);
  assert.throws(() => validateContention({
    writer, reader, claimedOverlap: 2, expectedCalls: 2,
  }), /overlap/);
  assert.throws(() => validateContention({
    writer, reader: reader.map((call) => ({ ...call, start_ns: 50, end_ns: 60 })),
    claimedOverlap: 0, expectedCalls: 2,
  }), /overlap/);
});

test("contention validation rejects missing calls and malformed intervals", () => {
  const writer = [{ logical_id: "contend-00", start_ns: 1, end_ns: 10 }];
  const reader = [{ start_ns: 2, end_ns: 3, vector_hits: 1 }];
  assert.equal(validateContention({ writer, reader, claimedOverlap: 1, expectedCalls: 1 }), 1);
  assert.throws(() => validateContention({
    writer, reader: [], claimedOverlap: 0, expectedCalls: 1,
  }), /count/);
  assert.throws(() => validateContention({
    writer: [{ ...writer[0], end_ns: 1 }], reader,
    claimedOverlap: 1, expectedCalls: 1,
  }), /interval/);
});

test("direct SQLite state belongs after the product timer", () => {
  const sdk = {
    anchor_retained: true, erased_writer_absent: true, erased_graph_absent: true,
    evidence_query_empty: true, graph_empty: true,
  };
  const final = {
    corpus_nodes: 32, graph_nodes: 0, graph_edges: 0,
    contended_nodes: 0, integrity_check: "ok",
  };
  assert.equal(validateTimedState({
    before_reopen: sdk, after_reopen: sdk, final_state: final,
    elapsed_ns: 100, verification_ns: 10,
  }), true);
  assert.throws(() => validateTimedState({
    before_reopen: { ...sdk, corpus_nodes: 32 }, after_reopen: sdk,
    final_state: final, elapsed_ns: 100, verification_ns: 10,
  }), /timer boundary/);
});
