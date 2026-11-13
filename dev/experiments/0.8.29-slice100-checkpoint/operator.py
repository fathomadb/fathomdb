"""Opt-in Slice100 owned-child checkpoint control; never launched by import or gates."""

from __future__ import annotations

import sys

# Direct script launch must not shadow Python's stdlib operator with this file.
if __name__ == "__main__" and sys.path and sys.path[0].replace("\\", "/") == __file__.replace("\\", "/").rsplit("/", 1)[0]:
    sys.path.pop(0)

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import shutil
import signal
import subprocess
import time

SQLITE_SHA256 = "0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52"
COPY_LINE = 69275
FIXTURE = "q1-v1-target1-survivor3-none-explicit"
GIB = 1024**3


class Unqualified(RuntimeError):
    """A required control fact is unavailable; no qualified result follows."""


class MIValue:
    """Parse MI values without evaluating expressions or discarding repeated list keys."""

    def __init__(self, text: str):
        self.text, self.pos = text, 0

    def quoted(self):
        self.pos += 1
        chars = []
        escapes = dict(zip("abfnrtv\\\"'", "\a\b\f\n\r\t\v\\\"'", strict=True))
        while self.pos < len(self.text):
            c = self.text[self.pos]
            self.pos += 1
            if c == '"':
                return "".join(chars)
            if c != "\\":
                chars.append(c)
                continue
            if self.pos == len(self.text):
                break
            c = self.text[self.pos]
            self.pos += 1
            if c in escapes:
                chars.append(escapes[c])
            elif c in "01234567":
                digits = c
                while self.pos < len(self.text) and len(digits) < 3 and self.text[self.pos] in "01234567":
                    digits += self.text[self.pos]
                    self.pos += 1
                chars.append(chr(int(digits, 8)))
            else:
                raise Unqualified("unsupported MI escape")
        raise Unqualified("unterminated MI string")

    def pair(self):
        m = re.match(r"[A-Za-z_][A-Za-z_0-9-]*=", self.text[self.pos :])
        if not m:
            raise Unqualified("malformed MI result")
        name = m.group()[:-1]
        self.pos += len(m.group())
        return name, self.value()

    def value(self):
        if self.pos >= len(self.text):
            raise Unqualified("missing MI value")
        c = self.text[self.pos]
        if c == '"':
            return self.quoted()
        if c not in "[{":
            raise Unqualified("non-string MI constant")
        self.pos += 1
        end = "]" if c == "[" else "}"
        result = [] if c == "[" else {}
        while self.pos < len(self.text) and self.text[self.pos] != end:
            if re.match(r"[A-Za-z_][A-Za-z_0-9-]*=", self.text[self.pos :]):
                key, value = self.pair()
                if isinstance(result, dict):
                    if key in result:
                        raise Unqualified("duplicate MI tuple key")
                    result[key] = value
                else:
                    result.append({key: value})
            else:
                if isinstance(result, dict):
                    raise Unqualified("unnamed MI tuple item")
                result.append(self.value())
            if self.pos < len(self.text) and self.text[self.pos] == ",":
                self.pos += 1
            elif self.pos < len(self.text) and self.text[self.pos] != end:
                raise Unqualified("missing MI separator")
        if self.pos >= len(self.text):
            raise Unqualified("unterminated MI container")
        self.pos += 1
        return result


def parse_mi(line: str) -> dict:
    """Return a structured MI record; reject malformed/trailing data with Unqualified."""
    m = re.fullmatch(r"(\d*)([\^*+=~@&])(.*)", line.rstrip("\r\n"))
    if not m:
        raise Unqualified("non-MI debugger output: " + line[:160])
    token, kind, tail = m.groups()
    parser = MIValue(tail)
    if kind in "~@&":
        value = parser.value()
        result = {"token": token, "kind": kind, "stream": value}
    else:
        cls, sep, rest = tail.partition(",")
        if not re.fullmatch(r"[A-Za-z][A-Za-z0-9-]*", cls):
            raise Unqualified("malformed MI class")
        parser = MIValue(rest)
        values = {}
        while sep and parser.pos < len(rest):
            key, value = parser.pair()
            if key in values:
                raise Unqualified("duplicate MI result key")
            values[key] = value
            if parser.pos < len(rest):
                if rest[parser.pos] != ",":
                    raise Unqualified("malformed MI result separator")
                parser.pos += 1
        result = {"token": token, "kind": kind, "class": cls, "values": values}
    if parser.pos != len(parser.text):
        raise Unqualified("trailing MI data")
    return result


