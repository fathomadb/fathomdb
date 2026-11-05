"""Current EARP emissions omit refused-open state; historical data stays readable."""

import json
from pathlib import Path


def test_report_schema_accepts_current_and_historical_observations() -> None:
    """Only the observed query backend is required after failed-open admission."""
    schema_path = Path(__file__).parents[2] / "eval/earp/schema/earp.result.v1.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    report = schema["properties"]["scenario"]["properties"]["projection_witnesses"]["properties"]["open_report"]
    assert report["required"] == ["query_backend"]
    assert {"dense_disabled", "dense_disabled_reason", "refusal_count"} <= report["properties"].keys()
