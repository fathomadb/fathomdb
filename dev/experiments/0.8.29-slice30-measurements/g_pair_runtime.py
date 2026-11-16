"""Private paired scope through the existing guardian, attempts, copies and native driver."""

import json
import os
from pathlib import Path
import sys
import time

from g_campaign import active_bytes
from m_guard import write_budget


def deadline(manifest, budget, owner, cell, now):
    """Bound one member by cumulative prior member, whole-phase and shared charges."""
    from g_pair import CALIBRATION_ID
    profile = manifest['profile']
    prior = budget.get('cell_used_ns', {}).get(cell['id'], 0)
    ends = [now + profile['whole_cell_ns'] - prior,
            now + profile['total_shared_ns'] - profile['reserved_other_work_ns'] - budget['used_ns'] - 20*10**9]
    if manifest['scope'] == 'calibration':
        limits = manifest['calibration']
        usage = budget['calibration_usage'][CALIBRATION_ID]
        spent = sum(budget.get('cell_used_ns', {}).get(member['id'], 0)
                    for member in manifest['plan']['calibration'])
        if spent + 7*10**9 >= limits['payload_ns']:
            raise ValueError('cumulative calibration work allowance exhausted')
        ends += [now + limits['payload_ns'] - spent,
                 owner['started_monotonic_ns'] + limits['phase_ns'] - usage['wall_ns'] - 20*10**9]
    end = min(ends)
    if end <= now + 7*10**9:
        raise ValueError('remaining shared/member work allowance cannot fund cleanup')
    return end


def execute_native(manifest, cell, attempt, deadline_ns, *, observe=None):
    """Copy admitted data and invoke the current native binary under the same deadline."""
    from g_closed_copy import copy_subject
    from g_driver import run_program_step
    attempt = Path(attempt)
    subject = (manifest['donors'][f"G{cell['n']}"] if cell['family'].startswith('G')
               else manifest['templates'][f"{cell['family']}-{cell['n']}"])
    store, preparation = copy_subject(subject, attempt / 'subject/store.sqlite', deadline_ns)
    config = dict(cell, store=store, corpus=manifest['inputs']['corpus']['path'],
                  queries=manifest['inputs']['queries']['path'])
    step = run_program_step([manifest['build']['binaries'][cell['artifact']]['path']],
        config, attempt / 'native', deadline_ns, observe=observe)
    return {'complete': step['complete'], 'steps': [step], 'preparation': preparation, 'store': store}


def payload(root, config, attempt, deadline_ns):
    """Execute only the exact currently registered private member and validate real facts."""
    from g_pair import canonical_members, qualify_step, worker_admission
    root = Path(root)
    manifest = json.loads((root / 'manifest.json').read_text())
    cell = json.loads(Path(config).read_text())
    worker_admission(manifest, cell, attempt)
    if cell not in canonical_members(manifest) or time.monotonic_ns() >= deadline_ns:
        raise RuntimeError('canonical paired member/deadline unavailable')
    os.environ.update(HF_HOME=manifest['hf_home'], HF_HUB_OFFLINE='1', TRANSFORMERS_OFFLINE='1',
                      FATHOMDB_EMBED_DEVICE=f"cuda:{manifest['gpu']['ordinal']}")
    observer = None
    if 'hybrid' in cell['routes']:
        from g_gpu import GpuObserver
        observer = GpuObserver(manifest['gpu'])
    result = execute_native(manifest, cell, attempt, deadline_ns,
                            observe=observer.sample if observer else None)
    issues = [issue for step in result['steps'] for issue in qualify_step(step, cell, manifest)]
    if 'hybrid' in cell['routes'] and not any(
            sample.get('pid') == step.get('pid') and sample.get('pid_memory_mib', 0) > 0
            and sample.get('gpu_memory_mib', 0) > sample.get('baseline_gpu_memory_mib', 0)
            and sample.get('utilization_percent', 0) > 0
            for step in result['steps'] for sample in step.get('gpu_samples', [])):
        issues.append('actual_reserved_gpu_engagement_unobserved')
    result.update(complete=result['complete'] and not issues, qualification_issues=sorted(set(issues)),
                  cell=cell, freeze_uuid=manifest['uuid'], scope=manifest['scope'],
                  selected_build=manifest['build']['binaries'][cell['artifact']])
    (Path(attempt) / 'result.json').write_text(json.dumps(result, indent=2, allow_nan=False) + '\n')
    return result


def _assessment(manifest, budget, remaining):
    from g_pair_freeze import require_forecasts
    forecasts = require_forecasts(manifest, manifest['plan']['primary'] + manifest['plan']['diagnostics'])
    profile = manifest['profile']
    required_ns = 2 * (forecasts['common_ns'] + sum(forecasts['cells'][cell['id']]['whole_ns'] for cell in remaining))
    required_bytes = 2 * (forecasts['common_bytes'] + sum(forecasts['cells'][cell['id']]['new_retained_bytes'] for cell in remaining))
    if budget['used_ns'] + required_ns + profile['reserved_other_work_ns'] > profile['total_shared_ns']:
        raise ValueError('complete remaining paired table exceeds shared time/headroom')
    if active_bytes(manifest['allocation']) + required_bytes + profile['reserved_other_work_bytes'] > profile['total_shared_retained_bytes']:
        raise ValueError('complete remaining paired table exceeds retained storage/headroom')


