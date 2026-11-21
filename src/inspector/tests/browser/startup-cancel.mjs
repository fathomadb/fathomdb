import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve, dirname } from "node:path";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../../..");
test("cancel owns and terminates a child before startup readiness within shutdown budget", async () => {
  const dir = await mkdtemp(`${tmpdir()}/inspector-context-`);
  let child, ownedPid, ownedScratch;
  try {
    const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir], {
      encoding: "utf8",
    }).trim();
    child = spawn(
      `${root}/target/debug/examples/driver`,
      [db, `${root}/target/debug/examples/blocked-startup`],
      { stdio: ["pipe", "pipe", "pipe"] },
    );
    child.stdin.on("error", () => {});
    const lines = createInterface({ input: child.stdout });
    const iter = lines[Symbol.asyncIterator]();
    const info = JSON.parse((await iter.next()).value);
    ownedScratch = dirname(info.bootstrap);
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
    const pending = [api({ op: "attach", path: db }).catch((error) => ({ error: error.message }))];
    const children = async () => {
      const result = [];
      for (const tid of await readdir(`/proc/${child.pid}/task`)) {
        try {
          const text = await readFile(`/proc/${child.pid}/task/${tid}/children`, "utf8");
          result.push(...text.trim().split(/\s+/).filter(Boolean));
        } catch {}
      }
      return [...new Set(result)];
    };
    let pid;
    const startDeadline = performance.now() + 2000;
    while (!(pid = (await children())[0])) {
      assert.ok(performance.now() < startDeadline);
      await new Promise((r) => setTimeout(r, 5));
    }
    ownedPid = pid;
    const began = performance.now();
    const cancelled = await api({ op: "cancel" });
    assert.equal(cancelled.data.status, "cancelled");
    assert.ok(performance.now() - began <= 250);
    const stopDeadline = performance.now() + 30000;
    while (true) {
      try {
        const status = await readFile(`/proc/${pid}/status`, "utf8");
        if (/^State:\s+Z/m.test(status)) break;
      } catch {
        break;
      }
      assert.ok(
        performance.now() < stopDeadline,
        "owned startup child must quiesce within30seconds after cancel",
      );
      await new Promise((r) => setTimeout(r, 5));
    }
    await Promise.allSettled(pending);
    execFileSync(`${root}/.venv/bin/python`, [
      "-c",
      'import fcntl,sys; f=open(sys.argv[1],"r+");fcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)',
      `${db}.lock`,
    ]);
    await rm(dirname(info.bootstrap), { recursive: true, force: true });
    console.log(
      JSON.stringify({
        fixture: "cancel_before_startup_readiness",
        quiescence_ms: performance.now() - began,
      }),
    );
  } finally {
    if (ownedPid) {
      try {
        const command = await readFile(`/proc/${ownedPid}/cmdline`, "utf8");
        if (
          command.includes(`${root}/target/debug/examples/blocked-startup`) &&
          command.includes("--session-worker")
        )
          process.kill(Number(ownedPid), "SIGKILL");
      } catch {}
    }
    if (ownedScratch) await rm(ownedScratch, { recursive: true, force: true });
    if (child && child.exitCode === null && child.signalCode === null) {
      child.stdin.end("close\n");
      await new Promise((r) => child.on("exit", r));
    }
    await rm(dir, { recursive: true, force: true });
  }
});
