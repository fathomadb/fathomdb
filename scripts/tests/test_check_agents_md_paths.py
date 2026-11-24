#!/usr/bin/env python3
"""Tests for scripts/check-agents-md-paths.py (AGENTS.md § 12 enforcement)."""

import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class CheckAgentsMdPathsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location(
            "check_agents_md_paths", ROOT / "scripts/check-agents-md-paths.py"
        )
        cls.mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.mod)

    def test_valid_paths_and_brace_expansion_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "dev/interfaces").mkdir(parents=True)
            (root / "dev/interfaces/rust.md").write_text("# rust\n")
            (root / "dev/interfaces/python.md").write_text("# py\n")
            (root / "scripts").mkdir()
            (root / "scripts/foo.sh").write_text("#!/bin/sh\n")
            (root / "AGENTS.md").write_text(
                "# AGENTS\n"
                "- See `dev/interfaces/{rust,python}.md`.\n"
                "- Run `./scripts/foo.sh --check`.\n"
                "- Placeholder `dev/plans/runs/STATUS-<version>.md` is ignored.\n"
                "- Ignored gitignored `.claude/settings.json`.\n"
            )
            self.assertEqual(self.mod.check_root(root), [])

    def test_missing_path_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "AGENTS.md").write_text(
                "# AGENTS\n- Broken citation `dev/agent-memory/does-not-exist.md`.\n"
            )
            errs = self.mod.check_root(root)
            self.assertEqual(len(errs), 1)
            self.assertIn("dev/agent-memory/does-not-exist.md", errs[0])

    def test_over_300_lines_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "AGENTS.md").write_text("\n".join(["line"] * 301) + "\n")
            errs = self.mod.check_root(root)
            self.assertTrue(any("limit is 300" in e for e in errs))


if __name__ == "__main__":
    unittest.main()
