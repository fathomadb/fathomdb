import assert from "node:assert/strict";
import test from "node:test";

import { summarize, validateInstalledIdentity } from "../slice135_ts_capabilities.mjs";

const operations = ["engine.open", "read.get"];
const executed = { status: "executed", positive: { case: "real-test" },
  negative: { case: "typed-refusal" }, reopen: { databases: 1, activeBodiesChecked: 1 } };

test("accounting rejects an omitted canonical operation", () => {
  assert.throws(() => summarize(operations, { "engine.open": executed }), /canonical/);
});

test("accounting rejects vacuous successful rows and unknown statuses", () => {
  assert.throws(() => summarize(operations, {
    "engine.open": { status: "executed", positive: executed.positive },
    "read.get": executed,
  }), /evidence/);
  assert.throws(() => summarize(operations, {
    "engine.open": { ...executed, status: "pass" }, "read.get": executed,
  }), /status/);
});

test("accounting disjoins executed, failed, gap and unavailable", () => {
  assert.deepEqual(summarize(operations, {
    "engine.open": executed,
    "read.get": { status: "gap", reason: "no positive case", owner: "Slice 135" },
  }), { supported: 2, executed: 1, failed: 0, gap: 1, unavailable: 0 });
});

test("installed identity rejects path escape and mismatched native bytes", () => {
  assert.throws(() => validateInstalledIdentity({
    installRoot: "/tmp/synthetic-consumer", nativePath: "/tmp/escape.node",
    expectedNativeSha256: "0".repeat(64),
  }), /installed/);
});
