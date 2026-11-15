"""Bounded sixteen-record JSONL checkpoints, without fsync durability claims."""

import json
import time


class CheckpointLog:
    """Append UTF8 JSONL and flush at sixteen records and explicit boundaries."""

    def __init__(self, path):
        self.stream = path.open("ab")
        self.records = self.encoded_bytes = self.checkpoints = 0
        self.persistence_ns = self.checkpoint_ns = self.pending = 0

    def write(self, value):
        """Persist a snapshot of one record outside client response timing."""
        began = time.monotonic_ns()
        raw = (json.dumps(value, ensure_ascii=False, allow_nan=False,
                          separators=(",", ":")) + "\n").encode("utf-8")
        self.stream.write(raw)
        self.records += 1
        self.pending += 1
        self.encoded_bytes += len(raw)
        if self.pending == 16:
            self.boundary()
        self.persistence_ns += time.monotonic_ns() - began

    def boundary(self):
        """Flush a phase boundary; retain honest record/checkpoint accounting."""
        if self.pending:
            began = time.monotonic_ns()
            self.stream.flush()
            self.checkpoint_ns += time.monotonic_ns() - began
            self.checkpoints += 1
            self.pending = 0

    def close(self):
        """Flush the final prefix and close this owned stream."""
        self.boundary()
        self.stream.close()

    def summary(self):
        """Return persistence costs separately; checkpoint time is a nested subset."""
        return {key: getattr(self, key) for key in
                ("records", "encoded_bytes", "checkpoints", "persistence_ns", "checkpoint_ns")}
