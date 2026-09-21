#!/usr/bin/env python3
"""Thin gauntlet adapter for GRAPH-EXPAND-01."""

from experiments import graph_expand_01

from graph_cell_adapter import run


if __name__ == "__main__":
    raise SystemExit(run(graph_expand_01))
