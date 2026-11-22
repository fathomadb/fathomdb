// Private typed property & lossless 64-bit integer codec for the TypeScript SDK.

import type { NativePropertyEntry, NativePropertyPredicateInput } from "./binding.js";
import { InvalidFilterError, WriteValidationError } from "./errors.js";
import { validateFfiString } from "./validation.js";

/** Scalar value stored in an inline property map or predicate. */
export type PropertyValue = string | number | boolean | bigint;

/** Canonical typed property map keyed by property name. */
export type PropertyMap = Record<string, PropertyValue>;

/** Entity class targeted by a declared property projection (`"node"` or `"edge"`). */
export type PropertyEntityClass = "node" | "edge";

/** Closed scalar comparison operator for {@link PropertyPredicate}. */
export type PropertyComparisonOp = "eq" | "ne" | "lt" | "lte" | "gt" | "gte";

/** Closed typed property predicate evaluated over declared properties. */
export interface PropertyPredicate {
  key: string;
  op: PropertyComparisonOp;
  value: PropertyValue;
}

const I64_MIN = -(1n << 63n);
const I64_MAX = (1n << 63n) - 1n;
const MIN_SAFE_BIGINT = BigInt(Number.MIN_SAFE_INTEGER);
const MAX_SAFE_BIGINT = BigInt(Number.MAX_SAFE_INTEGER);
const VALID_OPS = new Set<string>(["eq", "ne", "lt", "lte", "gt", "gte"]);
const ORDER_OPS = new Set<string>(["lt", "lte", "gt", "gte"]);

function encodePropertyScalar(
  value: unknown,
  asFilter: boolean,
): Omit<NativePropertyEntry, "key"> {
  const fail = (msg: string): never => {
    throw asFilter ? new InvalidFilterError(msg) : new WriteValidationError(msg);
  };
  if (typeof value === "boolean") {
    return { valueType: "bool", boolVal: value };
  }
  if (typeof value === "bigint") {
    if (value < I64_MIN || value > I64_MAX) {
      fail("integer property value out of signed 64-bit range");
    }
    return { valueType: "int", intVal: value.toString() };
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      fail("float property value must be finite");
    }
    if (Object.is(value, -0)) {
      return { valueType: "float", floatVal: 0 };
    }
    if (Number.isSafeInteger(value)) {
      return { valueType: "int", intVal: String(value) };
    }
    return { valueType: "float", floatVal: value };
  }
  if (typeof value === "string") {
    validateFfiString(value);
    if (value.includes("\x1e")) {
      fail("property string value must not contain record separator bytes");
    }
    return { valueType: "string", stringVal: value };
  }
  return fail("property value must be string, number, bigint, or boolean");
}

/**
 * Validate and encode an optional caller `properties` object into a JSON-safe
 * lossless wire envelope for `nativeEngine.write` / `nativeEngine.actuate`.
 */
export function encodePropertiesForWrite(props: unknown): unknown {
  if (props === undefined || props === null) return null;
  if (typeof props !== "object" || Array.isArray(props)) {
    throw new WriteValidationError("properties must be an object or null");
  }
  const entries: NativePropertyEntry[] = [];
  for (const [key, val] of Object.entries(props as Record<string, unknown>)) {
    validateFfiString(key);
    if (key.length === 0) {
      throw new WriteValidationError("property key must not be empty");
    }
    if (key.includes("\x1e")) {
      throw new WriteValidationError("property key must not contain record separator bytes");
    }
    const encoded = encodePropertyScalar(val, false);
    entries.push({ key, ...encoded });
  }
  if (entries.length === 0) return null;
  return { __fdb_props_v1__: entries };
}

