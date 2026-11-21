"""Resource samples preserve checked ownership when a known child exits."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys

import pytest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "slice70_orin_tc5", ROOT / "dev/experiments/slice70-qualification/run_orin_tc5.py"
)
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


def test_checked_child_snapshot_remains_conservative_after_child_is_reaped():
    script = """import subprocess,sys
child=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)'])
print(child.pid,flush=True)
sys.stdin.readline()
child.terminate();child.wait()
print('reaped',flush=True)
sys.stdin.readline()
"""
    process = subprocess.Popen(
        [sys.executable, "-c", script], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        text=True, start_new_session=True,
    )
    try:
        child = int(process.stdout.readline())
        birth = runner.guard.proc(process.pid)["start_ticks"]
        checked = runner.guard.proc(child)
        runner.guard.last_details = {str(child): checked}
        runner.guard.last_fuser_rss = 0
        process.stdin.write("reap\n")
        process.stdin.flush()
        assert process.stdout.readline().strip() == "reaped"
        assert not Path(f"/proc/{child}").exists()
        total, identities = runner.owned_sample_rss(process.pid, birth, [child])
        assert identities[str(child)] == checked
        assert total >= checked["rss"]
    finally:
        runner.GROUP.stop_owned_group(process)


@pytest.mark.parametrize("record", [{}, {"error": "gone"}, {"PGID": -1, "SID": -1}])
def test_unknown_or_foreign_snapshot_is_never_admitted(record):
    process = subprocess.Popen([sys.executable, "-c", "import time;time.sleep(60)"], start_new_session=True)
    try:
        birth = runner.guard.proc(process.pid)["start_ticks"]
        runner.guard.last_details = {"999999": json.loads(json.dumps(record))}
        with pytest.raises(RuntimeError):
            runner.owned_sample_rss(process.pid, birth, [999999])
    finally:
        runner.GROUP.stop_owned_group(process)
