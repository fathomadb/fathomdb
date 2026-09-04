// TS-side FFI string guard (AC-068a / AC-068b).
//
// napi-rs converts JS strings to Rust `String` via NAPI's UTF-8 path,
// which silently replaces lone UTF-16 surrogates with U+FFFD; the
// Rust-side guard never sees them. We catch them in TS BEFORE the
// native call so the no-row-written invariant holds end-to-end.

import { WriteValidationError } from "./errors.js";

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

/** Validate write strings without preempting provenance-specific NUL errors. */
export function validateWriteFfiTree(batch: unknown[]): void {
  for (const item of batch) {
    if (item === null || typeof item !== "object" || Array.isArray(item)) {
      validateFfiTreeEncoding(item, false);
      continue;
    }
    for (const [key, value] of Object.entries(item as Record<string, unknown>)) {
      validateFfiTreeEncoding(value, key === "provenance");
    }
  }
}
