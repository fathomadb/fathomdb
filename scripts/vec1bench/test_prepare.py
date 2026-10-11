"""Contract tests for the opt-in vec1bench corpus preparation."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path
from types import SimpleNamespace


MODULE_PATH = Path(__file__).with_name("prepare.py")
SPEC = importlib.util.spec_from_file_location("vec1bench_prepare", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
prepare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(prepare)


class CorpusContractTests(unittest.TestCase):
    def test_registry_covers_all_four_vec1bench_datasets_with_pinned_hashes(self) -> None:
        specs = prepare.load_specs()
        self.assertEqual(
            set(specs),
            {
                "sift-128-euclidean",
                "imagenet-clip-512-normalized",
                "landmark-dino-768-cosine",
                "agnews-mxbai-1024-euclidean",
            },
        )
        for spec in specs.values():
            self.assertRegex(spec["sha256"], r"^[0-9a-f]{64}$")
            self.assertGreater(spec["train_rows"], 700_000)
            self.assertGreater(spec["dimension"], 0)

    def test_layout_rejects_wrong_metric_and_incomplete_gold(self) -> None:
        spec = prepare.load_specs()["landmark-dino-768-cosine"]
        datasets = {
            "train": SimpleNamespace(shape=(spec["train_rows"], 768), dtype=SimpleNamespace(kind="f")),
            "test": SimpleNamespace(shape=(100, 768), dtype=SimpleNamespace(kind="f")),
            "neighbors": SimpleNamespace(shape=(100, 100), dtype=SimpleNamespace(kind="i")),
            "distances": SimpleNamespace(shape=(100, 100), dtype=SimpleNamespace(kind="f")),
        }
        source = SimpleNamespace(
            attrs={"dimension": 768, "distance": "cosine", "point_type": "float"},
            __getitem__=datasets.__getitem__,
        )
        # A small mapping proxy supports both HDF5-style keys and attributes.
        class FileProxy:
            attrs = source.attrs

            def __getitem__(self, key: str) -> object:
                return datasets[key]

        prepare.validate_layout(FileProxy(), spec)
        source.attrs["distance"] = "euclidean"
        with self.assertRaisesRegex(ValueError, "distance"):
            prepare.validate_layout(FileProxy(), spec)
        source.attrs["distance"] = "cosine"
        datasets["neighbors"] = SimpleNamespace(shape=(100, 9), dtype=SimpleNamespace(kind="i"))
        with self.assertRaisesRegex(ValueError, "neighbors"):
            prepare.validate_layout(FileProxy(), spec)

    def test_query_sample_is_deterministic_and_spans_full_query_set(self) -> None:
        indices = prepare.query_indices(10_000, 100)
        self.assertEqual(len(indices), 100)
        self.assertEqual(len(set(indices)), 100)
        self.assertEqual(indices[0], 0)
        self.assertEqual(indices[-1], 9_999)
        with self.assertRaises(ValueError):
            prepare.query_indices(9, 10)

    def test_unit_l2_preserves_cosine_order_without_zero_vectors(self) -> None:
        self.assertEqual(prepare.unit_l2([3.0, 4.0]), [0.6, 0.8])
        with self.assertRaisesRegex(ValueError, "zero"):
            prepare.unit_l2([0.0, 0.0])


if __name__ == "__main__":
    unittest.main()
