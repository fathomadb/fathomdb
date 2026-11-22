import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../../..");
test("paired local service attaches the actual supervised Engine and reads exact records", async () => {
  const dir = await mkdtemp(`${tmpdir()}/inspector-api-`);
  let child;
  try {
    const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir], {
      encoding: "utf8",
    }).trim();
    child = spawn(
      `${root}/target/debug/examples/driver`,
      [db, `${root}/target/debug/fathomdb-inspector`],
      { stdio: ["pipe", "pipe", "pipe"] },
    );
    const lines = createInterface({ input: child.stdout });
    const info = JSON.parse((await lines[Symbol.asyncIterator]().next()).value);
    const base = `http://${info.address}`;
    const html = await readFile(info.bootstrap, "utf8");
    const secret = html.match(/name="secret" value="([a-f0-9]+)"/)[1];
    let response = await fetch(`${base}/bootstrap`, {
      method: "POST",
      headers: { Origin: "null", "Content-Type": "application/x-www-form-urlencoded" },
      body: `secret=${secret}`,
      redirect: "manual",
    });
    assert.equal(response.status, 303);
    const cookie = response.headers.get("set-cookie").split(";")[0];
    const headers = { Cookie: cookie, Origin: base, "Content-Type": "application/json" };
    const pair = await (await fetch(`${base}/handshake`, { headers: { Cookie: cookie } })).json();
    async function api(request) {
      const res = await fetch(`${base}/api`, {
        method: "POST",
        headers,
        body: JSON.stringify({ ...pair, request, path: undefined, policy: undefined }),
      });
      assert.equal(res.status, 200);
      const value = await res.json();
      if (value.attachment !== undefined) pair.attachment = value.attachment;
      return value;
    }
    const attached = await api({ op: "attach", path: db });
    assert.equal(attached.status, "complete");
    assert.ok(attached.attachment);
    const catalog = await api({ op: "catalog" });
    assert.equal(catalog.counts.node.revisions, "3");
    const page = await api({
      op: "page",
      class: "node",
      after: "0",
      limit: 200,
      filter: { identity: "structured" },
    });
    assert.equal(page.items.length, 1);
    const detail = await api({ op: "detail", handle: page.items[0].handle });
    assert.match(detail.content.text, /9007199254740993123456789/);
    const detached = await api({ op: "detach" });
    assert.equal(detached.status, "complete");
    assert.equal(detached.attachment, null);
  } finally {
    if (child) {
      child.stdin.end("close\n");
      await new Promise((r) => child.on("exit", r));
    }
    await rm(dir, { recursive: true, force: true });
  }
});