def number(value: str) -> int:
    """Read a decimal/hex debugger scalar, refusing unavailable or decorated values."""
    if not isinstance(value, str) or not re.fullmatch(r"-?(?:0x[0-9a-fA-F]+|\d+)", value):
        raise Unqualified("unreadable native scalar: " + repr(value))
    return int(value, 16 if "0x" in value else 10)


def validate_witness(w: dict, subject: dict, pid: int, nonce: str) -> None:
    """Validate same-execution native facts; synthetic inputs only test this refusal oracle."""
    if w.get("subject") != subject or w.get("pid") != pid or w.get("nonce") != nonce:
        raise Unqualified("witness subject/actor/nonce mismatch")
    if w.get("ready_count") != 1 or w.get("done_count") != 0:
        raise Unqualified("missing/duplicate READY or premature completion")
    frame = w.get("frame", {})
    if frame.get("func") != "walCheckpoint" or number(frame.get("line")) != COPY_LINE:
        raise Unqualified("not the eligible native copy source position")
    pc = number(frame.get("addr"))
    if pc <= 0 or pc not in w.get("relocated_line_addresses", []) or w.get("instruction_address") != pc:
        raise Unqualified("unresolved/sliding relocated instruction")
    if frame.get("fullname") != subject.get("sqlite_source"):
        raise Unqualified("wrong native source file")
    names = [f.get("func", "") for f in w.get("stack", [])]
    for required in ["sqlite3WalCheckpoint", "wal_checkpoint_truncate_once", "checkpoint_erasure_at_rest", "complete_physical_erasure"]:
        if not any(required in name for name in names):
            raise Unqualified("wrong checkpoint callstack: missing " + required)
    values = w.get("locals", {})
    try:
        v = {k: number(values[k]) for k in ["rc", "pIter", "nBackfill", "mxSafeFrame", "iFrame", "iDbpage", "mxPage", "szPage", "eMode"]}
    except KeyError as error:
        raise Unqualified("missing native local: " + str(error)) from error
    if not (v["rc"] == 0 and v["pIter"] != 0 and v["nBackfill"] < v["mxSafeFrame"] and v["nBackfill"] < v["iFrame"] <= v["mxSafeFrame"] and 0 < v["iDbpage"] <= v["mxPage"] and v["szPage"] > 0 and v["eMode"] == 3):
        raise Unqualified("empty/ineligible/failed/TRUNCATE-mismatched frame")


def healthy_matches(result: dict, subject: dict) -> None:
    """Require a completed matched healthy control, not merely a prior breakpoint stop."""
    if result.get("status") != "QUALIFIED_HEALTHY" or result.get("subject") != subject or result.get("exit") != 0:
        raise Unqualified("healthy control does not match exact subject")
    if not all(key in result for key in ["witness", "pid", "nonce"]):
        raise Unqualified("healthy witness identity fields missing")
    validate_witness(result["witness"], subject, result["pid"], result["nonce"])
    if result.get("done_count") != 1:
        raise Unqualified("healthy normal completion missing")


