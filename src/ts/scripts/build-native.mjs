#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

const requestedTemporaryDirectory = process.env.FATHOMDB_NAPI_BUILD_TMPDIR;
const printPlan = process.argv.includes("--print-plan");
const plannedPlatform = printPlan && process.env.FATHOMDB_NAPI_BUILD_PRINT_PLATFORM === "win32"
  ? "win32"
  : process.platform;
const ownsTemporaryDirectory = requestedTemporaryDirectory === undefined;
const temporaryDirectory = requestedTemporaryDirectory
  ? resolve(requestedTemporaryDirectory)
  : printPlan
    ? resolve(tmpdir(), "fathomdb-napi-production-<allocated-at-execution>")
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
const napiArguments = [
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
const build = plannedPlatform === "win32"
  ? [
      process.execPath,
      resolve(import.meta.dirname, "..", "node_modules", "@napi-rs", "cli", "scripts", "index.js"),
      ...napiArguments,
    ]
  : ["npm", "exec", "--", "napi", ...napiArguments];
const environment = {
  ...process.env,
  TMPDIR: temporaryDirectory,
  TMP: temporaryDirectory,
  TEMP: temporaryDirectory,
};

if (printPlan) {
  process.stdout.write(
    `${JSON.stringify({
      temporary_directory: temporaryDirectory,
      clean,
      build,
      environment: {
        TMPDIR: environment.TMPDIR,
        TMP: environment.TMP,
        TEMP: environment.TEMP,
      },
    })}\n`,
  );
  process.exit(0);
}

mkdirSync(temporaryDirectory, { recursive: true });

function run(command) {
  const completed = spawnSync(command[0], command.slice(1), {
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
