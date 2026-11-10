"""Explicit exporter expectations independent of the mirrored name table."""
from pathlib import Path

from conftest import require


def test_tracked_exporter_is_discovered_but_untracked_text_is_not(tmp_path: Path):
    discovery = require('sbom_survey.discovery', 'exporter-discovery', 'tracked exact exporter basename')
    (tmp_path / 'export-requirements.txt').write_text('certifi==2026.7.22\n')
    refs = discovery.discover_manifests(tmp_path, ls_files=lambda _: ['dev/tools/onnx/export-requirements.txt', 'notes.txt'])
    assert [(r.path, r.ecosystem, r.kind) for r in refs] == [('dev/tools/onnx/export-requirements.txt', 'pypi', 'manifest')]
    assert discovery.discover_manifests(tmp_path, ls_files=lambda _: []) == []
