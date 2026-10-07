"""Check the S02-L process-memory counters and close-state guard."""

from copy import deepcopy
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_close_memory as close_memory  # noqa: E402


ROLLUP = """Rss: 200000 kB
Pss: 150000 kB
Private_Clean: 20000 kB
Private_Dirty: 100000 kB
"""


def test_parse_rollup_requires_all_memory_fields() -> None:
    assert close_memory.parse_smaps_rollup(ROLLUP) == {
        "rss_kib": 200000,
        "pss_kib": 150000,
        "private_kib": 120000,
    }
    with pytest.raises(ValueError, match="missing smaps_rollup"):
        close_memory.parse_smaps_rollup(ROLLUP.replace("Pss: 150000 kB\n", ""))


def test_close_observation_rejects_unreleased_or_invalid_state() -> None:
    observed = {
        "semantic_ok": True,
        "vector_dimension": 384,
        "vector_nonzero": True,
        "close_ns": 1000,
        "before": {"rss_kib": 1000, "pss_kib": 800, "private_kib": 700},
        "opened": {"rss_kib": 1500, "pss_kib": 1300, "private_kib": 1200},
        "closed_idle": {"rss_kib": 1200, "pss_kib": 1000, "private_kib": 900},
        "dropped": {"rss_kib": 1100, "pss_kib": 900, "private_kib": 800},
    }
    close_memory.validate_observation(observed)
    altered = deepcopy(observed)
    altered["semantic_ok"] = False
    with pytest.raises(ValueError, match="semantic"):
        close_memory.validate_observation(altered)
    altered = deepcopy(observed)
    altered["close_ns"] = 0
    with pytest.raises(ValueError, match="close duration"):
        close_memory.validate_observation(altered)
