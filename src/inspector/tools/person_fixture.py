"""Ordinary authored F07 people, provenance and directed relationships for Inspector."""

import hashlib
import json
from fathomdb import Engine, ProjectionSpec, ProjectionRole


def add_people(engine: Engine) -> None:
    """Write/drain real person revisions and links using the already confined Default session."""
    engine.configure_projections(
        [ProjectionSpec(name="biography", roles=frozenset({ProjectionRole.SEARCHABLE}),
                        source=["profile", "biography"], vector=True)]
    )
    source = "original authored people source: biographies and orchid laboratory relationships"
    digest = hashlib.sha256(source.encode()).hexdigest()

    def provenance(revision: str) -> dict:
        return {"schema_version": 1, "role": "derived", "artifact_revision_id": revision,
                "source_version_id": "people-source-v1", "source_revision_id": "people-source-r1",
                "source_locator": {"kind": "whole_body"},
                "canonical_source_hash": {"algorithm": "sha256", "digest_hex": digest}}

    def person(identity: str, revision: str, biography: str) -> dict:
        return {"kind": "doc", "logical_id": identity, "source_id": "fixture:people",
                "body": json.dumps({"name": "Alex Rivera", "city": "München 🦀",
                                    "identifier": 9007199254740993,
                                    "profile": {"biography": biography}}, ensure_ascii=False),
                "provenance": provenance(revision)}

    engine.write([{"kind": "document", "logical_id": "people-source", "source_id": "fixture:people",
                   "body": source, "provenance": {"schema_version": 1, "role": "canonical",
                   "artifact_revision_id": "people-source-r1", "source_version_id": "people-source-v1"}},
                  person("person-a", "person-a-r1", "retained early biography: Alex once studied botany")])
    biography = ("Alex Rivera is a software engineer and orchid researcher in München 🦀. "
                 "Alex records laboratory observations and helps people investigate data. ") * 35
    biography += "END OF COMPLETE BIOGRAPHY"
    engine.write([person("person-a", "person-a-r2", biography),
                  person("person-b", "person-b-r1", biography),
                  person("person-c", "person-c-r1", "Alex Rivera is an astronomer studying icy distant planets"),
                  {"edge": {"kind": "knows", "logical_id": "person-knows", "from": "person-a", "to": "person-b",
                            "source_id": "fixture:people", "body": "Alex and Alex met at the orchid laboratory; full directed relationship é🦀",
                            "provenance": provenance("person-knows-r1")}},
                  {"edge": {"kind": "mentions", "logical_id": "person-unresolved", "from": "person-a", "to": "unknown-person",
                            "source_id": "fixture:people", "body": "recorded unavailable endpoint",
                            "provenance": provenance("person-unresolved-r1")}}])
    for revision in ["person-a-r1", "person-a-r2", "person-b-r1", "person-c-r1", "person-knows-r1", "person-unresolved-r1"]:
        engine.register_source_dependency({"schema_version": 1, "dependency_id": "people-" + revision,
                                           "source_revision_id": "people-source-r1", "derived_revision_id": revision})
    engine.drain(timeout_s=25)
