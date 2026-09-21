#!/usr/bin/env python3
"""Thin gauntlet adapter for GRAPH-EVIDENCE-01."""

from experiments import graph_evidence_01

from graph_cell_adapter import run


if __name__ == "__main__":
    raise SystemExit(run(graph_evidence_01))
