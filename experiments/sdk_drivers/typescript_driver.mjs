#!/usr/bin/env node
// Fixed one-repetition TypeScript SDK benchmark boundary. stdout is protocol only.

import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { basename, isAbsolute } from 'node:path';

const require = createRequire(import.meta.url);
const protocol = 'sdk.driver.v1';
const operations = ['engine.search_text_only', 'read.get', 'graph.neighbors'];
const packageJson = new URL('../../src/ts/package.json', import.meta.url);
const packageEntry = new URL('../../src/ts/dist/index.js', import.meta.url);

class DriverFailure extends Error {
  constructor(code) {
    super(code);
    this.name = 'DriverFailure';
    this.code = code;
  }
}

function object(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function keys(value, allowed) {
  if (!object(value) || Object.keys(value).some((key) => !allowed.includes(key))) {
    throw new DriverFailure('invalid_request');
  }
}

function safeInteger(value, low, high) {
  if (!Number.isSafeInteger(value) || value < low || value > high) {
    throw new DriverFailure('invalid_request');
  }
  return value;
}

function validate(request) {
  keys(request, [
    'schema_version', 'mode', 'sdk', 'operation_id', 'operation_version',
    'database_path', 'queries', 'arguments', 'warmup_per_query', 'timeout_ms',
    'expected_native_sha256',
  ]);
  if (request.schema_version !== protocol || request.mode !== 'run-repetition' ||
      request.sdk !== 'typescript' || !operations.includes(request.operation_id) ||
      request.operation_version !== 1) {
    throw new DriverFailure('unsupported_operation');
  }
  if (typeof request.database_path !== 'string' || !isAbsolute(request.database_path)) {
    throw new DriverFailure('invalid_request');
  }
  const argumentFields = request.operation_id === 'engine.search_text_only'
    ? ['limit', 'view']
    : request.operation_id === 'graph.neighbors' ? ['depth', 'view'] : ['view'];
  keys(request.arguments, argumentFields);
  keys(request.arguments.view, ['valid_as_of']);
  if (request.operation_id === 'engine.search_text_only') {
    safeInteger(request.arguments.limit, 1, 100);
  }
  if (request.operation_id === 'graph.neighbors') {
    safeInteger(request.arguments.depth, 1, 3);
  }
  safeInteger(request.arguments.view.valid_as_of, 0, Number.MAX_SAFE_INTEGER);
  safeInteger(request.warmup_per_query, 0, 10000);
  safeInteger(request.timeout_ms, 1, 3600000);
  if (typeof request.expected_native_sha256 !== 'string' ||
      !/^[0-9a-f]{64}$/.test(request.expected_native_sha256)) {
    throw new DriverFailure('invalid_request');
  }
  if (!Array.isArray(request.queries) || request.queries.length === 0 || request.queries.length > 10000) {
    throw new DriverFailure('invalid_request');
  }
  const ids = new Set();
  for (const query of request.queries) {
    keys(query, ['id', 'text']);
    if (typeof query.id !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$/.test(query.id) ||
        ids.has(query.id) || typeof query.text !== 'string' || query.text.length === 0) {
      throw new DriverFailure('invalid_request');
    }
    ids.add(query.id);
  }
}

function loadedNative() {
  const paths = Object.keys(require.cache).filter((path) => /^fathomdb\.[a-z0-9-]+\.node$/.test(basename(path)));
  if (paths.length !== 1) {
    throw new DriverFailure('native_binding_ambiguous');
  }
  return createHash('sha256').update(readFileSync(paths[0])).digest('hex');
}

function normalizeIds(result) {
  if (!object(result) || !Array.isArray(result.results)) {
    throw new DriverFailure('invalid_sdk_result');
  }
  const prefix = { logical: 'l:', content: 'h:', passage: 'p:' };
  return result.results.map((hit) => {
    if (!object(hit) || !object(hit.id) || !Object.hasOwn(prefix, hit.id.space) ||
        typeof hit.id.value !== 'string') {
      throw new DriverFailure('invalid_sdk_result');
    }
    return prefix[hit.id.space] + hit.id.value;
  });
}

function normalizeNode(node) {
  if (!object(node) || typeof node.logicalId !== 'string' || node.logicalId.length === 0) {
    throw new DriverFailure('invalid_sdk_result');
  }
  return `l:${node.logicalId}`;
}

function normalizeResult(operationId, result) {
  if (operationId === 'engine.search_text_only') return normalizeIds(result);
  if (operationId === 'read.get') return result === null ? [] : [normalizeNode(result)];
  if (!Array.isArray(result)) throw new DriverFailure('invalid_sdk_result');
  return result.map(normalizeNode);
}

function publicCall(sdk, engine, request, options, query) {
  switch (request.operation_id) {
    case 'engine.search_text_only': return engine.searchTextOnly(query.text, options);
    case 'read.get': return sdk.read.get(engine, query.text, options);
    case 'graph.neighbors': return sdk.graph.neighbors(engine, query.text, options.depth, 'both', options.view);
    default: throw new DriverFailure('unsupported_operation');
  }
}

function typedError(error) {
  if (error instanceof DriverFailure) return { code: error.code, type: error.name };
  if (error instanceof TypeError || error instanceof RangeError) return { code: error.name, type: error.name };
  if (error instanceof Error) return { code: error.name || 'sdk_error', type: 'sdk_error' };
  return { code: 'unknown_error', type: 'unknown_error' };
}

async function bounded(call, deadline) {
  const remaining = deadline - process.hrtime.bigint();
  if (remaining <= 0n) throw new DriverFailure('timeout');
  let timer;
  try {
    return await Promise.race([
      Promise.resolve().then(call),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new DriverFailure('timeout')),
          Math.max(1, Number(remaining / 1000000n)));
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

function baseResponse(request) {
  return {
    schema_version: protocol,
    status: 'blocked',
    sdk: 'typescript',
    operation_id: request?.operation_id ?? null,
    operation_version: request?.operation_version ?? null,
    identity: { sdk_version: null, runtime: process.version, native_sha256: null },
    effective_options: null,
    samples: [],
    error: null,
  };
}

async function run(request) {
  const response = baseResponse(request);
  let engine;
  try {
    validate(request);
    if (Number(process.versions.node.split('.')[0]) !== 25) {
      throw new DriverFailure('unsupported_runtime');
    }
    const manifest = JSON.parse(readFileSync(packageJson, 'utf8'));
    if (manifest.name !== 'fathomdb' || typeof manifest.version !== 'string') {
      throw new DriverFailure('package_mismatch');
    }
    response.identity.sdk_version = manifest.version;
    const deadline = process.hrtime.bigint() + BigInt(request.timeout_ms) * 1000000n;
    let sdk;
    try {
      sdk = await bounded(() => import(packageEntry.href), deadline);
    } catch (error) {
      if (error instanceof DriverFailure && error.code === 'timeout') throw error;
      throw new DriverFailure('sdk_import_failed');
    }
    const nativeSha256 = loadedNative();
    response.identity.native_sha256 = nativeSha256;
    if (nativeSha256 !== request.expected_native_sha256) {
      throw new DriverFailure('native_digest_mismatch');
    }
    const view = { validAsOf: request.arguments.view.valid_as_of };
    const options = request.operation_id === 'engine.search_text_only'
      ? { ...view, limit: request.arguments.limit }
      : request.operation_id === 'graph.neighbors'
        ? { view, depth: request.arguments.depth }
        : view;
    response.effective_options = request.arguments;
    sdk.admin.configureRuntime({ sqliteMode: 'performance' });
    engine = await bounded(() => sdk.Engine.open(request.database_path, { useDefaultEmbedder: false }), deadline);
    if (!object(engine.openReport())) {
      throw new DriverFailure('invalid_open_report');
    }
    const samples = [];
    for (const query of request.queries) {
      for (let i = 0; i < request.warmup_per_query; i += 1) {
        await bounded(() => publicCall(sdk, engine, request, options, query), deadline);
      }
      const started = process.hrtime.bigint();
      const result = await bounded(() => publicCall(sdk, engine, request, options, query), deadline);
      const elapsed = process.hrtime.bigint() - started;
      const ids = normalizeResult(request.operation_id, result);
      samples.push({ query_id: query.id, elapsed_ns: Number(elapsed), ids });
    }
    await bounded(() => engine.close(), deadline);
    engine = undefined;
    response.samples = samples;
    response.status = 'complete';
    if (process.platform === 'linux') {
      try {
        const match = /^VmHWM:\s+(\d+)\s+kB$/m.exec(readFileSync('/proc/self/status', 'utf8'));
        if (match) {
          response.resources = { driver_peak_rss_bytes: Number(match[1]) * 1024 };
        }
      } catch {
        // A missing procfs observation leaves the requested capacity metric unavailable.
      }
    }
  } catch (error) {
    response.error = typedError(error);
  } finally {
    if (engine !== undefined) {
      try {
        await bounded(() => engine.close(), process.hrtime.bigint() + 1000000000n);
      } catch (error) {
        response.error = { code: 'close_failed', type: typedError(error).code };
      }
    }
  }
  return response;
}

async function main() {
  let response;
  try {
    if (process.argv.length !== 3) throw new DriverFailure('invalid_request');
    const request = JSON.parse(readFileSync(process.argv[2], 'utf8'));
    if (object(request) && request.schema_version === protocol && request.mode === 'capabilities') {
      keys(request, ['schema_version', 'mode']);
      response = {
        schema_version: protocol, status: 'complete', sdk: 'typescript',
        operations: operations.map((operation_id) => ({ operation_id, operation_version: 1 })), error: null,
      };
    } else {
      response = await run(request);
    }
  } catch (error) {
    response = baseResponse(null);
    response.error = typedError(error);
  }
  process.stdout.write(`${JSON.stringify(response)}\n`);
}

await main();
