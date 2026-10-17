"""0.8.28 Slice 41 R2 code-review fixes for the Python context pack.

Non-finite and oversized counts are ``InvalidArgumentError`` naming the field
(NaN must not surface as an anonymous "not valid JSON"), and ``keep_roles``
given as a bare ``str`` is a ``TypeError`` rather than per-character roles.
"""

from __future__ import annotations

import math

import pytest

from fathomdb import ContextItem, ContextPart, ContextPolicy, pack_context
from fathomdb.errors import InvalidArgumentError

_PART = (ContextPart(role="user", text="x"),)


@pytest.mark.parametrize("value", [math.nan, math.inf, -math.inf, 2**70])
def test_bad_position_names_the_field(value: float) -> None:
    with pytest.raises(InvalidArgumentError) as raised:
        pack_context([ContextItem(id="a", parts=_PART, position=value)])  # type: ignore[arg-type]
    assert "hits[0].position" in str(raised.value)


@pytest.mark.parametrize("value", [math.nan, math.inf])
def test_non_finite_policy_count_names_the_field(value: float) -> None:
    with pytest.raises(InvalidArgumentError) as raised:
        pack_context([], ContextPolicy(budget_chars=value))  # type: ignore[arg-type]
    assert "policy.budget_chars" in str(raised.value)


def test_non_finite_context_position_names_the_field() -> None:
    with pytest.raises(InvalidArgumentError) as raised:
        pack_context(
            [ContextItem(id="a", parts=_PART, group="g", position=0)],
            ContextPolicy(),
            [ContextItem(id="b", parts=_PART, group="g", position=math.nan)],  # type: ignore[arg-type]
        )
    assert "context[0].position" in str(raised.value)


def test_keep_roles_as_a_bare_string_is_a_type_error() -> None:
    with pytest.raises(TypeError) as raised:
        pack_context([], ContextPolicy(keep_roles="assistant"))  # type: ignore[arg-type]
    assert "keep_roles" in str(raised.value)
