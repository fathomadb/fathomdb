import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readdir, readFile } from "node:fs/promises";
import { setTimeout as delay } from "node:timers/promises";

export async function clipboardEquals(page, expected) {
  const deadline = performance.now() + 5000;
  let actual;
  while (performance.now() < deadline) {
    let timer;
    try {
      actual = await Promise.race([
        page.evaluate(() => navigator.clipboard.readText()),
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(Error("clipboard read deadline")),
            Math.max(1, deadline - performance.now()),
          );
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
    if (actual === expected) {
      assert.equal(actual, expected);
      return;
    }
    await delay(Math.min(10, Math.max(0, deadline - performance.now())));
  }
  assert.equal(actual, expected, "clipboard must contain the exact copied value before deadline");
}

export function boundedExecFileSync(file, args, options = {}, defaultTimeoutMs = 120000) {
  return execFileSync(file, args, { timeout: defaultTimeoutMs, ...options });
}

async function procDirectChildren(pid) {
  const kids = new Set();
  try {
    for (const tid of await readdir(`/proc/${pid}/task`)) {
      try {
        const raw = await readFile(`/proc/${pid}/task/${tid}/children`, "utf8");
        for (const token of raw.trim().split(/\s+/).filter(Boolean)) {
          const parsed = Number(token);
          if (Number.isInteger(parsed) && parsed > 0) kids.add(parsed);
        }
      } catch {}
    }
  } catch {}
  return kids;
}

async function procPpidMap() {
  const byParent = new Map();
  try {
    for (const entry of await readdir("/proc")) {
      if (!/^\d+$/.test(entry)) continue;
      const pid = Number(entry);
      try {
        const stat = await readFile(`/proc/${pid}/stat`, "utf8");
        const tail = stat.slice(stat.lastIndexOf(")") + 2).split(" ");
        const ppid = Number(tail[1]);
        if (Number.isInteger(ppid) && ppid > 0) {
          let list = byParent.get(ppid);
          if (!list) {
            list = [];
            byParent.set(ppid, list);
          }
          list.push(pid);
        }
      } catch {}
    }
  } catch {}
  return byParent;
}

export async function listDescendants(rootPid) {
  if (!Number.isInteger(rootPid) || rootPid <= 0) return [];
  const ppidMap = await procPpidMap();
  const seen = new Set([rootPid]);
  const order = [];
  const queue = [rootPid];
  while (queue.length > 0) {
    const current = queue.shift();
    const direct = await procDirectChildren(current);
    for (const child of ppidMap.get(current) ?? []) direct.add(child);
    for (const child of direct) {
      if (!seen.has(child)) {
        seen.add(child);
        order.push(child);
        queue.push(child);
      }
    }
  }
  return order;
}

export async function killDescendantTree(rootPid, signal = "SIGKILL") {
  if (!Number.isInteger(rootPid) || rootPid <= 0) return [];
  const descendants = await listDescendants(rootPid);
  const targets = [...descendants.reverse(), rootPid];
  for (const pid of targets) {
    try {
      process.kill(pid, signal);
    } catch {}
  }
  return targets;
}

export async function closeBrowserBounded(browser, timeoutMs = 5000) {
  if (!browser) return;
  await Promise.race([
    browser.close().catch(() => {}),
    delay(timeoutMs, undefined, { ref: false }),
  ]);
}

export async function stopChildBounded(
  child,
  {
    gracefulMs = 5000,
    sigtermMs = 2000,
    sigkillMs = 5000,
    closeStdin = "close\n",
    extraPids = [],
  } = {},
) {
  if (!child) return;
  const exited =
    child.exitCode !== null || child.signalCode !== null
      ? Promise.resolve()
      : new Promise((resolve) => child.once("exit", resolve));
  const alive = () => child.exitCode === null && child.signalCode === null;
  if (alive() && closeStdin !== null && child.stdin && !child.stdin.destroyed) {
    try {
      child.stdin.end(closeStdin);
    } catch {}
    await Promise.race([exited, delay(gracefulMs, undefined, { ref: false })]);
  }
  if (alive()) {
    for (const pid of extraPids) {
      if (Number.isInteger(pid) && pid > 0) {
        try {
          process.kill(pid, "SIGTERM");
        } catch {}
      }
    }
    try {
      child.kill("SIGTERM");
    } catch {}
    await Promise.race([exited, delay(sigtermMs, undefined, { ref: false })]);
  }
  if (alive()) {
    for (const pid of extraPids) {
      await killDescendantTree(pid, "SIGKILL");
    }
    await killDescendantTree(child.pid, "SIGKILL");
    try {
      child.kill("SIGKILL");
    } catch {}
    await Promise.race([exited, delay(sigkillMs, undefined, { ref: false })]);
  }
}
