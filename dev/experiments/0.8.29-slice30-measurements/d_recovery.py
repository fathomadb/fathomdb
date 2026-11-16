"""Explicit recovery of one physically terminated D guardian; never dispatch work."""

import fcntl
import json
from pathlib import Path
import subprocess
import time

from m_guard import write_budget


def observe_terminated(options):
    """Require absent original actors, exact removed boundaries and same-boot stop proof.

    Live and reused PID slots both refuse recovery. A transient unit's unloaded
    timestamps are insufficient: the exact original journal start/stop is required.
    """
    owner = options['guardian']
    for pid_key, start_key in (('pid', 'start'), ('controller_pid', 'controller_start')):
        pid, start = owner[pid_key], owner[start_key]
        try:
            fields = Path(f'/proc/{pid}/stat').read_text().rsplit(') ', 1)[1].split()
        except FileNotFoundError:
            continue
        state = 'live' if int(fields[19]) == start else 'reused'
        raise RuntimeError(f'{state} original D actor PID {pid} blocks recovery')
    expected = options['journal']
    boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip().replace('-', '')
    if expected['boot_id'].replace('-', '') != boot:
        raise RuntimeError('original D stop proof is not from this boot')
    unit = options['unit']
    if '/' in unit or not unit.endswith('.service'):
        raise ValueError('one original D service name required')
    shown = subprocess.run(['systemctl', '--user', 'show', unit, '--property=ActiveState',
        '--property=MainPID', '--property=ControlGroup'], capture_output=True, text=True, timeout=10)
    properties = dict(line.split('=', 1) for line in shown.stdout.splitlines() if '=' in line)
    if (properties.get('ActiveState') not in ('inactive', 'failed')
            or properties.get('MainPID') != '0' or properties.get('ControlGroup') != ''):
        raise RuntimeError('original D service is not terminal and empty')
    journal = subprocess.run(['journalctl', '--user', '-u', unit, '-o', 'json', '--no-pager'],
        capture_output=True, text=True, check=True, timeout=10)
    episodes = {'start': [], 'stop': []}
    for line in journal.stdout.splitlines():
        row = json.loads(line)
        kind = row.get('JOB_TYPE')
        if (row.get('USER_UNIT') == unit and row.get('_BOOT_ID') == boot
                and row.get('JOB_RESULT') == 'done' and kind in episodes):
            episodes[kind].append((int(row['__MONOTONIC_TIMESTAMP'])*1000,
                                   row.get('USER_INVOCATION_ID')))
    if episodes != {'start': [(expected['start_ns'], expected['invocation'])],
                    'stop': [(expected['stop_ns'], expected['invocation'])]}:
        raise RuntimeError('original D journal episode is ambiguous or unavailable')
    if not expected['start_ns'] <= owner['started_monotonic_ns'] < expected['stop_ns'] <= time.monotonic_ns():
        raise RuntimeError('original D historical stop clock is inconsistent')
    boundary = options['boundary']
    path = Path(boundary['path'])
    if (str(path) != '/sys/fs/cgroup'+boundary['relative_path']
            or path.name != boundary['name'] or path.parent.name != unit
            or boundary['guardian_pid'] != owner['pid'] or boundary['guardian_start'] != owner['start']):
        raise RuntimeError('foreign original D boundary identity')
    if path.exists() or path.parent.exists():
        raise RuntimeError('original D boundary or outer service boundary still exists')


def _read(path):
    return json.loads(Path(path).read_text())


def _historical(options, root, budget, records):
    prefix = options['prefix_attempts']
    if _read(root/'d-config.json')['review']['prefix_attempts'] != prefix:
        raise RuntimeError('original D prefix identity is unavailable')
    if any(records[key].get('status') != 'complete' or records[key].get('attempt_id') != attempt
           for key, attempt in prefix.items()):
        raise RuntimeError('original D qualified prefix differs')
    paths = [Path(path) for path in options['prior_guardians']]
    if (len(paths) != len(set(paths)) or set(paths) != set((root/'guardians').glob('*/guardian.json'))):
        raise RuntimeError('original completed D guardian accounting is unavailable')
    receipts = [_read(path) for path in paths]
    if any(row.get('exit') != 0 or row.get('cleanup_uncertain') is not False for row in receipts):
        raise RuntimeError('prior D guardian accounting is not qualified')
    prior_common = sum(row['accounting']['charged_ns'] for row in receipts) - sum(
        records[key]['elapsed_ns'] for key in prefix)
    if prior_common < 0:
        raise RuntimeError('prior D common accounting is inconsistent')
    cells = budget.get('cell_used_ns', {})
    if any(cells.get(key) != records[key]['elapsed_ns'] for key in prefix):
        raise RuntimeError('original D prefix charges differ')
    spec = _read(Path(__file__).with_name('d-cells.json'))
    selected = next(cell for cell in spec['cells'] if cell['id'] == options['inflight']['cell'])
    cohort = {cell['id'] for cell in spec['cells'] if (cell['n'], cell['mode']) ==
              (selected['n'], selected['mode'])}
    if set(records)-set(prefix)-cohort:
        raise RuntimeError('foreign D attempted work after original prefix')
    stop = options['journal']['stop_ns']
    began = options['guardian']['started_monotonic_ns']
    active_start = options['inflight']['started_monotonic_ns']
    if not began <= active_start < stop:
        raise RuntimeError('interrupted D cell clock is inconsistent')
    common = budget.get('common_used_ns', {}).get('D', 0)-prior_common
    committed = sum(cells.get(key, 0) for key in cohort)
    active_wall = stop-active_start
    cell_charge = active_wall-cells.get(selected['id'], 0)
    missing = stop-began-committed-common
    common_charge = missing-cell_charge
    if min(common, committed, active_wall, cell_charge, common_charge) < 0:
        raise RuntimeError('historical D cell/common conservation is inconsistent')
    return {'cell_ns': cell_charge, 'common_ns': common_charge,
            'active_wall_ns': active_wall, 'stopped_monotonic_ns': stop}


