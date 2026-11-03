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
    """Validate unique scheduled offers/finishes before qualifying percentiles."""
    offer_rows = [e for e in events if e['kind'] == 'offer' and e['stream'] == stream]
    finish_rows = [e for e in events if e['kind'] == 'finish' and e['stream'] == stream]
    offers = {e['id']: e for e in offer_rows}
    finishes = {e['id']: e for e in finish_rows}
    invalid = len(offer_rows)-len(offers)+len(finish_rows)-len(finishes)
    invalid += sum(not isinstance(i, int) or not 0 <= i < expected or e['state'] not in ('started', 'refused') for i, e in offers.items())
    invalid += sum(i not in offers or offers[i]['state'] != 'started' or e['outcome'] not in ('success', 'error') or e.get('latency_ns', -1) < 0 for i, e in finishes.items())
    success = [e['latency_ns'] for e in finishes.values() if e['outcome'] == 'success']
    refused = sum(e['state'] == 'refused' for e in offers.values())
    unfinished = sum(e['state'] == 'started' and i not in finishes for i, e in offers.items())
    errors = sum(e['outcome'] == 'error' for e in finishes.values())
    qualified = complete and len(offers) == expected and len(success) >= 1000 and not (invalid or refused or unfinished or errors)
    ordered = sorted(success)
    return dict(offered=len(offers), success=len(success), refused=refused, error=errors,
                unfinished=unfinished, not_offered=max(0, expected-len(offers)), accounting_errors=invalid,
                p50_ns=ordered[math.ceil(len(ordered)*.5)-1] if qualified else None,
                p99_ns=ordered[math.ceil(len(ordered)*.99)-1] if qualified else None,
                max_success_ns=max(success, default=None))


def completed(record):
    """A late, interrupted or malformed result cannot certify a completed replica."""
    result = record.get('result')
    return (not record.get('stop') and record.get('exit_code') == 0
            and record.get('deadline_overshoot_s', 0) == 0
            and isinstance(result, dict) and result.get('status') == 'complete')


def save(path, value):
    """Persist a complete JSON record before starting the next child."""
    path.write_text(json.dumps(value, indent=2) + '\n')


def scratch_bytes(root):
    """Count new campaign files; SQLite may retire a WAL between list and stat."""
    total = 0
    for path in root.rglob('*'):
        try:
            if path.is_file():
                total += path.stat().st_size
        except FileNotFoundError:
            pass
    return total


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
    """Bound one child and reap it on every exit, including monitor exceptions."""
    begun = time.monotonic()
    stop = None
    peak = dict(rss_bytes=0, scratch_bytes=0)
    last = {}
    process = None
    code = None
    try:
        with output.open('w') as log:
            if begun >= deadline:
                stop = 'time_limit'
            else:
                process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
                while process.poll() is None:
                    try:
                        last = resource_sample(process.pid, root)
                    except PermissionError as error:
                        # Linux may revoke /proc access while an exiting child retires.
                        if str(error.filename or '').startswith(f'/proc/{process.pid}/'):
                            try:
                                process.wait(timeout=.05)
                            except subprocess.TimeoutExpired:
                                raise error
                            break
                        raise
                    for key in peak:
                        peak[key] = max(peak[key], last[key])
                    if time.monotonic() >= deadline:
                        stop = 'time_limit'
                    elif peak['rss_bytes'] > 8*GIB:
                        stop = 'rss_limit'
                    elif peak['scratch_bytes'] > 12*GIB:
                        stop = 'disk_limit'
                    if stop:
                        break
                    time.sleep(.1)
    finally:
        if process is not None:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    process.kill()
            code = process.wait()
    ended = time.monotonic()
    if ended > deadline and stop is None:
        stop = 'time_limit'
    result = None
    for line in output.read_text().splitlines():
        try:
            value = json.loads(line)
            if isinstance(value, dict):
                result = value
        except json.JSONDecodeError:
            pass
    return dict(exit_code=code, stop=stop, elapsed_s=ended-begun,
                deadline_overshoot_s=max(0, ended-deadline), peak=peak,
                final_resource_sample=last, result=result)


def safe_child(*args):
    """Convert a reaped monitor exception into a durable campaign stop."""
    begun = time.monotonic()
    try:
        return child(*args)
    except Exception as error:
        return dict(exit_code=None, stop='monitor_error', error=repr(error),
                    elapsed_s=time.monotonic()-begun, result=None)


def stop_campaign(results, root, begun, reason):
    """Give every remaining planned attempt/control an explicit disposition."""
    results['stopped'] = reason
    results['elapsed_s'] = time.monotonic()-begun
    for row in results['attempts'] + results['controls']:
        if row.get('disposition') == 'not_run':
            row['reason'] = reason
    save(root/'results.json', results)


