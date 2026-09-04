// TS-side FFI string guard (AC-068a / AC-068b).
//
// napi-rs converts JS strings to Rust `String` via NAPI's UTF-8 path,
// which silently replaces lone UTF-16 surrogates with U+FFFD; the
// Rust-side guard never sees them. We catch them in TS BEFORE the
// native call so the no-row-written invariant holds end-to-end.

import { ActuationError, ProvenanceError, WriteValidationError } from "./errors.js";

export function validateFfiString(value: string): void {
  validateFfiStringEncoding(value, false);
}

function validateFfiStringEncoding(value: string, allowNul: boolean): void {
  for (let i = 0; i < value.length; i++) {
    const code = value.charCodeAt(i);
    if (code === 0 && !allowNul) {
      throw new WriteValidationError("embedded NUL byte in FFI string");
    }
    if (code >= 0xd800 && code <= 0xdbff) {
      if (i + 1 >= value.length) {
        throw new WriteValidationError("unpaired UTF-16 high surrogate in FFI string");
      }
      const next = value.charCodeAt(i + 1);
      if (next < 0xdc00 || next > 0xdfff) {
        throw new WriteValidationError("unpaired UTF-16 high surrogate in FFI string");
      }
      i++;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      throw new WriteValidationError("unpaired UTF-16 low surrogate in FFI string");
    }
  }
}

export function validateFfiTree(value: unknown): void {
  validateFfiTreeEncoding(value, false);
}

function escapeJsonPointerToken(value: string): string {
  return value.replaceAll("~", "~0").replaceAll("/", "~1");
}

function actuationEncodingError(fieldPath: string): ActuationError {
  return new ActuationError(
    `actuation nested_request_invalid at ${fieldPath}`,
    "nested_request_invalid",
    fieldPath,
  );
}

function validateActuationString(value: string, fieldPath: string, allowNul: boolean): void {
  for (let index = 0; index < value.length; index++) {
    const code = value.charCodeAt(index);
    if (code === 0 && !allowNul) throw actuationEncodingError(fieldPath);
    if (code >= 0xd800 && code <= 0xdbff) {
      if (index + 1 >= value.length) throw actuationEncodingError(fieldPath);
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) throw actuationEncodingError(fieldPath);
      index++;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      throw actuationEncodingError(fieldPath);
    }
  }
}

function validateActuationValue(value: unknown, fieldPath: string): void {
  if (typeof value === "string") {
    const allowNul = /^\/operations\/\d+\/record\/sourceId$/.test(fieldPath);
    validateActuationString(value, fieldPath, allowNul);
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => validateActuationValue(item, `${fieldPath}/${index}`));
    return;
  }
  if (value !== null && typeof value === "object") {
    for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
      const canonicalKey = key === "source_id" ? "sourceId" : key;
      validateActuationValue(nested, `${fieldPath}/${escapeJsonPointerToken(canonicalKey)}`);
    }
  }
}

/** Guard all actuation strings before napi-rs can replace malformed UTF-16. */
export function validateActuationFfiTree(request: unknown): void {
  validateActuationValue(request, "");
}

function validateFfiTreeEncoding(value: unknown, allowNul: boolean): void {
  if (typeof value === "string") {
    validateFfiStringEncoding(value, allowNul);
    return;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      validateFfiTreeEncoding(item, allowNul);
    }
    return;
  }
  if (value !== null && typeof value === "object") {
    for (const v of Object.values(value as Record<string, unknown>)) {
      validateFfiTreeEncoding(v, allowNul);
    }
  }
}

function provenanceEncodingError(reason: string, fieldPath: string): ProvenanceError {
  return new ProvenanceError(`provenance ${reason} at ${fieldPath}`, reason, fieldPath);
}

function validateProvenanceString(
  value: unknown,
  reason: string,
  fieldPath: string,
): value is string {
  if (typeof value !== "string") return false;
  try {
    validateFfiString(value);
  } catch (error) {
    if (error instanceof WriteValidationError) {
      throw provenanceEncodingError(reason, fieldPath);
    }
    throw error;
  }
  return true;
}

function hasOnlyKeys(value: Record<string, unknown>, allowed: readonly string[]): boolean {
  return Object.keys(value).every((key) => allowed.includes(key));
}

function isCallerIdentity(value: string): boolean {
  return (
    /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(value) && !value.startsWith("_fdb:")
  );
}

