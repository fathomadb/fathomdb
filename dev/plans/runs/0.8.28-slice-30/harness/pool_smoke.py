"""0.8.28 Slice 30 qualification Python consumer: one process, one JSON line
on stdout. Adapted from the pool study's pool_smoke.py; the product reports
its allocator decision in the open report, so nothing is parsed from stderr.

Env: FATHOMDB_DB_SCRATCH (directory for the per-run database), CONSUMER_MODE
(import | full | perf | oversize | cycles | soak | fork), HEAP_OBJECTS (live
objects after import, before open), WARMUP_ITERS, TIMED_ITERS and
BATCH_SIZES (perf), OVERSIZE_BATCH (oversize), CYCLES (cycles), SOAK_SECONDS
and SOAK_MEM_FLOOR_MIB (soak), IDLE_AFTER_CLOSE_S, EXPECT_DEVICE
(cuda|cpu|any; default cuda), EXPECT_PATH (private|default_pool|synchronous|
any; default any). fork mode (hazard H-1): the parent imports fathomdb and
does not use the GPU, forks a child, and the child opens an engine and embeds;
the parent reports the child's result.
"""

from __future__ import annotations

import hashlib
import json
import os
import struct
import sys
import tempfile
import time
from typing import Any

PASSAGES = [
    {"id": 1, "body": "The Jetson AGX Orin shares DRAM between CPU and GPU.", "score": 0.5},
    {"id": 2, "body": "Stream-ordered allocation uses a per-device memory pool.", "score": 0.4},
]
TEXT = "fathomdb sync allocator repair experiment"