def digest(path: Path) -> str:
    """Hash an existing file in bounded chunks; propagate I/O failure."""
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def source_subject(args) -> dict:
    """Bind an owned Cargo artifact, clean actual source/lock and pinned SQLite source."""
    source, task, binary = args.source_root.resolve(), args.task_root.resolve(), args.binary.resolve()
    if task.is_relative_to(source) or source.is_relative_to(task):
        raise Unqualified("source/task allocations must be disjoint")
    if not binary.is_relative_to(task / "target") or not args.artifact_json.resolve().is_relative_to(task):
        raise Unqualified("binary/build JSON is not in the own task allocation")
    sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
    delta = subprocess.check_output(["git", "status", "--porcelain"], cwd=source, text=True)
    if sha != args.source_sha or delta:
        raise Unqualified("source is not the clean frozen requested commit")
    artifacts = []
    for line in args.artifact_json.read_text().splitlines():
        record = json.loads(line)
        if record.get("reason") == "compiler-artifact" and record.get("target", {}).get("name") == "slice100_interruption" and record.get("profile", {}).get("test"):
            if record.get("executable") and Path(record["executable"]).resolve() == binary:
                artifacts.append(record)
    if not artifacts:
        raise Unqualified("no exact test executable in own Cargo build JSON")
    features = sorted(artifacts[-1]["features"])
    if not {"operator", "test-hooks"}.issubset(features) or any("cuda" in f or "metal" in f for f in features):
        raise Unqualified("wrong selected feature closure")
    sqlite = args.sqlite_source.resolve()
    if not sqlite.is_relative_to(task) or digest(sqlite) != SQLITE_SHA256:
        raise Unqualified("not the pinned own SQLite source")
    lines = sqlite.read_text().splitlines()
    if "sqlite3OsWrite(pWal->pDbFd" not in lines[COPY_LINE - 1]:
        raise Unqualified("pinned source copy line changed")
    notes = subprocess.check_output(["readelf", "--notes", "--wide", str(binary)], text=True)
    build_ids = re.findall(r"Build ID: ([0-9a-f]+)", notes)
    if len(build_ids) != 1:
        raise Unqualified("unique ELF build ID unavailable")
    return {"gdb": str(args.gdb.resolve()), "gdb_sha256": digest(args.gdb), "build_id": build_ids[0], "source_sha": sha, "lock_sha256": digest(source / "Cargo.lock"), "binary": str(binary), "binary_sha256": digest(binary), "artifact_json_sha256": digest(args.artifact_json), "features": features, "sqlite_source": str(sqlite), "sqlite_sha256": SQLITE_SHA256, "fixture": FIXTURE}


