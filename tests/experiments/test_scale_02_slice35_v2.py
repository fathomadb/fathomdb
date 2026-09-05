from __future__ import annotations

from experiments import scale_02_slice35
from tests.experiments.test_scale_02_slice35 import config


def test_v2_requires_reverse_campaign_and_the_preserved_v1_receipt() -> None:
    document = config()
    document["schema_version"] = "scale-02-slice35.v2"
    document["campaign_order"] = ["candidate", "baseline"]
    document["prior_receipt"] = {
        "path": "experiments/runs/prior/record.json",
        "sha256": "4" * 64,
        "verdict": "advisory_limit_observed",
    }

    resolved = scale_02_slice35.resolve_config(document, validate_files=False)

    assert resolved["campaign_order"] == ["candidate", "baseline"]


def test_independent_bootstrap_uses_both_balanced_campaigns() -> None:
    upper = scale_02_slice35.independent_relative_upper(
        [10.0] * 10,
        [10.1] * 10,
        seed=17,
        resamples=2_000,
    )
    assert 0.009 < upper < 0.011
