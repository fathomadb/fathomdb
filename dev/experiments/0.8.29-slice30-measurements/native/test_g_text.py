"""Manual text-only fixture: closed BGE copy, bounded accounting, literal records."""

import fcntl
import hashlib
import json
import os
from pathlib import Path
import selectors
import shutil
import subprocess
import sys
import time
import unittest

import test_g_mean as inference
from gd_device import _remaining, _accounting

GPU = inference.GPU
SOURCE_HASH = "f24e6f6b4f87d661fcdcda763819a03db66be8b7ebe5bac6cd9330b21cbbbdf4"


def _ready_line(child, deadline):
    data=b""
    descriptor=child.stdout.fileno()
    os.set_blocking(descriptor,False)
    with selectors.DefaultSelector() as selector:
        selector.register(descriptor,selectors.EVENT_READ)
        while b"\n" not in data:
            if len(data)>=256:
                raise RuntimeError("text READY protocol exceeds bound")
            if not selector.select(_remaining(deadline)):
                raise RuntimeError("text READY deadline exceeded")
            part=os.read(descriptor,256-len(data))
            if not part:
                raise RuntimeError("text READY disconnected before complete line")
            data+=part
    line,tail=data.split(b"\n",1)
    fields=line.decode("ascii").split()
    if tail or len(fields)!=3 or fields[0]!="READY" or int(fields[1])!=child.pid or fields[2]!=GPU:
        raise RuntimeError("text READY PID/UUID protocol invalid")


def _acknowledge(child, deadline):
    descriptor=child.stdin.fileno()
    os.set_blocking(descriptor,False)
    offset=0
    with selectors.DefaultSelector() as selector:
        selector.register(descriptor,selectors.EVENT_WRITE)
        while offset<4:
            if not selector.select(_remaining(deadline)):
                raise RuntimeError("text ACK deadline exceeded")
            try:
                offset+=os.write(descriptor,b"ACK\n"[offset:])
            except BlockingIOError:
                continue
    child.stdin.close()


