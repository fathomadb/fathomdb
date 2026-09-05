import assert from "node:assert/strict";
import test from "node:test";

import {
  validateNativeEvidenceSearch,
  validateNativeResolvedEvidence,
} from "../src/evidence-validation.js";
import { EvidenceError } from "../src/index.js";

test("unknown native evidence response versions fail closed", () => {
  for (const [validate, value] of [
    [validateNativeEvidenceSearch, { schemaVersion: 2 }],
    [validateNativeResolvedEvidence, { schemaVersion: 2 }],
  ] as const) {
    assert.throws(
      () => validate(value as never),
      (error: unknown) =>
        error instanceof EvidenceError &&
        error.reason === "unsupported_schema_version" &&
        error.fieldPath === "/schemaVersion",
    );
  }
});
