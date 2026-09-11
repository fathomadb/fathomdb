"""Admin namespace exposing schema configuration and startup runtime control.

Per `dev/interfaces/python.md` § Runtime surface, `admin.configure` is the
fifth canonical SDK verb. The native binding (`fathomdb._fathomdb`)
performs the writer-thread wiring; this module exposes the typed
Python signature and converts the native receipt to the public
`WriteReceipt` dataclass. `configure_runtime` selects SQLite behavior before
the first Engine open and does not submit an application command.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Literal

from fathomdb._fathomdb import admin_configure as _native_configure
from fathomdb._fathomdb import admin_configure_runtime as _native_configure_runtime
from fathomdb._fathomdb import RuntimeConfiguration
from fathomdb.types import WriteReceipt

if TYPE_CHECKING:
    from fathomdb.engine import Engine


def configure(engine: "Engine", *, name: str, body: str) -> WriteReceipt:
    """Submit an admin schema configuration."""

    if not name:
        raise ValueError("admin.configure requires a non-empty name")
    if body is None:
        raise ValueError("admin.configure requires a body")
    receipt = _native_configure(engine._native, name, body)
    return WriteReceipt(
        cursor=receipt.cursor,
        row_cursors=tuple(receipt.row_cursors),
        dangling_edge_endpoints=receipt.dangling_edge_endpoints,
    )


def configure_runtime(
    *, sqlite_mode: Literal["performance", "diagnostics"]
) -> RuntimeConfiguration:
    """Select SQLite's process-wide startup mode before opening an Engine."""

    if sqlite_mode not in {"performance", "diagnostics"}:
        raise ValueError("sqlite_mode must be 'performance' or 'diagnostics'")
    return _native_configure_runtime(sqlite_mode)


__all__ = ["configure", "configure_runtime"]
