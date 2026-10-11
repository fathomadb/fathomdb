import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time
from datetime import datetime, timezone

root = Path('/home/coreyt/projects/fathomdb-worktrees/release-0.8.27-slice-135')
protocol = root / 'dev/plans/0.8.27/features/slice-135/s01-python-comparison-protocol.json'
expected_protocol_sha = '899b357a43b487d63c633fe1bc06086212293e81a331fd2a8ae5bd03412f2064'
runner = root / 'scripts/slice135_python_s01_block.py'
wheel_name = 'fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl'
artifacts = {
    'baseline': {
        'checkout': '/home/coreyt/projects/fathomdb-worktrees/release-0.8.26-slice-135-baseline',
        'source': 'f99e002f0d2e4002f3694c9f8d4986b56089edaa',
        'wheel': '/tmp/slice135-python-wheel-baseline-f99e002/' + wheel_name,
        'wheel_sha': '7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282',
        'venv': '/tmp/slice135-python-venv-baseline-f99e002/bin/python',
    },
    'candidate': {
        'checkout': '/tmp/slice135-candidate-source-b2ac808',
        'source': 'b2ac8081e79a6e626d01f5f97331be331f1cc16d',
        'wheel': '/tmp/slice135-python-wheel-candidate-b2ac808/' + wheel_name,
        'wheel_sha': '6713ade54d62cc1e067fcaf1982a539c1fb8bd3d45d3db18e41155badd0c12e8',
        'venv': '/tmp/slice135-python-venv-candidate-b2ac808/bin/python',
    },
}
run_order = Path('/tmp/slice135-s01-paired-v2-run-order.jsonl')
if run_order.exists():
    raise SystemExit('run-order file already exists; refuse implicit resume')
if hashlib.sha256(protocol.read_bytes()).hexdigest() != expected_protocol_sha:
    raise SystemExit('frozen protocol hash changed')
spec = json.loads(protocol.read_text())
if spec['status'] != 'FROZEN_S01_PYTHON_PAIRED' or spec['warm_samples_per_cell'] != 1000:
    raise SystemExit('protocol status or sample count differs')
sequence = []
for size in (32, 256):
    first_pair = 2 if size == 32 else 1
    for pair in range(first_pair, 6):
        for role in spec['pair_order_each_size'][pair - 1]:
            sequence.append((size, pair, role))
for size, pair, role in sequence:
    time.sleep(spec['minimum_idle_seconds_between_blocks'])
    if hashlib.sha256(protocol.read_bytes()).hexdigest() != expected_protocol_sha:
        raise SystemExit('frozen protocol changed during campaign')
    artifact = artifacts[role]
    output = Path(f'/tmp/slice135-s01-paired-v2-{size}-{pair:02}-{role}')
    if output.exists():
        raise SystemExit(f'output already exists: {output}')
    command = [
        sys.executable, str(runner), '--checkout', artifact['checkout'],
        '--source-sha', artifact['source'], '--wheel', artifact['wheel'],
        '--wheel-sha256', artifact['wheel_sha'], '--venv-python', artifact['venv'],
        '--rows', str(size), '--samples', '1000', '--comparison-protocol', str(protocol),
        '--role', role, '--output-dir', str(output),
    ]
    started = datetime.now(timezone.utc).isoformat()
    completed = subprocess.run(command, cwd=root, capture_output=True, text=True, check=False)
    record = {
        'size': size, 'pair': pair, 'role': role, 'started_utc': started,
        'finished_utc': datetime.now(timezone.utc).isoformat(),
        'exit_code': completed.returncode, 'stdout': completed.stdout,
        'stderr': completed.stderr, 'output_dir': str(output),
    }
    if (output / 'attempt.json').exists():
        attempt = json.loads((output / 'attempt.json').read_text())
        record['attempt_status'] = attempt.get('status')
        record['invalidators'] = attempt.get('invalidators')
        record['protocol_sha256'] = attempt.get('comparison_protocol_sha256')
    with run_order.open('a') as stream:
        stream.write(json.dumps(record, sort_keys=True) + '\n')
    print(size, pair, role, completed.returncode, record.get('attempt_status'), flush=True)
    if completed.returncode != 0 or record.get('attempt_status') != 'VALID_S01_PAIRED_BLOCK':
        raise SystemExit(f'campaign stopped at {size}/{pair}/{role}; inspect {output}')
print('remaining scheduled blocks complete', flush=True)
