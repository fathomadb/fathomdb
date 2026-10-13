"""Attestation checks for the standalone driver build helper."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest


SOURCE = Path(__file__).resolve().parents[1] / "build_manifest.py"
SPEC = importlib.util.spec_from_file_location("rust_driver_build_manifest", SOURCE)
assert SPEC is not None and SPEC.loader is not None
build_manifest = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_manifest)


def metadata() -> dict:
    registry = "registry+https://github.com/rust-lang/crates.io-index"
    return {
        "packages": [
            {"id": "driver", "name": "driver", "version": "0.1.0", "source": None},
            {"id": "registry", "name": "serde", "version": "1.0.0", "source": registry},
        ],
        "resolve": {
            "root": "driver",
            "nodes": [
                {"id": "driver", "features": [], "deps": [{"pkg": "registry"}]},
                {"id": "registry", "features": ["std"], "deps": []},
            ],
        },
    }


def lockfile(checksum: str | None = "a" * 64) -> dict:
    return {
        "package": [
            {"name": "driver", "version": "0.1.0"},
            {
                "name": "serde",
                "version": "1.0.0",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                **({"checksum": checksum} if checksum is not None else {}),
            },
        ]
    }


class BuildManifestTests(unittest.TestCase):
    def test_registry_checksum_comes_from_lockfile(self) -> None:
        closure = build_manifest.resolved_closure(metadata(), lockfile())
        serde = next(item for item in closure if item["name"] == "serde")
        self.assertEqual(serde["checksum"], "a" * 64)

    def test_missing_registry_checksum_refuses_attestation(self) -> None:
        with self.assertRaisesRegex(ValueError, "registry checksum"):
            build_manifest.resolved_closure(metadata(), lockfile(None))

    def test_development_profile_matches_binary_runtime(self) -> None:
        self.assertEqual(build_manifest.cargo_profile_name("dev"), "debug")
        self.assertEqual(build_manifest.cargo_profile_name("release"), "release")


if __name__ == "__main__":
    unittest.main()
