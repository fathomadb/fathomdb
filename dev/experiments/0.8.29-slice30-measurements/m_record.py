"""Bounded sixteen-record JSONL checkpoints, without fsync durability claims."""

import json
import os
import threading
import time


JSONL_MAX_BYTES = 64 * 1024**2
JSON_MAX_BYTES = 8 * 1024**2
CONTROL_MAX_BYTES = 1024**2
LOG_MAX_BYTES = 1024**2


class OutputLimitError(RuntimeError):
    """A complete next record would exceed its retained capture limit."""


def write_json(path, value, *, limit=None, exclusive=False):
    """Write one complete JSON publication, refusing before opening on overflow."""
    limit = JSON_MAX_BYTES if limit is None else limit
    text = json.dumps(value, indent=2, allow_nan=False) + "\n"
    size = len(text.encode("utf-8"))
    if size > limit:
        raise OutputLimitError(f"{path.name}: capture limit {limit} bytes; next publication {size} bytes")
    if exclusive:
        with path.open("x", encoding="utf-8") as stream:
            return stream.write(text)
    return path.write_text(text, encoding="utf-8")


def write_bytes(path, value):
    """Retain one bounded control prefix without opening on an oversized value."""
    if len(value) > CONTROL_MAX_BYTES:
        raise OutputLimitError(f"{path.name}: capture limit {CONTROL_MAX_BYTES} bytes")
    return path.write_bytes(value)


def write_guardian(path, receipt):
    """Preserve cleanup/accounting when an oversized diagnostic receipt is refused."""
    try:
        write_json(path, receipt, limit=CONTROL_MAX_BYTES)
    except OutputLimitError as error:
        receipt.update(exit=125, error=str(error), output_overflow=str(error))
        receipt.pop("resources", None)
        receipt.pop("load", None)
        write_json(path, receipt, limit=CONTROL_MAX_BYTES)



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


class BoundedLog:
    """Capture an owned child's pipe with a pre-write byte limit and failure flag."""

    def __init__(self, path, *, exclusive=False):
        self.saved = path.open("xb" if exclusive else "ab")
        self.limit = LOG_MAX_BYTES
        self.retained_bytes = self.saved.tell()
        self.reader, self.writer = os.pipe()
        self.thread = None
        self.lock = threading.Lock()
        self.error = None

    def fileno(self):
        """Supply only the pipe writer to the owned subprocess."""
        return self.writer

    def start(self):
        """Start draining after the single-threaded fork or private attachment."""
        if self.thread is None:
            self.thread = threading.Thread(target=self._read, daemon=True)
            self.thread.start()

    def _read(self):
        try:
            while raw := os.read(self.reader, 65536):
                self._save(raw)
        except OSError as error:
            self.error = self.error or f"control log read: {error}"

    def _save(self, raw):
        with self.lock:
            if self.error:
                return
            if self.retained_bytes + len(raw) > self.limit:
                self.error = f"{self.saved.name}: capture limit {self.limit} bytes"
                return
            try:
                self.saved.write(raw)
                self.saved.flush()
                self.retained_bytes += len(raw)
            except OSError as error:
                self.error = f"control log write: {error}"

    def write(self, text):
        """Retain parent-side diagnostics under the same log bound."""
        self._save(text.encode("utf-8"))
        return len(text)

    def flush(self):
        """Flush the bounded saved prefix."""
        with self.lock:
            self.saved.flush()

    def close(self):
        """Drain after owned cleanup; an unavailable EOF remains a capture failure."""
        os.close(self.writer)
        if self.thread is not None:
            self.thread.join(timeout=2)
            if self.thread.is_alive():
                self.error = self.error or "control log reader did not reach EOF"
        os.close(self.reader)
        self.saved.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
