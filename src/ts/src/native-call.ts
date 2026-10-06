// Private native-call owner for the TypeScript SDK package root.

import { rethrowTyped } from "./errors.js";

export async function intercept<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (err) {
    rethrowTyped(err);
  }
}

export function interceptSync<T>(fn: () => T): T {
  try {
    return fn();
  } catch (err) {
    rethrowTyped(err);
  }
}
