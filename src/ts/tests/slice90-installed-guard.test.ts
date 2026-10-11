import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const guard = resolve(here, "../../../..", "scripts/slice90-node-installed-qualification.sh");

test("installed qualification requires an exact clean committed source tree", () => {
  const repo = mkdtempSync(join(tmpdir(), "fdb-node-qualification-guard-"));
  try {
    execFileSync("git", ["init", "-q", repo]);
    writeFileSync(join(repo, "tracked.txt"), "candidate\n");
    execFileSync("git", ["-C", repo, "add", "tracked.txt"]);
    execFileSync("git", ["-C", repo, "-c", "user.name=Test", "-c",
      "user.email=test@example.invalid", "commit", "-qm", "candidate"]);
    const sha = execFileSync("git", ["-C", repo, "rev-parse", "HEAD"],
      { encoding: "utf8" }).trim();
    const clean = spawnSync("bash", [guard, "--check-source", repo], { encoding: "utf8" });
    assert.equal(clean.status, 0, clean.stderr);
    assert.equal(clean.stdout.trim(), sha);

    writeFileSync(join(repo, "tracked.txt"), "mutated\n");
    const dirty = spawnSync("bash", [guard, "--check-source", repo], { encoding: "utf8" });
    assert.notEqual(dirty.status, 0);
    assert.match(dirty.stderr, /dirty|uncommitted/i);
  } finally {
    rmSync(repo, { recursive: true, force: true });
  }
});
