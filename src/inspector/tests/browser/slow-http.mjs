import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { createInterface } from "node:readline";
import { connect } from "node:net";
const root = resolve(import.meta.dirname, "../../../..");
test(
  "slow request bodies cannot reset the aggregate HTTP parsing deadline",
  { timeout: 10000 },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-slow-http-`);
    let child, socket, timer;
    try {
      child = spawn(
        `${root}/target/debug/examples/driver`,
        [`${dir}/not-attached.sqlite`, `${root}/target/debug/fathomdb-inspector`],
        { stdio: ["pipe", "pipe", "pipe"] },
      );
      const lines = createInterface({ input: child.stdout });
      const info = JSON.parse((await lines[Symbol.asyncIterator]().next()).value);
      const [host, port] = info.address.split(":");
      socket = connect(Number(port), host);
      await new Promise((r) => socket.once("connect", r));
      socket.on("error", () => {});
      socket.resume();
      let closed = false;
      socket.once("close", () => {
        closed = true;
      });
      const began = performance.now();
      socket.write(`POST /api HTTP/1.1\r\nHost: ${info.address}\r\nContent-Length: 1000\r\n\r\n`);
      timer = setInterval(() => socket.write("x"), 50);
      while (!closed && performance.now() - began < 6000)
        await new Promise((r) => setTimeout(r, 10));
      assert.equal(
        closed,
        true,
        "drip body must be closed within absolute five-second parse budget plus scheduling tolerance",
      );
    } finally {
      clearInterval(timer);
      socket?.destroy();
      if (child) {
        child.stdin.end("close\n");
        await new Promise((r) => child.once("exit", r));
      }
      await rm(dir, { recursive: true, force: true });
    }
  },
);
