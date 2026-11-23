"""Ordinary-write familiarization and unseen Inspector study databases."""

import hashlib
import json
from fathomdb import Engine, ProjectionRole, ProjectionSpec


def add_study(engine: Engine, variant: str) -> list[str]:
    """Write the fixed study variant into an already confined Default session."""
    if variant not in ("training", "unseen"):
        raise ValueError("unknown study variant")
    training = variant == "training"
    prefix = "practice" if training else "field"
    token = "iris" if training else "basalt"
    name = "Robin Lake" if training else "Sam Vale"
    biography = (f"{name} is a {'botanist studying iris flowers' if training else 'geologist surveying basalt coastlines'} "
                 "and records careful field observations. ") * 30 + "END OF COMPLETE FIELD NOTE"
    source = f"Authored {prefix} survey source: {token} observations and directed collaboration records."
    source_revision = prefix + "-source-r1"
    source_version = prefix + "-source-v1"
    digest = hashlib.sha256(source.encode()).hexdigest()
    engine.configure_projections([ProjectionSpec(name="biography", roles=frozenset({ProjectionRole.SEARCHABLE}),
                                                source=["profile", "biography"], vector=True)])
    def provenance(revision: str) -> dict:
        return {"schema_version": 1, "role": "derived", "artifact_revision_id": revision,
                "source_version_id": source_version, "source_revision_id": source_revision,
                "source_locator": {"kind": "whole_body"},
                "canonical_source_hash": {"algorithm": "sha256", "digest_hex": digest}}
    def person(suffix: str, revision: str, text: str) -> dict:
        return {"kind": "doc", "logical_id": prefix + "-" + suffix, "source_id": "study:" + prefix,
                "body": json.dumps({"name": name, "city": "Tromsø 🦀", "identifier": 9007199254740993,
                                    "profile": {"biography": text}}, ensure_ascii=False),
                "provenance": provenance(prefix + "-" + suffix + "-" + revision)}
    revised = "b" if training else "a"
    engine.write([{"kind": "document", "logical_id": prefix + "-source", "source_id": "study:" + prefix,
                   "body": source, "provenance": {"schema_version": 1, "role": "canonical",
                   "artifact_revision_id": source_revision, "source_version_id": source_version}},
                  person(revised, "r1", "Retained earlier note about sailing; different revision, same logical identity.")])
    engine.write([person("a", "r2" if revised == "a" else "r1", biography),
                  person("b", "r2" if revised == "b" else "r1", biography),
                  person("c", "r1", f"{name} is a musician collecting recordings of distant birds."),
                  {"kind": "document", "logical_id": prefix + "-missing", "source_id": "study:" + prefix,
                   "body": "temporary forensic unresolved endpoint"}])
    relationships = [("a", "b", "collaborates")] if training else [("a", "c", "records"), ("c", "b", "introduces")]
    relationships.append(("a", "missing", "mentions"))
    for index, (start, end, kind) in enumerate(relationships):
        logical = f"{prefix}-edge-{index + 1}"
        engine.write([{"edge": {"kind": kind, "logical_id": logical,
                     "from": prefix + "-" + start, "to": prefix + "-" + end,
                     "source_id": "study:" + prefix, "body": f"Directed {kind}: {start} to {end}; full {token} relationship é🦀",
                     "provenance": provenance(logical + "-r1")}}])
    revisions = [f"{prefix}-{revised}-r1", f"{prefix}-{revised}-r2",
                 f"{prefix}-{'a' if revised == 'b' else 'b'}-r1", f"{prefix}-c-r1"]
    revisions += [f"{prefix}-edge-{index + 1}-r1" for index in range(len(relationships))]
    for revision in revisions:
        engine.register_source_dependency({"schema_version": 1, "dependency_id": "study-" + revision,
                                           "source_revision_id": source_revision, "derived_revision_id": revision})
    engine.drain(timeout_s=25)
    return [prefix + "-missing"]
