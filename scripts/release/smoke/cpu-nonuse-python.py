#!/usr/bin/env python3
"""Exercise the installed binding without calling CUDA availability/device probes."""

import json
import os
import time
from pathlib import Path

import fathomdb
from fathomdb import Engine

root = Path(os.environ["FATHOMDB_CAPTURE_DIR"])
engine = Engine.open(str(root / "lifecycle.fdb"), use_default_embedder=True)
resolution = engine.open_report().embedder_device_resolution
vector = engine.embed("installed CUDA artifact explicit device lifecycle")
engine.write(
    [
        {"kind": "doc", "body": "cuda package rerank smoke one", "source_id": "nonuse"},
        {"kind": "doc", "body": "cuda package rerank smoke two", "source_id": "nonuse"},
    ]
)
result = engine.search("cuda package rerank smoke", rerank_depth=2, limit=2)
ready = {
    "pid": os.getpid(),
    "policy": resolution.requested_policy,
    "cuda_compiled": resolution.cuda_compiled,
    "effective_device": resolution.effective_device.kind,
    "selected_uuid": resolution.selected_cuda_uuid,
    "module_path": str(Path(fathomdb.__file__).resolve()),
    "vector_dim": len(vector),
    "result_count": len(result.results),
    "reranked": len(result.results) == 2
    and all(row.ce_score is not None for row in result.results),
}
(root / "ready.tmp").write_text(json.dumps(ready) + "\n")
(root / "ready.tmp").replace(root / "ready.json")
deadline = time.monotonic() + 30
while not (root / "release").exists():
    if time.monotonic() > deadline:
        raise TimeoutError("observer did not release the live consumer")
    time.sleep(0.1)
engine.close()
(root / "done.json").write_text(json.dumps({"pid": os.getpid(), "closed": True}) + "\n")
print("installed Python lifecycle closed", flush=True)
