#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

const requestedTemporaryDirectory = process.env.FATHOMDB_NAPI_BUILD_TMPDIR;
const ownsTemporaryDirectory = requestedTemporaryDirectory === undefined;
const temporaryDirectory = requestedTemporaryDirectory
  ? resolve(requestedTemporaryDirectory)
  : mkdtempSync(resolve(tmpdir(), "fathomdb-napi-production-"));

const clean = [
  "cargo",
  "clean",
  "--manifest-path",
  "../../Cargo.toml",
  "-p",
  "fathomdb-napi",
  "--release",
];
const build = [
  "npm",
  "exec",
  "--",
  "napi",
  "build",
  "--platform",
  "--release",
  "--cargo-cwd",
  "../rust/crates/fathomdb-napi",
  "--features",
  "default-embedder",
  "--js",
  "false",
];
const environment = {
  ...process.env,
  TMPDIR: temporaryDirectory,
  TMP: temporaryDirectory,
  TEMP: temporaryDirectory,
};

if (process.argv.includes("--print-plan")) {
  process.stdout.write(
    `${JSON.stringify({ temporary_directory: temporaryDirectory, clean, build, environment })}\n`,
  );
  process.exit(0);
}

mkdirSync(temporaryDirectory, { recursive: true });

function run(command) {
  const executable = process.platform === "win32" && command[0] === "npm" ? "npm.cmd" : command[0];
  const completed = spawnSync(executable, command.slice(1), {
    cwd: resolve(import.meta.dirname, ".."),
    env: environment,
    stdio: "inherit",
    shell: false,
  });
  if (completed.error) {
    throw completed.error;
  }
  if (completed.status !== 0) {
    process.exitCode = completed.status ?? 1;
    return false;
  }
  return true;
}

try {
  if (run(clean)) {
    run(build);
  }
} finally {
  if (ownsTemporaryDirectory) {
    rmSync(temporaryDirectory, { recursive: true, force: true });
  }
}
