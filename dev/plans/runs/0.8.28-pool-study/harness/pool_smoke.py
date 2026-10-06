"""0.8.28 pool study Python consumer (protocol section 4.3): one process, one
JSON line on stdout.

Env: FATHOMDB_DB_SCRATCH (directory for the per-run database), HEAP_OBJECTS
(live Python objects created after import, before open), CONSUMER_MODE
(full | perf | oversize | coresident | reset), WARMUP_ITERS, TIMED_ITERS and
BATCH_SIZES (perf mode), OVERSIZE_BATCH (oversize mode), MAPS_DIR. The
runner sets the device policy and the FATHOMDB_POOL_* variables; the
experiment wheel's import performs early cuInit (study ruling 1).

reset mode (C7): after the first embed, rerank and CLS batch, the primary
context is reset through ctypes, then all three run again, each on its own,
and their errors (or results) and the driver's context state are recorded.

coresident mode (C9 co-resident user): after FathomDB has embedded, a second
CUDA user in the same process queries the default pool and allocates 16 MiB
from it with cuMemAllocAsync through ctypes, as another library would.
"""

from __future__ import annotations

import ctypes
import hashlib
import json
import os
import struct
import sys
import tempfile
import time
from typing import Any

MIB = 1 << 20
CUDA_SUCCESS = 0
CU_MEMPOOL_ATTR_USED_MEM_CURRENT = 7


