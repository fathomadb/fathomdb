"""Explicit exporter expectations independent of the mirrored name table."""
from pathlib import Path

from conftest import require


def test_tracked_exporter_is_discovered_but_untracked_text_is_not(tmp_path: Path):
    discovery = require('sbom_survey.discovery', 'exporter-discovery', 'tracked exact exporter basename')
    (tmp_path / 'export-requirements.txt').write_text('certifi==2026.7.22\n')
    refs = discovery.discover_manifests(tmp_path, ls_files=lambda _: ['dev/tools/onnx/export-requirements.txt', 'notes.txt'])
    assert [(r.path, r.ecosystem, r.kind) for r in refs] == [('dev/tools/onnx/export-requirements.txt', 'pypi', 'manifest')]
    assert discovery.discover_manifests(tmp_path, ls_files=lambda _: []) == []


def test_real_exporter_dispatch_preserves_all_28_pins(tmp_path: Path, monkeypatch):
    from conftest import REPO_ROOT
    from packaging.utils import canonicalize_name
    survey = require('sbom_survey.survey', 'exporter-dispatch', 'exporter declarations at survey boundary')
    discovery = require('sbom_survey.discovery', 'exporter-dispatch', 'manifest reference')
    tiers = require('sbom_survey.tiers', 'exporter-dispatch', 'explicit dev-tooling tier')
    registry = require('sbom_survey.registry', 'exporter-dispatch', 'offline registry')
    path = 'dev/tools/onnx/export-requirements.txt'
    destination = tmp_path / path
    destination.parent.mkdir(parents=True)
    destination.write_bytes((REPO_ROOT / path).read_bytes())
    monkeypatch.setattr(survey, 'discover_manifests', lambda _: [discovery.ManifestRef(path, 'pypi', 'manifest')])
    rules = tiers.TierMap([tiers.TierRule(prefix='dev/tools/', action='tier', tier='dev-tooling')])
    result = survey.run_survey(tmp_path, published=registry.OfflineSource(), tier_map=rules)
    expected = dict(pair.split('==') for pair in '''certifi==2026.7.22
charset-normalizer==3.5.1
filelock==3.32.3
fsspec==2026.7.0
hf-xet==1.6.0
huggingface-hub==0.36.2
idna==3.20
jinja2==3.1.6
markupsafe==3.0.3
ml-dtypes==0.5.4
mpmath==1.3.0
networkx==3.6.1
numpy==1.26.4
onnx==1.23.0
packaging==26.3
protobuf==7.36.2
pyyaml==6.0.3
regex==2026.9.10
requests==2.34.2
safetensors==0.8.0
setuptools==78.1.0
sympy==1.14.0
tokenizers==0.19.1
torch==2.4.1+cpu
tqdm==4.70.1
transformers==4.44.2
typing-extensions==4.16.0
urllib3==2.8.0'''.splitlines())
    assert {canonicalize_name(c.name): c.version for c in result.components} == expected
    assert all(c.tier == 'dev-tooling' and c.depth == 'direct' and [o.path for o in c.origins] == [path] for c in result.components)


def test_preparation_tiers_and_exclusions_are_explicit(tmp_path: Path, monkeypatch):
    from conftest import PROJECT_ROOT
    tiers = require('sbom_survey.tiers', 'preparation-tiers', 'explicit purpose tiers')
    rules = tiers.load_tier_map(PROJECT_ROOT / 'tiers.toml')
    expected = {
        'dev/experiments/current-history-search/Cargo.toml': ('tier', 'eval-only'),
        'experiments/sdk_drivers/rust/Cargo.toml': ('tier', 'eval-only'),
        'experiments/comp_drivers/surrealdb/Cargo.toml': ('tier', 'eval-only'),
        'src/rust/crates/fathomdb-tc5-benchmark/Cargo.toml': ('tier', 'eval-only'),
        'src/rust/crates/fathomdb-mcp/Cargo.toml': ('tier', 'shipped'),
    }
    for path, wanted in expected.items():
        result = rules.classify(path)
        assert (result.action, result.tier) == wanted
    survey = require('sbom_survey.survey', 'preparation-tiers', 'exclusion before parse and lookup')
    discovery = require('sbom_survey.discovery', 'preparation-tiers', 'manifest references')
    paths = [('scripts/tests/fixtures/example/Cargo.toml', 'fixture'), ('dev/plans/0.8.27/features/slice-135/results/example/Cargo.lock', 'historical-evidence')]
    refs = [discovery.ManifestRef(path, 'cargo', 'lockfile' if path.endswith('.lock') else 'manifest') for path, _ in paths]
    monkeypatch.setattr(survey, 'discover_manifests', lambda _: refs)
    def forbidden(*args):
        raise AssertionError('excluded input reached parser or registry')
    monkeypatch.setattr(survey, '_read', forbidden)
    class NoLookup:
        latest = forbidden
    result = survey.run_survey(tmp_path, published=NoLookup(), tier_map=rules)
    assert [(e.path, e.reason) for e in result.excluded] == sorted(paths)
    assert result.components == []
