"""Supplemental installed production-wheel lifecycle, separate from test hooks."""

import hashlib
import json
import os
import pathlib
import sys

import fathomdb
import fathomdb._fathomdb as native

source = pathlib.Path(os.environ["S60_SOURCE"]).resolve()
database = pathlib.Path(sys.argv[1]).resolve()
assert database.is_relative_to(pathlib.Path(os.environ["S60_TASK"]).resolve())
assert "PYTHONPATH" not in os.environ
module = pathlib.Path(native.__file__).resolve()
assert pathlib.Path(sys.prefix).resolve() == source / ".venv"
assert module.is_relative_to(source / ".venv")
for _ in range(2):
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    assert engine.open_report().embedder_device_resolution is None
    engine.close()
print(
    json.dumps(
        {
            "kind": "installed-production-python",
            "module": str(module),
            "sha256": hashlib.sha256(module.read_bytes()).hexdigest(),
            "prefix": sys.prefix,
            "open_close_cycles": 2,
            "use_default_embedder": False,
        }
    )
)
