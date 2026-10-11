import { Engine, admin, read, graph, EvidenceError } from "fathomdb";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
admin.configureRuntime({ sqliteMode: "performance" });
const path = join(mkdtempSync(join(tmpdir(), "fdb-s120-consumer-")), "db.sqlite");
const engine = await Engine.open(path);
try {
  const receipt = await engine.write([
    { kind: "note", body: "hello slice", sourceId: "source-120", logicalId: "logical-120" },
  ]);
  const row = await read.get(engine, "logical-120");
  const result = await engine.search("hello");
  const neighbors = await graph.neighbors(engine, "logical-120", 1);
  const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
  let evidenceRefused = false;
  try {
    await engine.searchWithEvidence({ schemaVersion: 1, query: "hello", context: frozen });
  } catch (error) {
    evidenceRefused = error instanceof EvidenceError && error.reason === "evidence_incomplete";
  }
  if (
    !receipt.rowCursors.length ||
    row?.logicalId !== "logical-120" ||
    !Array.isArray(result.results) ||
    !Array.isArray(neighbors) ||
    !evidenceRefused
  )
    throw new Error("installed consumer smoke shape mismatch");
  console.log("PASS installed package: write/read/search/graph/frozen evidence/admin");
} finally {
  await engine.close();
}
