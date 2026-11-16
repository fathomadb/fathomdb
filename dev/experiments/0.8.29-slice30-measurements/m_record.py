"""Bounded sixteen-record JSONL checkpoints, without fsync durability claims."""

import json
import time


JSONL_MAX_BYTES = 64 * 1024**2
JSON_MAX_BYTES = 8 * 1024**2
CONTROL_MAX_BYTES = 1024**2


class OutputLimitError(RuntimeError):
    """A complete next record would exceed its retained capture limit."""


def write_json(path, value, *, limit=None):
    """Write one complete JSON publication, refusing before opening on overflow."""
    limit = JSON_MAX_BYTES if limit is None else limit
    text = json.dumps(value, indent=2, allow_nan=False) + "\n"
    size = len(text.encode("utf-8"))
    if size > limit:
        raise OutputLimitError(f"{path.name}: capture limit {limit} bytes; next publication {size} bytes")
    return path.write_text(text, encoding="utf-8")


class CheckpointLog:
    """Append UTF8 JSONL and flush at sixteen records and explicit boundaries."""

    def __init__(self, path, *, max_bytes=None):
        self.stream = path.open("ab")
        self.max_bytes = JSONL_MAX_BYTES if max_bytes is None else max_bytes
        self.retained_bytes = self.stream.tell()
        self.records = self.encoded_bytes = self.checkpoints = 0
        self.persistence_ns = self.checkpoint_ns = self.pending = 0

    def write(self, value):
        """Persist a snapshot of one record outside client response timing."""
        began = time.monotonic_ns()
        raw = (json.dumps(value, ensure_ascii=False, allow_nan=False,
                          separators=(",", ":")) + "\n").encode("utf-8")
        if self.retained_bytes + len(raw) > self.max_bytes:
            raise OutputLimitError(f"{self.stream.name}: capture limit {self.max_bytes} bytes")
        self.stream.write(raw)
        self.retained_bytes += len(raw)
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
