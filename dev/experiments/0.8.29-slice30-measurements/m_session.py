"""Actual MCP session over the owned invocation transport."""

import time

from m_client import RpcChild
from m_record import CheckpointLog
from m_wire import decode_tool


class McpSession(RpcChild):
    """Decode and checkpoint actual outcomes while retaining the original wire ledger."""

    def __init__(self, command, root, *, result_format="text", timeout=45, **kwargs):
        super().__init__(command, root, timeout=timeout, **kwargs)
        self.result_format = result_format
        self._outcomes = CheckpointLog(self.root / "outcomes.jsonl")

    def _consume(self, timeout):
        prior = len(self._completion_order)
        consumed = super()._consume(timeout)
        if len(self._completion_order) > prior:
            request_id = self._completion_order[-1]
            row = self.records[request_id]
            began = time.monotonic_ns()
            if row["request"]["method"] == "tools/call":
                try:
                    row.update(decode_tool(row["response"]))
                except (ValueError, KeyError, TypeError) as error:
                    row.update(ok=False, error="decode/incomplete", decode_issue=str(error))
            else:
                row["ok"] = "error" not in row["response"]
                if not row["ok"]:
                    row["error"] = f"protocol/{row['response']['error']['code']}"
            row["format"] = self.result_format
            row["decode_ns"] = time.monotonic_ns() - began
            self._outcomes.write(row)
        return consumed

    def initialize(self):
        """Complete the existing MCP handshake/discovery and retain both clocks."""
        request_id = self.submit("initialize", {"protocolVersion": "2025-06-18",
            "capabilities": {}, "clientInfo": {"name": "slice30-m", "version": "0"}}, phase="control")
        response = self.wait(request_id)
        if response is None or "result" not in response:
            raise RuntimeError("MCP initialize failed")
        self.notify("notifications/initialized", {})
        self.handshake_complete_ns = time.monotonic_ns()
        request_id = self.submit("tools/list", {}, phase="control")
        response = self.wait(request_id)
        if response is None or sorted(tool["name"] for tool in response["result"]["tools"]) != [
                "expand", "feedback", "fetch", "list_stores", "orient", "search", "use_store"]:
            raise RuntimeError("MCP discovery differs from the existing seven tools")
        self.discovery_complete_ns = time.monotonic_ns()
        return response

    def tool(self, name, arguments, *, phase="control", **facts):
        """Return this actual invocation record, including its decoded terminal outcome."""
        request_id = self.submit("tools/call", {"name": name, "arguments": arguments},
                                 phase=phase, tool=name, **facts)
        if self.wait(request_id) is None:
            raise TimeoutError(f"bounded MCP response absent for request{request_id}")
        return self.records[request_id]

    def phase_boundary(self):
        """Checkpoint both raw invocation facts and decoded outcomes after draining."""
        super().phase_boundary()
        self._outcomes.boundary()

    def close(self):
        """Observe normal child exit/EOF and close the decoded checkpoint stream."""
        result = super().close()
        if not self._outcomes.stream.closed:
            for request_id, row in self.records.items():
                if row["status"] != "completed":
                    self._outcomes.write(row)
            self._outcomes.close()
        result["persistence"]["outcomes"] = self._outcomes.summary()
        return result
