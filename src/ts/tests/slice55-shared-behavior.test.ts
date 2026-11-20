import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { Engine, read, WriteValidationError } from "../src/index.js";
import { freshDbPath } from "./helpers.js";

interface Fixture {
  query: string; instant: number; body: string; invalidQuery: string;
  currentIds: string[]; expiredIds: string[];
  nodes: { id: string; source: string; revision: string; version: string }[];
}
const fixture = JSON.parse(readFileSync(new URL("../../../conformance/slice55-behavior.json", import.meta.url), "utf8")) as Fixture;

test("shared native identity Unicode provenance time filter cursor and reopen", async () => {
  const path = freshDbPath();
  const engine = await Engine.open(path, { useDefaultEmbedder: false });
  try {
    await engine.configureProjections([{ name: "group", roles: ["filterable"], fts: false, vector: false }]);
    await engine.write(fixture.nodes.map((n, i) => ({
      kind: "doc", body: fixture.body, logicalId: n.id, sourceId: n.source,
      validFrom: 1_000_000_000, validUntil: i === 0 ? fixture.instant + 1 : null,
      provenance: { schemaVersion: 1 as const, role: "canonical" as const, artifactRevisionId: n.revision, sourceVersionId: n.version },
    })));
    await engine.drain(10_000);
    const result = await engine.searchTextOnly(fixture.query);
    assert.deepEqual(result.results.map(h => h.id.value), fixture.currentIds);
    assert.deepEqual(result.results.map(h => h.body), [fixture.body, fixture.body]);
    assert.deepEqual(result.results.map(h => h.sourceId), fixture.nodes.map(n => n.source));
    const expired = await engine.searchTextOnly(fixture.query, { validAsOf: fixture.instant + 1 });
    assert.deepEqual(expired.results.map(h => h.id.value), fixture.expiredIds);
    assert.equal((await engine.searchTextOnly(fixture.query, { filter: { attributes: [["group", "kept"]] } })).results.length, 2);
    assert.deepEqual((await engine.searchTextOnly(fixture.query, { filter: { attributes: [["group", "absent"]] } })).results, []);
    const frozen = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const first = await read.canonicalPage(engine, "doc", frozen, { schemaVersion: 1, limit: 1 });
    assert.ok(first.nextCursor);
    const second = await read.canonicalPage(engine, "doc", frozen, { schemaVersion: 1, limit: 1, cursor: first.nextCursor });
    assert.deepEqual([...first.items, ...second.items].map(r => r.logicalId), fixture.currentIds);
    assert.equal(second.nextCursor, null);
    await assert.rejects(engine.searchTextOnly(fixture.invalidQuery), WriteValidationError);
    const evidence = await engine.searchWithEvidence({ schemaVersion: 1, query: fixture.query, context: frozen });
    assert.deepEqual(evidence.evidence.map(e => e.artifactRevisionId), fixture.nodes.map(n => n.revision));
    for (const [i, entry] of evidence.evidence.entries()) {
      const expected = fixture.nodes[i]!;
      const resolved = await engine.resolveEvidence({ schemaVersion: 1, evidenceRef: entry.evidenceRef, context: frozen });
      assert.equal(resolved.canonicalSourceBody, fixture.body);
      assert.equal(resolved.sourceRevisionId, expected.revision);
      assert.equal(resolved.artifactRevisionId, expected.revision);
      assert.equal(resolved.sourceId, expected.source);
    }
  } finally { await engine.close(); }
  const reopened = await Engine.open(path, { useDefaultEmbedder: false });
  try {
    for (const n of fixture.nodes) assert.equal((await read.get(reopened, n.id))!.body, fixture.body);
    assert.equal(await read.get(reopened, "absent"), null);
  } finally { await reopened.close(); }
});