def status() -> dict[str, float]:
    values: dict[str, float] = {}
    with open("/proc/self/status", encoding="utf-8") as handle:
        for line in handle:
            key, _, rest = line.partition(":")
            if key in ("VmRSS", "VmSwap", "VmSize"):
                values[key] = int(rest.split()[0]) / 1024
    point = {"rssMiB": values.get("VmRSS", 0.0), "swapMiB": values.get("VmSwap", 0.0), "vszMiB": values.get("VmSize", 0.0)}
    # System-wide MemAvailable, for the CB1 sanity check (harness/cb1check.py).
    with open("/proc/meminfo", encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("MemAvailable:"):
                point["memAvailMiB"] = int(line.split()[1]) / 1024
    return point


def sha(vec: list[float]) -> str:
    return hashlib.sha256(struct.pack(f"{len(vec)}d", *vec)).hexdigest()[:16]


def snap(stage: str) -> None:
    maps_dir = os.environ.get("MAPS_DIR")
    if maps_dir:
        with open("/proc/self/maps", encoding="utf-8") as src, open(
            f"{maps_dir}/maps-{os.getpid()}-{stage}.txt", "w", encoding="utf-8"
        ) as dst:
            dst.write(src.read())


def coresident_user() -> dict[str, Any]:
    """A second CUDA user in this process: default-pool query, then a 16 MiB
    stream-ordered allocation on the primary context."""
    cuda = ctypes.CDLL("libcuda.so.1")
    dev = ctypes.c_int()
    ctx = ctypes.c_void_p()
    out: dict[str, Any] = {}
    cuda.cuDeviceGet(ctypes.byref(dev), 0)
    cuda.cuDevicePrimaryCtxRetain(ctypes.byref(ctx), dev)
    cuda.cuCtxSetCurrent(ctx)
    current = ctypes.c_void_p()
    default = ctypes.c_void_p()
    out["currentPoolRc"] = cuda.cuDeviceGetMemPool(ctypes.byref(current), dev)
    out["defaultPoolRc"] = cuda.cuDeviceGetDefaultMemPool(ctypes.byref(default), dev)
    out["currentIsDefault"] = bool(current.value) and current.value == default.value
    used0 = ctypes.c_uint64()
    used1 = ctypes.c_uint64()
    if out["defaultPoolRc"] == CUDA_SUCCESS:
        cuda.cuMemPoolGetAttribute(default, CU_MEMPOOL_ATTR_USED_MEM_CURRENT, ctypes.byref(used0))
    ptr = ctypes.c_uint64()
    out["allocRc"] = cuda.cuMemAllocAsync(ctypes.byref(ptr), ctypes.c_size_t(16 * MIB), None)
    cuda.cuCtxSynchronize()
    if out["defaultPoolRc"] == CUDA_SUCCESS:
        cuda.cuMemPoolGetAttribute(default, CU_MEMPOOL_ATTR_USED_MEM_CURRENT, ctypes.byref(used1))
        out["defaultUsedDelta"] = used1.value - used0.value
    if out["allocRc"] == CUDA_SUCCESS:
        cuda.cuMemFreeAsync(ctypes.c_uint64(ptr.value), None)
        cuda.cuCtxSynchronize()
    cuda.cuDevicePrimaryCtxRelease(dev)
    return out


def primary_ctx_reset() -> dict[str, Any]:
    """cuDevicePrimaryCtxReset(0) through ctypes, as a co-resident library
    would (C7)."""
    cuda = ctypes.CDLL("libcuda.so.1")
    dev = ctypes.c_int()
    cuda.cuDeviceGet(ctypes.byref(dev), 0)
    return {"resetRc": cuda.cuDevicePrimaryCtxReset(dev)}


def driver_state() -> dict[str, Any]:
    """What the driver reports right after the failed calls: the current
    context and a synchronize on it (C7)."""
    cuda = ctypes.CDLL("libcuda.so.1")
    ctx = ctypes.c_void_p()
    return {
        "getCurrentRc": cuda.cuCtxGetCurrent(ctypes.byref(ctx)),
        "hasCurrent": bool(ctx.value),
        "synchronizeRc": cuda.cuCtxSynchronize(),
    }


def main() -> int:
    out: dict[str, Any] = {
        "outcome": "fail",
        "failedStep": None,
        "error": None,
        "python": sys.version.split()[0],
        "variant": os.environ.get("FATHOMDB_POOL_VARIANT", "S"),
        "consumerMode": os.environ.get("CONSUMER_MODE", "full"),
        "pid": os.getpid(),
        "mem": {"points": {}},
        "timingsMs": {},
    }
    step = "import"
    keep: list[Any] = []
    engine = None
    try:
        out["mem"]["points"]["start"] = status()
        t0 = time.perf_counter()
        import fathomdb  # pyright: ignore[reportMissingImports]

        out["timingsMs"]["import"] = (time.perf_counter() - t0) * 1000
        step = "heap"
        keep.extend({"i": i, "s": f"obj-{i}", "a": [i, i + 1]} for i in range(int(os.environ.get("HEAP_OBJECTS", "0"))))
        out["mem"]["points"]["beforeOpen"] = status()
        snap("before-open")
        step = "open"
        db_dir = tempfile.mkdtemp(dir=os.environ["FATHOMDB_DB_SCRATCH"])
        t0 = time.perf_counter()
        engine = fathomdb.Engine.open(f"{db_dir}/t.fdb", use_default_embedder=True)
        out["timingsMs"]["open"] = (time.perf_counter() - t0) * 1000
        report = engine.open_report()
        resolution = report.embedder_device_resolution
        out["embedderDevice"] = resolution.effective_device.kind if resolution else None
        step = "embed"
        text = "fathomdb sync allocator repair experiment"
        out["embedSha"] = sha(engine.embed(text))
        step = "rerank"
        passages = [
            {"id": 1, "body": "The Jetson AGX Orin shares DRAM between CPU and GPU.", "score": 0.5},
            {"id": 2, "body": "Stream-ordered allocation uses a per-device memory pool.", "score": 0.4},
        ]
        out["rerankScores"] = [r["ce_score"] for r in fathomdb.rerank("How does CUDA stream-ordered allocation work?", passages, 2)]
        step = "embedBatchCls"
        out["clsSha"] = sha(fathomdb.embed_batch_cls([text])[0])
        if out["consumerMode"] == "perf":
            # P7: the Node consumer's perf measures, per-process lists of ms.
            step = "perf"
            warm = int(os.environ.get("WARMUP_ITERS", "5"))
            timed = int(os.environ.get("TIMED_ITERS", "50"))

            def timed_ms(call: Any) -> list[float]:
                kept = []
                for i in range(warm + timed):
                    t0 = time.perf_counter()
                    call(i)
                    if i >= warm:
                        kept.append((time.perf_counter() - t0) * 1000)
                return kept

            out["timingsMs"]["embedSteady"] = timed_ms(lambda i: engine.embed(f"steady state embedding number {i} about tegra allocators"))
            out["timingsMs"]["embedBatch"] = {}
            for size in [int(x) for x in os.environ.get("BATCH_SIZES", "1,8,32,128").split(",")]:
                texts = [f"batch {size} item {k} about unified memory" for k in range(size)]
                out["timingsMs"]["embedBatch"][str(size)] = timed_ms(lambda _i, texts=texts: fathomdb.embed_batch_cls(texts))
            out["timingsMs"]["rerankSteady"] = timed_ms(lambda _i: fathomdb.rerank("Which device shares DRAM?", passages, 2))
        out["mem"]["points"]["afterWork"] = status()
        if out["consumerMode"] == "oversize":
            step = "oversize"
            long = " ".join(f"token{k}" for k in range(400))
            over: dict[str, Any] = {"batch": int(os.environ.get("OVERSIZE_BATCH", "128"))}
            try:
                fathomdb.embed_batch_cls([f"{k} {long}" for k in range(over["batch"])])
                over["ok"] = True
            except Exception as err:  # noqa: BLE001 - the error is the measurement
                over["ok"] = False
                over["error"] = {
                    "name": type(err).__name__,
                    "kind": getattr(err, "kind", None),
                    "ordinal": getattr(err, "ordinal", None),
                    "maxSizeBytes": getattr(err, "max_size_bytes", None),
                    "message": str(err)[:300],
                }
            step = "embedAfterOversize"
            over["hashAfter"] = sha(engine.embed(text))
            over["sameHash"] = over["hashAfter"] == out["embedSha"]
            after = engine.open_report().embedder_device_resolution
            over["deviceAfter"] = after.effective_device.kind if after else None
            out["oversize"] = over
        if out["consumerMode"] == "coresident":
            step = "coresident"
            out["coresident"] = coresident_user()
        if out["consumerMode"] == "reset":
            # C7 co-resident reset: another library resets the primary
            # context while FathomDB holds model weights on the device. Each
            # call afterwards is tried on its own so every first error is kept.
            step = "reset"
            out["reset"] = primary_ctx_reset()
            after: dict[str, Any] = {}

            def attempt(name: str, call: Any) -> None:
                try:
                    after[name] = {"ok": True, "value": call()}
                except Exception as err:  # noqa: BLE001 - the error is the measurement
                    after[name] = {"ok": False, "name": type(err).__name__, "kind": getattr(err, "kind", None), "message": str(err)[:300]}

            attempt("embedBatchCls", lambda: sha(fathomdb.embed_batch_cls([text])[0]) == out["clsSha"])
            attempt("embed", lambda: sha(engine.embed(text)) == out["embedSha"])
            attempt(
                "rerank",
                lambda: [r["ce_score"] for r in fathomdb.rerank("How does CUDA stream-ordered allocation work?", passages, 2)]
                == out["rerankScores"],
            )
            out["reset"]["after"] = after
            out["reset"]["driverAfter"] = driver_state()
            if not all(v["ok"] and v["value"] for v in after.values()):
                raise RuntimeError("calls after the reset failed or changed: " + json.dumps(after)[:300])
        step = "close"
        engine.close()
        engine = None
        out["mem"]["points"]["afterClose"] = status()
        out["outcome"] = "pass"
    except Exception as err:  # noqa: BLE001 - recorded, process exits 1
        out["failedStep"] = step
        out["error"] = {"name": type(err).__name__, "kind": getattr(err, "kind", None), "message": str(err)[:500]}
    finally:
        if engine is not None:
            try:
                engine.close()
            except Exception:  # noqa: BLE001 - the failure is already recorded
                pass
    out["mem"]["points"]["end"] = status()
    out["keep"] = len(keep)
    print(json.dumps(out), flush=True)
    return 0 if out["outcome"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
