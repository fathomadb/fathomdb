import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, mkdir, copyFile, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir, arch, cpus } from "node:os";
import { resolve, join, dirname } from "node:path";
import { createInterface } from "node:readline";
import { pathToFileURL } from "node:url";
import { setTimeout as delay } from "node:timers/promises";
import {
  clipboardEquals,
  killDescendantTree,
} from "../tests/browser/helpers.mjs";
assert.equal(
  process.env.AGENT_LONG,
  "1",
  "explicit AGENT_LONG=1 required for real F06 qualification",
);
const root = resolve(import.meta.dirname, "../../..");
const playwrightModule =
  process.env.INSPECTOR_PLAYWRIGHT ??
  "/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs";
const browserExecutable =
  process.env.INSPECTOR_CHROMIUM ??
  "/home/coreyt/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome";

const scratch = await mkdtemp(join(tmpdir(), "inspector-scale-"));
let child,
  browser,
  monitor,
  stopping = false,
  servicePeak = 0,
  heapPeak = 0,
  heapSamples = 0,
  measurementError = null;
const ownedWorkers = new Set();
const sampleMs = Number(process.env.INSPECTOR_SAMPLE_MS ?? 25);
assert.ok(Number.isFinite(sampleMs) && sampleMs >= 1 && sampleMs <= 100);
const report = { qualification: "slice20_real_F06_native_and_browser", synthetic_pause: false };
async function childPids() {
  const ids = new Set();
  for (const tid of await readdir(`/proc/${child.pid}/task`)) {
    try {
      for (const pid of (await readFile(`/proc/${child.pid}/task/${tid}/children`, "utf8"))
        .trim()
        .split(/\s+/)
        .filter(Boolean))
        ids.add(Number(pid));
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  for (const pid of ids) ownedWorkers.add(pid);
  return [...ids];
}
async function rss(pid) {
  try {
    const status = await readFile(`/proc/${pid}/status`, "utf8"),
      match = status.match(/^VmRSS:\s+(\d+)/m);
    if (!match) {
      assert.match(status, /^State:\s+Z/m, "RSS missing for a non-zombie process");
      return 0;
    }
    return Number(match[1]) * 1024;
  } catch (error) {
    if (error.code === "ENOENT") return 0;
    throw error;
  }
}
async function gone(pid) {
  try {
    await readFile(`/proc/${pid}/status`);
    return false;
  } catch (error) {
    if (error.code === "ENOENT") return true;
    throw error;
  }
}
async function cpu(pid) {
  const stat = await readFile(`/proc/${pid}/task/${pid}/stat`, "utf8");
  const fields = stat.slice(stat.lastIndexOf(")") + 2).split(" ");
  return Number(fields[11]) + Number(fields[12]);
}
async function before(promise, deadline, message) {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(Error(message)), Math.max(1, deadline - performance.now()));
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}
async function until(predicate, deadline, message) {
  while (true) {
    assert.ok(performance.now() < deadline, message);
    if (await predicate()) return;
    await delay(Math.min(5, Math.max(0, deadline - performance.now())));
  }
}
function lockFree(path, deadline = performance.now() + 5000) {
  assert.ok(performance.now() < deadline, "lock check must fit shared deadline");
  execFileSync(
    join(root, ".venv/bin/python"),
    [
      "-c",
      'import fcntl,sys\nf=open(sys.argv[1],"r+")\nfcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)',
      `${path}.lock`,
    ],
    { timeout: Math.max(1, Math.ceil(deadline - performance.now())) },
  );
}
try {
  const baseDir = join(scratch, "producer");
  await mkdir(baseDir);
  const baseline = execFileSync(join(root, "target/debug/examples/fixture"), [baseDir, "scale"], {
    cwd: root,
    encoding: "utf8",
    timeout: 600000,
  }).trim();
  const paths = {};
  for (const phase of ["cancel", "fault", "close", "browser"]) {
    const dir = join(scratch, phase);
    await mkdir(dir);
    const path = join(dir, "fixture.sqlite");
    await copyFile(baseline, path);
    await copyFile(`${baseline}.lock`, `${path}.lock`);
    paths[phase] = path;
  }
  const manifest = JSON.parse(
    execFileSync(
      join(root, ".venv/bin/python"),
      [
        "-c",
        'import json,sqlite3,sys\nc=sqlite3.connect("file:"+sys.argv[1]+"?mode=ro",uri=True)\nprint(json.dumps({"nodes":c.execute("select count(*) from canonical_nodes").fetchone()[0],"edges":c.execute("select count(*) from canonical_edges").fetchone()[0],"logs":c.execute("select count(*) from operational_mutations").fetchone()[0],"large_bytes":c.execute("select length(CAST(body AS BLOB)) from canonical_nodes where logical_id=\'scale-large\'").fetchone()[0]}))',
        baseline,
      ],
      { encoding: "utf8", timeout: 30000 },
    ),
  );
  assert.ok(manifest.nodes >= 100000);
  assert.equal(manifest.edges, 10000);
  assert.equal(manifest.logs, 100000);
  assert.ok(manifest.large_bytes > 10 * 1024 * 1024);
  report.fixture = manifest;
  const description = JSON.parse(
    execFileSync(join(root, "target/debug/fathomdb-inspector"), ["--build-description"], {
      env: {},
      encoding: "utf8",
      timeout: 30000,
    }),
  );
  assert.equal(description.fixture_control, false);
  report.build = description;
  const digest = async (path) =>
    createHash("sha256")
      .update(await readFile(path))
      .digest("hex");
  report.runtime = {
    os: (await readFile("/etc/os-release", "utf8")).match(/^PRETTY_NAME="(.*)"$/m)?.[1],
    glibc: execFileSync("/usr/bin/getconf", ["GNU_LIBC_VERSION"], {
      encoding: "utf8",
      timeout: 10000,
    }).trim(),
    architecture: arch(),
    cpu: cpus()[0].model,
    node_version: process.version,
    node_executable_sha256: await digest(process.execPath),
    playwright_version: JSON.parse(
      await readFile(join(dirname(playwrightModule), "package.json"), "utf8"),
    ).version,
    playwright_entrypoint_sha256: await digest(playwrightModule),
    chromium_executable_sha256: await digest(browserExecutable),
  };

  child = spawn(
    join(root, "target/debug/examples/driver"),
    [paths.browser, join(root, "target/debug/fathomdb-inspector")],
    { cwd: root, stdio: ["pipe", "pipe", "pipe"] },
  );
  child.stdin.on("error", () => {});
  const iter = createInterface({ input: child.stdout })[Symbol.asyncIterator]();
  const info = JSON.parse(
    (await before(iter.next(), performance.now() + 10000, "driver readiness deadline")).value,
  );
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
  assert.equal(boot.status, 303);
  const cookie = boot.headers.get("set-cookie").split(";")[0];
  const handshake = await (
    await fetch(`${origin}/handshake`, { headers: { Cookie: cookie } })
  ).json();
  const pair = {
    build: handshake.build,
    protocol: handshake.protocol,
    launch: handshake.launch,
    attachment: handshake.attachment,
  };
  async function api(request) {
    const response = await fetch(`${origin}/api`, {
      method: "POST",
      headers: { Cookie: cookie, Origin: origin, "Content-Type": "application/json" },
      body: JSON.stringify({ ...pair, request }),
      signal: AbortSignal.timeout(35000),
    });
    const value = await response.json();
    if ("attachment" in value) pair.attachment = value.attachment;
    return value;
  }
  const serviceIdle = await rss(child.pid);
  let session;
  async function sampleMemory() {
    const pids = await childPids();
    servicePeak = Math.max(
      servicePeak,
      (await rss(child.pid)) + (await Promise.all(pids.map(rss))).reduce((a, b) => a + b, 0),
    );
    if (session) {
      const heap = await before(
        session.send("Runtime.getHeapUsage"),
        performance.now() + 5000,
        "browser heap sample deadline",
      );
      const value =
        heap.usedSize + (heap.embedderHeapUsedSize ?? 0) + (heap.backingStorageSize ?? 0);
      assert.ok(Number.isFinite(value) && value > 0, "valid browser heap measurement required");
      heapPeak = Math.max(heapPeak, value);
      heapSamples++;
      return value;
    }
  }
  monitor = (async () => {
    while (!stopping) {
      await sampleMemory();
      await delay(sampleMs);
    }
  })().catch((error) => {
    measurementError = error;
    stopping = true;
  });
  async function nativeRead(path) {
    assert.equal((await api({ op: "attach", path })).status, "complete");
    const pids = await childPids();
    assert.equal(pids.length, 1);
    const pid = pids[0],
      start = await cpu(pid);
    let completed = false;
    const pending = api({ op: "catalog" }).then((value) => {
      completed = true;
      return value;
    });
    await until(
      async () => completed || (await cpu(pid)) >= start + 2,
      performance.now() + 2000,
      "real catalog dispatch thread must make CPU progress before interruption",
    );
    assert.equal(completed, false, "catalog must still be executing when fault/cancel is issued");
    return { pid, pending };
  }
  let active = await nativeRead(paths.cancel);
  let began = performance.now();
  let deadline = began + 30000;
  const cancelled = await before(
    api({ op: "cancel" }),
    deadline,
    "cancel acknowledgement deadline",
  );
  const ack = performance.now() - began;
  assert.equal(cancelled.status, "cancelled");
  assert.ok(ack <= 250, `native cancellation acknowledgement ${ack}ms`);
  const late = await before(active.pending, deadline, "native cancelled response deadline");
  assert.equal(late.status, "cancelled");
  assert.equal(late.counts, undefined);
  await until(async () => await gone(active.pid), deadline, "cancelled native worker must quiesce");
  lockFree(paths.cancel, deadline);
  assert.ok(performance.now() <= deadline, "cancel including response/reap/lock must fit30s");
  report.native_cancel = {
    ack_ms: ack,
    quiescence_and_lock_ms: performance.now() - began,
    no_late_payload: true,
  };
  active = await nativeRead(paths.fault);
  began = performance.now();
  deadline = began + 30000;
  process.kill(active.pid, "SIGKILL");
  const fault = await before(active.pending, deadline, "fault response deadline");
  assert.equal(fault.error, "worker_disconnected");
  assert.equal(fault.attachment, null);
  assert.equal(fault.counts, undefined);
  await until(async () => await gone(active.pid), deadline, "faulted native worker must be reaped");
  lockFree(paths.fault, deadline);
  assert.ok(performance.now() <= deadline, "fault including response/reap/lock must fit30s");
  report.native_fault = {
    quiescence_and_lock_ms: performance.now() - began,
    error: fault.error,
    no_late_payload: true,
  };
  active = await nativeRead(paths.close);
  began = performance.now();
  deadline = began + 30000;
  const detached = await before(api({ op: "detach" }), deadline, "native detach shared deadline");
  assert.equal(detached.status, "complete");
  const finished = await before(active.pending, deadline, "preceding catalog shared deadline");
  assert.ok(["complete", "partial"].includes(finished.status));
  await until(
    async () => await gone(active.pid),
    deadline,
    "detached native worker must be reaped",
  );
  lockFree(paths.close, deadline);
  report.native_detach = {
    total_ms: performance.now() - began,
    preceding_operation: finished.status,
  };
  assert.ok(report.native_detach.total_ms <= 30000);
  const { chromium } = await import(pathToFileURL(playwrightModule));
  browser = await chromium.launch({
    executablePath: browserExecutable,
    headless: true,
    env: { LANG: "C.UTF-8", LC_ALL: "C.UTF-8", TMPDIR: scratch },
    args: ["--no-sandbox", "--disable-background-networking", "--no-proxy-server"],
  });
  report.runtime.chromium_version = browser.version();
  const context = await browser.newContext();
  await context.addCookies([
    {
      name: cookie.split("=")[0],
      value: cookie.slice(cookie.indexOf("=") + 1),
      url: origin,
      httpOnly: true,
      sameSite: "Strict",
    },
  ]);
  await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin });
  const page = await context.newPage();
  page.setDefaultTimeout(30000);
  const outside = [];
  page.on("request", (request) => {
    if (!request.url().startsWith(origin) && !request.url().startsWith("file:"))
      outside.push(request.url());
  });
  await page.goto(`${origin}/`);
  session = await context.newCDPSession(page);
  const idleHeap = await before(
    session.send("Runtime.getHeapUsage"),
    performance.now() + 5000,
    "idle heap sample deadline",
  );
  const heapIdle =
    idleHeap.usedSize + (idleHeap.embedderHeapUsedSize ?? 0) + (idleHeap.backingStorageSize ?? 0);
  heapPeak = heapIdle;
  await page.getByRole("button", { name: "Attach database", exact: true }).click();
  await page.getByRole("heading", { name: "Database inventory", exact: true }).waitFor();
  await page.getByRole("button", { name: "Canonical nodes", exact: true }).click();
  await page.getByLabel("Identity filter", { exact: true }).fill("scale-large");
  await page.getByRole("button", { name: "Apply filters", exact: true }).click();
  await page
    .getByText(
      "No matches in the examined range; incomplete work or classification cannot prove absence.",
      { exact: true },
    )
    .waitFor();
  await page.getByRole("button", { name: "Next page", exact: true }).click();
  const inspect = page.getByRole("button", { name: "Inspect scale-large", exact: true });
  await inspect.waitFor();
  await inspect.focus();
  await page.keyboard.press("Enter");
  await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
  const expected = '{"integer":9007199254740993123,"large":"' + "é🦀".repeat(1800000) + '"}';
  assert.equal(Buffer.byteLength(expected), manifest.large_bytes);
  await page.getByRole("button", { name: "Copy content", exact: true }).focus();
  await page.keyboard.press("Enter");
  await clipboardEquals(page, expected);
  const copiedHeap = await sampleMemory();
  await page.getByRole("button", { name: "Refresh this view", exact: true }).waitFor();
  await page.getByRole("button", { name: "Refresh this view", exact: true }).click();
  await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
  await page.getByRole("button", { name: "Back", exact: true }).click();
  await page.getByRole("button", { name: "Refresh this view", exact: true }).waitFor();
  await page.getByRole("button", { name: "Refresh this view", exact: true }).click();
  await page.getByRole("heading", { name: "node records", exact: true }).waitFor();
  assert.equal(
    await page.getByLabel("Identity filter", { exact: true }).inputValue(),
    "scale-large",
  );
  const returnedHeap = await sampleMemory();
  assert.deepEqual(outside, []);
  await page.getByRole("button", { name: "Detach database", exact: true }).click();
  await page.getByRole("button", { name: "Attach database", exact: true }).waitFor();
  lockFree(paths.browser);
  const detachedHeap = await sampleMemory();
  stopping = true;
  await monitor;
  monitor = null;
  if (measurementError) throw measurementError;
  assert.ok(heapSamples >= 3, "successful post-baseline and milestone heap samples required");
  assert.ok(heapPeak - heapIdle <= 512 * 1024 * 1024);
  assert.ok(servicePeak - serviceIdle <= 1024 ** 3);
  report.browser = {
    large_exact_clipboard: true,
    keyboard_back_and_refresh: true,
    heap_idle_bytes: heapIdle,
    observed_heap_peak_bytes: heapPeak,
    heap_growth_bytes: heapPeak - heapIdle,
    heap_samples: heapSamples,
    milestone_heap_bytes: { copied: copiedHeap, returned: returnedHeap, detached: detachedHeap },
    external_requests: outside.length,
  };
  report.service = {
    unattached_supervisor_idle_bytes: serviceIdle,
    observed_peak_bytes: servicePeak,
    growth_bytes: servicePeak - serviceIdle,
  };
  report.sampling = {
    interval_ms: sampleMs,
    scope: "observed samples; not an instantaneous-allocation peak",
    native_execution_witness:
      "session main dispatch-thread CPU progress while real catalog response is pending",
    composition: "default/debug paired driver+session; installed release separate",
  };
} finally {
  stopping = true;
  let cleanupError = null;
  if (monitor) {
    try {
      await before(monitor, performance.now() + 6000, "monitor cleanup deadline");
    } catch (error) {
      cleanupError = error;
    }
  }
  if (browser) {
    try {
      await before(browser.close(), performance.now() + 10000, "browser cleanup deadline");
    } catch (error) {
      cleanupError ??= error;
    }
  }
  if (child && child.exitCode === null) {
    const exited = new Promise((resolve) => child.once("exit", resolve));
    child.stdin.end("close\n");
    try {
      await before(exited, performance.now() + 30000, "driver graceful cleanup deadline");
    } catch (error) {
      cleanupError ??= error;
      await killDescendantTree(child.pid, "SIGKILL");
      try {
        child.kill("SIGKILL");
      } catch {}
      try {
        await before(exited, performance.now() + 5000, "driver kill/reap cleanup deadline");
      } catch (failure) {
        cleanupError ??= failure;
      }
    }
  }
  try {
    const workersDeadline = performance.now() + 5000;
    for (const pid of ownedWorkers)
      await until(() => gone(pid), workersDeadline, "owned worker cleanup deadline");
  } catch (error) {
    cleanupError ??= error;
  }
  if (cleanupError) {
    console.error(`qualification cleanup failed; preserve owned scratch: ${scratch}`);
    throw cleanupError;
  }
  await rm(scratch, { recursive: true, force: true });
}

report.cleanup = "owned resources closed and scratch removed";
console.log(JSON.stringify(report));
