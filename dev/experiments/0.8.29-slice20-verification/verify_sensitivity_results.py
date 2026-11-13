"""Check that monitor completion preserves all seven original control results."""
import hashlib
import json
import os
from pathlib import Path


def main():
    task = Path(os.environ['S20_TASK'])
    monitor_path = task/'receipts/approved-sensitivity.json'
    case_path = task/'receipts/approved-sensitivity-cases.json'
    monitor = json.loads(monitor_path.read_text())
    assert monitor['phase'] == 'approved-sensitivity'
    assert monitor['exit'] == 0 and monitor['resource_stop'] is None
    payload = json.loads(case_path.read_text()) if case_path.exists() else {'cases': []}
    cases = payload['cases']
    expected = {'stale-typed-error', 'stale-bypass', 'RRF_K', 'RRF_WEIGHT_TEXT',
                'RRF_WEIGHT_VECTOR', 'DEFAULT_EMBEDDER_NAME', 'MAX_SEQUENCE_TOKENS'}
    assert len(cases) == 7, 'case results lost after monitor completion: expected seven original cases'
    assert {case['case'] for case in cases} == expected
    assert payload['control_sha'] == 'ef1f8d137ef647f172e81f400ad3e5caddef0192'
    assert payload['clean_after'] is True and payload['product_red_claim'] is False
    for case in cases:
        assert case['exit'] == 101
        assert case['source_sha'] == payload['control_sha']
        assert case['classification'] == 'EXPECTED_SENSITIVITY_FAILURE'
        assert case['original_sha256'] != case['mutation_sha256']
        assert case['executables'] and '--exact' in case['argv']
        for executable in case['executables']:
            assert len(executable['sha256']) == 64
        raw = Path(case['log']).read_text()
        assert 'test result: FAILED. 0 passed; 1 failed' in raw
    if 'recovered_from' in payload:
        origin = Path(payload['recovered_from'])
        assert hashlib.sha256(origin.read_bytes()).hexdigest() == payload['recovered_from_sha256']
        records = [json.loads(line) for line in origin.read_text().splitlines() if line.startswith('{"case":')]
        assert records == cases, 'recovery must preserve the original runtime records exactly'
    print('PASS: distinct monitor and case outputs retain all seven original runtime failures')


if __name__ == '__main__':
    main()
