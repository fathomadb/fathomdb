import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../../..");
test("private driver control uses owned Engine and never accepts a control over HTTP", async () => {
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
    child.stdin.on("error", () => {});
    const lines = createInterface({ input: child.stdout });
    const iter = lines[Symbol.asyncIterator]();
    const info = JSON.parse((await iter.next()).value);
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
    child.stdin.write(
      JSON.stringify({ control: "transition", identity: "alpha", deleted: true }) + "\n",
    );
    const ack = await iter.next();
    assert.equal(ack.done, false, "private driver must acknowledge fixture command");
    assert.equal(JSON.parse(ack.value).status, "complete");
    const refused = (
      await api({
        op: "catalog",
        origin: "fixture",
        control: { control: "transition", identity: "alpha", deleted: false },
      })
    ).data;
    assert.equal(refused.status, "unsupported");
    const again = (
      await api({ op: "page", class: "node", after: "0", limit: 10, filter: { identity: "alpha" } })
    ).data;
    assert.equal(again.items[0].inactive_or_history, true);
    assert.equal(again.context.attachment, nonce);
    await api({ op: "detach" });
  } finally {
    if (child && child.exitCode === null) {
      child.stdin.end("close\n");
      await new Promise((r) => child.on("exit", r));
    }
    await rm(dir, { recursive: true, force: true });
  }
});
