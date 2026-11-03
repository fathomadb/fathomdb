"""Run the fixed sanitation protocol. Usage: campaign.py PROBE NEW_OUTPUT_DIR."""
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

GIB = 1024 ** 3


def cells():
    """Return all24 core cells, including ones a stopped campaign never executes."""
    return [dict(family=f, workload=w, load=load, arm=arm)
            for f, w, load, arm in itertools.product(
                ('node', 'edge', 'property'), ('fresh', 'legacy'), ('idle', 'loaded'), ('A', 'B'))]


def summarize(events, stream, complete, expected=1500):
    """Reconcile offers and finishes; unqualified percentiles remain null."""
    offers = {e['id']: e for e in events if e['kind'] == 'offer' and e['stream'] == stream}
    finishes = {e['id']: e for e in events if e['kind'] == 'finish' and e['stream'] == stream}
    success = [e['latency_ns'] for e in finishes.values() if e['outcome'] == 'success']
    refused = sum(e['state'] == 'refused' for e in offers.values())
    unfinished = sum(e['state'] == 'started' and i not in finishes for i, e in offers.items())
    errors = sum(e['outcome'] == 'error' for e in finishes.values())
    qualified = complete and len(offers) == expected and len(success) >= 1000 and not (refused or unfinished or errors)
    ordered = sorted(success)
    return dict(offered=len(offers), success=len(success), refused=refused, error=errors,
                unfinished=unfinished, not_offered=expected-len(offers),
                p50_ns=ordered[math.ceil(len(ordered)*.5)-1] if qualified else None,
                p99_ns=ordered[math.ceil(len(ordered)*.99)-1] if qualified else None,
                max_success_ns=max(success, default=None))


def save(path, value):
    """Persist a complete JSON record before starting the next child."""
    path.write_text(json.dumps(value, indent=2) + '\n')


def scratch_bytes(root):
    """Count all new campaign files, including retained masters and failed copies."""
    return sum(p.stat().st_size for p in root.rglob('*') if p.is_file())


def resource_sample(pid, root):
    """Sample the one Rust process (all worker threads share its address space)."""
    result = dict(rss_bytes=0, scratch_bytes=scratch_bytes(root))
    try:
        for line in Path(f'/proc/{pid}/status').read_text().splitlines():
            if line.startswith('VmRSS:'):
                result['rss_bytes'] = int(line.split()[1]) * 1024
        result['cpu_ticks'] = list(map(int, Path(f'/proc/{pid}/stat').read_text().split()[13:15]))
        result['io'] = {k: int(v) for k, v in
                        (line.split(':') for line in Path(f'/proc/{pid}/io').read_text().splitlines())}
    except (FileNotFoundError, ProcessLookupError):
        pass
    return result


def child(command, output, root, deadline):
    """Bound one child, retain stops/overshoot, and always join it."""
    begun = time.monotonic()
    stop = None
    peak = dict(rss_bytes=0, scratch_bytes=0)
    last = {}
    with output.open('w') as log:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        while process.poll() is None:
            last = resource_sample(process.pid, root)
            for key in peak:
                peak[key] = max(peak[key], last[key])
            if time.monotonic() >= deadline:
                stop = 'time_limit'
            elif peak['rss_bytes'] > 8*GIB:
                stop = 'rss_limit'
            elif peak['scratch_bytes'] > 12*GIB:
                stop = 'disk_limit'
            if stop:
                process.terminate()
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                break
            time.sleep(.1)
        code = process.wait()
    ended = time.monotonic()
    result = None
    for line in output.read_text().splitlines():
        try:
            result = json.loads(line)
        except json.JSONDecodeError:
            pass
    return dict(exit_code=code, stop=stop, elapsed_s=ended-begun,
                deadline_overshoot_s=max(0, ended-deadline), peak=peak,
                final_resource_sample=last, result=result)


def events_at(path):
    """Read complete event lines retained even when a replica is interrupted."""
    rows = []
    if path.exists():
        for line in path.read_text().splitlines():
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    return rows


def remove_replica_files(directory):
    """Delete only this attempt's generated SQLite copies after evidence is retained."""
    for suffix in ('', '-wal', '-shm'):
        path = directory / ('replica.sqlite' + suffix)
        if path.exists():
            path.unlink()


