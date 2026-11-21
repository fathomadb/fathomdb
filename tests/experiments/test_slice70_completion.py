"""The recovery parent must await its guard's worker cleanup on cancellation."""

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
MODULE = ROOT / "dev/experiments/slice70-qualification/complete_tc5_bridge.py"


def test_batch_cancellation_awaits_guard_and_native_worker_cleanup(tmp_path):
    ready, cleaned = tmp_path / "ready.json", tmp_path / "cleaned"
    guard = tmp_path / "guard.py"
    guard.write_text('''import json,os,signal,subprocess,sys,time
from pathlib import Path
child=subprocess.Popen([sys.executable,"-c","import time;time.sleep(60)"],start_new_session=True)
def stop(*_):
 child.terminate();child.wait();Path(sys.argv[2]).write_text("cleaned");sys.exit(0)
signal.signal(signal.SIGTERM,stop)
Path(sys.argv[1]).write_text(json.dumps([os.getpid(),child.pid]))
while True: time.sleep(.01)
''')
    parent_code = '''import importlib.util,signal,sys,time
spec=importlib.util.spec_from_file_location("completion",sys.argv[1]);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
if hasattr(m,"request_stop"): signal.signal(signal.SIGTERM,m.request_stop)
m.run_guard([sys.executable,*sys.argv[2:]],time.monotonic()+30)
'''
    parent = subprocess.Popen([sys.executable, "-c", parent_code, str(MODULE), str(guard), str(ready), str(cleaned)], start_new_session=True)
    identities = []
    try:
        until = time.monotonic() + 5
        while not ready.exists() and time.monotonic() < until:
            time.sleep(.01)
        identities = json.loads(ready.read_text())
        parent.send_signal(signal.SIGTERM)
        parent.wait(timeout=5)
        assert cleaned.exists(), "batch parent exited before forwarding cancellation and awaiting guard cleanup"
        assert all(not Path(f"/proc/{pid}").exists() for pid in identities)
    finally:
        if parent.poll() is None:
            parent.terminate()
            parent.wait(timeout=5)
        for pid in identities:
            if Path(f"/proc/{pid}").exists():
                os.kill(pid, signal.SIGTERM)


def test_native_query_refuses_a_manifest_that_points_outside_owned_storage(tmp_path, monkeypatch):
    import hashlib
    import pytest
    from test_slice70_owned_snapshot import runner

    owned = tmp_path / "owned"
    owned.mkdir()
    binary = owned / "executor"
    binary.write_bytes(b"native")
    manifest = owned / "query.manifest.json"
    foreign = tmp_path / "foreign.sqlite"
    foreign.write_bytes(b"user data")
    manifest.write_text(json.dumps({"database": str(foreign)}))
    spec = owned / "query.spec.json"
    spec.write_text(json.dumps({"manifest": str(manifest)}))
    monkeypatch.setattr(runner, "OWNED_ROOT", owned)
    config = {"candidate": {"benchmark_binary": str(binary), "benchmark_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}}
    with pytest.raises(ValueError):
        runner.native_query_command(config, spec, owned / "result.json", {})
