#!/usr/bin/env python3
"""Portable Slice 130 instrumented-soak measurement helpers (not a full E verdict)."""

from __future__ import annotations


def evaluate_arm(arm: dict) -> dict:
    """Evaluate a completed instrumented arm; incomplete implementation stays unmeasured."""
    return {"status": "UNMEASURED", "reasons": ["evaluation not implemented"]}


def evaluate_pair(on: dict, off: dict) -> dict:
    """Evaluate one SDK's ordered instrumented pair, without claiming shipping proof."""
    return {"status": "UNMEASURED", "reasons": ["evaluation not implemented"]}
