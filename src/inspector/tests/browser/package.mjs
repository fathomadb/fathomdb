import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, rm, mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { boundedExecFileSync } from "./helpers.mjs";

const root = resolve(import.meta.dirname, "../../../..");

async function inspectorVersion() {
  const cargo = await readFile(join(root, "src/inspector/Cargo.toml"), "utf8");
  let inPkg = false;
  for (const line of cargo.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[")) inPkg = trimmed === "[package]";
    else if (inPkg) {
      const match = trimmed.match(/^version\s*=\s*"([^"]+)"/);
      if (match) return match[1];
    }
  }
  throw new Error("missing [package].version in src/inspector/Cargo.toml");
}

test(
  "offline default tar artifact contains a self-identifying executable without developer runtimes",
  { timeout: 1800000 },
  async () => {
    const version = await inspectorVersion();
    const bundleName = `fathomdb-inspector-${version}-linux-x86_64`;
    const scratch = await mkdtemp(join(tmpdir(), "inspector-package-"));
    try {
      const output = join(scratch, "bundle");
      const built = spawnSync("bash", [join(root, "src/inspector/tools/package.sh"), output], {
        cwd: root,
        encoding: "utf8",
        timeout: 1700000,
        maxBuffer: 16 * 1024 * 1024,
      });
      if (built.error) throw built.error;
      if (built.status !== 0) process.stderr.write(built.stderr ?? "");
      assert.equal(built.status, 0, "real offline packaging must succeed");
      const archive = built.stdout.trim();
      assert.equal(archive, join(output, `${bundleName}.tar.gz`));
      const entries = boundedExecFileSync("tar", ["-tzf", archive], { encoding: "utf8" }, 30000)
        .trim()
        .split("\n");
      assert.ok(entries.includes(`${bundleName}/fathomdb-inspector`));
      assert.ok(entries.includes(`${bundleName}/build-description.json`));
      assert.ok(entries.includes(`${bundleName}/LICENSE`));
      assert.ok(!entries.some((name) => /node_modules|\.venv|fixture|driver|tests\//.test(name)));
      const installed = join(scratch, "installed");
      await mkdir(installed);
      boundedExecFileSync("tar", ["-xzf", archive, "-C", installed], {}, 30000);
      const directory = join(installed, bundleName),
        binary = join(directory, "fathomdb-inspector");
      const description = JSON.parse(
        boundedExecFileSync(binary, ["--build-description"], { encoding: "utf8", env: {} }, 15000),
      );
      assert.equal(description.version, version);
      assert.equal(description.compiled_schema, 38);
      assert.equal(description.fixture_control, false);
      assert.equal(
        description.cached_default_cpu,
        true,
        "shipping artifact must compile supported cached CPU serving",
      );
      assert.equal(description.profile, "release");
      assert.equal(description.target, "x86_64-unknown-linux-gnu");
      assert.equal(description.protocol, 1);
      assert.deepEqual(
        JSON.parse(await readFile(join(directory, "build-description.json"), "utf8")),
        description,
      );
      const digest = createHash("sha256")
        .update(await readFile(binary))
        .digest("hex");
      const sums = await readFile(join(directory, "SHA256SUMS"), "utf8");
      assert.ok(sums.includes(`${digest}  fathomdb-inspector`));
      const help = boundedExecFileSync(binary, ["--help"], { encoding: "utf8", env: {} }, 15000);
      for (const flag of ["--no-show-inactive", "--show-erased", "--show-payloads"])
        assert.ok(help.includes(flag));
      console.log(
        JSON.stringify({
          qualification: "default_offline_package_skeleton",
          version,
          compiled_schema: description.compiled_schema,
          sha256: digest,
          build: description.build,
          profile: description.profile,
          target: description.target,
          fixture_control: false,
          installed_help_and_description: true,
          native_gui: "separate_mandatory_qualification",
        }),
      );
    } finally {
      await rm(scratch, { recursive: true, force: true });
    }
  },
);
