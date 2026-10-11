"""Slice 110 allocator-fix smoke for the installed Tegra Python wheel.

Usage: python python_smoke.py <db-path> [fragment]

With `fragment`, three 4 KiB PROT_NONE pages are mapped at 38, 68 and 98 GiB
before fathomdb (and so CUDA) is imported, which leaves no room for the
driver's default memory pool. Forced CUDA must still open, write, search and
produce an allocation witness.
"""

import ctypes
import json
import os
import sys
import time

if len(sys.argv) > 2 and sys.argv[2] == "fragment":
    libc = ctypes.CDLL(None, use_errno=True)
    libc.mmap.restype = ctypes.c_void_p
    libc.mmap.argtypes = [
        ctypes.c_void_p,
        ctypes.c_size_t,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_long,
    ]
    MAP_PRIVATE, MAP_ANONYMOUS, MAP_FIXED_NOREPLACE = 0x02, 0x20, 0x100000
    for gib in (38, 68, 98):
        address = gib << 30
        mapped = libc.mmap(
            address, 4096, 0, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0
        )
        if mapped != address:
            raise SystemExit(
                f"blocker at {address:#x} failed: errno {ctypes.get_errno()}"
            )

os.environ["FATHOMDB_EMBED_DEVICE"] = "cuda:0"
os.environ["FATHOMDB_GPU_ALLOCATION_WITNESS"] = "1"
from fathomdb import Engine  # noqa: E402

out = {"layout": sys.argv[2] if len(sys.argv) > 2 else "normal", "outcome": "fail"}
t0 = time.perf_counter()
engine = Engine.open(sys.argv[1], use_default_embedder=True)
try:
    out["open_ms"] = round((time.perf_counter() - t0) * 1000, 1)
    report = engine.open_report()
    resolution = report.embedder_device_resolution
    witness = report.embedder_gpu_allocation_witness
    out["device"] = resolution.effective_device.kind if resolution else None
    out["witness_delta"] = witness.delta_bytes if witness else None
    if out["device"] != "cuda" or witness is None:
        raise SystemExit(f"forced CUDA did not produce CUDA and a witness: {out}")
    engine.write(
        [{"kind": "doc", "body": "{}", "source_id": "smoke:slice110-allocator"}]
    )
    engine.search("smoke")
    out["outcome"] = "pass"
finally:
    engine.close()
print(json.dumps(out))
