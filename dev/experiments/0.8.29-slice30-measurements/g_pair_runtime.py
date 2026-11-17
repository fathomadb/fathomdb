"""Private paired scope through the existing guardian, attempts, copies and native driver."""

import fcntl
import json
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
    else:
        limits = manifest['primary_limits']
        pairs = manifest['plan']['primary'] + manifest['plan']['diagnostics']
        index = next(index for index, item in enumerate(pairs) if item['id'] == cell['id'])
        members = pairs[index-index%2:index-index%2+2]
        prior_pair = sum(budget.get('cell_used_ns', {}).get(item['id'], 0) for item in members)
        from g_pair_selection import pair_allowance
        _, pair_ns = pair_allowance(manifest, budget, cell)
        ends += [now + pair_ns - prior_pair,
                 owner['started_monotonic_ns'] + limits['phase_ns'] - 20*10**9]
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
    if cell['family'].startswith('G'):
        config['slice50_gpu'] = {'uuid': manifest['gpu']['uuid'], 'ordinal': manifest['gpu']['ordinal'],
                                 'hf_home': manifest['hf_home']}
    step = run_program_step([manifest['build']['binaries'][cell['artifact']]['path']],
        config, attempt / 'native', deadline_ns, observe=observe)
    return {'complete': step['complete'], 'steps': [step], 'preparation': preparation, 'store': store}


def payload(root, config, attempt, deadline_ns, *, selection=None):
    """Execute only the exact currently registered private member and validate real facts."""
    from g_pair import canonical_members, qualify_step, worker_admission
    root = Path(root)
    from g_pair_freeze import prepared_inputs
    manifest = prepared_inputs(json.loads((root / 'manifest.json').read_text()))
    if selection is not None:
        from g_pair_selection import apply
        manifest = apply(manifest, selection)
    if manifest.get('_report_recovery_selection'):
        raise ValueError('recovery selection is report-only')
    cell = json.loads(Path(config).read_text())
    worker_admission(manifest, cell, attempt)
    if cell not in canonical_members(manifest) or time.monotonic_ns() >= deadline_ns:
        raise RuntimeError('canonical paired member/deadline unavailable')
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


def assess_primary(manifest, budget, retained_bytes):
    """Fund only explicit selected stops against actual shared charges and reservations."""
    from g_pair_freeze import primary_limits
    limits = primary_limits(manifest)
    profile = manifest['profile']
    if budget['used_ns'] + limits['phase_ns'] + profile['reserved_other_work_ns'] >= profile['total_shared_ns']:
        raise ValueError('selected primary phase allowance exceeds actual shared time/headroom')
    if retained_bytes + limits['new_retained_bytes'] + profile['reserved_other_work_bytes'] >= profile['total_shared_retained_bytes']:
        raise ValueError('selected primary growth allowance exceeds actual shared storage/headroom')


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
        from g_pair import required_prefix
        from g_pair_freeze import prepare_inputs
        baseline = active_bytes(manifest['allocation'])
        remaining_bytes = manifest['primary_limits']['new_retained_bytes']
        assess_primary(manifest, budget, baseline)
        cells = required_prefix(manifest, root)
        manifest.update(prepare_inputs(manifest, cells, owner['started_monotonic_ns'] + manifest['common_limit_ns'] - 20*10**9))
        manifest['_active_members'] = cells
    blocked = None
    cells = manifest.get('_active_members', canonical_members(manifest))
    def execute(cell, index):
        nonlocal blocked
        value = json.loads(path.read_text())
        started = time.monotonic_ns()
        retained_before = active_bytes(manifest['allocation'])
        load = [observe_load()]
        try:
            if blocked:
                raise ValueError(blocked)
            end = deadline(manifest, value, owner, cell, started)
        except ValueError as error:
            blocked = str(error)
            return {'id': cell['id'], 'cell': cell, 'status': 'held', 'complete': False, 'reason': blocked}
        def register(selected, attempt):
            current = json.loads(path.read_text())
            if manifest['scope'] == 'primary':
                from g_pair_selection import pair_allowance
                key, cap = pair_allowance(manifest, current, selected)
                current.setdefault('s50_pair_limits', {})[key] = cap
            current['inflight'] = {'cell': selected['id'], 'started_monotonic_ns': started,
                'started_at_ns': time.time_ns(), 'attempt': str(Path(attempt).resolve()), 'freeze_uuid': manifest['uuid']}
            write_budget(path, current)
        def monitor():
            load.append(observe_load())
            current_bytes = active_bytes(manifest['allocation'])
            if remaining_bytes is not None and current_bytes - baseline >= remaining_bytes:
                return 'selected phase retained growth exhausted'
            if current_bytes >= manifest['profile']['total_shared_retained_bytes'] - manifest['profile']['reserved_other_work_bytes']:
                return 'paired shared retained storage exhausted'
            return None
        def command(selected, attempt, end):
            command = [sys.executable, str(Path(__file__).with_name('g_runner.py')), '_pair_cell',
                       '--root', str(root), '--deadline-ns', str(end)]
            if manifest.get('_selection_path'):
                command += ['--selection', manifest['_selection_path']]
            return command
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
                       guardian_owner=owner, selection=manifest.get('_selection_path'),
                       retained_growth_bytes=max(0, active_bytes(manifest['allocation']) - retained_before))
        if (Path(outcome['attempt']) / 'result.json').exists():
            actual = json.loads((Path(outcome['attempt']) / 'result.json').read_text())
            outcome['store'] = actual.get('store')
        return outcome
    return drive_members(manifest, root, execute)