class CoreTextTests(unittest.TestCase):
    adapter="engine"
    mode="engine_text"
    samples=128
    extra_config={}

    def run_fixture(self):
        if os.environ.get("CUDA_VISIBLE_DEVICES")!=GPU or os.environ.get("FATHOMDB_EMBED_DEVICE")!="cuda:0":
            raise RuntimeError("normal Default open requires the fixed forced CUDA identity")
        task=Path(os.environ["S30_GD_TASK"])
        source=task/"proofs/g-mean/1794488732288403793/out/store.sqlite"
        wal=Path(str(source)+"-wal")
        if source.stat().st_size!=696320 or hashlib.sha256(source.read_bytes()).hexdigest()!=SOURCE_HASH:
            raise RuntimeError("closed source fixture hash/size mismatch")
        if wal.exists() and wal.stat().st_size:
            raise RuntimeError("source fixture WAL is not empty")
        root=task/"proofs/g-text"/str(time.time_ns())
        out=root/"out"
        out.mkdir(parents=True)
        store=out/"store.sqlite"
        shutil.copyfile(source,store)
        if hashlib.sha256(store.read_bytes()).hexdigest()!=SOURCE_HASH:
            raise RuntimeError("initial copied fixture hash mismatch")
        for row in json.loads((task/"receipts/gd-pinned-inputs.json").read_text())["copied"]:
            if hashlib.sha256(Path(row["destination"]).read_bytes()).hexdigest()!=row["sha256"]:
                raise RuntimeError("pinned input invalid before loader")
        binary=Path(os.environ["CARGO_TARGET_DIR"])/"release/examples/slice30_g_mean"
        identity={"binary":str(binary),"binary_sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),
            "oracle_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "runner_sha256":hashlib.sha256(Path(sys.argv[0]).read_bytes()).hexdigest(),
            "source_fixture":str(source),"source_fixture_sha256":SOURCE_HASH,"root":str(root),
            "source":subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),
            "delta":subprocess.check_output(["git","status","--porcelain"],text=True),
            "features":["embed-cuda"],"observer":False,"test_hooks":False}
        (root/"identity.json").write_text(json.dumps(identity,indent=2)+"\n")
        print(json.dumps(identity),flush=True)
        config=root/"cell.json"
        config.write_text(json.dumps({"mode":self.mode,"samples":self.samples,"query":"alphaonly",
            "gpu_uuid":GPU,"text_fixture_handshake":True,**self.extra_config}))
        (task/"locks").mkdir(exist_ok=True)
        snapshots=[]
        ack=False
        with (task/"locks"/(GPU+".lock")).open("a") as lock:
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
            before=_accounting(None,time.monotonic()+15)
            if before["util"]!=0 or before["total_MiB"]-before["used_MiB"]<8192:
                raise RuntimeError("reserved GPU is not idle/free before Default open")
            with (root/"child.log").open("wb") as log:
                child=subprocess.Popen([str(binary),str(config),str(out)],stdin=subprocess.PIPE,
                    stdout=subprocess.PIPE,stderr=log,bufsize=0)
                assert child.stdin is not None and child.stdout is not None
                started=time.monotonic()
                try:
                    _ready_line(child,started+15)
                    deadline=time.monotonic()+15
                    snapshots.append(_accounting(child.pid,deadline,require_pid=True))
                    _acknowledge(child,deadline)
                    ack=True
                    next_sample=time.monotonic()+1
                    while child.poll() is None:
                        _remaining(started+180)
                        if time.monotonic()>=next_sample:
                            snapshots.append(_accounting(child.pid,started+180))
                            next_sample=time.monotonic()+1
                        try:
                            child.wait(timeout=min(0.1,_remaining(started+180)))
                        except subprocess.TimeoutExpired:
                            pass
                    snapshots.append(_accounting(child.pid,time.monotonic()+15))
                finally:
                    if child.stdin and not child.stdin.closed:
                        child.stdin.close()
                    if child.poll() is None:
                        child.terminate()
                        try:
                            child.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            child.kill()
                            child.wait(timeout=5)
                    else:
                        child.wait(timeout=5)
                    child.stdout.close()
                    (root/"gpu-accounting.json").write_text(json.dumps({"before":before,
                        "snapshots":snapshots,"ack_sent":ack,"pid":child.pid,"exit":child.returncode,
                        "positive_query_utilization_required":False,"query_gpu_work_claimed":False},indent=2)+"\n")
            if child.returncode:
                raise RuntimeError("text fixture API/protocol failure: "+(root/"child.log").read_text())
        proof=json.loads((out/"api-proof.json").read_text())
        if proof["api_calls"]!=self.samples or proof["fixture_ids"]!=["l:alpha"] or proof["fixture_ack_ns"]<=0:
            raise RuntimeError("existing text calls/literal identity/ACK proof invalid")
        if (proof["fixture"]["rows"],proof["fixture"]["vectors"],proof["fixture"]["pending"],proof["fixture"]["raw_fts_matches"])!=(2,2,0,1):
            raise RuntimeError("independent copied fixture counts invalid")
        if proof["selected_cuda_uuid"]!=GPU or proof["cuda_ordinal"]!=0 or proof["cuda_compiled"] is not True:
            raise RuntimeError("normal Default CUDA OpenReport invalid")
        if hashlib.sha256(source.read_bytes()).hexdigest()!=SOURCE_HASH:
            raise RuntimeError("source fixture changed")
        issues=[]
        rows=inference.read_jsonl_records(out/"samples.jsonl",issues)
        print(json.dumps({"api_proof":proof,"observed_records":len(rows),"issues":issues}),flush=True)
        return root,proof,rows,issues

    def test_literal_text_calls_emit_their_population(self):
        root,proof,rows,issues=self.run_fixture()
        out=root/"out"
        self.assertEqual(len(rows),128,f"missing actual {self.adapter} direct-text population")
        self.assertEqual(issues,[])
        for index,row in enumerate(rows):
            self.assertEqual((row["adapter"],row["mode"],row["ok"]),(self.adapter,"direct_text",True))
            self.assertEqual((row["sample_index"],row["query_index"]),(index,0))
            self.assertEqual((row["ids"],row["returned_hits"]),(["l:alpha"],1))
            self.assertGreater(row["elapsed_ns"],0)
        persistence=proof["raw_persistence"]
        self.assertEqual((persistence["records"],persistence["checkpoints"],persistence["checkpoint_records"]),(128,8,16))
        self.assertEqual(persistence["encoded_bytes"],(out/"samples.jsonl").stat().st_size)
        self.assertGreater(persistence["checkpoint_ns"],0)


if __name__ == "__main__":
    unittest.main()
