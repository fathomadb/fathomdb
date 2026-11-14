"""Private helper controls; no production hook, observer or ordinary timing claim."""

import json
from pathlib import Path
import socket
import time

from m_record import CheckpointLog


class PrivateControl:
    """Own one bounded Unix connection to the existing feature-gated Seams helper."""

    def __init__(self, path, root, *, timeout=5):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.settimeout(timeout)
        self.timeout = timeout
        self.next_id = 1
        self.events = []
        self.failures = []
        self.log = CheckpointLog(Path(root) / "control.jsonl")
        until = time.monotonic() + timeout
        while True:
            try:
                self.socket.connect(str(path))
                break
            except (FileNotFoundError, ConnectionRefusedError):
                if time.monotonic() >= until:
                    self.socket.close()
                    self.log.close()
                    raise TimeoutError("private helper control did not become available") from None
                time.sleep(0.01)

    def send(self, command, store="a"):
        """Retain the actual command/ack pair and reject mismatched or unbounded replies."""
        request = {"id": self.next_id, "command": command, "store": store}
        self.next_id += 1
        began = time.monotonic_ns()
        self.socket.sendall((json.dumps(request) + "\n").encode())
        response = bytearray()
        until = time.monotonic() + self.timeout
        while not response.endswith(b"\n"):
            if len(response) >= 4096 or time.monotonic() >= until:
                raise TimeoutError("private helper control reply incomplete")
            byte = self.socket.recv(1)
            if not byte:
                raise EOFError("private helper control closed")
            response.extend(byte)
        arrived = time.monotonic_ns()
        value = json.loads(response.decode("utf-8"))
        if value.get("id") != request["id"] or value.get("ok") is not True:
            raise ValueError("private helper control ack differs")
        event = {"request": request, "response": value, "response_raw": response.decode(),
                 "send_start_ns": began, "arrival_ns": arrived}
        self.events.append(event)
        self.log.write(event)
        return value["facts"]

    def wait_held(self, expected, store="a"):
        """Observe existing held_calls; sleeping alone never acknowledges a stage."""
        until = time.monotonic() + self.timeout
        while time.monotonic() < until:
            facts = self.send("held_calls", store)
            if facts["held_calls"] == expected:
                return facts
            time.sleep(0.01)
        raise TimeoutError(f"actual held_calls did not reach{expected}")

    def close(self):
        """Release all native holds and close the control connection on every exit."""
        if self.socket.fileno() >= 0:
            try:
                self.send("release_all")
            except (OSError, ValueError, TimeoutError) as error:
                self.failures.append(str(error))
            finally:
                self.socket.close()
                self.log.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


def run_case(case, session, control):
    """Run a tiny actual Adapter call; diagnostic orchestration is case-specific."""
    return session.tool("search", {"store": "a", "query": "alpha", "mode": "text", "limit": 10})
