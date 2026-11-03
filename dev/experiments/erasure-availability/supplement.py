"""Reviewed never-started continuation. Usage: supplement.py PROBE ORIGINAL_ROOT [--plan]."""
import itertools
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

from campaign import (GIB, completed, digest, events_at, remove_replica_files,
                      safe_child, save, summarize)


def pending_attempts(original, manifest):
    """Select only never-started attempts in the original paired order."""
    lookup = {(a['family'], a['workload'], a['load'], a['replica'], a['arm']): a
              for a in original['attempts']}
    selected = []
    for family, workload, load in itertools.product(('node', 'edge', 'property'), ('fresh', 'legacy'), ('idle', 'loaded')):
        for replica in range(3):
            for arm in manifest['arm_order'][replica]:
                attempt = lookup[(family, workload, load, replica, arm)]
                if attempt['disposition'] == 'not_run':
                    selected.append(attempt)
    return selected


def remaining_seconds(original, cell, deadline, now):
    """Carry the original cell's spent time into the unchanged90-second limit."""
    spent = sum(a.get('elapsed_s', 0) for a in original['attempts']
                if a['cell'] == cell and a['disposition'] != 'not_run')
    return max(0, min(30, 90-spent, deadline-now))


def prepare(binary, root):
    """Verify immutable inputs and return the exact reviewed continuation plan."""
    subprocess.run(['git', 'diff', '--quiet'], check=True)
    subprocess.run(['git', 'diff', '--cached', '--quiet'], check=True)
    manifest = json.loads((root/'manifest.json').read_text())
    original = json.loads((root/'results.json').read_text())
    if digest(binary) != manifest['binary_sha256']:
        raise ValueError('the original release probe must be reused unchanged')
    masters = {}
    for name, record in original['fixtures'].items():
        path = root/f'{name}.sqlite'
        actual = digest(path)
        if actual != record['fixture_sha256']:
            raise ValueError(f'original closed master changed: {name}')
        masters[name] = dict(path=str(path), sha256=actual)
    keys = ('cell', 'replica', 'family', 'workload', 'load', 'arm')
    selected = [{k: row[k] for k in keys} for row in pending_attempts(original, manifest)]
    plan = dict(controller_source_sha=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                probe_source_sha=manifest['source_sha'], binary_sha256=manifest['binary_sha256'],
                original_manifest_sha256=digest(root/'manifest.json'),
                original_results_sha256=digest(root/'results.json'), original_elapsed_s=original['elapsed_s'],
                original_started_wall=manifest['started_wall'], masters=masters,
                protocol_sha256=digest(Path('dev/plans/0.8.28/features/slice-81/experiment-design.md')),
                additional_active_budget_s=1500, replica_budget_s=30, cell_budget_s=90,
                carried_cell_spent_s={str(i): sum(a.get('elapsed_s', 0) for a in original['attempts'] if a['cell'] == i and a['disposition'] != 'not_run') for i in range(24)},
                arm_order=manifest['arm_order'], candidate_attempts=selected,
                read_write_controls='original three successful controls; retained unchanged',
                thresholds='all original load, physical-proof, qualification and sampled resource limits unchanged',
                scope='only original never-started attempts; no retries, C or scheduler',
                budget_revision='separate explicitly reviewed25-minute supplement, not an extension of original45-minute campaign or an unused-C allowance')
    return original, plan


def main():
    """Run one separately bounded supplement, charging hash validation and setup."""
    begun = time.monotonic()
    started_wall = time.time()
    deadline = begun + 25*60
    binary = Path(sys.argv[1]).resolve()
    root = Path(sys.argv[2]).resolve()
    original, plan = prepare(binary, root)
    if '--plan' in sys.argv[3:]:
        print(json.dumps(plan, indent=2))
        return
    output = root/'supplement'
    if output.exists():
        raise SystemExit('supplement output must be new')
    if shutil.disk_usage(root).free < 12*GIB:
        raise SystemExit('12GiB scratch reservation unavailable')
    output.mkdir()
    plan['started_wall'] = started_wall
    save(output/'manifest.json', plan)
    results = dict(attempts=[dict(a, disposition='not_run') for a in plan['candidate_attempts']],
                   elapsed_s=0, original_results_sha256=plan['original_results_sha256'])
    save(output/'results.json', results)
    stop = None
    for attempt in results['attempts']:
        combined = dict(attempts=original['attempts']+results['attempts'])
        remaining = remaining_seconds(combined, attempt['cell'], deadline, time.monotonic())
        if remaining <= 0:
            attempt['reason'] = 'original cell or supplement active budget exhausted'
            continue
        out = output/f"cell-{attempt['cell']:02}-replica-{attempt['replica']}"
        out.mkdir()
        policy = 'legacy' if attempt['workload'] == 'legacy' else ('secure' if attempt['arm'] == 'A' else 'default')
        master = plan['masters'][f"{attempt['family']}-{policy}"]['path']
        record = safe_child([str(binary), 'replica', master, str(out), attempt['family'],
                             attempt['arm'], attempt['workload'], attempt['load']],
                            out/'child.log', root, time.monotonic()+remaining)
        complete = completed(record)
        events = events_at(out/'events.jsonl')
        record['streams'] = {stream: summarize(events, stream, complete,
                                               1500 if attempt['load'] == 'loaded' else 0)
                             for stream in ('read', 'write')}
        record['raw_sha256'] = {p.name: digest(p) for p in out.iterdir() if p.suffix in ('.log', '.jsonl')}
        attempt.update(record, disposition='complete' if complete else 'incomplete', raw=str(out))
        remove_replica_files(out)
        results['elapsed_s'] = time.monotonic()-begun
        save(output/'results.json', results)
        if record['stop'] in ('rss_limit', 'disk_limit', 'monitor_error'):
            stop = record['stop']
            break
    for attempt in results['attempts']:
        if attempt['disposition'] == 'not_run' and 'reason' not in attempt:
            attempt['reason'] = stop or 'supplement active budget exhausted'
    results.update(elapsed_s=time.monotonic()-begun, stopped=stop,
                   total_active_s=original['elapsed_s']+time.monotonic()-begun,
                   total_wall_span_s=time.time()-plan['original_started_wall'])
    save(output/'results.json', results)
    print(json.dumps(dict(output=str(output), elapsed_s=results['elapsed_s'],
                         complete=sum(a['disposition'] == 'complete' for a in results['attempts']))))


if __name__ == '__main__':
    main()
