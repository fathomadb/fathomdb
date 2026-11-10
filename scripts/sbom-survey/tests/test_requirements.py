"""Human-specified hash-bearing requirement declarations."""
from conftest import require


def test_inline_sha256_hashes_preserve_the_exact_requirement():
    parser = require('sbom_survey.parse.python', 'inline-hashes', 'declarative hash support')
    text = 'certifi==2026.7.22 --hash=sha256:' + 'a' * 64 + ' --hash=sha256:' + 'B' * 64 + '\n'
    pairs = parser.parse_requirements('export-requirements.txt', text)
    assert [(d.name, d.constraint, pin) for d, pin in pairs] == [('certifi', '==2026.7.22', '2026.7.22')]
    assert parser.parse_requirements('export-requirements.txt', '--extra-index-url https://download.pytorch.org/whl/cpu\n# comment\n') == []
