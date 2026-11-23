import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  readdir,
  lstat,
  readlink,
  rm,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
const root = resolve(import.meta.dirname, "../../..");
const scratch = await mkdtemp(join(tmpdir(), "inspector-startup-"));
const cache = join(scratch, "cache");
async function inventory(path) {
  const result = {};
  async function walk(file) {
    const info = await lstat(file);
    const entry = {
      type: info.isDirectory()
        ? "directory"
        : info.isFile()
          ? "file"
          : info.isSymbolicLink()
            ? "symlink"
            : "other",
      mode: info.mode,
      uid: info.uid,
      gid: info.gid,
      device: info.dev,
      inode: info.ino,
      size: info.size,
      mtime: info.mtimeMs,
      ctime: info.ctimeMs,
    };
    if (info.isFile())
      entry.sha256 = createHash("sha256")
        .update(await readFile(file))
        .digest("hex");
    if (info.isSymbolicLink()) entry.target = await readlink(file);
    result[file.slice(cache.length)] = entry;
    if (info.isDirectory())
      for (const name of (await readdir(file)).sort()) await walk(join(file, name));
  }
  await walk(path);
  return result;
}

try {
  const artifact = execFileSync(
    "cargo",
    [
      "test",
      "-p",
      "fathomdb-inspector",
      "--test",
      "worker",
      "--offline",
      "--no-run",
      "--message-format=json",
    ],
    { cwd: root, encoding: "utf8", timeout: 120000 },
  )
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line))
    .find(
      (value) =>
        value.reason === "compiler-artifact" && value.target.name === "worker" && value.executable,
    );
  assert.ok(artifact, "checkout-owned worker test artifact");
  const description = JSON.parse(
    execFileSync(join(root, "target/debug/fathomdb-inspector"), ["--build-description"], {
      encoding: "utf8",
      env: {},
      timeout: 30000,
    }),
  );
  const partial = join(
    cache,
    "hub",
    `models--${description.model_repository.replaceAll("/", "--")}`,
    "snapshots",
    description.model_revision,
  );
  await mkdir(partial, { recursive: true });
  await writeFile(join(partial, "config.json"), "owned-partial-cache-canary");
  const before = await inventory(cache);
  const strace = process.env.INSPECTOR_STRACE ?? "/usr/bin/strace";
  const common = [
    "-ff",
    "-s",
    "512",
    "-e",
    "trace=%network,%file,%process",
    "--inject=socket:error=EACCES",
  ];
  const calibration = execFileSync(
    strace,
    [
      ...common,
      "-o",
      join(scratch, "calibration"),
      join(root, ".venv/bin/python"),
      "-c",
      'import errno,socket\ntry: socket.socket(socket.AF_INET,socket.SOCK_STREAM)\nexcept OSError as e: assert e.errno==errno.EACCES\nelse: raise AssertionError("network denial did not apply")',
    ],
    { cwd: root, encoding: "utf8", env: { LANG: "C.UTF-8", LC_ALL: "C.UTF-8" }, timeout: 60000 },
  );
  assert.equal(calibration, "");
  const hostile = {
    ...process.env,
    HOME: cache,
    HF_HOME: cache,
    HF_HUB_CACHE: join(cache, "hub"),
    XDG_CACHE_HOME: cache,
    FATHOMDB_PERF_EXPERIMENTS: "1",
    FATHOMDB_PERF_WRITER_PRAGMAS: "application_id=987654",
    FATHOMDB_PERF_READER_PRAGMAS: "query_only=0",
    FATHOMDB_PERF_SEARCH_LIMIT: "1",
    FATHOMDB_EMBED_DEVICE: "cuda:999",
    HTTP_PROXY: "http://127.0.0.1:9",
    HTTPS_PROXY: "http://127.0.0.1:9",
    ALL_PROXY: "http://127.0.0.1:9",
  };
  const execution = spawnSync(
    strace,
    [
      ...common,
      "-o",
      join(scratch, "worker"),
      artifact.executable,
      "--exact",
      "real_cleared_session_worker_owns_lock_and_preserves_header_under_hostile_parent",
      "--nocapture",
    ],
    { cwd: root, encoding: "utf8", env: hostile, timeout: 60000 },
  );
  if (execution.error) throw execution.error;
  assert.equal(
    execution.signal,
    null,
    `strace worker terminated by signal ${execution.signal}: ${execution.stderr}`,
  );
  assert.equal(execution.status, 0, execution.stderr);
  const result = execution.stdout + execution.stderr;
  assert.match(result, /1 passed/);
  assert.doesNotMatch(result, /original text|owned-partial-cache-canary/);
  const traces = new Map();
  for (const name of await readdir(scratch))
    if (/^worker\.\d+$/.test(name))
      traces.set(name.split(".").at(-1), await readFile(join(scratch, name), "utf8"));
  const owned = new Set(
    [...traces]
      .filter(([, text]) => /execve\([^\n]+--session-worker/.test(text))
      .map(([pid]) => pid),
  );
  assert.equal(owned.size, 1, "real absolute session-worker exec must be traced");
  let changed = true;
  while (changed) {
    changed = false;
    for (const pid of [...owned])
      for (const match of (traces.get(pid) ?? "").matchAll(/\bclone(?:3)?\b[^\n]*=\s*(\d+)/g))
        if (!owned.has(match[1])) {
          owned.add(match[1]);
          changed = true;
        }
  }
  const worker = [...owned].map((pid) => traces.get(pid) ?? "").join("\n");
  assert.doesNotMatch(
    worker,
    /AF_INET6?\b/,
    "cleared model-free startup must not attempt Internet sockets",
  );
  assert.ok(!worker.includes(cache), "worker must not touch hostile cache");
  assert.doesNotMatch(
    worker,
    /\/huggingface\/|\/models--|\/\.cache\//,
    "worker must not probe an ambient model cache",
  );
  assert.deepEqual(
    await inventory(cache),
    before,
    "partial cache entry/content snapshot must remain identical; measuring reads exclude atime",
  );
  console.log(
    JSON.stringify({
      qualification: "slice20_model_free_startup",
      build: description.build,
      network_denial_calibrated: true,
      traced_owned_processes_and_threads: owned.size,
      internet_attempts: 0,
      ambient_cache_accesses: 0,
      partial_cache_snapshot_unchanged: true,
      cache_snapshot_scope:
        "entries/type/mode/uid/gid/device/inode/size/mtime/ctime/link-target/content-sha256; atime excluded",
      log_canaries_absent_from_captured_producer_streams: true,
      header_environment_lock_and_close: "existing_real_worker_oracle_passed",
      model_operation: "slice40_not_claimed",
    }),
  );
} finally {
  await rm(scratch, { recursive: true, force: true });
}
