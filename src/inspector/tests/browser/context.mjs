import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../../..");
test("read observations preserve the authoritative attachment, build, time and generation scope", async () => {
  const dir = await mkdtemp(`${tmpdir()}/inspector-context-`);
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
    const pair = {
      build: h.build,
      protocol: h.protocol,
      launch: h.launch,
      attachment: h.attachment,
    };
    async function api(request, override = {}) {
      const response = await fetch(`${origin}/api`, {
        method: "POST",
        headers: { Cookie: cookie, Origin: origin, "Content-Type": "application/json" },
        body: JSON.stringify({ ...pair, ...override, request }),
      });
      const data = await response.json();
      if ("attachment" in data) pair.attachment = data.attachment;
      return { response, data };
    }
    const attached = (await api({ op: "attach", path: db })).data;
    assert.equal(attached.status, "complete");
    const nonce = attached.attachment;
    const page = (
      await api({
        op: "page",
        class: "node",
        after: "0",
        limit: 10,
        filter: { identity: "structured" },
      })
    ).data;
    const context = page.context;
    assert.equal(context.attachment, nonce);
    assert.equal(context.build, h.build);
    assert.equal(context.protocol, 1);
    assert.equal(context.dependency_generation, "0");
    assert.equal(context.snapshot, "this_request_only");
    assert.match(context.read_started_epoch_ms, /^\d+$/);
    assert.match(context.read_ended_epoch_ms, /^\d+$/);
    assert.ok(BigInt(context.read_ended_epoch_ms) >= BigInt(context.read_started_epoch_ms));
    assert.equal(context.write_boundary.state, "unknown");
    const handle = page.items[0].handle;
    const detail = (await api({ op: "detail", handle })).data;
    assert.equal(detail.context.attachment, nonce);
    assert.ok(BigInt(detail.context.operation) > BigInt(context.operation));
    assert.match(detail.content.text, /9007199254740993123456789/);
    const stale = await api({ op: "detail", handle }, { attachment: "stale-attachment" });
    assert.equal(stale.response.status, 409);
    assert.equal(stale.data.status, "expired");
    const again = (await api({ op: "detail", handle })).data;
    assert.equal(again.context.attachment, nonce);
    assert.equal(BigInt(again.context.operation), BigInt(detail.context.operation) + 1n);
    await api({ op: "detach" });
  } finally {
    if (child) {
      child.stdin.end("close\n");
      await new Promise((r) => child.on("exit", r));
    }
    await rm(dir, { recursive: true, force: true });
  }
});
