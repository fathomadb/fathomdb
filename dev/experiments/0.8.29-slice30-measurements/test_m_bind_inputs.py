"""Exercise the real metadata binder with tiny owned files; never execute native/model work."""

from contextlib import ExitStack
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch

from g_freeze import identify
import m_freeze
from m_calibration import CELL_ID, registered_cell, selected_cell
from test_m_calibration import LIMITS


class InputBindingTests(unittest.TestCase):
    def setUp(self):
        self.stack = ExitStack()
        self.addCleanup(self.stack.close)
        self.root = Path(self.stack.enter_context(tempfile.TemporaryDirectory()))
        source = self.root / "source"
        state = source / "dev/plans/release-state-0.8.29.json"
        state.parent.mkdir(parents=True)
        main = self.root / "closed.sqlite"
        connection = sqlite3.connect(main)
        try:
            connection.execute("CREATE TABLE literal (body TEXT)")
            connection.execute("INSERT INTO literal VALUES ('alpha')")
            connection.commit()
        finally:
            connection.close()
        state.write_text(json.dumps({"nested": {"g_selected_input_for_m": {
            "source_sha": "original-g-source", "campaign_uuid": "original-g-campaign", "store": str(main)}}}))
        custody = {"owner_released_closed": True, "source_sha": "original-g-source",
            "campaign_uuid": "original-g-campaign", "main": str(main), "schema_version": 37,
            "readiness": "Ready", "pending": 0,
            "counts": {"natural": 100000, "vectors": 100000, "physical": 100000, "spans": 100000},
            "stored_coverage": {"full": 83644, "partial": 16356, "unknown": 0}, "package": [str(main)]}
        inputs = {}
        for key, content in (("corpus", 'literal corpus\n'), ("queries", '"alpha"\n' * 500), ("license", 'literal license\n')):
            path = self.root / key
            path.write_text(content)
            inputs[key] = path
        hf = self.root / "hf"
        cache = hf / "hub/models--BAAI--bge-small-en-v1.5/snapshots/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a"
        cache.mkdir(parents=True)
        config = cache / "config.json"
        config.write_text('{}')
        builds = {}
        for variant, features in m_freeze.VARIANTS.items():
            binary = self.root / (variant + "-unexecuted-native")
            binary.write_text("literal native metadata fixture " + variant)
            receipt = self.root / (variant + ".json")
            receipt.write_text(json.dumps({"schema": "slice30-m-build/v1", "variant": variant,
                "features": features, "compiler_inputs": "literal-compiler",
                "source_sha": "original-native-source", "binary": identify(binary)}))
            builds[variant] = str(receipt)
        self.options = dict({key: str(path) for key, path in inputs.items()},
            allocation=str(self.root), closed_subject=custody, model_cache=str(cache), builds=builds,
            gpu={"ordinal": 0, "uuid": "literal-device", "driver_version": "580.178.04",
                 "primary_cuda": "/usr/local/cuda-12.6"}, calibration=LIMITS, cell_allowance=LIMITS)
        self.profile = {"other_work_cost_status": "HELD_UNAVAILABLE"}
        self.stack.enter_context(patch.object(m_freeze, "SOURCE", source))
        self.stack.enter_context(patch.object(m_freeze, "PINS", {key: identify(path)["sha256"] for key, path in inputs.items()}))
        self.stack.enter_context(patch.object(m_freeze, "MODEL_FILES", {"config.json": (2, identify(config)["sha256"])}))
        self.stack.enter_context(patch.object(m_freeze, "compiler_inputs", return_value="literal-compiler"))
        self.stack.enter_context(patch.object(m_freeze, "git", side_effect=lambda *args: "" if args[0] == "status" else "actual-tools-source"))
        self.stack.enter_context(patch.dict(os.environ, S30_TASK=str(self.root), HF_HOME=str(hf)))

    def bind(self, name, **selection):
        try:
            value = m_freeze._bind_inputs(self.options, self.root / name, self.profile, {}, **selection)
        except KeyError as error:
            value = {"binder_error": str(error)}
        self.assertNotIn("binder_error", value, "G provenance displaced canonical cell selection")
        return value

    def test_selected_required_cell_survives_real_input_binding(self):
        cell = registered_cell("M-R-1-1-debug")
        value = self.bind("selected", selected=cell)
        self.assertEqual(value["scope"], "registered-M-selected-cell")
        self.assertEqual(value["plan"], [cell])
        self.assertEqual(set(value["builds"]), {"ordinary"})
        self.assertEqual(value["closed_subject"]["source_sha"], "original-g-source")
        self.assertEqual(value["builds"]["ordinary"]["source_sha"], "original-native-source")

    def test_calibration_scope_and_original_id_survive_real_input_binding(self):
        cell = selected_cell(CELL_ID)
        value = self.bind("calibration", calibration=cell)
        self.assertEqual(value["scope"], "bounded-natural-calibration")
        self.assertEqual(value["plan"], [cell])
        self.assertFalse(value["qualifies_primary"])
        self.assertEqual(set(value["builds"]), {"ordinary"})

    def test_unselected_primary_binding_preserves_all46_configurations(self):
        value = self.bind("ordinary")
        self.assertNotIn("scope", value)
        self.assertEqual(len(value["plan"]), 46)
        self.assertEqual(set(value["builds"]), {"ordinary", "providerfree", "helper"})


if __name__ == "__main__":
    unittest.main()