def status() -> dict[str, float]:
    values: dict[str, float] = {}
    with open("/proc/self/status", encoding="utf-8") as handle:
        for line in handle:
            key, _, rest = line.partition(":")
            if key in ("VmRSS", "VmSwap", "VmSize"):
                values[key] = int(rest.split()[0]) / 1024
    point = {"rssMiB": values.get("VmRSS", 0.0), "swapMiB": values.get("VmSwap", 0.0), "vszMiB": values.get("VmSize", 0.0)}
    with open("/proc/meminfo", encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("MemAvailable:"):
                point["memAvailMiB"] = int(line.split()[1]) / 1024
    return point


def sha(vec: list[float]) -> str:
    return hashlib.sha256(struct.pack(f"{len(vec)}d", *vec)).hexdigest()[:16]


def allocator_of(resolution: Any) -> dict[str, Any] | None:
    device = resolution.effective_device if resolution else None
    info = getattr(device, "cuda_device", None)
    alloc = getattr(info, "cuda_allocator", None)
    if alloc is None:
        return None
    return {
        "path": alloc.path,
        "reason": alloc.reason,
        "poolMaxSizeBytes": alloc.pool_max_size_bytes,
        "releaseThreshold": alloc.release_threshold,
        "moduleLoadInit": alloc.module_load_init,
    }


def alloc_key(alloc: dict[str, Any] | None) -> str:
    return f"{alloc['path']}/{alloc['reason']}" if alloc else "none"


def err_info(err: BaseException) -> dict[str, Any]:
    return {
        "name": type(err).__name__,
        "kind": getattr(err, "kind", None),
        "ordinal": getattr(err, "ordinal", None),
        "maxSizeBytes": getattr(err, "max_size_bytes", None),
        "message": str(err)[:300],
    }


def fork_child_result(fathomdb: Any) -> dict[str, Any]:
    """Runs in the forked child: open, embed, report."""
    res: dict[str, Any] = {"pid": os.getpid()}
    try:
        db_dir = tempfile.mkdtemp(dir=os.environ["FATHOMDB_DB_SCRATCH"])
        engine = fathomdb.Engine.open(f"{db_dir}/t.fdb", use_default_embedder=True)
        resolution = engine.open_report().embedder_device_resolution
        res["device"] = resolution.effective_device.kind if resolution else None
        res["reason"] = resolution.reason if resolution else None
        res["allocator"] = allocator_of(resolution)
        res["embedSha"] = sha(engine.embed(TEXT))
        engine.close()
        res["ok"] = True
    except Exception as err:  # noqa: BLE001 - the error is the measurement
        res["ok"] = False
        res["error"] = err_info(err)
    return res


def main() -> int:
    mode = os.environ.get("CONSUMER_MODE", "full")
    expect = os.environ.get("EXPECT_DEVICE", "cuda")
    expect_path = os.environ.get("EXPECT_PATH", "any")
    out: dict[str, Any] = {
        "outcome": "fail",
        "failedStep": None,
        "error": None,
        "python": sys.version.split()[0],
        "label": os.environ.get("QUAL_LABEL"),
        "poolMode": os.environ.get("FATHOMDB_POOL_MODE"),
        "earlyInit": os.environ.get("FATHOMDB_CUDA_EARLY_INIT"),
        "consumerMode": mode,
        "pid": os.getpid(),
        "mem": {"points": {}},
        "timingsMs": {},
        "allocatorsLater": [],
    }
    step = "import"
    keep: list[Any] = []
    engine = None
    try:
        out["mem"]["points"]["start"] = status()
        t0 = time.perf_counter()
        import fathomdb  # pyright: ignore[reportMissingImports]

        out["timingsMs"]["import"] = (time.perf_counter() - t0) * 1000
        native = getattr(fathomdb, "_fathomdb", None)
        probe = getattr(native, "_cuda_module_load_init", None)
        out["moduleLoadInitAtImport"] = probe() if probe else None
        step = "heap"
        keep.extend({"i": i, "s": f"obj-{i}", "a": [i, i + 1]} for i in range(int(os.environ.get("HEAP_OBJECTS", "0"))))
        out["mem"]["points"]["beforeOpen"] = status()
        if mode == "import":
            out["outcome"] = "pass"
        elif mode == "fork":
            step = "fork"
            read_fd, write_fd = os.pipe()
            pid = os.fork()
            if pid == 0:
                os.close(read_fd)
                os.write(write_fd, json.dumps(fork_child_result(fathomdb)).encode())
                os._exit(0)
            os.close(write_fd)
            chunks = []
            while True:
                chunk = os.read(read_fd, 65536)
                if not chunk:
                    break
                chunks.append(chunk)
            _, wait_status = os.waitpid(pid, 0)
            raw = b"".join(chunks).decode() or "null"
            out["forkChild"] = {"result": json.loads(raw), "waitStatus": wait_status}
            out["outcome"] = "pass"
        elif mode == "soak":
            step = "soak"
            seconds = float(os.environ.get("SOAK_SECONDS", "1200"))
            floor_mib = float(os.environ.get("SOAK_MEM_FLOOR_MIB", "8192"))
            db_dir = tempfile.mkdtemp(dir=os.environ["FATHOMDB_DB_SCRATCH"])
            engine = fathomdb.Engine.open(f"{db_dir}/t.fdb", use_default_embedder=True)
            out["allocator"] = allocator_of(engine.open_report().embedder_device_resolution)
            want = sha(engine.embed(TEXT))
            out["embedSha"] = want
            soak: dict[str, Any] = {"samples": [], "iterations": 0, "mismatches": 0, "errors": [], "aborted": None}
            out["soak"] = soak
            t_start = time.perf_counter()
            next_sample = t_start + 60
            embed_ms: list[float] = []
            rerank_ms: list[float] = []
            batch_ms: list[float] = []

            def med(xs: list[float]) -> float | None:
                return sorted(xs)[len(xs) // 2] if xs else None

            i = 0
            while time.perf_counter() - t_start < seconds:
                try:
                    t1 = time.perf_counter()
                    v = engine.embed(TEXT)
                    embed_ms.append((time.perf_counter() - t1) * 1000)
                    if sha(v) != want:
                        soak["mismatches"] += 1
                    t1 = time.perf_counter()
                    fathomdb.embed_batch_cls([f"soak {i} item {k} about pools" for k in range(32)])
                    batch_ms.append((time.perf_counter() - t1) * 1000)
                    t1 = time.perf_counter()
                    fathomdb.rerank("Which device shares DRAM?", PASSAGES, 2)
                    rerank_ms.append((time.perf_counter() - t1) * 1000)
                except Exception as err:  # noqa: BLE001 - the error is the measurement
                    soak["errors"].append(err_info(err))
                i += 1
                if time.perf_counter() >= next_sample:
                    keep.extend({"i": k} for k in range(200_000))
                    keep.clear()
                    st = status()
                    soak["samples"].append(
                        {
                            "minute": round((time.perf_counter() - t_start) / 60),
                            "rssMiB": st["rssMiB"],
                            "swapMiB": st["swapMiB"],
                            "memAvailMiB": st.get("memAvailMiB"),
                            "embedMs": med(embed_ms),
                            "batch32Ms": med(batch_ms),
                            "rerankMs": med(rerank_ms),
                            "iterations": len(embed_ms),
                        }
                    )
                    embed_ms, rerank_ms, batch_ms = [], [], []
                    next_sample += 60
                    if st.get("memAvailMiB", floor_mib + 1) < floor_mib:
                        soak["aborted"] = f"MemAvailable {round(st['memAvailMiB'])} MiB below {round(floor_mib)} MiB"
                        break
            soak["iterations"] = i
            engine.close()
            engine = None
            ok = soak["mismatches"] == 0 and soak["aborted"] is None and not soak["errors"]
            out["outcome"] = "pass" if ok else "fail"
            if not ok:
                out["failedStep"] = "soak-floor" if soak["aborted"] else "soak-identity-or-error"
        elif mode == "cycles":
            paths: set[str] = set()
            out["cyclesRssMiB"] = []
            for c in range(int(os.environ.get("CYCLES", "50"))):
                step = f"cycle-{c}-open"
                db_dir = tempfile.mkdtemp(dir=os.environ["FATHOMDB_DB_SCRATCH"])
                engine = fathomdb.Engine.open(f"{db_dir}/t.fdb", use_default_embedder=True)
                paths.add(alloc_key(allocator_of(engine.open_report().embedder_device_resolution)))
                step = f"cycle-{c}-embed"
                engine.embed(f"cycle {c} embedding")
                step = f"cycle-{c}-close"
                engine.close()
                engine = None
                out["cyclesRssMiB"].append(round(status()["rssMiB"], 2))
            out["cyclePaths"] = sorted(paths)
            out["outcome"] = "pass"
        else:
            step = "open"
            db_dir = tempfile.mkdtemp(dir=os.environ["FATHOMDB_DB_SCRATCH"])
            t0 = time.perf_counter()
            engine = fathomdb.Engine.open(f"{db_dir}/t.fdb", use_default_embedder=True)
            out["timingsMs"]["open"] = (time.perf_counter() - t0) * 1000
            report = engine.open_report()
            resolution = report.embedder_device_resolution
            out["embedderDevice"] = resolution.effective_device.kind if resolution else None
            out["embedderReason"] = resolution.reason if resolution else None
            out["allocator"] = allocator_of(resolution)
            out["rerankerAllocator"] = allocator_of(report.reranker_device_resolution)
            if expect != "any" and out["embedderDevice"] != expect:
                raise RuntimeError(f"embedder device {out['embedderDevice']}, want {expect}")
            if expect_path != "any" and (out["allocator"] or {}).get("path") != expect_path:
                raise RuntimeError(f"allocator path {(out['allocator'] or {}).get('path')}, want {expect_path}")
            step = "embed"
            out["embedSha"] = sha(engine.embed(TEXT))
            step = "rerank"
            out["rerankScores"] = [r["ce_score"] for r in fathomdb.rerank("How does CUDA stream-ordered allocation work?", PASSAGES, 2)]
            step = "embedBatchCls"
            out["clsSha"] = sha(fathomdb.embed_batch_cls([TEXT])[0])
            if mode == "perf":
                step = "perf"
                warm = int(os.environ.get("WARMUP_ITERS", "5"))
                timed = int(os.environ.get("TIMED_ITERS", "50"))

                def timed_ms(call: Any) -> list[float]:
                    kept = []
                    for i in range(warm + timed):
                        t1 = time.perf_counter()
                        call(i)
                        if i >= warm:
                            kept.append((time.perf_counter() - t1) * 1000)
                    return kept

                out["timingsMs"]["embedSteady"] = timed_ms(lambda i: engine.embed(f"steady state embedding number {i} about tegra allocators"))
                out["timingsMs"]["embedBatch"] = {}
                for size in [int(x) for x in os.environ.get("BATCH_SIZES", "1,8,32,128").split(",")]:
                    texts = [f"batch {size} item {k} about unified memory" for k in range(size)]
                    out["timingsMs"]["embedBatch"][str(size)] = timed_ms(lambda _i, texts=texts: fathomdb.embed_batch_cls(texts))
                out["timingsMs"]["rerankSteady"] = timed_ms(lambda _i: fathomdb.rerank("Which device shares DRAM?", PASSAGES, 2))
            out["mem"]["points"]["afterWork"] = status()
            if mode == "oversize":
                step = "oversize"
                long = " ".join(f"token{k}" for k in range(400))
                over: dict[str, Any] = {"batch": int(os.environ.get("OVERSIZE_BATCH", "128"))}
                try:
                    fathomdb.embed_batch_cls([f"{k} {long}" for k in range(over["batch"])])
                    over["ok"] = True
                except Exception as err:  # noqa: BLE001 - the error is the measurement
                    over["ok"] = False
                    over["error"] = err_info(err)
                step = "embedAfterOversize"
                over["hashAfter"] = sha(engine.embed(TEXT))
                over["embedAfterOk"] = True
                over["sameHash"] = over["hashAfter"] == out["embedSha"]
                after = engine.open_report().embedder_device_resolution
                over["deviceAfter"] = after.effective_device.kind if after else None
                over["pathAfter"] = alloc_key(allocator_of(after))
                fathomdb.embed_batch_cls(["a short batch after the oversized one"])
                out["oversize"] = over
            step = "close"
            engine.close()
            engine = None
            out["mem"]["points"]["afterClose"] = status()
            out["outcome"] = "pass"
    except Exception as err:  # noqa: BLE001 - recorded, process exits 1
        out["failedStep"] = step
        out["error"] = err_info(err)
    finally:
        if engine is not None:
            try:
                engine.close()
            except Exception:  # noqa: BLE001 - the failure is already recorded
                pass
    idle = float(os.environ.get("IDLE_AFTER_CLOSE_S", "0"))
    if idle > 0:
        time.sleep(idle)
        out["mem"]["points"]["afterIdle"] = status()
    out["mem"]["points"]["end"] = status()
    out["keep"] = len(keep)
    print(json.dumps(out), flush=True)
    return 0 if out["outcome"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
