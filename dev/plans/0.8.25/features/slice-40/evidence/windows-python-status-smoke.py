import json
import pathlib
import sys

import fathomdb
from fathomdb import read
from fathomdb.types import ProjectionRole, ProjectionSpec


database = pathlib.Path(sys.argv[1])
engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
first = read.projection_generation_status(engine)
assert first.origin == "fresh"
assert first.readiness == "ready"
engine.actuate(
    {
        "schema_version": 1,
        "operation_id": "slice40-windows-package-seed",
        "operations": [
            {
                "type": "put_canonical_node",
                "record": {
                    "kind": "doc",
                    "body": "seed",
                    "logical_id": "seed",
                    "source_id": "source:slice40-windows-package-seed",
                    "provenance": {
                        "schema_version": 1,
                        "artifact_revision_id": "slice40-windows-package-seed-r1",
                        "source_version_id": "slice40-windows-package-seed-v1",
                        "role": "canonical",
                    },
                },
            }
        ],
    }
)
engine.configure_projections(
    [
        ProjectionSpec(
            name="memory",
            roles=frozenset({ProjectionRole.SEARCHABLE}),
            vector=True,
        )
    ]
)
engine.drain(timeout_s=10)
engine.close()
engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
receipt = engine.actuate(
    {
        "schema_version": 1,
        "operation_id": "slice40-windows-package-smoke",
        "operations": [
            {
                "type": "put_canonical_node",
                "record": {
                    "kind": "doc",
                    "body": "windows package generation status",
                    "logical_id": "node",
                    "source_id": "source:slice40-windows-package-smoke",
                    "provenance": {
                        "schema_version": 1,
                        "artifact_revision_id": "slice40-windows-package-smoke-r1",
                        "source_version_id": "slice40-windows-package-smoke-v1",
                        "role": "canonical",
                    },
                },
            }
        ],
    }
)
assert receipt.projection_generation_id is not None
assert len(receipt.pending_projection_write_cursors) == 1
status = read.mutation_projection_status(
    engine,
    {
        "schemaVersion": 1,
        "operationId": receipt.operation_id,
        "writeCursor": receipt.pending_projection_write_cursors[0],
        "expectedGenerationId": receipt.projection_generation_id,
    },
)
assert status.readiness == "blocked"
assert status.runtime_state == "absent"
current_id = read.projection_generation_status(engine).generation_id
engine.close()

reopened = fathomdb.Engine.open(str(database), use_default_embedder=False)
assert read.projection_generation_status(reopened).generation_id == current_id
reopened.close()
print(
    json.dumps(
        {
            "outcome": "pass",
            "consumer": "python-wheel",
            "module": str(pathlib.Path(fathomdb.__file__).resolve()),
            "generation_id": current_id,
            "mutation_readiness": status.readiness,
            "runtime_state": status.runtime_state,
        },
        sort_keys=True,
    )
)
