"""One private cgroup boundary for an owned G phase and all nested sessions."""

import json
import os
from pathlib import Path
import time
import uuid


def require_known_containment(task):
    """Refuse retained unknown/stale state; a live owning boundary permits its own runner only."""
    state_path = Path(task) / "g-containment-state.json"
    try:
        value = state_path.read_text()
    except FileNotFoundError:
        if not state_path.is_symlink():
            return
        raise RuntimeError("G containment marker is unavailable; redispatch refused") from None
    try:
        state = json.loads(value)
        if state["schema"] != "slice30-g-containment/v1" or state["status"] != "active":
            raise RuntimeError("retained containment is not active")
        fields = Path(f"/proc/{state['guardian_pid']}/stat").read_text().rsplit(") ", 1)[1].split()
        if fields[0] in ("Z", "X") or int(fields[19]) != state["guardian_start"]:
            raise RuntimeError("containment guardian identity is stale")
        relative = next(line.split(":", 2)[2] for line in Path("/proc/self/cgroup").read_text().splitlines()
                        if line.startswith("0:"))
        expected = state["relative_path"]
        if relative != expected and not relative.startswith(expected + "/"):
            raise RuntimeError("caller is outside the active private containment")
        identity = Path(state["path"]).stat()
        if (identity.st_dev, identity.st_ino) != (state["identity"]["device"], state["identity"]["inode"]):
            raise RuntimeError("private containment identity changed")
    except (OSError, ValueError, KeyError, TypeError, RuntimeError, StopIteration) as error:
        raise RuntimeError("G containment is unknown or unavailable; redispatch refused") from error


class TaskBoundary:
    """Hold exact private controls; unsupported or unknown containment fails closed."""

    def __init__(self, task):
        self.state_path = Path(task) / "g-containment-state.json"
        self.name = "fathomdb-s30-g-" + uuid.uuid4().hex
        self.parent_fd = self.directory_fd = self.procs_fd = self.kill_fd = self.events_fd = None
        self.created = False
        self.owns_state = False
        fields = Path('/proc/self/stat').read_text().rsplit(') ', 1)[1].split()
        self.evidence = dict(schema="slice30-g-containment/v1", status="preparing",
                             guardian_pid=os.getpid(), guardian_start=int(fields[19]), name=self.name)

    def publish(self):
        with self.state_path.open("w" if self.owns_state else "x") as stream:
            self.owns_state = True
            stream.write(json.dumps(self.evidence, indent=2) + "\n")

    def create(self):
        relative = next(line.split(":", 2)[2] for line in Path("/proc/self/cgroup").read_text().splitlines()
                        if line.startswith("0:"))
        parent = Path("/sys/fs/cgroup") / relative.lstrip("/")
        self.evidence["path"] = str(parent / self.name)
        self.evidence["relative_path"] = relative + "/" + self.name
        self.publish()
        self.parent_fd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        os.mkdir(self.name, mode=0o700, dir_fd=self.parent_fd)
        self.created = True
        self.directory_fd = os.open(self.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                                    dir_fd=self.parent_fd)
        identity = os.fstat(self.directory_fd)
        self.evidence["identity"] = dict(device=identity.st_dev, inode=identity.st_ino)
        self.procs_fd = os.open("cgroup.procs", os.O_WRONLY | os.O_CLOEXEC, dir_fd=self.directory_fd)
        self.kill_fd = os.open("cgroup.kill", os.O_WRONLY | os.O_CLOEXEC, dir_fd=self.directory_fd)
        self.events_fd = os.open("cgroup.events", os.O_RDONLY | os.O_CLOEXEC, dir_fd=self.directory_fd)
        if self.populated():
            raise RuntimeError("new private boundary is unexpectedly populated")
        if len(list(Path("/proc/self/task").iterdir())) != 1:
            raise RuntimeError("preexec attachment requires a single-threaded guardian")
        self.evidence["status"] = "active"
        self.publish()

    def attach(self):
        # Only preopened-descriptor write/close may execute between fork and exec.
        os.write(self.procs_fd, b"0")
        os.close(self.procs_fd)

    def populated(self):
        if self.events_fd is None:
            raise RuntimeError("private boundary population is unavailable")
        os.lseek(self.events_fd, 0, os.SEEK_SET)
        values = dict(line.split() for line in os.read(self.events_fd, 4096).decode().splitlines())
        if values.get("populated") not in ("0", "1"):
            raise RuntimeError("private boundary population is malformed")
        return values["populated"] == "1"

    def cleanup(self):
        try:
            if self.created:
                if self.kill_fd is None:
                    raise RuntimeError("private boundary kill control is unavailable")
                os.write(self.kill_fd, b"1")
                until = time.monotonic() + 2
                while self.populated():
                    if time.monotonic() >= until:
                        raise TimeoutError("private boundary remains populated after forced cleanup")
                    time.sleep(0.005)
                current = os.stat(self.name, dir_fd=self.parent_fd, follow_symlinks=False)
                held = os.fstat(self.directory_fd)
                if (current.st_dev, current.st_ino) != (held.st_dev, held.st_ino):
                    raise RuntimeError("private boundary identity changed before removal")
                self.evidence["populated_after"] = False
                os.rmdir(self.name, dir_fd=self.parent_fd)
                self.evidence["removed_exact_empty_boundary"] = True
            for attribute in ("events_fd", "kill_fd", "procs_fd", "directory_fd", "parent_fd"):
                descriptor = getattr(self, attribute)
                if descriptor is not None:
                    os.close(descriptor)
                    setattr(self, attribute, None)
            self.evidence["status"] = "quiescent"
            if self.owns_state:
                self.state_path.unlink()
        except BaseException as error:
            self.evidence.update(status="unknown", error=f"{type(error).__name__}: {error}")
            if self.owns_state:
                self.publish()
            raise