def control(manifest, owner):
    """Dispatch canonical members sequentially, retaining every failure and cumulative charge."""
    from g_pair import CALIBRATION_ID, canonical_members, drive_members
    from m_attempt import run_attempt
    from m_load import observe_load, summarize_load
    path = Path(manifest['allocation']) / 'g-budget.json'
    budget = json.loads(path.read_text())
    if budget.get('s50_guardian') != owner or budget.get('inflight'):
        raise RuntimeError('paired control lacks current original-ledger guardian admission')
    root = Path(manifest['root'])
    if manifest['scope'] == 'calibration':
        baseline = active_bytes(manifest['allocation'])
        remaining_bytes = manifest['calibration']['new_retained_bytes'] - budget['calibration_usage'][CALIBRATION_ID]['new_retained_bytes']
    else:
        baseline, remaining_bytes = 0, None
    blocked = None
    cells = canonical_members(manifest)
    def execute(cell, index):
        nonlocal blocked
        value = json.loads(path.read_text())
        started = time.monotonic_ns()
        retained_before = active_bytes(manifest['allocation'])
        load = [observe_load()]
        try:
            if blocked:
                raise ValueError(blocked)
            if manifest['scope'] == 'primary':
                _assessment(manifest, value, cells[index:])
            end = deadline(manifest, value, owner, cell, started)
        except ValueError as error:
            blocked = str(error)
            return {'id': cell['id'], 'cell': cell, 'status': 'held', 'complete': False, 'reason': blocked}
        def register(selected, attempt):
            current = json.loads(path.read_text())
            current['inflight'] = {'cell': selected['id'], 'started_monotonic_ns': started,
                'started_at_ns': time.time_ns(), 'attempt': str(Path(attempt).resolve()), 'freeze_uuid': manifest['uuid']}
            write_budget(path, current)
        def monitor():
            load.append(observe_load())
            current_bytes = active_bytes(manifest['allocation'])
            if remaining_bytes is not None and current_bytes - baseline >= remaining_bytes:
                return 'calibration retained growth exhausted'
            if current_bytes >= manifest['profile']['total_shared_retained_bytes'] - manifest['profile']['reserved_other_work_bytes']:
                return 'paired shared retained storage exhausted'
            return None
        def command(selected, attempt, end):
            return [sys.executable, str(Path(__file__).with_name('g_runner.py')), '_pair_cell',
                    '--root', str(root), '--deadline-ns', str(end)]
        outcome = run_attempt(cell, root / 'results', command, deadline_ns=end, register=register, monitor=monitor)
        elapsed = time.monotonic_ns() - started
        current = json.loads(path.read_text())
        current['used_ns'] += elapsed
        used = current.setdefault('cell_used_ns', {})
        used[cell['id']] = used.get(cell['id'], 0) + elapsed
        clean = not outcome['cleanup_uncertain'] and outcome['live_owned_pids'] == []
        if clean:
            current['inflight'] = None
        else:
            current['inflight']['status'] = 'cleanup_uncertain'
            blocked = 'prior paired actor cleanup uncertain'
        write_budget(path, current)
        outcome.update(charged_ns=elapsed, load=summarize_load(load), qualifies_primary=manifest['scope'] == 'primary',
                       retained_growth_bytes=max(0, active_bytes(manifest['allocation']) - retained_before))
        if (Path(outcome['attempt']) / 'result.json').exists():
            actual = json.loads((Path(outcome['attempt']) / 'result.json').read_text())
            outcome['store'] = actual.get('store')
        return outcome
    return drive_members(manifest, root, execute)


def finish(manifest, rows):
    """Compare and report before optional strict quiescent disposable-copy retirement."""
    from g_pair import report, retire_pair
    root = Path(manifest['root'])
    result = report(root)
    previous = {row['id']: row for row in rows}
    retired = []
    for name in (['calibration'] if manifest['scope'] == 'calibration' else ['primary', 'diagnostics']):
        for pair in result['comparisons'][name]:
            origin = pair['origin_id']
            required = [item for item in result['comparisons']['diagnostics'] if item['origin_id'] == origin]
            if name == 'calibration':
                required = result['comparisons']['calibration']
            outcomes = [previous[identifier] for identifier in pair['members'] if identifier in previous]
            retired += retire_pair(root / 'results', outcomes,
                enabled=manifest['retire_complete_copies'], comparison=pair['complete'],
                diagnostics_complete=all(item['complete'] for item in required))
    result['retired_copies'] = retired
    (root / 'results/report.json').write_text(json.dumps(result, indent=2, allow_nan=False) + '\n')
    return result['calibration_complete'] if manifest['scope'] == 'calibration' else result['complete']


def control_entry(operation, root, options, admission_fd):
    """Own freeze/run/report under the existing inherited guardian admission."""
    from g_pair import freeze
    from g_pair_freeze import validate
    from m_runtime import controller_admission
    owner = controller_admission(admission_fd, root)
    if operation in ('freeze', 'calibration'):
        selected = json.loads(Path(options).read_text())
        manifest = freeze(selected, root, scope='calibration' if operation == 'calibration' else 'primary')
    else:
        manifest = validate(root)
    path = Path(manifest['allocation']) / 'g-budget.json'
    if json.loads(path.read_text()).get('s50_guardian') != owner:
        raise RuntimeError('paired operation lacks actual selected original guardian')
    if operation == 'freeze':
        return True
    if operation == 'report':
        state = Path(root) / 'results/campaign-state.json'
        return finish(manifest, json.loads(state.read_text()) if state.exists() else [])
    return finish(manifest, control(manifest, owner))
