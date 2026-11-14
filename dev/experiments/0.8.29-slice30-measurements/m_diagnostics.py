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
            except (OSError, ValueError, TimeoutError, EOFError) as error:
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
    if case == "queue_full":
        return _queue_full(session, control)
    if case == "queued_cancel":
        return _queued_cancel(session, control)
    if case == "wait_deadline":
        return _wait_deadline(session, control)
    if case == "running_cancel":
        return _running_cancel(session, control)
    if case == "run_limit":
        return _run_limit(session, control)
    return session.tool("search", {"store": "a", "query": "alpha", "mode": "text", "limit": 10})


def _submit_search(session, token=None):
    params = {"name": "search", "arguments": {"store": "a", "query": "alpha", "mode": "text", "limit": 10}}
    if token is not None:
        params["_meta"] = {"progressToken": token}
    return session.submit("tools/call", params, phase="control", tool="search", store="a", mode="text")


def _progress(session, predicate, timeout=5):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        for notification in session.notifications:
            if notification.get("method") == "notifications/progress" and predicate(notification["params"]):
                return notification
        session._consume(min(0.2, max(0.001, until - time.monotonic())))
    raise TimeoutError("actual queued/open progress stage was not observed")


def _drain(session, requests, timeout=5):
    until = time.monotonic() + timeout
    pending = {request_id for request_id in requests if "response" not in session.records[request_id]
               and not session.records[request_id]["cancellation_requested"]}
    while pending:
        request_id = session.wait_any(pending, timeout=max(0.001, until - time.monotonic()))
        if request_id is None:
            raise TimeoutError("released helper calls did not drain under the source bound")
        pending.remove(request_id)


def _hold_four(session, control):
    session.tool("orient", {"store": "a"})
    control.send("block_calls")
    requests = [_submit_search(session) for _ in range(4)]
    control.wait_held(4)
    return requests


def _queue_full(session, control):
    requests = _hold_four(session, control)
    try:
        requests.extend(_submit_search(session, f"queue-{index}") for index in range(32))
        stage = _progress(session, lambda params: params.get("message") == "queued: 32 waiting, 4 running")
        result = session.tool("search", {"store": "a", "query": "alpha", "mode": "text", "limit": 10})
        before = control.wait_held(4)["held_calls"]
    finally:
        control.send("release_all")
        _drain(session, requests)
    after = control.wait_held(0)["held_calls"]
    unexpected = [session.records[request_id].get("error", "missing successful outcome")
                  for request_id in requests if session.records[request_id].get("ok") is not True]
    return dict(result, diagnostic={"case": "queue_full", "held_before_release": before,
                "held_after_release": after, "queued_stage": stage, "unexpected_errors": unexpected,
                "complete": result.get("error") == "busy/admission/queue_full" and not unexpected})


def _queued_cancel(session, control):
    requests = _hold_four(session, control)
    try:
        target = _submit_search(session, "cancel-target")
        requests.append(target)
        _progress(session, lambda params: params.get("progressToken") == "cancel-target"
                  and params.get("message") == "queued: 1 waiting, 4 running")
        witness = _submit_search(session, "cancel-witness")
        requests.append(witness)
        before_stage = _progress(session, lambda params: params.get("progressToken") == "cancel-witness"
                                 and params.get("message") == "queued: 2 waiting, 4 running")
        session.notify("notifications/cancelled", {"requestId": target, "reason": "source diagnostic"})
        after_stage = _progress(session, lambda params: params.get("progressToken") == "cancel-witness"
            and params.get("progress", -1) > before_stage["params"]["progress"]
            and params.get("message") == "queued: 1 waiting, 4 running")
        session.wait(target, timeout=0.2)
        before = control.wait_held(4)["held_calls"]
    finally:
        control.send("release_all")
        _drain(session, requests)
    after = control.wait_held(0)["held_calls"]
    result = session.records[target]
    terminal_ok = (result["status"] == "unresolved" or result.get("error") == "timeout/cancelled")
    return dict(result, diagnostic={"case": "queued_cancel", "held_before_release": before,
        "held_after_release": after, "witness_rpc_id": witness,
        "queued_before_cancel": int(before_stage["params"]["message"].split()[1]),
        "queued_after_cancel": int(after_stage["params"]["message"].split()[1]),
        "before_cancel_stage": before_stage, "after_cancel_stage": after_stage,
        "complete": terminal_ok and all(session.records[item].get("ok") is True
                                        for item in requests if item != target)})


def _wait_deadline(session, control):
    requests = _hold_four(session, control)
    try:
        result = session.tool("search", {"store": "a", "query": "alpha", "mode": "text", "limit": 10})
        before = control.wait_held(4)["held_calls"]
    finally:
        control.send("release_all")
        _drain(session, requests)
    after = control.wait_held(0)["held_calls"]
    return dict(result, diagnostic={"case": "wait_deadline", "held_before_release": before,
        "held_after_release": after,
        "complete": result.get("error") == "busy/admission/deadline"
                    and all(session.records[item].get("ok") is True for item in requests)})


def _recover_four(session, control):
    control.send("block_calls")
    requests = [_submit_search(session) for _ in range(4)]
    try:
        held = control.wait_held(4)["held_calls"]
    finally:
        control.send("release_all")
        _drain(session, requests)
    control.wait_held(0)
    return held, requests


def _running_cancel(session, control):
    requests = _hold_four(session, control)
    target = requests[0]
    try:
        session.notify("notifications/cancelled", {"requestId": target, "reason": "source diagnostic"})
        probe = session.tool("search", {"store": "a", "query": "alpha", "mode": "text", "limit": 10})
        session.wait(target, timeout=0.2)
        before = control.wait_held(4)["held_calls"]
    finally:
        control.send("release_all")
        _drain(session, requests)
    after = control.wait_held(0)["held_calls"]
    held, recovery = _recover_four(session, control)
    result = session.records[target]
    busy = probe.get("payload", {}).get("error", {}).get("busy", {})
    terminal_ok = result["status"] == "unresolved" or result.get("error") == "timeout/cancelled"
    return dict(result, diagnostic={"case": "running_cancel", "held_before_release": before,
        "held_after_release": after, "probe": probe, "recovery_held": held,
        "recovery_rpc_ids": recovery,
        "complete": terminal_ok and busy.get("abandoned") == 1 and busy.get("in_flight") == 4
                    and all(session.records[item].get("ok") is True for item in recovery)})


def _run_limit(session, control):
    requests = _hold_four(session, control)
    try:
        session.wait(requests[0])
        _drain(session, requests)
        probe = session.tool("search", {"store": "a", "query": "alpha", "mode": "text", "limit": 10})
        before = control.wait_held(4)["held_calls"]
    finally:
        control.send("release_all")
        _drain(session, requests)
    after = control.wait_held(0)["held_calls"]
    held, recovery = _recover_four(session, control)
    result = session.records[requests[0]]
    busy = probe.get("payload", {}).get("error", {}).get("busy", {})
    return dict(result, diagnostic={"case": "run_limit", "held_before_release": before,
        "held_after_release": after, "original_rpc_ids": requests, "probe": probe,
        "recovery_held": held, "recovery_rpc_ids": recovery,
        "complete": busy.get("abandoned") == 4 and busy.get("in_flight") == 4
                    and all(session.records[item].get("error") == "timeout/deadline" for item in requests)
                    and all(session.records[item].get("ok") is True for item in recovery)})
