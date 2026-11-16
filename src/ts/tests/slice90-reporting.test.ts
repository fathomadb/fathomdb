import test from "node:test";
import assert from "node:assert/strict";
import { decodeStoredHeadCoverage } from "../src/coverage.js";
import { mapNativeSnippets } from "../src/search.js";
import type { NativeSnippetV1 } from "../src/binding.js";
import { ProjectionGenerationError } from "../src/errors.js";

function native(): Record<string, unknown> {
  return {status: "available", population: "stored_mapped_spans", profile: {key: "default", name: "fixture", revision: "v1", dimension: 8},
    records: "4", knownRecords: "3", unknownRecords: "1", truncatedKnownRecords: "1", knownBodyBytes: "16", unrepresentedKnownBytes: "6", unattributedHeads: "0",
    truncatedKnownShare: {numerator: "1", denominator: "3"}, unrepresentedKnownByteShare: {numerator: "6", denominator: "16"}};
}

function corrupt(value: unknown, path: string): void {
  assert.throws(() => decodeStoredHeadCoverage(value), error => error instanceof ProjectionGenerationError && error.reason === "projection_generation_corrupt" && error.fieldPath === `/vectorCoverage/${path}`);
}

test("s90 exact i64/u128 decimal transport and numeric dimensions", () => {
  const value = native();
  Object.assign(value, {records: "9223372036854775807", knownRecords: "9223372036854775806", unknownRecords: "1", truncatedKnownRecords: "9223372036854775805",
    knownBodyBytes: "340282366920938463463374607431768211455", unrepresentedKnownBytes: "340282366920938463463374607431768211454", unattributedHeads: "9223372036854775807",
    truncatedKnownShare: {numerator: "9223372036854775805", denominator: "9223372036854775806"},
    unrepresentedKnownByteShare: {numerator: "340282366920938463463374607431768211454", denominator: "340282366920938463463374607431768211455"}});
  const result = decodeStoredHeadCoverage(value);
  assert.equal(result.status, "available");
  if (result.status !== "available") assert.fail("available variant");
  assert.equal(result.records, "9223372036854775807");
  assert.equal(result.knownBodyBytes, "340282366920938463463374607431768211455");
  assert.equal(result.profile.dimension, 8);
  assert.equal(typeof result.profile.dimension, "number");
  assert.deepEqual(JSON.parse(JSON.stringify(result)), result);
  for (const reason of ["cache_cold", "state_changed"]) {
    const result = decodeStoredHeadCoverage({status: "unavailable", population: "stored_mapped_spans", profile: value["profile"], reason});
    assert.equal(result.status, "unavailable");
    assert.equal(result.profile.dimension, 8);
    assert.equal("records" in result, false);
    assert.equal("truncatedKnownShare" in result, false);
  }
});

test("s90 malformed native values name exact RFC6901 paths", () => {
  const cases: [string, unknown][] = [
    ["population", "live_view"], ["status", "ready"], ["reason", "cache_cold"], ["profile/key", "other"], ["profile/dimension", "8"], ["profile/dimension", 0], ["profile/dimension", 2**32],
    ["records", "5"], ["knownRecords", 3], ["knownRecords", "03"], ["knownRecords", "+3"], ["knownRecords", "-1"], ["knownRecords", "3.0"], ["knownRecords", "9223372036854775808"],
    ["knownBodyBytes", "340282366920938463463374607431768211456"], ["truncatedKnownRecords", "4"], ["unrepresentedKnownBytes", "17"],
    ["truncatedKnownShare/numerator", "0"], ["truncatedKnownShare/denominator", "1"], ["unrepresentedKnownByteShare/denominator", "0"],
    ["truncatedKnownShare", null], ["unrepresentedKnownByteShare", null], ["profile", null],
  ];
  for (const [path, replacement] of cases) {
    const value = native();
    const pieces = path.split("/");
    const key = pieces.pop()!;
    let owner = value;
    for (const piece of pieces) owner = owner[piece] as Record<string, unknown>;
    owner[key] = replacement;
    corrupt(value, path);
  }
  for (const path of ["knownRecords", "profile/revision", "truncatedKnownShare/numerator"]) {
    const value = native();
    const pieces = path.split("/"); const key = pieces.pop()!;
    let owner = value; for (const piece of pieces) owner = owner[piece] as Record<string, unknown>;
    delete owner[key]; corrupt(value, path);
  }
  corrupt({status: "unavailable", population: "stored_mapped_spans", profile: native()["profile"], reason: "other"}, "reason");
  corrupt({status: "unavailable", population: "stored_mapped_spans", profile: native()["profile"], reason: "cache_cold", records: "0"}, "records");
});

test("s90 scored numeric carrier preserves zero and rejects malformed native values", () => {
  const snippet: NativeSnippetV1 = {text: "", anchor: "whole", windows: [], spans: [], highlights: [], cut: false};
  for (const value of [undefined, null, 0, 0xffffffff]) {
    const got = mapNativeSnippets([{...snippet, scoredBytes: value}]).snippets![0]!;
    assert.equal(got.scoredBytes, value ?? undefined);
    if (value == null) assert.equal("scoredBytes" in got, false);
  }
  for (const value of [-1, 2**32, 1.5, Number.NaN, Number.POSITIVE_INFINITY, "0", true]) {
    assert.throws(() => mapNativeSnippets([{...snippet, scoredBytes: value} as NativeSnippetV1]), error => error instanceof ProjectionGenerationError && error.fieldPath === "/scoredBytes");
  }
});