def digest(path):
    """Hash a closed artifact without duplicating its bytes in memory."""
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


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
                    foreground_read='Engine text-only survivorneedle; property p1; edge endpoint stable text (not edge ranking)',
                    foreground_write='unique foreground-{id} canonical node, source foreground, body unrelated service',
                    observer_overhead='identical unbuffered offer/finish logging, phase events and100ms polling included; not separately estimated',
                    control_rule='failed no-erasure offered-rate control makes load comparison inconclusive',
                    arm_order=[['A', 'B'], ['B', 'A'], ['A', 'B']], cells=cells(),
                    controls=[dict(family=f, load='loaded', arm='control') for f in ('node', 'edge', 'property')],
                    supplement='ordinary expiry deferred: outside revised sanitation scope',
                    conditional_c='pending A/B screening; no production staging design approved')
    save(root/'manifest.json', manifest)
    results = dict(fixtures={}, controls=[dict(family=f, disposition='not_run') for f in ('node', 'edge', 'property')], attempts=[], elapsed_s=0)
    for index, cell in enumerate(cells()):
        for replica in range(3):
            results['attempts'].append(dict(cell=index, replica=replica, **cell, disposition='not_run'))
    save(root/'results.json', results)
    masters = {}
    for family in ('node', 'edge', 'property'):
        for policy in ('default', 'secure'):
            path = root/f'{family}-{policy}.sqlite'
            record = safe_child([str(binary), 'fixture', str(path), family, policy],
                           root/f'fixture-{family}-{policy}.log', root, deadline)
            results['fixtures'][f'{family}-{policy}'] = record
            save(root/'results.json', results)
            if record['exit_code'] or record['stop']:
                stop_campaign(results, root, begun, 'fixture failure or campaign limit')
                return
            record['fixture_sha256'] = digest(path)
            masters[(family, policy)] = path
        legacy = root/f'{family}-legacy.sqlite'
        record = safe_child([str(binary), 'legacy', str(masters[(family, 'default')]), str(legacy), family],
                       root/f'fixture-{family}-legacy.log', root, deadline)
        results['fixtures'][f'{family}-legacy'] = record
        if record['exit_code'] or record['stop']:
            stop_campaign(results, root, begun, 'legacy positive-control failure or budget exhaustion')
            return
        record['fixture_sha256'] = digest(legacy)
        masters[(family, 'legacy')] = legacy
    for family in ('node', 'edge', 'property'):
        if time.monotonic() >= deadline:
            stop_campaign(results, root, begun, 'campaign budget exhausted before controls')
            return
        out = root/f'control-{family}'
        out.mkdir()
        record = safe_child([str(binary), 'replica', str(masters[(family, 'default')]), str(out), family,
                        'control', 'fresh', 'loaded'], out/'child.log', root, min(deadline, time.monotonic()+30))
        events = events_at(out/'events.jsonl')
        complete = completed(record)
        record['streams'] = {s: summarize(events, s, complete) for s in ('read', 'write')}
        record['raw_sha256'] = {p.name: digest(p) for p in out.iterdir() if p.suffix in ('.log', '.jsonl')}
        next(c for c in results['controls'] if c['family'] == family).update(record, disposition='complete' if complete else 'incomplete')
        remove_replica_files(out)
        save(root/'results.json', results)
        if record['stop'] in ('rss_limit', 'disk_limit', 'monitor_error'):
            stop_campaign(results, root, begun, record['stop'])
            return
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
                record = safe_child([str(binary), 'replica', str(masters[(family, policy)]), str(out), family,
                                arm, workload, load], out/'child.log', root, time.monotonic()+remaining)
                cell_spent[attempt['cell']] += record['elapsed_s']
                complete = completed(record)
                events = events_at(out/'events.jsonl')
                record['streams'] = {s: summarize(events, s, complete, 1500 if load == 'loaded' else 0) for s in ('read', 'write')}
                record['raw_sha256'] = {p.name: digest(p) for p in out.iterdir() if p.suffix in ('.log', '.jsonl')}
                attempt.update(record, disposition='complete' if complete else 'incomplete', raw=str(out))
                remove_replica_files(out)
                results['elapsed_s'] = time.monotonic()-begun
                save(root/'results.json', results)
                if record['stop'] in ('rss_limit', 'disk_limit', 'monitor_error'):
                    stop_campaign(results, root, begun, record['stop'])
                    return
    results['elapsed_s'] = time.monotonic()-begun
    save(root/'results.json', results)
    print(json.dumps(dict(output=str(root), elapsed_s=results['elapsed_s'], complete=sum(a['disposition'] == 'complete' for a in results['attempts']))))


if __name__ == '__main__':
    main()
