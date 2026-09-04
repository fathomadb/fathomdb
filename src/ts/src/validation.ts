// TS-side FFI string guard (AC-068a / AC-068b).
//
// napi-rs converts JS strings to Rust `String` via NAPI's UTF-8 path,
// which silently replaces lone UTF-16 surrogates with U+FFFD; the
// Rust-side guard never sees them. We catch them in TS BEFORE the
// native call so the no-row-written invariant holds end-to-end.

import { ProvenanceError, WriteValidationError } from "./errors.js";

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
): void {
  if (typeof value !== "string") return;
  try {
    validateFfiString(value);
  } catch (error) {
    if (error instanceof WriteValidationError) {
      throw provenanceEncodingError(reason, fieldPath);
    }
    throw error;
  }
}

function validateProvenanceFfiTree(value: unknown): void {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return;
  const provenance = value as Record<string, unknown>;
  if (provenance.schemaVersion !== 1) return;
  if (provenance.role !== "canonical" && provenance.role !== "derived") return;

  validateProvenanceString(
    provenance.artifactRevisionId,
    "revision_id_invalid",
    "/provenance/artifactRevisionId",
  );
  validateProvenanceString(
    provenance.sourceVersionId,
    "source_version_invalid",
    "/provenance/sourceVersionId",
  );
  validateProvenanceString(
    provenance.sourceRevisionId,
    "revision_id_invalid",
    "/provenance/sourceRevisionId",
  );

  const locator = provenance.sourceLocator;
  if (locator !== null && typeof locator === "object" && !Array.isArray(locator)) {
    for (const key of ["kind", "startInclusive", "endExclusive"]) {
      validateProvenanceString(
        (locator as Record<string, unknown>)[key],
        "locator_invalid",
        `/provenance/sourceLocator/${key}`,
      );
    }
  }
  const hash = provenance.canonicalSourceHash;
  if (hash !== null && typeof hash === "object" && !Array.isArray(hash)) {
    for (const key of ["algorithm", "digestHex"]) {
      validateProvenanceString(
        (hash as Record<string, unknown>)[key],
        "hash_invalid",
        `/provenance/canonicalSourceHash/${key}`,
      );
    }
  }
}

function validateWriteEntityFfiTree(value: unknown): void {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    validateFfiTreeEncoding(value, false);
    return;
  }
  for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
    if (key === "provenance") {
      validateProvenanceFfiTree(nested);
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
      } else {
        validateFfiTreeEncoding(value, false);
      }
    }
  }
}
