"""Recover the original seven raw case records without rerunning controls."""
import hashlib
import json
import os
from pathlib import Path


def main():
    task = Path(os.environ['S20_TASK'])
    raw = task/'logs/approved-sensitivity.log'
    monitor_path = task/'receipts/approved-sensitivity.json'
    output = task/'receipts/approved-sensitivity-cases.json'
    assert not output.exists(), 'refuse to overwrite retained case results'
    text = raw.read_text()
    monitor = json.loads(monitor_path.read_text())
    assert monitor['exit'] == 0 and monitor['resource_stop'] is None
    assert monitor['sha_start'] == monitor['sha_end'] and not monitor['delta_start'] and not monitor['delta_end']
    cases = [json.loads(line) for line in text.splitlines() if line.startswith('{"case":')]
    assert len(cases) == 7 and 'PASS 7 serialized sensitivity controls; exact source restored' in text
    attribution = []
    for case in cases:
        log = Path(case['log']).read_text()
        artifacts = [json.loads(line) for line in log.splitlines() if line.startswith('{"reason":"compiler-artifact"')]
        tests = [item for item in artifacts if item['profile']['test'] and item.get('executable')]
        assert len(tests) == 1, 'actual test artifact must be unambiguous'
        binary = tests[0]['executable']
        running = [line for line in log.splitlines() if 'Running ' in line and f'({binary})' in line]
        assert len(running) == 1 and 'running 1 test' in log
        assert 'test result: FAILED. 0 passed; 1 failed' in log
        original = next(item for item in case['executables'] if item['path'] == binary)
        attribution.append({'case': case['case'], 'executed_test': original,
                            'running_line': running[0], 'profile_test': True,
                            'built_artifact_field': 'executables; includes unexecuted server where emitted'})
    payload = {'cases': cases, 'control_sha': 'ef1f8d137ef647f172e81f400ad3e5caddef0192',
               'clean_after': True, 'product_red_claim': False,
               'recovered_from': str(raw), 'recovered_from_sha256': hashlib.sha256(raw.read_bytes()).hexdigest(),
               'monitor_receipt': str(monitor_path), 'runtime_driver_sha': monitor['sha_start'],
               'runtime_attribution': attribution,
               'recovery_note': 'Original case records unchanged; monitor receipt untouched; no control rerun.'}
    output.write_text(json.dumps(payload, indent=2)+'\n')
    print(output)


if __name__ == '__main__':
    main()
