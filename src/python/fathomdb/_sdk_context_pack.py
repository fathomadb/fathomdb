"""Context pack (0.8.28 Slice 41): render ranked items into prompt context.

`pack_context` renders caller items within a character budget: the first
`full_items` in selection order show every part, the rest keep the
`keep_roles` parts (and role-less parts) whole and share `compact_chars`
characters among the others. `encode_round_body` / `decode_round_body` are
the opt-in role-tagged body convention; `context_item_from_hit` builds an
item from a `SearchHit`. Counts are Unicode scalar values (Python `len`).
All logic is the engine's single Rust implementation; this module only
converts values, so Python, TypeScript and Rust return identical output.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass, field
from typing import Any, Iterable, Literal

from fathomdb import _fathomdb as _native

ContextSource = Literal["hit", "context"]
ContextTier = Literal["full", "compact", "omitted"]
ContextOrder = Literal["rank", "chronological"]

_ID_PREFIX = {"logical": "l:", "content": "h:", "passage": "p:"}


@dataclass(frozen=True)
class ContextPart:
    """One piece of an item: free-text ``role`` (or ``None``) and its text."""

    role: str | None
    text: str


@dataclass(frozen=True)
class ContextItem:
    """One caller item.

    ``group`` and ``position`` drive group expansion, ``order_key`` drives
    chronological order, and ``header`` is rendered verbatim on its own line
    above the item text without being charged to the budget.
    """

    id: str
    parts: tuple[ContextPart, ...] = ()
    group: str | None = None
    position: int | None = None
    order_key: str | None = None
    header: str | None = None


@dataclass(frozen=True)
class ContextPolicy:
    """Budget and tier settings; the defaults are the product defaults."""

    budget_chars: int = 30000
    item_chars: int = 4000
    full_items: int = 3
    compact_chars: int = 300
    keep_roles: tuple[str, ...] = ("user",)
    expand_groups: bool = True
    order: ContextOrder = "rank"


@dataclass(frozen=True)
class ContextPartEntry:
    """Rendered characters of one part out of its full length."""

    role: str | None
    chars: int
    of_chars: int


@dataclass(frozen=True)
class ContextPackEntry:
    """Provenance of one input item.

    ``chars`` counts the item text including the ``"\\n"`` joins between parts.
    ``truncated`` is true when a rendered item shows fewer characters of some
    part than it has; it is false for omitted items.
    """

    id: str
    source: ContextSource
    tier: ContextTier
    chars: int
    truncated: bool
    rendered_index: int | None
    parts: tuple[ContextPartEntry, ...]


@dataclass(frozen=True)
class ContextPack:
    """Rendered text plus one provenance entry per input item.

    ``chars_used`` is the sum of item ``chars`` (bounded by ``budget_chars``);
    ``text_chars`` is ``len(text)``, headers and separators included.
    """

    text: str
    chars_used: int
    text_chars: int
    items: tuple[ContextPackEntry, ...] = field(default_factory=tuple)


def _dumps(value: Any) -> str:
    # ensure_ascii: a lone surrogate becomes an escape the core refuses as an
    # invalid argument instead of failing UTF-8 encoding at the FFI boundary.
    return json.dumps(value, ensure_ascii=True)


def _count_wire(value: Any) -> Any:
    # A non-finite float cannot be JSON; forward it as a string so the core
    # refuses it as a non-integer count naming the field.
    if isinstance(value, float) and not math.isfinite(value):
        return repr(value)
    return value


def _part_wire(part: ContextPart) -> dict[str, Any]:
    if not isinstance(part, ContextPart):
        raise TypeError(f"parts must be ContextPart, got {type(part).__name__}")
    return {"role": part.role, "text": part.text}


def _item_wire(item: ContextItem) -> dict[str, Any]:
    if not isinstance(item, ContextItem):
        raise TypeError(f"items must be ContextItem, got {type(item).__name__}")
    wire: dict[str, Any] = {"id": item.id, "parts": [_part_wire(p) for p in item.parts]}
    for name in ("group", "position", "order_key", "header"):
        value = getattr(item, name)
        if value is not None:
            wire[name] = _count_wire(value) if name == "position" else value
    return wire


def _policy_wire(policy: ContextPolicy) -> dict[str, Any]:
    if not isinstance(policy, ContextPolicy):
        raise TypeError(f"policy must be ContextPolicy, got {type(policy).__name__}")
    if isinstance(policy.keep_roles, str):
        raise TypeError("policy.keep_roles must be a sequence of role strings, not a str")
    return {
        "budget_chars": _count_wire(policy.budget_chars),
        "item_chars": _count_wire(policy.item_chars),
        "full_items": _count_wire(policy.full_items),
        "compact_chars": _count_wire(policy.compact_chars),
        "keep_roles": list(policy.keep_roles),
        "expand_groups": policy.expand_groups,
        "order": policy.order,
    }


def _pack_from_wire(value: dict[str, Any]) -> ContextPack:
    return ContextPack(
        text=value["text"],
        chars_used=value["chars_used"],
        text_chars=value["text_chars"],
        items=tuple(
            ContextPackEntry(
                id=entry["id"],
                source=entry["source"],
                tier=entry["tier"],
                chars=entry["chars"],
                truncated=entry["truncated"],
                rendered_index=entry["rendered_index"],
                parts=tuple(
                    ContextPartEntry(role=p["role"], chars=p["chars"], of_chars=p["of_chars"])
                    for p in entry["parts"]
                ),
            )
            for entry in value["items"]
        ),
    )


def pack_context(
    hits: Iterable[ContextItem],
    policy: ContextPolicy | None = None,
    context: Iterable[ContextItem] = (),
) -> ContextPack:
    """Render ranked ``hits`` (and, with group expansion, ``context``).

    Selection is the hits in order, then for each hit group (in order of its
    first hit) the ``context`` items of that group by ``position``. Items are
    cut to ``item_chars`` and then to what is left of ``budget_chars``.

    Raises:
        InvalidArgumentError: the message names the offending field, for a zero
            ``budget_chars`` or ``item_chars``, a negative or non-integer count,
            an unknown ``order``, an empty or repeated id, or a grouped
            ``context`` item without ``position`` while ``expand_groups`` is on.
        TypeError: an argument is not the documented dataclass, or
            ``policy.keep_roles`` is a bare ``str``.
    """
    request = {
        "hits": [_item_wire(item) for item in hits],
        "policy": _policy_wire(policy if policy is not None else ContextPolicy()),
        "context": [_item_wire(item) for item in context],
    }
    return _pack_from_wire(json.loads(_native._pack_context_json(_dumps(request))))


def encode_round_body(parts: Iterable[ContextPart]) -> str:
    """Encode parts as an opt-in role-tagged round body.

    Each part is written as U+001E, a role code (role ``None`` U+0010, ``user``
    U+0011, ``assistant`` U+0012, ``system`` U+0013, ``tool`` U+0014) and the
    text; parts are joined by ``"\\n"``. Removing the markers gives the plain
    ``"\\n"``-joined texts, and FTS ranking is unchanged. Do not call
    ``str.splitlines()`` on a tagged body: it splits on U+001E.

    Raises:
        InvalidArgumentError: an empty part list, another role, or a text that
            contains U+001E or a role code.
    """
    wire = [_part_wire(part) for part in parts]
    return str(_native._encode_round_body_json(_dumps(wire)))


def decode_round_body(body: str) -> tuple[ContextPart, ...]:
    """Decode a body into parts; an untagged body is one part with role ``None``.

    Raises:
        InvalidArgumentError: the body starts with U+001E but does not decode.
    """
    if not isinstance(body, str):
        raise TypeError(f"body must be str, got {type(body).__name__}")
    parts = json.loads(_native._decode_round_body_json(_dumps(body)))
    return tuple(ContextPart(role=p["role"], text=p["text"]) for p in parts)


def context_item_from_hit(
    hit: Any,
    *,
    group: str | None = None,
    position: int | None = None,
    order_key: str | None = None,
    header: str | None = None,
) -> ContextItem:
    """Build a ``ContextItem`` from a ``SearchHit``.

    The id is the hit's prefixed id (``l:``, ``h:`` or ``p:`` plus the bare
    value) and the parts are ``decode_round_body(hit.body)``.

    Raises:
        InvalidArgumentError: the hit body starts with U+001E but does not
            decode.
        ValueError: the hit id space is not ``logical``, ``content`` or
            ``passage``.
    """
    space = hit.id.space
    if space not in _ID_PREFIX:
        raise ValueError(f"unknown hit id space {space!r}")
    return ContextItem(
        id=f"{_ID_PREFIX[space]}{hit.id.value}",
        parts=decode_round_body(hit.body),
        group=group,
        position=position,
        order_key=order_key,
        header=header,
    )
