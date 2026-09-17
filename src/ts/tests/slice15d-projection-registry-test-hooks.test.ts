// Debug-build-only parity cases for Slice 15d projection registry behavior.
// Kept separate because release-artifact smoke tests intentionally exercise a
// native binary that does not export private test hooks.

import test from "node:test";
import assert from "node:assert/strict";

import { Engine, read } from "../src/index.js";
import type { ProjectionSpec, ProjectionRole } from "../src/index.js";
import { WriteValidationError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

const SOURCE = "ts-test:slice15d";

function node(logicalId: string, source: string, bodyJson: string): object {
  return { kind: "doc", body: bodyJson, logicalId, sourceId: source };
}

test("R-20-SV — a LEGACY registry row reads back verbatim but no longer re-applies", async () => {
  // 0.8.20 Slice 23 (`R-20-SV`) — controlled inert registry shape.
  //
  // A debug-only hook atomically changes a valid filterable declaration to the
  // old `fts`/`vector`-without-`searchable` shape and advances current generation
  // authority. `read.projections` reports the injected row VERBATIM, but feeding
  // that output straight back into `configureProjections` RAISES
  // `WriteValidationError` (`FDB_WRITE_VALIDATION`). This fixture is not an
  // upgrade-admission promise.
  //
  // That asymmetry is the honest, documented consequence of the HITL ruling
  // (2026-07-24, `dev/plans/plan-0.8.20.md` §11 item 4, option (b)): for the
  // injected inert shape the round-trip is broken BY DESIGN, and the remedy is to
  // ADD the `searchable` role (asserted here) or to name the projection in
  // `drop`.
  //
  // X1 parity twin of the Rust
  // `a_legacy_registry_row_reads_back_verbatim_but_no_longer_re_applies`
  // (`slice23_spec_validation_reject.rs`) and the Python
  // `test_a_legacy_registry_row_reads_back_verbatim_but_no_longer_re_applies`.
  // Same seed, same three oracles, same semantics — deliberately NOT a
  // per-binding invention.
  const path = freshDbPath();
  let injectedGenerationId = "";
  {
    const engine = await Engine.open(path);
    try {
      await engine.write([node("N1", SOURCE, JSON.stringify({ status: "open" }))]);
      await engine.configureProjections([
        { name: "status", roles: ["filterable"], fts: false, vector: false },
      ]);
      const before = await read.projectionGenerationStatus(engine);
      const inject = engine._native.setLegacyProjectionSearchSubobjectsForTest;
      assert.ok(inject, "debug test-hook binding is required");
      await inject.call(engine._native, "status");
      const injected = await read.projectionGenerationStatus(engine);
      assert.notEqual(injected.generationId, before.generationId);
      assert.equal(injected.origin, "configuration");
      injectedGenerationId = injected.generationId;
    } finally {
      await engine.close();
    }
  }

  const engine = await Engine.open(path);
  try {
    const reopenedGeneration = await read.projectionGenerationStatus(engine);
    assert.equal(reopenedGeneration.generationId, injectedGenerationId);
    assert.equal(reopenedGeneration.origin, "configuration");

    // (a) Still READABLE, and reported verbatim.
    const back = await read.projections(engine);
    assert.equal(back.length, 1, "a legacy row must still be readable");
    assert.equal(back[0].name, "status");
    assert.deepEqual([...back[0].roles].sort(), ["filterable"]);
    assert.equal(back[0].fts, true, "the legacy `fts` sub-object is reported verbatim");
    assert.equal(back[0].vector, true, "…and the legacy `vector` sub-object too");

    // (b) …but it can no longer be RE-APPLIED. Feeding `read.projections`
    // output straight back in is exactly the shipped fix-4 round-trip.
    await assert.rejects(
      engine.configureProjections(back),
      WriteValidationError,
      "BREAKING, by design: for the injected fts/vector-without-`searchable` shape the " +
        "read.projections -> configureProjections round-trip no longer closes",
    );

    // (c) The remedy closes it again: ADD the `searchable` role. Everything else
    // about the declaration is untouched.
    const fixed: ProjectionSpec = {
      ...back[0],
      roles: [...back[0].roles, "searchable"] as ProjectionRole[],
    };
    await engine.configureProjections([fixed]);
    const after = await read.projections(engine);
    assert.equal(after.length, 1);
    assert.deepEqual([...after[0].roles].sort(), ["filterable", "searchable"]);
    assert.equal(after[0].fts, true);
    assert.equal(after[0].vector, true);
    await engine.drain(5_000);
  } finally {
    await engine.close();
  }
});