function isDecimalOffset(value: string): boolean {
  if (!/^(0|[1-9][0-9]*)$/.test(value)) return false;
  try {
    return BigInt(value) <= 9223372036854775807n;
  } catch {
    return false;
  }
}

function validateProvenanceFfiTree(value: unknown): void {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return;
  const provenance = value as Record<string, unknown>;
  if (provenance.schemaVersion !== 1) return;
  if (provenance.role !== "canonical" && provenance.role !== "derived") return;
  if (
    !hasOnlyKeys(provenance, [
      "schemaVersion",
      "role",
      "artifactRevisionId",
      "sourceVersionId",
      "sourceRevisionId",
      "sourceLocator",
      "canonicalSourceHash",
    ])
  ) {
    return;
  }

  if (
    !validateProvenanceString(
      provenance.artifactRevisionId,
      "revision_id_invalid",
      "/provenance/artifactRevisionId",
    ) ||
    !isCallerIdentity(provenance.artifactRevisionId)
  ) {
    return;
  }
  if (
    !validateProvenanceString(
      provenance.sourceVersionId,
      "source_version_invalid",
      "/provenance/sourceVersionId",
    ) ||
    !isCallerIdentity(provenance.sourceVersionId)
  ) {
    return;
  }

  if (provenance.role === "canonical") return;

  if (
    !validateProvenanceString(
      provenance.sourceRevisionId,
      "revision_id_invalid",
      "/provenance/sourceRevisionId",
    ) ||
    !isCallerIdentity(provenance.sourceRevisionId)
  ) {
    return;
  }

  const locator = provenance.sourceLocator;
  if (locator === null || typeof locator !== "object" || Array.isArray(locator)) return;
  const locatorObject = locator as Record<string, unknown>;
  if (
    !validateProvenanceString(
      locatorObject.kind,
      "locator_invalid",
      "/provenance/sourceLocator/kind",
    )
  ) {
    return;
  }
  if (locatorObject.kind === "whole_body") {
    if (!hasOnlyKeys(locatorObject, ["kind"])) return;
  } else if (locatorObject.kind === "utf8_bytes") {
    if (!hasOnlyKeys(locatorObject, ["kind", "startInclusive", "endExclusive"])) return;
    if (
      !validateProvenanceString(
        locatorObject.startInclusive,
        "locator_invalid",
        "/provenance/sourceLocator/startInclusive",
      ) ||
      !isDecimalOffset(locatorObject.startInclusive)
    ) {
      return;
    }
    if (
      !validateProvenanceString(
        locatorObject.endExclusive,
        "locator_invalid",
        "/provenance/sourceLocator/endExclusive",
      ) ||
      !isDecimalOffset(locatorObject.endExclusive)
    ) {
      return;
    }
  } else {
    return;
  }

  const hash = provenance.canonicalSourceHash;
  if (hash === null || typeof hash !== "object" || Array.isArray(hash)) return;
  const hashObject = hash as Record<string, unknown>;
  if (!hasOnlyKeys(hashObject, ["algorithm", "digestHex"])) return;
  if (
    !validateProvenanceString(
      hashObject.algorithm,
      "hash_invalid",
      "/provenance/canonicalSourceHash/algorithm",
    ) ||
    hashObject.algorithm !== "sha256"
  ) {
    return;
  }
  validateProvenanceString(
    hashObject.digestHex,
    "hash_invalid",
    "/provenance/canonicalSourceHash/digestHex",
  );
}

function validateWriteEntityFfiTree(value: unknown): void {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    validateFfiTreeEncoding(value, false);
    return;
  }
  for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
    if (key === "provenance") {
      validateProvenanceFfiTree(nested);
    } else if (key === "sourceId" || key === "source_id") {
      validateFfiTreeEncoding(nested, true);
    } else {
      validateFfiTreeEncoding(nested, false);
    }
  }
}

/** Validate direct and wrapped write strings with provenance-specific errors. */
export function validateWriteFfiTree(batch: unknown[]): void {
  for (const item of batch) {
    if (item === null || typeof item !== "object" || Array.isArray(item)) {
      validateFfiTreeEncoding(item, false);
      continue;
    }
    for (const [key, value] of Object.entries(item as Record<string, unknown>)) {
      if (key === "node" || key === "edge") {
        validateWriteEntityFfiTree(value);
      } else if (key === "provenance") {
        validateProvenanceFfiTree(value);
      } else if (key === "sourceId" || key === "source_id") {
        validateFfiTreeEncoding(value, true);
      } else {
        validateFfiTreeEncoding(value, false);
      }
    }
  }
}