/** Encode one write batch item, translating any `properties` map losslessly. */
export function prepareWriteBatchItem(item: unknown): unknown {
  if (item === null || typeof item !== "object" || Array.isArray(item)) return item;
  const obj = item as Record<string, unknown>;
  if ("node" in obj && obj.node !== null && typeof obj.node === "object" && !Array.isArray(obj.node)) {
    const inner = obj.node as Record<string, unknown>;
    if ("properties" in inner) {
      return { ...obj, node: { ...inner, properties: encodePropertiesForWrite(inner.properties) } };
    }
    return item;
  }
  if ("edge" in obj && obj.edge !== null && typeof obj.edge === "object" && !Array.isArray(obj.edge)) {
    const inner = obj.edge as Record<string, unknown>;
    if ("properties" in inner) {
      return { ...obj, edge: { ...inner, properties: encodePropertiesForWrite(inner.properties) } };
    }
    return item;
  }
  if ("properties" in obj) {
    return { ...obj, properties: encodePropertiesForWrite(obj.properties) };
  }
  return item;
}

/** Encode one actuation batch, translating any record `properties` map losslessly. */
export function prepareActuationBatch(request: unknown): unknown {
  if (request === null || typeof request !== "object" || Array.isArray(request)) return request;
  const req = request as Record<string, unknown>;
  if (!Array.isArray(req.operations)) return request;
  let changed = false;
  const operations = req.operations.map((op) => {
    if (op === null || typeof op !== "object" || Array.isArray(op)) return op;
    const opObj = op as Record<string, unknown>;
    if (
      opObj.record !== null &&
      typeof opObj.record === "object" &&
      !Array.isArray(opObj.record) &&
      "properties" in (opObj.record as Record<string, unknown>)
    ) {
      changed = true;
      const rec = opObj.record as Record<string, unknown>;
      return {
        ...opObj,
        record: { ...rec, properties: encodePropertiesForWrite(rec.properties) },
      };
    }
    return op;
  });
  return changed ? { ...req, operations } : request;
}

/** Decode lossless native `PropertyEntry[]` into a JS `PropertyMap | null`. */
export function decodeNativeProperties(
  entries: NativePropertyEntry[] | null | undefined,
): PropertyMap | null {
  if (!entries || entries.length === 0) return null;
  const out: PropertyMap = {};
  for (const entry of entries) {
    switch (entry.valueType) {
      case "string":
        out[entry.key] = entry.stringVal ?? "";
        break;
      case "bool":
        out[entry.key] = Boolean(entry.boolVal);
        break;
      case "float": {
        const f = entry.floatVal ?? 0;
        out[entry.key] = Object.is(f, -0) ? 0 : f;
        break;
      }
      case "int": {
        const bi = BigInt(entry.intVal ?? "0");
        out[entry.key] =
          bi >= MIN_SAFE_BIGINT && bi <= MAX_SAFE_BIGINT ? Number(bi) : bi;
        break;
      }
      default:
        break;
    }
  }
  return Object.keys(out).length === 0 ? null : out;
}

/** Validate and encode a public `PropertyPredicate` into N-API input shape. */
export function encodePropertyPredicate(pred: PropertyPredicate): NativePropertyPredicateInput {
  if (typeof pred !== "object" || pred === null || Array.isArray(pred)) {
    throw new InvalidFilterError("property predicate must be an object");
  }
  if (typeof pred.key !== "string") {
    throw new InvalidFilterError("property predicate key must be a string");
  }
  validateFfiString(pred.key);
  if (pred.key.length === 0 || pred.key.includes("\x1e")) {
    throw new InvalidFilterError(
      "property predicate key must be non-empty and contain no record separator bytes",
    );
  }
  if (typeof pred.op !== "string" || !VALID_OPS.has(pred.op)) {
    throw new InvalidFilterError(
      `unknown property predicate op ${JSON.stringify(pred.op)}; expected eq/ne/lt/lte/gt/gte`,
    );
  }
  if (typeof pred.value === "boolean" && ORDER_OPS.has(pred.op)) {
    throw new InvalidFilterError("ordering operators are invalid for boolean properties");
  }
  const scalar = encodePropertyScalar(pred.value, true);
  return {
    key: pred.key,
    op: pred.op,
    ...scalar,
  };
}