def recover(options, *, task, observe=None, clock=None, after_publish=None):
    """Recover one stopped D attempt with charge-once durable publication.

    ``observe`` and ``clock`` are literal-fixture seams for source controls; the
    public CLI always uses real host observations. No native execution follows.
    Retain recovery custody until state, exact marker removal and budget finish.
    """
    clock = clock or time.monotonic_ns
    began = clock()
    task = Path(task).resolve()
    allocation, root = Path(options['allocation']).resolve(), Path(options['root']).resolve()
    if not root.is_relative_to(allocation) or not allocation.is_relative_to(task):
        raise ValueError('original D recovery paths must belong to the owned task')
    key = f"{options['guardian']['pid']}:{options['guardian']['start']}"
    state_path, marker = root/'d-state.json', task/'g-containment-state.json'
    publish = after_publish or (lambda _stage: None)
    with (allocation/'g-campaign.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        (observe or observe_terminated)(options)
        budget = _read(allocation/'g-budget.json')
        if budget.get('schema') != 'slice30-gd-budget/v1':
            raise ValueError('original shared D allocation required')
        entry = budget.get('d_recoveries', {}).get(key)
        if any(budget.get(field) for field in ('m_guardian', 's50_guardian', 'closed_guardian')):
            raise RuntimeError('foreign shared custody blocks D recovery')
        expected_owner = dict(options['guardian'], status='recovery_pending') if entry else options['guardian']
        expected_active = dict(options['inflight'], status='recovery_pending') if entry else options['inflight']
        if entry and entry['identity'] != options:
            raise RuntimeError('different original D recovery identity')
        if entry and entry['state'] == 'complete':
            if budget.get('d_guardian') or budget.get('inflight') or marker.exists():
                raise RuntimeError('foreign custody or boundary after completed D recovery')
            return {'complete': True, 'recovery': entry, 'already_complete': True}
        if budget.get('d_guardian') != expected_owner or budget.get('inflight') != expected_active:
            raise RuntimeError('foreign D custody differs from original recovery')
        if marker.exists():
            if _read(marker) != options['boundary']:
                raise RuntimeError('foreign containment boundary blocks D recovery')
        elif entry is None:
            raise RuntimeError('original D boundary marker is unavailable')
        records = _read(state_path)
        active = options['inflight']['cell']
        attempt = root/'attempts'/active/options['attempt_id']
        if not attempt.is_dir():
            raise RuntimeError('original interrupted D raw attempt is unavailable')
        if active in records and (not entry or records[active].get('status') != 'interrupted'):
            raise RuntimeError('existing D attempted result cannot be replaced by recovery')
        if entry is None:
            historical = _historical(options, root, budget, records)
            entry = {'identity': options, 'state': 'publishing', 'historical': historical,
                     'recovery_common_ns': 0}
            budget.setdefault('d_recoveries', {})[key] = entry
            budget['used_ns'] += historical['cell_ns']+historical['common_ns']
            usage = budget.setdefault('cell_used_ns', {})
            usage[active] = usage.get(active, 0)+historical['cell_ns']
            common = budget.setdefault('common_used_ns', {})
            common['D'] = common.get('D', 0)+historical['common_ns']
            budget['d_guardian'], budget['inflight'] = (
                dict(options['guardian'], status='recovery_pending'),
                dict(options['inflight'], status='recovery_pending'))
            write_budget(allocation/'g-budget.json', budget)
        try:
            publish('budget')
            records[active] = {'id': active, 'attempt_id': options['attempt_id'], 'attempt': str(attempt),
                'status': 'interrupted', 'error': 'original D service terminated before result publication',
                'elapsed_ns': entry['historical']['active_wall_ns'], 'cleanup_uncertain': False,
                'live_owned_pids': [], 'native': None, 'recovery_key': key,
                'stopped_monotonic_ns': options['journal']['stop_ns']}
            write_budget(state_path, records)
            publish('state')
            if marker.exists():
                if _read(marker) != options['boundary']:
                    raise RuntimeError('foreign containment boundary changed during recovery')
                marker.unlink()
            publish('marker')
            entry['state'] = 'complete'
            budget['d_guardian'] = budget['inflight'] = None
            write_budget(allocation/'g-budget.json', budget)
            publish('complete')
        finally:
            elapsed = clock()-began
            if elapsed < 0:
                raise RuntimeError('D recovery monotonic clock moved backwards')
            budget['used_ns'] += elapsed
            budget['common_used_ns']['D'] += elapsed
            entry['recovery_common_ns'] += elapsed
            write_budget(allocation/'g-budget.json', budget)
        return {'complete': True, 'recovery': entry, 'already_complete': False}
