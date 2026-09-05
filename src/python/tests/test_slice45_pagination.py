"""Slice 45 governed pagination parity for the public Python SDK."""

from __future__ import annotations

import fathomdb
from fathomdb import read


def test_canonical_and_operational_pages_share_frozen_authority(db_path: str) -> None:
    engine = fathomdb.Engine.open(db_path, use_default_embedder=False)
    try:
        engine.write(
            [
                {
                    "kind": "slice45_doc",
                    "body": f'{{"n":{index}}}',
                    "logical_id": f"n-{index}",
                    "source_id": "python:slice45",
                }
                for index in range(3)
            ]
        )
        engine.write(
            [
                {
                    "admin_schema": {
                        "name": "slice45_state",
                        "kind": "latest_state",
                        "schema_json": "{}",
                        "retention_json": "{}",
                    }
                }
            ]
        )
        for index in range(3):
            engine.write(
                [
                    {
                        "op_store": {
                            "collection": "slice45_state",
                            "record_key": f"k-{index}",
                            "body": f'{{"n":{index}}}',
                        }
                    }
                ]
            )

        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        first = read.canonical_page(
            engine,
            "slice45_doc",
            frozen,
            fathomdb.PageRequestV1(limit=2),
        )
        second = read.canonical_page(
            engine,
            "slice45_doc",
            frozen,
            fathomdb.PageRequestV1(limit=2, cursor=first.next_cursor),
        )
        assert [row.logical_id for row in first.items + second.items] == ["n-0", "n-1", "n-2"]
        assert second.next_cursor is None

        state = read.operational_state(engine, "slice45_state", "k-0", frozen)
        state_page = read.operational_state_page(
            engine,
            "slice45_state",
            frozen,
            fathomdb.PageRequestV1(limit=2),
        )
        assert state == state_page.items[0]
    finally:
        engine.close()
