"""Human-specified hash-bearing requirement declarations."""
import subprocess

import pytest

from conftest import require, run_cli


def test_inline_sha256_hashes_preserve_the_exact_requirement():
    parser = require('sbom_survey.parse.python', 'inline-hashes', 'declarative hash support')
    text = 'certifi==2026.7.22 --hash=sha256:' + 'a' * 64 + ' --hash=sha256:' + 'B' * 64 + '\n'
    pairs = parser.parse_requirements('export-requirements.txt', text)
    assert [(d.name, d.constraint, pin) for d, pin in pairs] == [('certifi', '==2026.7.22', '2026.7.22')]
    assert parser.parse_requirements('export-requirements.txt', '--extra-index-url https://download.pytorch.org/whl/cpu\n# comment\n') == []


@pytest.mark.parametrize('text', [
    'certifi==2026.7.22 --hash=sha256:abc',
    'certifi==2026.7.22 --hash=sha256:' + 'z' * 64,
    'certifi==2026.7.22 --hash=sha512:' + 'a' * 64,
    'certifi==2026.7.22 --hash',
    '--hash=sha256:' + 'a' * 64,
    'certifi==2026.7.22 --hash=sha256:' + 'a' * 64 + ' --hash=bogus',
    'certifi==2026.7.22 \\\n --hash=sha256:' + 'a' * 64,
])
def test_malformed_or_unsupported_hash_syntax_is_named(text):
    parser = require('sbom_survey.parse.python', 'malformed-hashes', 'named parse error')
    errors = require('sbom_survey.parse', 'malformed-hashes', 'ManifestParseError')
    with pytest.raises(errors.ManifestParseError, match='export-requirements'):
        parser.parse_requirements('export-requirements.txt', text)


def test_malformed_hash_cli_exits_three_naming_manifest(tmp_path):
    (tmp_path / 'export-requirements.txt').write_text('certifi==2026.7.22 --hash=sha256:bad\n')
    subprocess.run(['git', '-C', str(tmp_path), 'init', '-q'], check=True)
    subprocess.run(['git', '-C', str(tmp_path), 'add', 'export-requirements.txt'], check=True)
    tier_file = tmp_path / 'tiers.toml'
    tier_file.write_text('schema=1\n[[rule]]\nprefix="export-requirements.txt"\naction="tier"\ntier="dev-tooling"\n')
    result = run_cli(['--repo', str(tmp_path), '--offline', '--tiers', str(tier_file), '--out', str(tmp_path / 'out')], 'malformed-hashes', 'exit3 for malformed exporter hash')
    assert result.returncode == 3, result.stderr
    assert 'export-requirements.txt' in result.stderr
