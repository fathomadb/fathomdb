"""Verify an exact installed wheel with a real store in two process phases."""

import importlib.machinery
import importlib.metadata
import os
from pathlib import Path
import sys


def main() -> None:
    """Check native identity, exact content, missing lookup, and close."""
    expected, store, phase, *expected_native = sys.argv[1:]
    if phase not in {"write", "reopen"}:
        raise SystemExit("expected write or reopen phase")
    if phase == "reopen" and not Path(store).is_file():
        raise SystemExit("persisted store is missing")

    actual = importlib.metadata.version("fathomdb")
    if actual != expected:
        raise SystemExit(f"wheel version mismatch: expected={expected} resolved={actual}")
    from fathomdb import Engine, _fathomdb, read

    native = Path(_fathomdb.__file__).resolve()
    prefix = Path(sys.prefix).resolve()
    if not native.is_relative_to(prefix) or not any(
        native.name.endswith(suffix) for suffix in importlib.machinery.EXTENSION_SUFFIXES
    ):
        raise SystemExit(f"unexpected installed native module {native} for {prefix}")
    if expected_native and native != Path(expected_native[0]).resolve():
        raise SystemExit(f"loaded native module {native} differs from checked binary {expected_native[0]}")

    record_id = "registry-smoke:record"
    body = "registry Python persisted café"
    engine = Engine.open(store, use_default_embedder=False)
    try:
        if phase == "write":
            engine.write([{
                "kind": "doc", "body": body, "source_id": "smoke:pypi-wheel", "logical_id": record_id,
            }])
        found = read.get(engine, record_id)
        if found is None or found.body != body:
            raise AssertionError("persisted body is absent or wrong")
        if read.get(engine, "registry-smoke:missing") is not None:
            raise AssertionError("missing lookup resolved a record")
    finally:
        engine.close()
    print(f"wheel {phase}: pid={os.getpid()} exact body and missing record verified; native={native}")


if __name__ == "__main__":
    main()
