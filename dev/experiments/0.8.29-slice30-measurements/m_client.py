"""Bounded child JSON-RPC transport; no database or GPU qualification."""

import json
from pathlib import Path
import queue
import subprocess
import threading
import time


class RpcChild:
    """Own one stdio child and retain submission facts until bounded completion."""

    def __init__(self, command, root, *, attempt="source-control", timeout=5,
                 max_line_bytes=1048576, max_stream_bytes=67108864):
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=True)
        self.attempt = attempt
        self.timeout = timeout
        self.started_ns = time.monotonic_ns()
        self.child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                      stderr=subprocess.PIPE, cwd=self.root)
        self.records = {}
        self._completion_order = []
        self.notifications = []
        self.failures = []
        self._next_id = 1
        self._events = queue.Queue(maxsize=4096)
        self._stop = threading.Event()
        self._threads = [threading.Thread(target=self._read, args=(name, pipe,
                         max_line_bytes, max_stream_bytes), daemon=True)
                         for name, pipe in (("stdout", self.child.stdout),
                                            ("stderr", self.child.stderr))]
        for thread in self._threads:
            thread.start()

    def _read(self, name, pipe, max_line, max_stream):
        total = 0
        try:
            with (self.root / f"{name}.raw").open("wb") as saved:
                while not self._stop.is_set():
                    line = pipe.readline(max_line + 1)
                    arrived = time.monotonic_ns()
                    if not line:
                        break
                    total += len(line)
                    saved.write(line)
                    saved.flush()
                    if len(line) > max_line or total > max_stream or not line.endswith(b"\n"):
                        raise ValueError(f"{name}: bounded stream incomplete")
                    if name == "stdout":
                        self._events.put((arrived, line), timeout=self.timeout)
        except (OSError, ValueError, queue.Full) as error:
            self.failures.append(str(error))
        finally:
            pipe.close()

    def submit(self, method, params, **facts):
        """Serialize before send-start; capture the actual write/flush boundary."""
        if len(self.records) >= 10000:
            raise ValueError("request retention bound exceeded")
        request_id = self._next_id
        self._next_id += 1
        request = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
        raw = (json.dumps(request, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
        sent = time.monotonic_ns()
        row = dict(facts, attempt=self.attempt, pid=self.child.pid, rpc_id=request_id,
                   request=request, request_raw=raw.decode(), request_bytes=len(raw),
                   send_start_ns=sent, status="submitted", cancellation_requested=False)
        self.records[request_id] = row
        self.child.stdin.write(raw)
        self.child.stdin.flush()
        row["flush_end_ns"] = time.monotonic_ns()
        return request_id

    def wait(self, request_id, *, timeout=None):
        """Wait for one response under a bounded deadline; absent replies stay explicit."""
        until = time.monotonic() + (self.timeout if timeout is None else timeout)
        while True:
            if "response" in self.records[request_id]:
                return self.records[request_id]["response"]
            remaining = until - time.monotonic()
            if remaining <= 0:
                self.records[request_id]["status"] = "unresolved"
                return None
            if not self._consume(remaining):
                self.records[request_id]["status"] = "unresolved"
                return None

    def _consume(self, timeout):
        try:
            arrived, raw = self._events.get(timeout=timeout)
        except queue.Empty:
            return False
        parse_start = time.monotonic_ns()
        message = json.loads(raw)
        if "id" not in message:
            self.notifications.append(message)
            return True
        row = self.records[message["id"]]
        row.update(response=message, response_raw=raw.decode(), response_bytes=len(raw),
                   arrival_ns=arrived, elapsed_ns=arrived - row["send_start_ns"],
                   parse_ns=time.monotonic_ns() - parse_start, status="completed")
        self._completion_order.append(message["id"])
        return True

    def wait_any(self, pending, *, timeout=None):
        """Return the earliest terminal request from the caller's outstanding set."""
        until = time.monotonic() + (self.timeout if timeout is None else timeout)
        while True:
            for request_id in self._completion_order:
                if request_id in pending:
                    return request_id
            remaining = until - time.monotonic()
            if remaining <= 0 or not self._consume(remaining):
                for request_id in pending:
                    self.records[request_id]["status"] = "unresolved"
                return None

    def close(self):
        """Close stdin, wait normally, and stop only this child on a bounded failure."""
        if self.child.stdin and not self.child.stdin.closed:
            self.child.stdin.close()
        try:
            code = self.child.wait(timeout=self.timeout)
        except subprocess.TimeoutExpired:
            self.child.terminate()
            try:
                code = self.child.wait(timeout=self.timeout)
            except subprocess.TimeoutExpired:
                self.child.kill()
                code = self.child.wait(timeout=self.timeout)
            self.failures.append("child required bounded termination")
        for thread in self._threads:
            thread.join(timeout=self.timeout)
        if any(thread.is_alive() for thread in self._threads):
            self.failures.append("stream reader did not reach EOF")
        self._stop.set()
        self.exit = {"code": code, "exit_ns": time.monotonic_ns(),
                     "readers_joined": all(not thread.is_alive() for thread in self._threads)}
        return self.exit

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