def main():
    """Freeze the source/protocol before fixture preparation and run without replacement."""
    binary = Path(sys.argv[1]).resolve()
    root = Path(sys.argv[2]).resolve()
    if root.exists():
        raise SystemExit('output must be new')
    if shutil.disk_usage(root.parent).free < 12*GIB:
        raise SystemExit('12GiB scratch reservation unavailable')
    subprocess.run(['git', 'diff', '--quiet'], check=True)
    subprocess.run(['git', 'diff', '--cached', '--quiet'], check=True)
    root.mkdir()
    source = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    protocol = Path('dev/plans/0.8.28/features/slice-81/experiment-design.md')
    begun = time.monotonic()
    deadline = begun + 45*60
    manifest = dict(source_sha=source, binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                    protocol_sha256=hashlib.sha256(protocol.read_bytes()).hexdigest(),
                    host=os.uname()._asdict() if hasattr(os.uname(), '_asdict') else list(os.uname()),
                    seed='0x81f00d', profile='release/performance',
                    owners=10000, target_owners=1000, history=4, primary_bytes=1024,
                    merge_pages=64, inter_unit_gap_ms=1,
                    offer_window_s=15, reads_per_s=100, readers=8, writes_per_s=100,
                    core_budget_s=2700, replica_budget_s=30, cell_budget_s=90,
                    rss_limit_bytes=8*GIB, scratch_limit_bytes=12*GIB,
                    sample_period_s=.1, started_wall=time.time(),
                    arm_order=[['A', 'B'], ['B', 'A'], ['A', 'B']], cells=cells(),
                    controls=[dict(family=f, load='loaded', arm='control') for f in ('node', 'edge', 'property')],
                    supplement='ordinary expiry deferred: outside revised sanitation scope',
                    conditional_c='pending A/B screening; no production staging design approved')
    save(root/'manifest.json', manifest)
    results = dict(fixtures={}, controls=[], attempts=[], elapsed_s=0)
    for index, cell in enumerate(cells()):
        for replica in range(3):
            results['attempts'].append(dict(cell=index, replica=replica, **cell, disposition='not_run'))
    save(root/'results.json', results)
    masters = {}
    for family in ('node', 'edge', 'property'):
        for policy in ('default', 'secure'):
            path = root/f'{family}-{policy}.sqlite'
            record = child([str(binary), 'fixture', str(path), family, policy],
                           root/f'fixture-{family}-{policy}.log', root, deadline)
            results['fixtures'][f'{family}-{policy}'] = record
            save(root/'results.json', results)
            if record['exit_code'] or record['stop']:
                results['stopped'] = 'fixture failure or campaign limit'
                results['elapsed_s'] = time.monotonic()-begun
                save(root/'results.json', results)
                return
            masters[(family, policy)] = path
        legacy = root/f'{family}-legacy.sqlite'
        record = child([str(binary), 'legacy', str(masters[(family, 'default')]), str(legacy), family],
                       root/f'fixture-{family}-legacy.log', root, deadline)
        results['fixtures'][f'{family}-legacy'] = record
        if record['exit_code'] or record['stop']:
            results['stopped'] = 'legacy positive-control failure'
            results['elapsed_s'] = time.monotonic()-begun
            save(root/'results.json', results)
            return
        masters[(family, 'legacy')] = legacy
    for family in ('node', 'edge', 'property'):
        out = root/f'control-{family}'
        out.mkdir()
        record = child([str(binary), 'replica', str(masters[(family, 'default')]), str(out), family,
                        'control', 'fresh', 'loaded'], out/'child.log', root, min(deadline, time.monotonic()+30))
        events = events_at(out/'events.jsonl')
        complete = not record['stop'] and record['exit_code'] == 0 and record['result'].get('status') == 'complete'
        record['streams'] = {s: summarize(events, s, complete) for s in ('read', 'write')}
        results['controls'].append(dict(family=family, **record))
        remove_replica_files(out)
        save(root/'results.json', results)
    cell_spent = {i: 0. for i in range(24)}
    for family, workload, load in itertools.product(('node', 'edge', 'property'), ('fresh', 'legacy'), ('idle', 'loaded')):
        for replica in range(3):
            for arm in manifest['arm_order'][replica]:
                attempt = next(a for a in results['attempts'] if (a['family'], a['workload'], a['load'], a['replica'], a['arm']) == (family, workload, load, replica, arm))
                remaining = min(30, 90-cell_spent[attempt['cell']], deadline-time.monotonic())
                if remaining <= 0:
                    attempt['reason'] = 'cell or campaign budget exhausted'
                    continue
                out = root/f"cell-{attempt['cell']:02}-replica-{replica}"
                out.mkdir()
                policy = 'legacy' if workload == 'legacy' else ('secure' if arm == 'A' else 'default')
                record = child([str(binary), 'replica', str(masters[(family, policy)]), str(out), family,
                                arm, workload, load], out/'child.log', root, time.monotonic()+remaining)
                cell_spent[attempt['cell']] += record['elapsed_s']
                complete = not record['stop'] and record['exit_code'] == 0 and record['result'].get('status') == 'complete'
                events = events_at(out/'events.jsonl')
                record['streams'] = {s: summarize(events, s, complete, 1500 if load == 'loaded' else 0) for s in ('read', 'write')}
                attempt.update(record, disposition='complete' if complete else 'incomplete', raw=str(out))
                remove_replica_files(out)
                results['elapsed_s'] = time.monotonic()-begun
                save(root/'results.json', results)
    results['elapsed_s'] = time.monotonic()-begun
    save(root/'results.json', results)
    print(json.dumps(dict(output=str(root), elapsed_s=results['elapsed_s'], complete=sum(a['disposition'] == 'complete' for a in results['attempts']))))


if __name__ == '__main__':
    main()
