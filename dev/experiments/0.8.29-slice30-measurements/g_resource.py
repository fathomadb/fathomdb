"""Exact accepted owned-ENOENT sampler from source100 a9b2bbc7a; G caps live in g_phase.py."""

import os
from pathlib import Path
import re
import shutil
import subprocess


class Unqualified(RuntimeError):
    """A required control fact is unavailable; no qualified result follows."""


def vanished_owned_files(error: str, source: Path, task: Path) -> bool:
    """Recognize only confirmed absent paths within the two owned allocations."""
    # No partial/error-bearing size result is usable. Only a fresh observation can pass.
    lines, roots = error.splitlines(), (source.resolve(), task.resolve())
    if not lines:
        return False
    for line in lines:
        match = re.fullmatch(r"(?:/[^:\n]*/)?du: cannot access '(.+)': No such file or directory", line)
        if match is None:
            return False
        path = Path(match[1])
        if not path.is_absolute() or not any(path.resolve().is_relative_to(root) for root in roots):
            return False
        try:
            path.lstat()
        except FileNotFoundError:
            continue
        except OSError:
            return False
        return False
    return True


def resource_sample(source: Path, task: Path, *, actors=None) -> dict:
    """Observe owned size/free/memory, retrying only confirmed vanished own files."""
    retries = []
    for attempt in range(3):
        observation = subprocess.Popen(["du", "-sb", str(source), str(task)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=dict(os.environ, LC_ALL="C"))
        if actors is not None:
            actors.processes.append(observation)
        try:
            sizes, error = observation.communicate(timeout=3)
        except subprocess.TimeoutExpired as error:
            observation.kill()
            observation.communicate(timeout=5)
            raise Unqualified("owned allocation observation deadline") from error
        if observation.returncode == 0 and not error:
            break
        if observation.returncode != 1 or attempt == 2 or not vanished_owned_files(error, source, task):
            raise Unqualified("owned allocation observation failed: " + error)
        retries.append({"attempt": attempt + 1, "diagnostic": error})
    values = [int(line.split()[0]) for line in sizes.splitlines()]
    if len(values) != 2:
        raise Unqualified("owned allocation measurement unavailable")
    memory = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
    return {"root_free": shutil.disk_usage("/").free, "task_free": shutil.disk_usage(task).free, "memory_available": int(memory["MemAvailable"].split()[0]) * 1024, "owned": sum(values), "measurement_retries": retries}
