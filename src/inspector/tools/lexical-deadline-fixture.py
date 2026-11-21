"""Build a real bounded-text corpus for installed release native deadline qualification."""

import pathlib
import sys
from fathomdb import Engine, EngineConfig


def build(directory: pathlib.Path) -> None:
    """Write4800 authored1332-term records with ordinary model-free Engine writes."""
    directory.mkdir()
    work = directory / "work"
    work.mkdir()
    path = work / "fixture.sqlite"
    engine = Engine.open(str(path), use_default_embedder=False,
                         config=EngineConfig(scheduler_runtime_threads=1))
    alphabet = "abcdefghijklmnopqrstuvwxyz0123456789"
    text = " ".join([*alphabet, *(a + b for a in alphabet for b in alphabet)])
    try:
        for start in range(0, 4800, 100):
            engine.write([{"kind": "document", "logical_id": f"deadline-{n}",
                           "source_id": "fixture:installed-deadline", "body": f"{text} row{n}"}
                          for n in range(start, start + 100)])
    finally:
        engine.close()
    print(path)


if __name__ == "__main__":
    build(pathlib.Path(sys.argv[1]))
