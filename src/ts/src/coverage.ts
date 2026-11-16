import { ProjectionGenerationError } from "./errors.js";

/** Stored default profile; dimension is numeric, separate from exact counters. */
export interface StoredHeadCoverageProfile {
  key: "default";
  name: string;
  revision: string;
  dimension: number;
}

/** Exact canonical decimal numerator and positive denominator, unreduced. */
export interface StoredHeadCoverageFraction {
  numerator: string;
  denominator: string;
}

/** Exact committed mapped-head facts, including retained historical records. */
export interface StoredHeadCoverageAvailable {
  status: "available";
  population: "stored_mapped_spans";
  profile: StoredHeadCoverageProfile;
  records: string;
  knownRecords: string;
  unknownRecords: string;
  truncatedKnownRecords: string;
  knownBodyBytes: string;
  unrepresentedKnownBytes: string;
  unattributedHeads: string;
  truncatedKnownShare: StoredHeadCoverageFraction | null;
  unrepresentedKnownByteShare: StoredHeadCoverageFraction | null;
}

/** No matching committed capture; contains no numeric coverage fields. */
export interface StoredHeadCoverageUnavailable {
  status: "unavailable";
  population: "stored_mapped_spans";
  profile: StoredHeadCoverageProfile;
  reason: "cache_cold" | "state_changed";
}

/** Metadata coverage, independent of current eligibility or physical health. */
export type StoredHeadCoverageV1 = StoredHeadCoverageAvailable | StoredHeadCoverageUnavailable;

/** Decode exact native metadata, refusing corruption at its RFC6901 field path. */
export function decodeStoredHeadCoverage(value: unknown): StoredHeadCoverageV1 {
  const prefix = "/vectorCoverage";
  function invalid(path: string): never {
    throw new ProjectionGenerationError(`projection generation projection_generation_corrupt at ${path}`, "projection_generation_corrupt", path);
  }
  function object(raw: unknown, path: string): Record<string, unknown> {
    if (raw === null || typeof raw !== "object" || Array.isArray(raw)) invalid(path);
    return raw as Record<string, unknown>;
  }
  function decimal(raw: unknown, path: string, maximum: bigint): string {
    if (typeof raw !== "string" || !/^(0|[1-9][0-9]*)$/.test(raw)) invalid(path);
    if (raw.length > 39 || BigInt(raw) > maximum) invalid(path);
    return raw;
  }
  const raw = object(value, prefix);
  if (raw["population"] !== "stored_mapped_spans") invalid(`${prefix}/population`);
  const p = object(raw["profile"], `${prefix}/profile`);
  if (p["key"] !== "default") invalid(`${prefix}/profile/key`);
  if (typeof p["name"] !== "string" || p["name"].length === 0) invalid(`${prefix}/profile/name`);
  if (typeof p["revision"] !== "string" || p["revision"].length === 0) invalid(`${prefix}/profile/revision`);
  const dimension = p["dimension"];
  if (typeof dimension !== "number" || !Number.isInteger(dimension) || dimension <= 0 || dimension > 0xffffffff) invalid(`${prefix}/profile/dimension`);
  const profile: StoredHeadCoverageProfile = {key: "default", name: p["name"], revision: p["revision"], dimension};
  const names = ["records", "knownRecords", "unknownRecords", "truncatedKnownRecords", "knownBodyBytes", "unrepresentedKnownBytes", "unattributedHeads"] as const;
  const shares = ["truncatedKnownShare", "unrepresentedKnownByteShare"] as const;
  if (raw["status"] === "unavailable") {
    const reason = raw["reason"];
    if (reason !== "cache_cold" && reason !== "state_changed") invalid(`${prefix}/reason`);
    for (const name of [...names, ...shares]) if (raw[name] != null) invalid(`${prefix}/${name}`);
    return {status: "unavailable", population: "stored_mapped_spans", profile, reason};
  }
  if (raw["status"] !== "available") invalid(`${prefix}/status`);
  if (raw["reason"] != null) invalid(`${prefix}/reason`);
  const u128 = (1n << 128n) - 1n;
  const counts = Object.fromEntries(names.map(name => [name, decimal(raw[name], `${prefix}/${name}`, name === "knownBodyBytes" || name === "unrepresentedKnownBytes" ? u128 : (1n << 63n) - 1n)])) as Record<typeof names[number], string>;
  if (BigInt(counts.records) !== BigInt(counts.knownRecords) + BigInt(counts.unknownRecords)) invalid(`${prefix}/records`);
  if (BigInt(counts.truncatedKnownRecords) > BigInt(counts.knownRecords)) invalid(`${prefix}/truncatedKnownRecords`);
  if (BigInt(counts.unrepresentedKnownBytes) > BigInt(counts.knownBodyBytes)) invalid(`${prefix}/unrepresentedKnownBytes`);
  function fraction(name: typeof shares[number], numerator: string, denominator: string): StoredHeadCoverageFraction | null {
    const path = `${prefix}/${name}`;
    if (denominator === "0") {
      if (raw[name] != null) invalid(path);
      return null;
    }
    const share = object(raw[name], path);
    const a = decimal(share["numerator"], `${path}/numerator`, u128);
    const b = decimal(share["denominator"], `${path}/denominator`, u128);
    if (a !== numerator) invalid(`${path}/numerator`);
    if (b !== denominator) invalid(`${path}/denominator`);
    return {numerator: a, denominator: b};
  }
  return {status: "available", population: "stored_mapped_spans", profile, ...counts,
    truncatedKnownShare: fraction("truncatedKnownShare", counts.truncatedKnownRecords, counts.knownRecords),
    unrepresentedKnownByteShare: fraction("unrepresentedKnownByteShare", counts.unrepresentedKnownBytes, counts.knownBodyBytes)};
}
