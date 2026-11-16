"""Exact reporting transport domains and malformed native-payload paths."""
from copy import deepcopy
from dataclasses import FrozenInstanceError
from types import SimpleNamespace as NS

import pytest

from fathomdb._sdk_search import _map_native_snippet
from fathomdb.errors import ProjectionGenerationError
from fathomdb.read import _map_stored_head_coverage


def native():
    return NS(status="available", population="stored_mapped_spans", reason=None,
              profile=NS(key="default", name="fixture", revision="v1", dimension=8),
              records=4, known_records=3, unknown_records=1, truncated_known_records=1,
              known_body_bytes=16, unrepresented_known_bytes=6, unattributed_heads=0,
              truncated_known_share=NS(numerator=1, denominator=3),
              unrepresented_known_byte_share=NS(numerator=6, denominator=16))


def test_exact_integer_domains_and_frozen_variants():
    value = native()
    value.records = value.unattributed_heads = 2**63 - 1
    value.known_records = 2**63 - 2
    value.unknown_records = 1
    value.truncated_known_records = 2**63 - 3
    value.known_body_bytes = 2**128 - 1
    value.unrepresented_known_bytes = 2**128 - 2
    value.truncated_known_share = NS(numerator=value.truncated_known_records, denominator=value.known_records)
    value.unrepresented_known_byte_share = NS(numerator=value.unrepresented_known_bytes, denominator=value.known_body_bytes)
    got = _map_stored_head_coverage(value)
    assert got.records == 9223372036854775807
    assert got.known_body_bytes == 340282366920938463463374607431768211455
    assert type(got.known_body_bytes) is int
    assert got.profile.dimension == 8 and type(got.profile.dimension) is int
    assert got.unrepresented_known_byte_share.numerator == 340282366920938463463374607431768211454
    with pytest.raises(FrozenInstanceError):
        got.records = 0
    for reason in ("cache_cold", "state_changed"):
        unavailable = NS(status="unavailable", population="stored_mapped_spans", profile=value.profile, reason=reason)
        got = _map_stored_head_coverage(unavailable)
        assert got.reason == reason and got.profile.dimension == 8
        assert not hasattr(got, "records") and not hasattr(got, "truncated_known_share")


@pytest.mark.parametrize("path,replacement", [
    ("population", "live_view"), ("status", "ready"), ("reason", "cache_cold"),
    ("profile.key", "other"), ("profile.dimension", "8"), ("profile.dimension", 0),
    ("profile.dimension", 2**32), ("profile.name", ""), ("records", 5),
    ("records", True), ("known_records", "03"), ("known_records", -1),
    ("known_records", 2**63), ("known_body_bytes", 2**128),
    ("truncated_known_records", 4), ("unrepresented_known_bytes", 17),
    ("truncated_known_share.numerator", 0), ("truncated_known_share.denominator", 1),
    ("unrepresented_known_byte_share.denominator", 0),
    ("unrepresented_known_byte_share.numerator", 6.0),
])
def test_malformed_native_values_refuse_at_exact_field_path(path, replacement):
    value = deepcopy(native())
    pieces = path.split(".")
    owner = value
    for piece in pieces[:-1]:
        owner = getattr(owner, piece)
    setattr(owner, pieces[-1], replacement)
    with pytest.raises(ProjectionGenerationError) as error:
        _map_stored_head_coverage(value)
    assert error.value.reason == "projection_generation_corrupt"
    assert error.value.field_path == "/vector_coverage/" + path.replace(".", "/")


@pytest.mark.parametrize("path", ["known_records", "profile.revision", "truncated_known_share.numerator"])
def test_missing_native_fields_refuse_at_exact_path(path):
    value = native()
    pieces = path.split(".")
    owner = value
    for piece in pieces[:-1]:
        owner = getattr(owner, piece)
    delattr(owner, pieces[-1])
    with pytest.raises(ProjectionGenerationError) as error:
        _map_stored_head_coverage(value)
    assert error.value.field_path == "/vector_coverage/" + path.replace(".", "/")


@pytest.mark.parametrize("field", ["truncated_known_share", "unrepresented_known_byte_share", "profile"])
def test_null_required_native_objects_name_the_corrupt_object(field):
    value = native()
    setattr(value, field, None)
    with pytest.raises(ProjectionGenerationError) as error:
        _map_stored_head_coverage(value)
    assert error.value.field_path == "/vector_coverage/" + field


def test_unavailable_cannot_carry_numbers_or_unknown_reason():
    for field, replacement in [("records", 0), ("reason", "other")]:
        value = NS(status="unavailable", population="stored_mapped_spans", profile=native().profile, reason="cache_cold")
        setattr(value, field, replacement)
        with pytest.raises(ProjectionGenerationError) as error:
            _map_stored_head_coverage(value)
        assert error.value.field_path == "/vector_coverage/" + field


def test_scored_scalar_preserves_zero_and_checked_u32_domain():
    value = NS(text="", anchor="whole", windows=[], spans=[], highlights=[], cut=False)
    for extent in (None, 0, 2**32 - 1):
        value.scored_bytes = extent
        assert _map_native_snippet(value).scored_bytes == extent
    for extent in (True, -1, 2**32, 1.5, "0"):
        value.scored_bytes = extent
        with pytest.raises(ProjectionGenerationError) as error:
            _map_native_snippet(value)
        assert error.value.field_path == "/scored_bytes"
