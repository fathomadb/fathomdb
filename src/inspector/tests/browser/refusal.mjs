import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../../..");
test("public admission refusals preserve observed/supported versions and truthful effect phase", async () => {
  const dir = await mkdtemp(`${tmpdir()}/inspector-refusal-`);
  let child;
  try {
    const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir], {
      encoding: "utf8",
    }).trim();
    const original = await readFile(db);
    child = spawn(
      `${root}/target/debug/examples/driver`,
      [db, `${root}/target/debug/fathomdb-inspector`],
      { stdio: ["pipe", "pipe", "pipe"] },
    );
    const lines = createInterface({ input: child.stdout });
    const info = JSON.parse((await lines[Symbol.asyncIterator]().next()).value);
    const origin = `http://${info.address}`;
    const secret = (await readFile(info.bootstrap, "utf8")).match(
      /name="secret" value="([a-f0-9]+)"/,
    )[1];
    const boot = await fetch(`${origin}/bootstrap`, {
      method: "POST",
      headers: { Origin: "null", "Content-Type": "application/x-www-form-urlencoded" },
      body: `secret=${secret}`,
      redirect: "manual",
    });
    const cookie = boot.headers.get("set-cookie").split(";")[0];
    const h = await (await fetch(`${origin}/handshake`, { headers: { Cookie: cookie } })).json();
    const pair = { build: h.build, protocol: h.protocol, launch: h.launch, attachment: null };
    const supportedSchema = original.readUInt32BE(60);
    for (const version of [supportedSchema - 1, supportedSchema + 1]) {
      const path = `${dir}/schema-${version}.sqlite`;
      const bytes = Buffer.from(original);
      bytes.writeUInt32BE(version, 60);
      await writeFile(path, bytes);
      await writeFile(`${path}.lock`, "existing lock sentinel");
      const response = await fetch(`${origin}/api`, {
        method: "POST",
        headers: { Cookie: cookie, Origin: origin, "Content-Type": "application/json" },
        body: JSON.stringify({ ...pair, request: { op: "attach", path } }),
      });
      const result = await response.json();
      assert.equal(result.status, "refused");
      assert.equal(result.error, "unsupported_schema");
      assert.equal(result.observed_schema, version);
      assert.equal(result.supported_schema, supportedSchema);
      assert.equal(result.phase, "preflight");
      assert.equal(result.effects_possible, false);
      assert.deepEqual(await readFile(path), bytes);
      assert.equal(await readFile(`${path}.lock`, "utf8"), "existing lock sentinel");
    }
  } finally {
    if (child) {
      child.stdin.end("close\n");
      await new Promise((r) => child.on("exit", r));
    }
    await rm(dir, { recursive: true, force: true });
  }
});
