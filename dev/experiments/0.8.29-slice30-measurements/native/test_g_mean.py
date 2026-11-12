"""Manual pinned3090 core-mean population oracle; source TDD, not a campaign."""

import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from control import read_jsonl_records

GPU = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b"
SAMPLES = 128


def query_gpu():
    child = subprocess.run(["nvidia-smi","-i",GPU,
        "--query-gpu=uuid,name,driver_version,memory.total,memory.used,utilization.gpu",
        "--format=csv,noheader,nounits"],capture_output=True,text=True,timeout=10)
    if child.returncode:
        raise RuntimeError("GPU inventory unavailable: "+child.stderr)
    values = [value.strip() for value in child.stdout.strip().split(",")]
    if len(values)!=6 or values[0]!=GPU:
        raise RuntimeError("approved GPU inventory did not resolve exactly")
    return {"uuid":values[0],"name":values[1],"driver":values[2],
            "total_MiB":int(values[3]),"used_MiB":int(values[4]),"util":int(values[5])}


def own_memory(pid):
    child = subprocess.run(["nvidia-smi","--query-compute-apps=pid,gpu_uuid,used_gpu_memory",
        "--format=csv,noheader,nounits"],capture_output=True,text=True,timeout=10)
    if child.returncode:
        raise RuntimeError("GPU process inventory unavailable: "+child.stderr)
    rows=[]
    for line in child.stdout.splitlines():
        values = [value.strip() for value in line.split(",")]
        if len(values)==3 and values[1]==GPU:
            rows.append({"pid":int(values[0]),"uuid":values[1],"used_MiB":int(values[2])})
    return next((row["used_MiB"] for row in rows if row["pid"]==pid),0),rows


class CoreMeanTests(unittest.TestCase):
    adapter = "engine"
    mode = "engine_mean"

    def test_actual_mean_calls_emit_each_measurement_with_real_gpu_engagement(self):
        if os.environ.get("CUDA_VISIBLE_DEVICES")!=GPU or os.environ.get("FATHOMDB_EMBED_DEVICE")!="cuda:0":
            raise RuntimeError("manual fixture requires the exact reserved3090 and forced CUDA")
        task = Path(os.environ["S30_GD_TASK"])
        binary = Path(os.environ["CARGO_TARGET_DIR"])/"release/examples/slice30_g_mean"
        root = task/"proofs"/"g-mean"/str(time.time_ns())
        root.mkdir(parents=True)
        identity = {"binary":str(binary),"sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),
                    "test_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "runner":str(Path(sys.argv[0]).resolve()),
                    "runner_sha256":hashlib.sha256(Path(sys.argv[0]).read_bytes()).hexdigest(),
                    "source":subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),
                    "delta":subprocess.check_output(["git","status","--porcelain"],text=True),
                    "features":["embed-cuda"],"observer":False,"test_hooks":False,"root":str(root)}
        (root/"identity.json").write_text(json.dumps(identity,indent=2)+"\n")
        print(json.dumps(identity),flush=True)
        pins = json.loads((task/"receipts/gd-pinned-inputs.json").read_text())
        for row in pins["copied"]:
            path = Path(row["destination"])
            if hashlib.sha256(path.read_bytes()).hexdigest()!=row["sha256"]:
                raise RuntimeError("pinned input unavailable before loader: "+str(path))
        config = root/"cell.json"
        config.write_text(json.dumps({"mode":self.mode,"gpu_uuid":GPU,
            "samples":SAMPLES,"query":"alpha "*510}))
        locks = task/"locks"
        locks.mkdir(exist_ok=True)
        samples=[]
        with (locks/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before=query_gpu()
            _,other=own_memory(-1)
            if "3090" not in before["name"] or not before["driver"].startswith("580."):
                raise RuntimeError("approved3090/R580 identity unavailable")
            if before["util"]!=0 or before["total_MiB"]-before["used_MiB"]<8192 or other:
                raise RuntimeError("approved3090 not idle/free for exclusive source fixture")
            with (root/"child.log").open("wb") as log:
                child=subprocess.Popen([str(binary),str(config),str(root/"out")],stdout=log,stderr=subprocess.STDOUT)
                started=time.monotonic()
                try:
                    while child.poll() is None:
                        gpu=query_gpu()
                        memory,_=own_memory(child.pid)
                        samples.append({"at_s":time.monotonic()-started,"gpu":gpu,
                                        "pid":child.pid,"own_used_MiB":memory})
                        if memory>8192:
                            raise RuntimeError("owned GPU memory exceeds fixed8GiB cap")
                        if time.monotonic()-started>180:
                            raise RuntimeError("bounded source fixture deadline exceeded")
                        time.sleep(1)
                finally:
                    if child.poll() is None:
                        child.terminate()
                        try:
                            child.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            child.kill()
                            child.wait(timeout=5)
                    else:
                        child.wait()
                    (root/"gpu-engagement.json").write_text(json.dumps({"before":before,
                        "samples":samples,"pid":child.pid,"exit":child.returncode},indent=2)+"\n")
            if child.returncode:
                raise RuntimeError("real mean API fixture failed: "+(root/"child.log").read_text())
            if not any(row["own_used_MiB"]>0 and row["gpu"]["util"]>0 for row in samples):
                raise RuntimeError("no own PID/UUID memory plus positive utilization during BGE work")
        proof=json.loads((root/"out/api-proof.json").read_text())
        if proof["api_calls"]!=SAMPLES or proof["dimension"]!=384 or proof["finite_nonzero"] is not True:
            raise RuntimeError("existing real mean API validation did not pass")
        if proof["selected_cuda_uuid"]!=GPU or proof["cuda_ordinal"]!=0 or proof["cuda_compiled"] is not True:
            raise RuntimeError("runtime OpenReport did not attest the reserved CUDA route")
        issues=[]
        rows=read_jsonl_records(root/"out/samples.jsonl",issues)
        print(json.dumps({"api_proof":proof,"observed_records":len(rows),"issues":issues}),flush=True)
        self.assertEqual(len(rows),SAMPLES,f"missing actual {self.adapter} mean measurement population")
        self.assertEqual(issues,[])
        for index,row in enumerate(rows):
            self.assertEqual((row["sample_index"],row["query_index"]),(index,0))
            self.assertEqual((row["adapter"],row["mode"],row["ok"]),(self.adapter,"mean_embed",True))
            self.assertEqual((row["dimension"],row["finite"]),(384,True))
            self.assertGreater(row["norm_squared"],0)
            self.assertGreater(row["elapsed_ns"],0)
        persistence=proof["raw_persistence"]
        self.assertEqual(persistence["records"],SAMPLES)
        self.assertEqual(persistence["checkpoints"],8)
        self.assertEqual(persistence["checkpoint_records"],16)
        self.assertEqual(persistence["encoded_bytes"],(root/"out/samples.jsonl").stat().st_size)
        self.assertGreater(persistence["checkpoint_ns"],0)
        self.assertGreaterEqual(persistence["persistence_ns"],persistence["checkpoint_ns"])


if __name__ == "__main__":
    unittest.main()
