"""Run one bounded normal-BGE preparation series with actual owned CUDA accounting."""

import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

from test_g_text import GPU, _accounting, _remaining


def run(config, root):
    """Run the owned normal producer; refuse missing CUDA proof and preserve all output."""
    task=Path(os.environ["S30_GD_TASK"])
    binary=Path(os.environ["CARGO_TARGET_DIR"])/"release/examples/slice30_g_mean"
    if os.environ.get("CUDA_VISIBLE_DEVICES")!=GPU or os.environ.get("FATHOMDB_EMBED_DEVICE")!="cuda:0":
        raise RuntimeError("normal BGE template requires approved forced CUDA")
    model=[]
    for row in json.loads((task/"receipts/gd-pinned-inputs.json").read_text())["copied"]:
        path=Path(row["destination"])
        actual=hashlib.sha256(path.read_bytes()).hexdigest()
        if actual!=row["sha256"]:
            raise RuntimeError("pinned input invalid before Default loader")
        model.append({"path":str(path),"sha256":actual})
    cell=json.loads(config.read_text())
    paths=[config,*[Path(row["records"]) for row in cell["templates"]]]
    feature_path=task/"proofs/gd-gn-feature-closure.json"
    features=json.loads(feature_path.read_text())
    actual_binary=hashlib.sha256(binary.read_bytes()).hexdigest()
    if features["binary_sha256"]!=actual_binary or features["normal_gn"] is not True:
        raise RuntimeError("normal producer feature closure does not match executable")
    identity={"source":subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),
        "delta":subprocess.check_output(["git","status","--porcelain"],text=True),
        "binary":str(binary),"binary_sha256":actual_binary,"controller_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "inputs":[{"path":str(path),"sha256":hashlib.sha256(path.read_bytes()).hexdigest()} for path in paths],
        "pins":model,"feature_closure":features,"scope":"Default open/configure/governed seed/drain/verification/close series; no query timing"}
    (root/"identity.json").write_text(json.dumps(identity,indent=2)+"\n")
    print(json.dumps(identity),flush=True)
    snapshots=[]
    child=None
    (task/"locks").mkdir(exist_ok=True)
    with (task/"locks"/(GPU+".lock")).open("a") as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        before=_accounting(None,time.monotonic()+15)
        if before["util"]!=0 or before["total_MiB"]-before["used_MiB"]<8192:
            raise RuntimeError("approved GPU not idle/free for preparation")
        with (root/"child.log").open("wb") as log:
            child=subprocess.Popen([str(binary),str(config),str(root/"out")],stdin=subprocess.DEVNULL,stdout=log,stderr=subprocess.STDOUT)
            started=time.monotonic()
            work_end=started+150
            next_probe=started
            try:
                while child.poll() is None:
                    now=time.monotonic()
                    _remaining(work_end)
                    if now>=next_probe:
                        snapshots.append(_accounting(child.pid,work_end))
                        while next_probe<=time.monotonic():
                            next_probe+=0.1
                    try:
                        child.wait(timeout=min(max(0.001,next_probe-time.monotonic()),_remaining(work_end)))
                    except subprocess.TimeoutExpired:
                        pass
            finally:
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait(timeout=5)
                else:
                    child.wait(timeout=5)
                (root/"gpu-engagement.json").write_text(json.dumps({"before":before,"samples":snapshots,
                    "pid":child.pid,"exit":child.returncode,"probe_cadence_target_ms":100,
                    "scope":"whole real preparation series; no per-row or text-query inference claim"},indent=2)+"\n")
        if child.returncode:
            raise RuntimeError("normal preparation failed NOT_RED: "+(root/"child.log").read_text())
        if not any(row["own_MiB"]>0 and row["util"]>0 for row in snapshots):
            raise RuntimeError("actual own PID/UUID positive CUDA engagement absent, NOT_RED")


if __name__=="__main__":
    run(Path(sys.argv[1]),Path(sys.argv[2]))
