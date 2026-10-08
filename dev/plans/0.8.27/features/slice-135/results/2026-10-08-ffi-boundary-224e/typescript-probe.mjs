import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Engine, admin, WriteValidationError } from 'fathomdb';

const directory = mkdtempSync(join(tmpdir(), 'slice135-ts-ffi-224e-'));
const path = join(directory, 'ffi.sqlite');
const engine = await Engine.open(path);
await admin.configure(engine, {name: 'ffi_col', body: '{}'});
const before = engine.counters().writeRows;
const errors = [];
for (const body of ['{"x":"a\u0000b"}', '{"x":"a\ud800b"}']) {
  try {
    await engine.write([{opStore: {collection: 'ffi_col', recordKey: 'k1', body}}]);
    throw new Error('invalid FFI body was accepted');
  } catch (error) {
    if (!(error instanceof WriteValidationError)) throw error;
    errors.push(error.constructor.name);
  }
}
const after = engine.counters().writeRows;
if (before !== after) throw new Error('invalid write changed count');
await engine.close();
const reopened = await Engine.open(path);
const reopenedRows = reopened.counters().writeRows;
if (reopenedRows !== before) throw new Error('reopened count changed');
await reopened.close();
console.log(JSON.stringify({source_sha:'224e44c593c13d86ece648adabe445723db04070',npm_archive_sha256:'8c8a97fbe26c541cddadfec7f9faef518102307f927ce6b4b095e5da721ca49d',node_version:process.version,errors,before_write_rows:before,after_write_rows:after,reopened_write_rows:reopenedRows,status:'PASS'}));
