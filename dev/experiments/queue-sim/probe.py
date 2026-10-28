"""Live burst probe of the staged fathomdb-mcp over stdio.

usage: python -I probe.py <binary> <locomo10.json> <out.csv>
Writes k tools/call requests in ONE stdin write (a fan-out burst, as Claude Code dispatches a turn) and
timestamps each response line as it arrives on stdout. This is the server-side time at the stdio boundary:
no LLM client in the loop. Also sweeps concurrency 1..7 at cap 7 for service time against concurrency.
"""
import csv, json, os, random, subprocess, sys, threading, time

BIN, LOCOMO, OUT = sys.argv[1:4]
HERE = os.path.dirname(os.path.abspath(__file__))
qs = [q["question"] for q in json.load(open(LOCOMO))[0]["qa"]]
rng = random.Random(7)
rows = []


class Server:
    def __init__(self, cfg):
        env = {**os.environ, "CUDA_VISIBLE_DEVICES": "0", "FATHOMDB_EMBED_DEVICE": "cuda:0"}
        self.p = subprocess.Popen([BIN, "--config", cfg], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, env=env)
        self.recv = {}
        self.cv = threading.Condition()
        self.nid = 0
        threading.Thread(target=self._reader, daemon=True).start()
        self.call_sync("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "probe", "version": "0"}})
        self._write([{"jsonrpc": "2.0", "method": "notifications/initialized"}])

    def _reader(self):
        for line in self.p.stdout:
            t = time.perf_counter()
            o = json.loads(line)
            if "id" in o:
                with self.cv:
                    self.recv[o["id"]] = (t, o)
                    self.cv.notify_all()

    def _write(self, msgs):
        self.p.stdin.write(b"".join((json.dumps(m) + "\n").encode() for m in msgs))
        self.p.stdin.flush()

    def burst(self, arglist):
        ids = []
        msgs = []
        for args in arglist:
            self.nid += 1
            ids.append(self.nid)
            msgs.append({"jsonrpc": "2.0", "id": self.nid, "method": "tools/call", "params": {"name": "search", "arguments": args}})
        t0 = time.perf_counter()
        self._write(msgs)
        with self.cv:
            self.cv.wait_for(lambda: all(i in self.recv for i in ids), timeout=60)
        out = []
        for i in ids:
            t, o = self.recv.pop(i)
            r = o.get("result", {})
            txt = json.dumps(r)
            out.append(((t - t0) * 1000, "busy" if "busy/admission" in txt else ("err" if r.get("isError") else "ok")))
        return out

    def call_sync(self, m, params):
        self.nid += 1
        i = self.nid
        self._write([{"jsonrpc": "2.0", "id": i, "method": m, "params": params}])
        with self.cv:
            self.cv.wait_for(lambda: i in self.recv, timeout=120)
        return self.recv.pop(i)[1]

    def close(self):
        self.p.stdin.close()
        self.p.wait(30)


def rec(phase, cap, mode, k, rep, res):
    for j, (ms, outc) in enumerate(sorted(res)):
        rows.append({"phase": phase, "cap": cap, "mode": mode, "k": k, "rep": rep, "rank": j, "ms": round(ms, 3), "outcome": outc})


def args(mode):
    return {"query": rng.choice(qs), "mode": mode, "limit": 10}


# 1) cold: fresh server, the first MCP traffic is a 10-call hybrid burst (as in the incident)
for rep in range(5):
    s = Server(os.path.join(HERE, "probe-config-cap4.json"))
    rec("cold_first_burst", 4, "hybrid", 10, rep, s.burst([args("hybrid") for _ in range(10)]))
    time.sleep(0.5)
    rec("second_burst_0.5s_later", 4, "hybrid", 10, rep, s.burst([args("hybrid") for _ in range(10)]))
    s.close()
# 2) warm bursts at cap 4 and cap 7, k = 6 and 10, text and hybrid, after 3 s idle (an LLM turn)
for cap in (4, 7):
    s = Server(os.path.join(HERE, f"probe-config-cap{cap}.json"))
    for _ in range(20):
        s.burst([args("hybrid")])
    for rep in range(15):
        for mode in ("text", "hybrid"):
            for k in (6, 10):
                time.sleep(3.0)
                rec("warm_after_3s_idle", cap, mode, k, rep, s.burst([args(mode) for _ in range(k)]))
    # 3) concurrency sweep, back-to-back, k <= cap so nothing is refused
    if cap == 7:
        for mode in ("text", "hybrid"):
            for k in range(1, 8):
                for rep in range(40):
                    rec("concurrency_sweep", cap, mode, k, rep, s.burst([args(mode) for _ in range(k)]))
    s.close()

with open(OUT, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
print("rows", len(rows))
