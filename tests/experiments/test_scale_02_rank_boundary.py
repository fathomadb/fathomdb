"""Contract tests for the SCALE-02 rank-boundary off-shoot."""

from __future__ import annotations

from experiments import scale_02_rank_boundary as followup


def test_boundary_summary_requires_exact_routes_and_complete_ties():
    routes = [
        {
            "schema_version": "scale-02-fts-route.v1",
            "route": "rank_stream_strict_boundary",
        },
        {
            "schema_version": "scale-02-fts-route.v1",
            "route": "rank_stream_tie_completed",
        },
    ]
    boundaries = [
        {
            "schema_version": "scale-02-fts-boundary.v1",
            "route": "rank_stream_strict_boundary",
            "candidate_limit": 100,
            "rows_consumed": 101,
            "boundary_group_size": 1,
        },
        {
            "schema_version": "scale-02-fts-boundary.v1",
            "route": "rank_stream_tie_completed",
            "candidate_limit": 100,
            "rows_consumed": 108,
            "boundary_group_size": 9,
        },
    ]

    summary = followup.summarize_boundary_witnesses(
        routes, boundaries, expected_queries=2
    )

    assert summary["full_sort_fallbacks"] == 0
    assert summary["route_counts"] == {
        "rank_stream_strict_boundary": 1,
        "rank_stream_tie_completed": 1,
    }
    assert summary["rows_consumed"]["max"] == 108
    assert summary["boundary_group_size"]["max"] == 9


def test_selection_prefers_default_reader_only_when_all_points_pass():
    cells = []
    for reader in ("default", "mmap128"):
        for point in (25000, 40000, 50000):
            cells.append(
                {
                    "boundary_handling": "stream_complete_boundary_tie",
                    "reader_profile": reader,
                    "point": point,
                    "decision": {"eligible": reader == "mmap128", "reasons": []},
                }
            )

    selection = followup.select_candidate(cells)

    assert selection == {
        "state": "result_pending_hitl",
        "recommended_cell": "stream_mmap128",
        "eligible_through": {"stream_default": None, "stream_mmap128": 50000},
    }


def test_cell_decision_requires_performance_equivalence_and_no_fallback():
    cell = {
        "point": 50000,
        "complete_repetitions": True,
        "errors": 0,
        "timeouts": 0,
        "steady": {"p50": 45.0, "p99": 140.0},
        "upper_95": {"rss_fraction": 0.02},
        "equivalence": {
            "query_count": 100,
            "top100_mismatch_count": 0,
            "top10_mismatch_count": 0,
            "full_sort_fallbacks": 0,
        },
    }
    policy = {
        "steady_p50_ms_by_point": {"50000": 50},
        "steady_p99_ms": 150,
        "max_rss_fraction": 0.8,
        "max_errors": 0,
        "max_timeouts": 0,
    }

    assert followup.evaluate_stream_cell(cell, policy) == {
        "eligible": True,
        "reasons": [],
    }

    cell["equivalence"]["full_sort_fallbacks"] = 1
    assert followup.evaluate_stream_cell(cell, policy)["reasons"] == [
        "full_sort_fallback"
    ]


def test_connection_witness_requires_wal_and_normal_writer():
    rows = [
        {
            "schema_version": "scale-02-connection-settings.v1",
            "role": "writer",
            "journal_mode": "wal",
            "synchronous": 1,
            "sqlite_version": "3.53.2",
        },
        {
            "schema_version": "scale-02-connection-settings.v1",
            "role": "reader",
            "journal_mode": "wal",
            "synchronous": 2,
            "sqlite_version": "3.53.2",
        },
    ]

    summary = followup.validate_connection_witnesses(rows)

    assert summary == {
        "connection_count": 2,
        "role_counts": {"reader": 1, "writer": 1},
        "reader_synchronous_values": [2],
        "sqlite_version": "3.53.2",
    }


def test_query_plan_witness_rejects_a_temporary_order_by_sort():
    row = {
        "schema_version": "scale-02-fts-query-plan.v1",
        "statement": "stream_complete_boundary_tie",
        "uses_temp_btree_for_order_by": False,
    }

    assert followup.validate_query_plan_witness([row]) == row

    row["uses_temp_btree_for_order_by"] = True
    try:
        followup.validate_query_plan_witness([row])
    except followup.Scale02RankBoundaryError as exc:
        assert str(exc) == "stream statement requires a temporary ORDER BY sort"
    else:  # pragma: no cover
        raise AssertionError("temporary sort was accepted")
