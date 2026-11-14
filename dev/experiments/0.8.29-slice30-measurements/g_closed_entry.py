"""Closed-profile public operations under the named common reserve and guardian."""

import fcntl
import json
from pathlib import Path
import sys
import uuid

from g_closed_common import run_common


def worker_command(job):
    """Return the fixed committed common worker command, with no shell evaluation."""
    return [sys.executable, str(Path(__file__).with_name("g_runner.py")), "_common", str(job)]


def freeze_entry(options):
    """Execute blocking freeze in a supervised child; caller supplies known guardian."""
    from g_boundary import require_known_containment
    import os
    require_known_containment(os.environ["S30_TASK"])
    allocation = Path(options["allocation"])
    mode = options["stage"]
    jobs = allocation / "common" / "jobs"
    jobs.mkdir(parents=True, exist_ok=True)
    job = jobs / f"freeze-{uuid.uuid4().hex}.json"
    job.write_text(json.dumps({"action": "freeze", "options": options}, default=str) + "\n")
    with (allocation / "g-campaign.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        outcome = run_common(allocation, mode, worker_command(job))
    if outcome["status"] != "complete":
        raise RuntimeError(f"supervised freeze incomplete: {outcome}")
    return json.loads(Path(outcome["attempt"], "result.json").read_text())["value"]


def common_worker(job, output):
    """Run only a named committed operation; result is retained in the owned attempt."""
    request = json.loads(Path(job).read_text())
    if request["action"] != "freeze":
        raise ValueError("unsupported closed common operation")
    from g_freeze import freeze
    value = freeze(request["options"])
    Path(output, "result.json").write_text(json.dumps({"complete": True, "value": value}) + "\n")