def finish(manifest, rows):
    """Record native comparison facts; the outer guardian must finalize before retirement."""
    from g_pair import report
    root = Path(manifest['root'])
    result = report(root)
    cached = root / 'results/report.json'
    previous = json.loads(cached.read_text()) if cached.exists() else {}
    result['retired_copies'] = previous.get('retired_copies', [])
    recoveries = previous.get('report_recoveries', []) + ([manifest['_report_recovery']] if '_report_recovery' in manifest else [])
    if recoveries:
        result['report_recoveries'] = recoveries
    if '_reassessment' in manifest:
        result['reassessment'] = manifest['_reassessment']
    active = [cell['id'] for cell in manifest.get('_active_members', [])]
    window = bool(active) and all(pair['complete'] for name in ('primary', 'diagnostics')
        for pair in result['comparisons'][name] if set(pair['members']).issubset(active))
    complete = result['calibration_complete'] if manifest['scope'] == 'calibration' else window or result['complete']
    result.update(active_members=active, native_window_complete=window,
                  selection=manifest.get('_selection_record'), controller_source_sha=manifest.get('controller_source_sha'))
    budget = json.loads((Path(manifest['allocation']) / 'g-budget.json').read_text())
    result.update(native_primary_complete=result['complete'], native_calibration_complete=result['calibration_complete'],
                  complete=False, calibration_complete=False, guardian_owner=budget['s50_guardian'])
    (root / 'results/report.json').write_text(json.dumps(result, indent=2, allow_nan=False) + '\n')
    return complete