def resource_sample(source: Path, task: Path, *, actors=None) -> dict:
    """Use the existing Slice60 monitor's owned-du/free/MemAvailable observations."""
    observation = subprocess.Popen(["du", "-sb", str(source), str(task)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if actors is not None:
        actors.processes.append(observation)
    try:
        sizes, error = observation.communicate(timeout=3)
    except subprocess.TimeoutExpired as error:
        observation.kill()
        observation.communicate(timeout=5)
        raise Unqualified("owned allocation observation deadline") from error
    if observation.returncode:
        raise Unqualified("owned allocation observation failed: " + error)
    values = [int(line.split()[0]) for line in sizes.splitlines()]
    if len(values) != 2:
        raise Unqualified("owned allocation measurement unavailable")
    memory = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
    return {"root_free": shutil.disk_usage("/").free, "task_free": shutil.disk_usage(task).free, "memory_available": int(memory["MemAvailable"].split()[0]) * 1024, "owned": sum(values)}


def check_resources(sample: dict, start: bool = False) -> None:
    """Enforce only the reviewed start/stop envelope, raising Unqualified on refusal."""
    floors = (32, 128, 16) if start else (20, 64, 8)
    if any(sample[k] < v * GIB for k, v in zip(["root_free", "task_free", "memory_available"], floors, strict=True)) or sample["owned"] >= 64 * GIB:
        raise Unqualified("resource floor/cap: " + json.dumps(sample))


class Actors:
    """Track only directly owned GDB/utility children and its one pidfd-bound inferior."""

    def __init__(self):
        self.processes = []
        self.inferior = None
        self.pidfd = None
        self.starttime = None
        self.adoption_scope = False
        self.adopted_cleanup = []

    def direct_children(self):
        children = set()
        for thread in Path("/proc/self/task").iterdir():
            try:
                children.update(int(pid) for pid in (thread / "children").read_text().split())
            except FileNotFoundError:
                continue
        return children

    def begin_launch_scope(self):
        # Standalone, single-threaded operator: after this empty baseline EVERY
        # launch is registered, including synchronous resource observations.
        # A later direct adopted child can therefore only descend from these roots.
        if self.processes or self.direct_children() or len(list(Path("/proc/self/task").iterdir())) != 1:
            raise Unqualified("exclusive owned-launch baseline unavailable")
        self.adoption_scope = True

    def bind_inferior(self, pid: int, gdb_pid: int, binary: Path):
        if self.inferior is not None:
            raise Unqualified("multiple inferior identities")
        proc = Path(f"/proc/{pid}")
        fields = (proc / "stat").read_text().rsplit(")", 1)[1].split()
        if int(fields[1]) != gdb_pid:
            raise Unqualified("inferior is not the direct owned debugger child")
        self.pidfd = os.pidfd_open(pid)
        self.starttime, self.inferior, self.binary = fields[19], pid, binary
        if (proc / "exe").resolve() != binary:
            raise Unqualified("owned inferior is not the selected executable")

    def check_inferior(self):
        if self.pidfd is None or self.inferior is None:
            raise Unqualified("no registered inferior")
        fields = Path(f"/proc/{self.inferior}/stat").read_text().rsplit(")", 1)[1].split()
        if fields[19] != self.starttime or Path(f"/proc/{self.inferior}/exe").resolve() != self.binary:
            raise Unqualified("inferior identity changed")

    def kill_inferior(self):
        self.check_inferior()
        signal.pidfd_send_signal(self.pidfd, signal.SIGKILL)

    def cleanup(self):
        errors = []
        if self.pidfd is not None:
            try:
                signal.pidfd_send_signal(self.pidfd, signal.SIGKILL)
            except ProcessLookupError:
                pass
            except OSError as error:
                errors.append(str(error))
            os.close(self.pidfd)
            self.pidfd = None
        for process in reversed(self.processes):
            if process.poll() is None:
                process.kill()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                errors.append("owned process did not reap: " + str(process.pid))
        # Killing/reaping the registered launch roots transfers descendants to
        # this subreaper even when MI decoding or pidfd registration never finished.
        # This fallback is CLEANUP ONLY; qualified fault still requires its pidfd.
        if self.adoption_scope:
            until = time.monotonic() + 5
            while True:
                try:
                    children = self.direct_children()
                    if not children:
                        break
                    for pid in children:
                        try:
                            fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
                            if int(fields[1]) != os.getpid():
                                errors.append("refused non-adopted actor: " + str(pid))
                                continue
                            self.adopted_cleanup.append({"pid": pid, "starttime": fields[19], "ppid": os.getpid(), "registered_roots": [p.pid for p in self.processes]})
                            # A direct unreaped child cannot be PID-reused until WE reap it.
                            os.kill(pid, signal.SIGKILL)
                            os.waitpid(pid, os.WNOHANG)
                        except (FileNotFoundError, ChildProcessError, ProcessLookupError):
                            continue
                    if time.monotonic() >= until:
                        errors.append("owned adopted-descendant reap deadline: " + repr(sorted(self.direct_children())))
                        break
                    time.sleep(0.02)
                except OSError as error:
                    errors.append("adopted-descendant cleanup unavailable: " + str(error))
                    break
        elif self.inferior:
            # No adoption claim is made by callers outside the exclusive run_case context.
            try:
                os.waitpid(self.inferior, os.WNOHANG)
            except ChildProcessError:
                pass
        for process in self.processes:
            for stream in [process.stdin, process.stdout, process.stderr]:
                if stream is not None:
                    stream.close()
        return errors


class Debugger:
    """Tokenized, bounded MI/PTY transport; construction is runtime technical-GO only."""

    def __init__(self, args, case: Path, nonce: str, actors: Actors):
        import pty
        import tty

        self.args, self.case, self.nonce, self.actors = args, case, nonce, actors
        self.deadline = time.monotonic() + 120
        self.token = 0
        self.records, self.events, self.ready, self.done = [], [], [], []
        self.inferior_pid = None
        self.resources = []
        self.next_sample = 0.0
        self.logs = {"mi": (case / "gdb-mi.log").open("wb"), "target": (case / "target.log").open("wb")}
        self.buffers = {"mi": b"", "target": b""}
        self.counts = {"mi": 0, "target": 0}
        master, slave = pty.openpty()
        self.master, self.slave = master, slave
        tty.setraw(slave)
        env = {k: v for k, v in os.environ.items() if not k.startswith("S100_")}
        env.update(S100_CASE=str(case), S100_NONCE=nonce, TOKIO_WORKER_THREADS="2")
        self.process = subprocess.Popen([str(args.gdb), "--nx", "--nh", "--interpreter=mi2", "--quiet"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=env, cwd=args.source_root, start_new_session=True)
        actors.processes.append(self.process)
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ, "mi")
        self.selector.register(master, selectors.EVENT_READ, "target")
        self.tty_path = os.ttyname(slave)

    def pump(self, until):
        now = time.monotonic()
        if now >= min(until, self.deadline):
            raise Unqualified("actor/phase deadline")
        if now >= self.next_sample:
            sample = resource_sample(self.args.source_root, self.args.task_root, actors=self.actors)
            self.resources.append(sample)
            check_resources(sample)
            sample["case_bytes"] = sum(p.stat().st_size for p in self.case.rglob("*") if p.is_file())
            if sample["case_bytes"] >= GIB:
                raise Unqualified("case/evidence cap")
            self.next_sample = now + 5
        for key, _ in self.selector.select(min(0.1, until - now, self.deadline - now)):
            name = key.data
            try:
                chunk = os.read(key.fd, 65536)
            except OSError as error:
                raise Unqualified("transport read failure: " + str(error)) from error
            if not chunk:
                self.selector.unregister(key.fileobj)
                if name == "mi":
                    raise Unqualified("unexpected debugger EOF")
                continue
            self.counts[name] += len(chunk)
            if self.counts[name] > 64 * 1024**2:
                raise Unqualified("per-actor log cap")
            self.logs[name].write(chunk)
            self.logs[name].flush()
            self.buffers[name] += chunk
            while b"\n" in self.buffers[name]:
                raw, self.buffers[name] = self.buffers[name].split(b"\n", 1)
                line = raw.decode("utf-8", errors="strict").rstrip("\r")
                if name == "target":
                    for prefix, destination in [("Q1_READY ", self.ready), ("Q1_DONE ", self.done)]:
                        if line.startswith(prefix):
                            event = json.loads(line[len(prefix) :])
                            if event.get("nonce") != self.nonce or event.get("fixture") != FIXTURE:
                                raise Unqualified("target event identity mismatch")
                            destination.append(event)
                            if len(destination) != 1:
                                raise Unqualified("duplicate target event")
                elif line.strip() and line.strip() != "(gdb)":
                    record = parse_mi(line)
                    self.records.append(record)
                    if record.get("kind") in ["*", "="]:
                        self.events.append(record)
                        if record.get("class") == "thread-group-started":
                            pid = number(record["values"]["pid"])
                            if self.inferior_pid is not None:
                                raise Unqualified("unexpected extra inferior")
                            self.inferior_pid = pid
                            self.actors.bind_inferior(pid, self.process.pid, self.args.binary.resolve())

    def command(self, text, seconds=10):
        self.token += 1
        token = str(self.token)
        self.process.stdin.write((token + text + "\n").encode())
        self.process.stdin.flush()
        until = min(self.deadline, time.monotonic() + seconds)
        while True:
            for i, record in enumerate(self.records):
                if record.get("kind") == "^" and record.get("token") == token:
                    self.records.pop(i)
                    if record["class"] not in ["done", "running", "exit"]:
                        raise Unqualified("debugger command failed: " + json.dumps(record))
                    return record.get("values", {})
            self.pump(until)

    def stopped(self, seconds):
        until = min(self.deadline, time.monotonic() + seconds)
        while True:
            for i, record in enumerate(self.events):
                if record.get("kind") == "*" and record.get("class") == "stopped":
                    self.events.pop(i)
                    return record["values"]
            self.pump(until)

    def finish(self):
        for stream in self.logs.values():
            stream.close()
        self.selector.close()
        os.close(self.master)
        os.close(self.slave)


def snapshot_database(case: Path) -> Path:
    """Preserve quiescent raw files before any SQLite/Engine observation or retry."""
    snapshot = case / "untouched"
    snapshot.mkdir()
    records = []
    for suffix in ["", "-wal", "-shm"]:
        path = case / ("candidate.sqlite" + suffix)
        if path.exists():
            dest = snapshot / path.name
            shutil.copyfile(path, dest)
            records.append({"name": path.name, "bytes": dest.stat().st_size, "sha256": digest(dest)})
        else:
            records.append({"name": path.name, "absent": True})
    (snapshot / "files.json").write_text(json.dumps(records, indent=2) + "\n")
    return snapshot


def utility_child(args, case, snapshot, action, nonce, actors, deadline):
    """Run a separately registered observer/retry on its own raw-copy subject, bounded."""
    copy = case / action
    copy.mkdir()
    for path in snapshot.iterdir():
        if path.name.startswith("candidate.sqlite"):
            shutil.copyfile(path, copy / path.name)
    env = {k: v for k, v in os.environ.items() if not k.startswith("S100_")}
    env.update(S100_CASE=str(case), S100_NONCE=nonce, S100_DATABASE=str(copy / "candidate.sqlite"), TOKIO_WORKER_THREADS="2")
    with (case / (action + ".log")).open("wb") as log:
        child = subprocess.Popen([str(args.binary), "--exact", "q1_" + action, "--ignored", "--nocapture", "--test-threads=1"], cwd=args.source_root, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        actors.processes.append(child)
        until = min(deadline, time.monotonic() + 30)
        while child.poll() is None:
            if time.monotonic() >= until:
                raise Unqualified(action + " deadline")
            check_resources(resource_sample(args.source_root, args.task_root, actors=actors))
            try:
                child.wait(timeout=min(5, until - time.monotonic()))
            except subprocess.TimeoutExpired:
                pass
        if child.returncode != 0:
            raise Unqualified(action + " failed; preserve exact assertion log")


def run_case(args) -> dict:
    """Execute only an explicitly technical-GO selected case; always preserve abort and reap."""
    if not args.technical_go:
        raise Unqualified("parent technical GO flag required; scope approval alone is insufficient")
    subject = source_subject(args)
    check_resources(resource_sample(args.source_root, args.task_root), start=True)
    if args.mode == "interrupt":
        if args.healthy_result is None:
            raise Unqualified("matched healthy result required")
        healthy_matches(json.loads(args.healthy_result.read_text()), subject)
    case = args.case_root.resolve()
    if not case.is_relative_to(args.task_root.resolve()):
        raise Unqualified("case outside owned task")
    case.mkdir(exist_ok=False)
    nonce = os.urandom(16).hex()
    result = {"status": "UNQUALIFIED", "subject": subject, "nonce": nonce, "mode": args.mode}
    actors, debugger = Actors(), None
    try:
        # Lazy: parser/guard tests never load ctypes or request process controls.
        import ctypes

        if ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) != 0:
            raise Unqualified("owned subreaper unavailable")
        actors.begin_launch_scope()
        debugger = Debugger(args, case, nonce, actors)
        for command in ["-gdb-set auto-load off", "-gdb-set debuginfod enabled off", "-gdb-set confirm off", "-gdb-set pagination off", "-gdb-set style enabled off", "-gdb-set startup-with-shell off", "-gdb-set follow-fork-mode parent", "-gdb-set detach-on-fork on"]:
            debugger.command(command)
        debugger.command("-inferior-tty-set " + json.dumps(debugger.tty_path))
        debugger.command("-file-exec-and-symbols " + json.dumps(str(args.binary.resolve())))
        debugger.command('-exec-arguments --exact q1_child --ignored --nocapture --test-threads=1')
        debugger.command('-interpreter-exec console "starti"', seconds=30)
        debugger.stopped(30)
        if debugger.inferior_pid is None:
            raise Unqualified("inferior PID unavailable at entry")
        actors.check_inferior()
        result["pid"] = actors.inferior
        ready_break = debugger.command('-break-insert "slice100_interruption::q1_ready_after_queue"')["bkpt"]
        debugger.command("-exec-continue")
        stop = debugger.stopped(45)
        # Drain any already-written PTY data before interpreting the marker.
        end = time.monotonic() + 2
        while not debugger.ready and time.monotonic() < end:
            debugger.pump(end)
        if stop.get("bkptno") != ready_break["number"] or len(debugger.ready) != 1 or debugger.ready[0].get("pid") != actors.inferior or debugger.ready[0].get("executable") != str(args.binary.resolve()):
            raise Unqualified("READY/marker/actor mismatch")
        preimage = json.loads((case / "preimage.json").read_text())
        if preimage.get("nonce") != nonce or preimage.get("ticket") != debugger.ready[0].get("ticket"):
            raise Unqualified("READY ticket does not match preserved preimage")
        native_break = debugger.command("-break-insert " + json.dumps(str(args.sqlite_source.resolve()) + ":" + str(COPY_LINE)))["bkpt"]
        debugger.command("-break-delete " + ready_break["number"])
        debugger.command("-exec-continue")
        stop = debugger.stopped(30)
        if stop.get("bkptno") != native_break["number"]:
            raise Unqualified("unexpected stop before active witness")
        stack_records = debugger.command("-stack-list-frames")["stack"]
        stack = [row["frame"] for row in stack_records]
        debugger.command("-stack-select-frame 0")
        values = {name: debugger.command("-data-evaluate-expression " + json.dumps(name))["value"] for name in ["rc", "pIter", "nBackfill", "mxSafeFrame", "iFrame", "iDbpage", "mxPage", "szPage", "eMode"]}
        lines = debugger.command("-symbol-list-lines " + json.dumps(str(args.sqlite_source.resolve())))["lines"]
        addresses = [number(row["pc"]) for row in lines if number(row["line"]) == COPY_LINE]
        pc = number(stop["frame"]["addr"])
        instructions = debugger.command(f"-data-disassemble -s {pc} -e {pc + 16} -- 0")["asm_insns"]
        witness = {"subject": subject, "pid": actors.inferior, "nonce": nonce, "ready_count": len(debugger.ready), "done_count": len(debugger.done), "frame": stop["frame"], "stack": stack, "locals": values, "relocated_line_addresses": addresses, "instruction_address": number(instructions[0]["address"]), "instructions": instructions, "ready": debugger.ready[0]}
        validate_witness(witness, subject, actors.inferior, nonce)
        actors.check_inferior()
        (case / "witness.json").write_text(json.dumps(witness, indent=2) + "\n")
        result["witness"] = witness
        if args.mode == "healthy":
            debugger.command("-break-delete " + native_break["number"])
            debugger.command("-exec-continue")
            stop = debugger.stopped(30)
            if stop.get("reason") != "exited-normally":
                raise Unqualified("healthy did not exit normally: " + json.dumps(stop))
            end = time.monotonic() + 2
            while not debugger.done and time.monotonic() < end:
                debugger.pump(end)
            if len(debugger.done) != 1:
                raise Unqualified("normal healthy DONE missing")
            result.update(status="QUALIFIED_HEALTHY", exit=0, done_count=1)
        else:
            # No timeout/earlier-run witness reaches this branch.
            actors.kill_inferior()
            # A stopped MI target needs an execution transition to report its pending exit.
            exit_until = min(debugger.deadline, time.monotonic() + 5)
            debugger.command("-exec-continue", seconds=exit_until - time.monotonic())
            stop = debugger.stopped(exit_until - time.monotonic())
            if stop.get("reason") != "exited-signalled" or stop.get("signal-name") != "SIGKILL":
                raise Unqualified("exact owned SIGKILL not observed")
            result["fault_exit"] = stop
        debugger.command("-gdb-exit")
        debugger.process.wait(timeout=5)
        if args.mode == "interrupt":
            snapshot = snapshot_database(case)
            utility_child(args, case, snapshot, "observer", nonce, actors, debugger.deadline)
            utility_child(args, case, snapshot, "retry", nonce, actors, debugger.deadline)
            result.update(status="QUALIFIED_INTERRUPTION", done_count=0)
        result["resource_samples"] = debugger.resources
    except Exception as error:
        result.update(status="UNQUALIFIED", error=type(error).__name__ + ": " + str(error))
    finally:
        errors = actors.cleanup()
        result["adopted_cleanup"] = actors.adopted_cleanup
        if debugger is not None:
            debugger.finish()
        if errors:
            result.update(status="UNQUALIFIED", cleanup_errors=errors)
        (case / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def main() -> int:
    """Parse an explicit opt-in command; return nonzero for every unqualified outcome."""
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--mode", choices=["healthy", "interrupt"], required=True)
    for name in ["binary", "source-root", "task-root", "artifact-json", "sqlite-source", "case-root"]:
        p.add_argument("--" + name, type=Path, required=True)
    p.add_argument("--source-sha", required=True)
    p.add_argument("--gdb", type=Path, default=Path("/usr/bin/gdb"))
    p.add_argument("--healthy-result", type=Path)
    p.add_argument("--technical-go", action="store_true")
    args = p.parse_args()
    try:
        result = run_case(args)
    except (OSError, ValueError, KeyError, TypeError, Unqualified) as error:
        print(json.dumps({"status": "UNQUALIFIED", "error": str(error)}))
        return 2
    print(json.dumps({k: v for k, v in result.items() if k not in ["witness", "resource_samples"]}))
    return 0 if result["status"].startswith("QUALIFIED_") else 2


if __name__ == "__main__":
    raise SystemExit(main())