/** Decode native `PropertyPredicateInput[]` back into public `PropertyPredicate[]`. */
export function decodeNativePropertyPredicates(
  preds: NativePropertyPredicateInput[] | null | undefined,
): PropertyPredicate[] | undefined {
  if (!preds || preds.length === 0) return undefined;
  return preds.map((p) => {
    let value: PropertyValue;
    switch (p.valueType) {
      case "string":
        value = p.stringVal ?? "";
        break;
      case "bool":
        value = Boolean(p.boolVal);
        break;
      case "float": {
        const f = p.floatVal ?? 0;
        value = Object.is(f, -0) ? 0 : f;
        break;
      }
      case "int": {
        const bi = BigInt(p.intVal ?? "0");
        value = bi >= MIN_SAFE_BIGINT && bi <= MAX_SAFE_BIGINT ? Number(bi) : bi;
        break;
      }
      default:
        value = p.stringVal ?? "";
        break;
    }
    return {
      key: p.key,
      op: p.op as PropertyComparisonOp,
      value,
    };
  });
}

/**
 * Parse JSON while preserving exact 64-bit signed integers outside
 * `Number.MIN_SAFE_INTEGER..=Number.MAX_SAFE_INTEGER` as `bigint` and
 * normalizing `-0` to `0`.
 */
export function parseLosslessJson(text: string): unknown {
  return JSON.parse(text, ((
    _key: string,
    value: unknown,
    context?: { source?: string },
  ): unknown => {
    if (typeof value === "number") {
      if (Object.is(value, -0)) return 0;
      const src = context?.source;
      if (
        typeof src === "string" &&
        /^-?(?:0|[1-9][0-9]*)$/.test(src) &&
        !Number.isSafeInteger(value)
      ) {
        const bi = BigInt(src);
        if (bi >= I64_MIN && bi <= I64_MAX) return bi;
      }
    }
    return value;
  }) as (this: unknown, key: string, value: unknown) => unknown);
}

/**
 * Serialize JSON-compatible values including `bigint` (exact JSON integer
 * digits) and normalizing `-0` to `0`.
 */
export function stringifyLosslessJson(value: unknown): string {
  if (value === null) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "bigint") return value.toString();
  if (typeof value === "number") {
    if (!Number.isFinite(value)) return "null";
    return Object.is(value, -0) ? "0" : String(value);
  }
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) {
    return `[${value.map((item) => (item === undefined ? "null" : stringifyLosslessJson(item))).join(",")}]`;
  }
  if (typeof value === "object") {
    const parts: string[] = [];
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      if (v === undefined) continue;
      parts.push(`${JSON.stringify(k)}:${stringifyLosslessJson(v)}`);
    }
    return `{${parts.join(",")}}`;
  }
  return "null";
}

/** Validate an optional JSON-decoded property map inside graph / evidence wire responses. */
export function validateWireProperties(
  value: unknown,
  path: string,
  refuse: (reason: string, fieldPath: string) => never,
): PropertyMap | undefined {
  if (value === undefined) return undefined;
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    refuse("graph_corrupt", path);
  }
  const out: PropertyMap = {};
  for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
    if (k.length === 0 || k.includes("\0") || k.includes("\x1e")) {
      refuse("graph_corrupt", path);
    }
    if (typeof v === "boolean") {
      out[k] = v;
    } else if (typeof v === "bigint") {
      if (v < I64_MIN || v > I64_MAX) refuse("graph_corrupt", `${path}/${k}`);
      out[k] = v;
    } else if (typeof v === "number") {
      if (!Number.isFinite(v)) refuse("graph_corrupt", `${path}/${k}`);
      out[k] = Object.is(v, -0) ? 0 : v;
    } else if (typeof v === "string") {
      if (v.includes("\0") || v.includes("\x1e")) refuse("graph_corrupt", `${path}/${k}`);
      out[k] = v;
    } else {
      refuse("graph_corrupt", `${path}/${k}`);
    }
  }
  return Object.keys(out).length === 0 ? undefined : out;
}