def retire_qualified(root, outcome=None, *, manifest=None):
    """Retire disposable copies only after final report and original guardian qualification."""
    from g_pair import CALIBRATION_ID, retire_pair
    from g_pair_report import published_report
    from m_guard import Accounting
    root = Path(root)
    manifest = manifest if manifest is not None else json.loads((root / 'manifest.json').read_text())
    cached = root / 'results/report.json'
    if not manifest['retire_complete_copies'] or not cached.exists():
        return
    allocation = Path(manifest['allocation'])
    with (allocation / 'g-campaign.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        budget = json.loads((allocation / 'g-budget.json').read_text())
        if any(budget.get(key) for key in ('inflight', 'm_guardian', 'd_guardian', 'closed_guardian', 's50_guardian')):
            return
        began = time.monotonic_ns()
        accounting = Accounting(allocation / 'g-budget.json', began, phase='S50')
        try:
            result = published_report(root)
            if not result['phase_qualified'] or outcome and any(
                    result['guardian'].get(key) != outcome.get(key)
                    for key in ('started_monotonic_ns', 'controller_pid')):
                return
            limits = manifest['calibration'] if manifest['scope'] == 'calibration' else manifest['primary_limits']
            common = (limits['phase_ns'] - limits['payload_ns'] if manifest['scope'] == 'calibration' else limits['common_ns'])
            end = began + min(common - (outcome or {}).get('accounting', {}).get('common_ns', 0),
                manifest['profile']['total_shared_ns'] - manifest['profile']['reserved_other_work_ns'] - budget['used_ns'])
            if outcome:
                end = min(end, outcome['started_monotonic_ns'] + limits['phase_ns'])
            credited = {row['id'] for name in ('primary', 'diagnostics', 'calibration')
                        for row in result[name] if row.get('credited')}
            previous = {row['id']: row for name in ('primary', 'diagnostics', 'calibration') for row in result[name]}
            retired = result.get('retired_copies', [])
            for name in (['calibration'] if manifest['scope'] == 'calibration' else ['primary', 'diagnostics']):
                for pair in result['comparisons'][name]:
                    required = (result['comparisons']['calibration'] if name == 'calibration' else
                        [item for item in result['comparisons']['diagnostics'] if item['origin_id'] == pair['origin_id']])
                    diagnostics_complete = all(set(item['members']).issubset(credited) for item in required)
                    if time.monotonic_ns() >= end:
                        break
                    outcomes = [previous[identifier] for identifier in pair['members']
                                if identifier in previous and not previous[identifier].get('reference')]
                    retired += retire_pair(root / 'results', outcomes, enabled=True,
                        comparison=pair['complete'] and set(pair['members']).issubset(credited),
                        diagnostics_complete=diagnostics_complete)
            # Preserve native credit facts: published_report derives final credit on each read.
            pending = json.loads(cached.read_text())
            pending['retired_copies'] = retired
            write_budget(cached, pending)
        finally:
            charged = accounting.finish(uncertain=False, stopped_ns=time.monotonic_ns())['charged_ns']
            if manifest['scope'] == 'calibration':
                budget = accounting.read()
                budget['calibration_usage'][CALIBRATION_ID]['wall_ns'] += charged
                write_budget(accounting.path, budget)


def _recover_report(manifest, owner):
    from g_freeze import identify
    from g_pair import qualify_step, required_prefix
    from g_pair_report import comparisons, clean
    from g_pair_selection import apply
    root = Path(manifest['root'])
    original_path = Path(manifest['_report_recovery_selection']).resolve()
    original = json.loads(original_path.read_text())
    if (original.get('schema') != 'slice50-pair-adoption/v1'
            or original.get('native_source_sha') != manifest['source_sha']
            or original.get('compiler_inputs') != manifest['compiler_inputs']
            or 'report_recovery' in original.get('selection', {})):
        raise ValueError('report recovery original adoption differs')
    original_manifest = apply(manifest, original['selection'])
    cells = required_prefix(original_manifest, root)
    if len(cells) != 2 * original_manifest['primary_limits']['pairs']:
        raise ValueError('report recovery requires exact original whole-pair window')
    state = {row['id']: row for row in json.loads((root / 'results/campaign-state.json').read_text())}
    rows = [state.get(cell['id'], {}) for cell in cells]
    original_owner = original.get('guardian_owner') or {}
    for row in rows:
        bound = row.get('guardian_owner') or {}
        if (not clean(row) or row.get('exit') != 0 or row.get('child_exit') != 0 or row.get('error')
                or row.get('selection') != str(original_path)
                or any(bound.get(key) != original_owner.get(key) for key in ('pid', 'start', 'root', 'started_monotonic_ns'))
                or bound.get('controller_pid') != rows[0].get('guardian_owner', {}).get('controller_pid')
                or type(row.get('charged_ns')) is not int or not 0 < row['charged_ns'] < 900 * 10**9):
            raise ValueError('report recovery requires exact complete clean original members')
    receipts = [(path, json.loads(path.read_text())) for path in (root / 'guardians').glob('*/guardian.json')]
    matches = [(path, receipt) for path, receipt in receipts
               if receipt.get('started_monotonic_ns') == original_owner.get('started_monotonic_ns')
               and receipt.get('controller_pid') == rows[0]['guardian_owner'].get('controller_pid')]
    if len(matches) != 1:
        raise ValueError('report recovery original guardian unavailable')
    receipt_path, receipt = matches[0]
    accounting, boundary = receipt.get('accounting') or {}, receipt.get('boundary') or {}
    limits = original_manifest['primary_limits']
    budget = json.loads((Path(manifest['allocation']) / 'g-budget.json').read_text())
    if (receipt.get('exit') != 124 or receipt.get('stop') != 'shared cleanup reserve'
            or receipt.get('error') or receipt.get('cleanup_uncertain') is not False
            or boundary.get('status') != 'quiescent' or boundary.get('populated_after') is not False
            or boundary.get('removed_exact_empty_boundary') is not True
            or boundary.get('guardian_pid') != original_owner.get('pid')
            or boundary.get('guardian_start') != original_owner.get('start')
            or accounting.get('recovered_cell_ns') != 0
            or type(accounting.get('common_ns')) is not int
            or accounting['common_ns'] < limits['common_ns'] - 20 * 10**9
            or accounting.get('charged_ns') != accounting['common_ns'] + sum(row['charged_ns'] for row in rows)
            or type(receipt.get('stopped_monotonic_ns')) is not int
            or receipt['stopped_monotonic_ns'] + 20 * 10**9 >= original_owner['started_monotonic_ns'] + limits['phase_ns']
            or budget.get('inflight') or budget['used_ns'] + 20 * 10**9 >= original_manifest['profile']['total_shared_ns']
                - original_manifest['profile']['reserved_other_work_ns']):
        raise ValueError('report recovery requires post-native common-only strict guardian stop')
    for index in range(0, len(rows), 2):
        pair = rows[index:index+2]
        cap = budget.get('s50_pair_limits', {}).get(cells[index]['id'], limits['pair_ns'])
        if sum(row['charged_ns'] for row in pair) > min(cap, limits['pair_ns']):
            raise ValueError('report recovery original pair cap exceeded')
    identities = [identify(original_path), identify(receipt_path)]
    for cell, row in zip(cells, rows, strict=True):
        attempt = Path(row['attempt'])
        result = json.loads((attempt / 'result.json').read_text())
        steps = result.get('steps') or []
        if (result.get('complete') is not True or result.get('qualification_issues') != []
                or result.get('cell') != cell or result.get('freeze_uuid') != manifest['uuid']
                or result.get('scope') != 'primary' or result.get('selected_build') != manifest['build']['binaries'][cell['artifact']]
                or len(steps) != 1 or steps[0].get('exit') != 0 or qualify_step(steps[0], cell, manifest)):
            raise ValueError('report recovery native evidence incomplete')
        paths = [attempt / 'attempt.json', attempt / 'result.json']
        for step in steps:
            paths.extend(sorted(Path(step['output']).glob('*.json*')))
            if step.get('config'):
                paths.append(Path(step['config']))
        identities.extend(identify(path) for path in paths)
    if not all(pair['complete'] for pair in comparisons(cells, state)):
        raise ValueError('report recovery exact native comparison incomplete')
    return cells, {'members': [cell['id'] for cell in cells], 'guardian_owner': owner,
        'original_guardian': receipt, 'original_selection': str(original_path),
        'original_members': [{key: row[key] for key in ('id', 'attempt', 'selection', 'guardian_owner')} for row in rows],
        'native_source_sha': manifest['source_sha'], 'compiler_inputs': manifest['compiler_inputs'],
        'controller_source_sha': manifest['controller_source_sha'], 'identities': identities}


def control_entry(operation, root, options, admission_fd, *, selection=None):
    """Own freeze/run/report under the existing inherited guardian admission."""
    from g_pair import freeze
    from g_pair_freeze import validate
    from m_runtime import controller_admission
    owner = controller_admission(admission_fd, root)
    if operation in ('freeze', 'calibration'):
        selected = json.loads(Path(options).read_text())
        manifest = freeze(selected, root, scope='calibration' if operation == 'calibration' else 'primary')
    else:
        manifest = validate(root, selection=selection)
    if manifest.get('_report_recovery_selection') and operation != 'report':
        raise ValueError('recovery selection is report-only')
    path = Path(manifest['allocation']) / 'g-budget.json'
    if json.loads(path.read_text()).get('s50_guardian') != owner:
        raise RuntimeError('paired operation lacks actual selected original guardian')
    if selection is not None:
        original = manifest['_selection_record']['guardian_owner']
        if any(original.get(key) != owner.get(key) for key in ('pid', 'started_monotonic_ns', 'root')):
            raise RuntimeError('selection is outside its actual admitting guardian')
    if operation == 'freeze':
        return True
    if operation == 'report':
        if manifest.get('_report_recovery_selection'):
            manifest['_active_members'], manifest['_report_recovery'] = _recover_report(manifest, owner)
        cached = Path(root) / 'results/report.json'
        if selection is not None and cached.exists():
            from g_freeze import identify
            from g_pair_report import published_report
            previous = json.loads(cached.read_text())
            active = previous.get('active_members', [])
            cells = {cell['id']: cell for name in ('primary', 'diagnostics') for cell in manifest['plan'][name]}
            pairs = [[cell['id'] for cell in manifest['plan'][name][index:index+2]]
                     for name in ('primary', 'diagnostics') for index in range(0, len(manifest['plan'][name]), 2)]
            if (not isinstance(active, list) or any(not isinstance(identifier, str) for identifier in active)
                    or len(active) != len(set(active)) or any(identifier not in cells for identifier in active)
                    or any(active[index:index+2] not in pairs for index in range(0, len(active), 2))):
                raise ValueError('retained report has invalid active members')
            if not manifest.get('_report_recovery_selection'):
                manifest['_active_members'] = [cells[identifier] for identifier in active]
            previous_guardian = published_report(root).get('guardian')
            history = cached.parent / 'report-history'
            history.mkdir(exist_ok=True)
            snapshot = history / f"{owner['started_monotonic_ns']}-{owner['pid']}.json"
            with snapshot.open('xb') as stream:
                stream.write(cached.read_bytes())
            snapshot.chmod(0o444)
            manifest['_reassessment'] = {'previous_report': identify(snapshot),
                'previous_guardian': previous_guardian, 'controller_source_sha': manifest['controller_source_sha'],
                'native_source_sha': manifest['source_sha'], 'compiler_inputs': manifest['compiler_inputs']}
        state = Path(root) / 'results/campaign-state.json'
        finish(manifest, json.loads(state.read_text()) if state.exists() else [])
        return True
    return finish(manifest, control(manifest, owner))
