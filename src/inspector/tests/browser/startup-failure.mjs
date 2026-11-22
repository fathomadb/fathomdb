import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { createInterface } from "node:readline";
import { setTimeout as delay } from "node:timers/promises";
const root = resolve(import.meta.dirname, "../../../..");
test(
  "refused startup forgets a reaped worker before later Cancel",
  { timeout: 45000 },
  async () => {
    const dir = await mkdtemp(join(tmpdir(), "inspector-startup-failure-"));
    let child;
    try {
      const db = execFileSync(join(root, "target/debug/examples/fixture"), [dir], {
        encoding: "utf8",
      }).trim();
      const description = JSON.parse(
        execFileSync(join(root, "target/debug/fathomdb-inspector"), ["--build-description"], {
          encoding: "utf8",
          env: {},
        }),
      );
      execFileSync(join(root, ".venv/bin/python"), [
        "-c",
        'import sys\nf=open(sys.argv[1],"r+b")\nf.seek(60)\nf.write(int(sys.argv[2]).to_bytes(4,"big"))\nf.close()',
        db,
        String(description.compiled_schema + 1),
      ]);
      const trace = join(dir, "process.trace");
      child = spawn(
        process.env.INSPECTOR_STRACE ?? "/usr/bin/strace",
        [
          "-f",
          "-e",
          "trace=kill,tgkill,wait4,waitid",
          "-o",
          trace,
          join(root, "target/debug/examples/driver"),
          db,
          join(root, "target/debug/fathomdb-inspector"),
        ],
        { stdio: ["pipe", "pipe", "pipe"], detached: true },
      );
      child.stdin.on("error", () => {});
      const iter = createInterface({ input: child.stdout })[Symbol.asyncIterator]();
      const info = JSON.parse((await iter.next()).value);
      const base = `http://${info.address}`;
      const secret = (await readFile(info.bootstrap, "utf8")).match(
        /name="secret" value="([a-f0-9]+)"/,
      )[1];
      const boot = await fetch(`${base}/bootstrap`, {
        method: "POST",
        headers: { Origin: "null", "Content-Type": "application/x-www-form-urlencoded" },
        body: `secret=${secret}`,
        redirect: "manual",
      });
      assert.equal(boot.status, 303);
      const cookie = boot.headers.get("set-cookie").split(";")[0];
      const infoPair = await (
        await fetch(`${base}/handshake`, { headers: { Cookie: cookie } })
      ).json();
      const pair = {
        build: infoPair.build,
        protocol: infoPair.protocol,
        launch: infoPair.launch,
        attachment: infoPair.attachment,
      };
      async function api(request) {
        const result = await (
          await fetch(`${base}/api`, {
            method: "POST",
            headers: { Cookie: cookie, Origin: base, "Content-Type": "application/json" },
            body: JSON.stringify({ ...pair, request }),
            signal: AbortSignal.timeout(30000),
          })
        ).json();
        if ("attachment" in result) pair.attachment = result.attachment;
        return result;
      }
      const refused = await api({ op: "attach", path: db });
      assert.equal(refused.error, "unsupported_schema");
      assert.equal(refused.phase, "preflight");
      assert.equal(refused.effects_possible, false);
      await delay(20);
      const before = await readFile(trace);
      assert.match(
        before.toString(),
        /wait4\(/,
        "actual worker reap must precede the cancellation boundary",
      );
      assert.equal((await api({ op: "cancel" })).status, "cancelled");
      await delay(20);
      const tail = (await readFile(trace)).subarray(before.length).toString();
      assert.doesNotMatch(
        tail,
        /\bkill\(-\d+, SIGKILL\)/,
        "Cancel after refused/reaped startup must not signal an exited PID/process group",
      );
    } finally {
      if (child && child.exitCode === null) {
        const exited = new Promise((r) => child.once("exit", r));
        child.stdin.end("close\n");
        let timer;
        try {
          await Promise.race([
            exited,
            new Promise((_, reject) => {
              timer = setTimeout(() => reject(Error("traced driver cleanup deadline")), 10000);
            }),
          ]);
        } catch (error) {
          process.kill(-child.pid, "SIGKILL");
          await exited;
          throw error;
        } finally {
          clearTimeout(timer);
        }
      }
      await rm(dir, { recursive: true, force: true });
    }
  },
);
