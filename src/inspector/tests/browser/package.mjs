import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync, execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm, mkdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
const root = resolve(import.meta.dirname, "../../../..");
test(
  "offline default tar artifact contains a self-identifying executable without developer runtimes",
  { timeout: 1800000 },
  async () => {
    const scratch = await mkdtemp(join(tmpdir(), "inspector-package-"));
    try {
      const output = join(scratch, "bundle");
      const built = spawnSync("bash", [join(root, "src/inspector/tools/package.sh"), output], {
        cwd: root,
        encoding: "utf8",
        timeout: 1700000,
        maxBuffer: 16 * 1024 * 1024,
      });
      if (built.status !== 0) process.stderr.write(built.stderr ?? "");
      assert.equal(built.status, 0, "real offline packaging must succeed");
      const archive = built.stdout.trim();
      assert.equal(archive, join(output, "fathomdb-inspector-0.2.1-linux-x86_64.tar.gz"));
      const entries = execFileSync("tar", ["-tzf", archive], { encoding: "utf8" })
        .trim()
        .split("\n");
      assert.ok(entries.includes("fathomdb-inspector-0.2.1-linux-x86_64/fathomdb-inspector"));
      assert.ok(entries.includes("fathomdb-inspector-0.2.1-linux-x86_64/build-description.json"));
      assert.ok(entries.includes("fathomdb-inspector-0.2.1-linux-x86_64/LICENSE"));
      assert.ok(!entries.some((name) => /node_modules|\.venv|fixture|driver|tests\//.test(name)));
      const installed = join(scratch, "installed");
      await mkdir(installed);
      execFileSync("tar", ["-xzf", archive, "-C", installed]);
      const directory = join(installed, "fathomdb-inspector-0.2.1-linux-x86_64"),
        binary = join(directory, "fathomdb-inspector");
      const description = JSON.parse(
        execFileSync(binary, ["--build-description"], { encoding: "utf8", env: {} }),
      );
      assert.equal(description.version, "0.2.1");
      assert.equal(description.fixture_control, false);
      assert.equal(description.cached_default_cpu, true, "shipping artifact must compile supported cached CPU serving");
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
      const help = execFileSync(binary, ["--help"], { encoding: "utf8", env: {} });
      for (const flag of ["--no-show-inactive", "--show-erased", "--show-payloads"])
        assert.ok(help.includes(flag));
      console.log(
        JSON.stringify({
          qualification: "default_offline_package_skeleton",
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
