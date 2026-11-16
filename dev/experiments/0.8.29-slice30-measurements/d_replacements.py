"""The two explicitly reviewed ordinary100k replacements; no general retry policy."""

import copy
from pathlib import Path

from d_controller import _qualified_record, cell_limit_ns


ORDER = ('d-t-overlap-n100000-r1', 'd-c-overlap-n100000-r1')
CURRENT = ('d-c-overlap-n100000-r0', 'd-t-overlap-n100000-r0')


def reference(selection):
    """Return the immutable old-attempt reference retained on its replacement."""
    old = selection['record']
    return {'attempt_id':old['attempt_id'], 'attempt':old['attempt'], 'status':old['status'],
            'charged_ns':selection['charged_ns']}


def plan(records, budget, review):
    """Admit only the named pair, preserving consumed attempts and qualified pins."""
    def held(reason):
        return {'status':'not_run_stage_review', 'reason':reason}
    selections = review.get('replacements')
    if ((review.get('n'),review.get('mode')) != (100000,'overlap')
            or not isinstance(selections,dict) or set(selections) != set(ORDER)):
        return held('only the exact ordinary100k replacement pair may be selected')
    pins = review.get('qualified_current_attempts')
    if not isinstance(pins,dict) or set(pins) != set(CURRENT):
        return held('qualified current C-r0/T-r0 attempt identities are unavailable')
    for key in CURRENT:
        row = records.get(key)
        if (not _qualified_record(row,100000,'C' if key == CURRENT[0] else 'T','overlap')
                or pins[key] != {name:row.get(name) for name in ('attempt_id','attempt')}):
            return held('qualified current C-r0/T-r0 evidence differs')
    pending, remaining = [], {}
    tokens = budget.get('d_replacements',{})
    for key,status in zip(ORDER,('failed','interrupted')):
        selected = selections[key]
        old = selected.get('record') if isinstance(selected,dict) else None
        spent = budget.get('cell_used_ns',{}).get(key)
        if (not isinstance(old,dict) or old.get('id') != key or old.get('status') != status
                or old.get('attempt_id') != 'attempt-0001' or not isinstance(old.get('attempt'),str)
                or selected.get('next_attempt_id') != 'attempt-0002'
                or type(selected.get('charged_ns')) is not int or selected['charged_ns'] <= 0
                or type(spent) is not int or spent < 0):
            return held('original failed/interrupted attempt and cumulative charge must be explicit')
        token = tokens.get(key)
        current = records.get(key)
        if token is None:
            if current != old or spent != selected['charged_ns']:
                return held('original failed/interrupted attempt or charge differs')
            pending.append(key)
            remaining[key] = cell_limit_ns(100000,budget.get('cell_limits_ns'))-spent
        elif (token.get('selection') != selected or current is None
                or current.get('attempt_id') != 'attempt-0002'
                or current.get('attempt') != token.get('attempt')
                or current.get('replacement_of') != reference(selected)
                or current.get('prior_attempts') != [old]
                or not _qualified_record(current,100000,'T' if key == ORDER[0] else 'C','overlap')
                or spent < selected['charged_ns']+current.get('elapsed_ns',0)
                or spent >= cell_limit_ns(100000,budget.get('cell_limits_ns'))):
            return held('selected replacement is already consumed and does not qualify; reassessment required')
    return {'status':'admit', 'pending_cells':pending, 'remaining_ns':remaining,
            'replacement_selection':copy.deepcopy(selections)}


def consume(cell_id, records, budget, selection, root, started_ns):
    """Consume one fresh attempt before payload entry under the caller's custody."""
    if cell_id not in ORDER or cell_id in budget.get('d_replacements',{}):
        raise RuntimeError('ordinary replacement is already consumed or outside the named pair')
    old = selection['record']
    directory = Path(root).resolve()/cell_id
    if (records.get(cell_id) != old or Path(old['attempt']).resolve() != directory/'attempt-0001'
            or sorted(path.name for path in directory.glob('attempt-*')) != ['attempt-0001']
            or not (directory/'attempt-0001').is_dir()):
        raise RuntimeError('original replacement raw identity is unavailable or next attempt already exists')
    token = {'selection':copy.deepcopy(selection), 'attempt_id':'attempt-0002',
             'attempt':str(directory/'attempt-0002'), 'started_monotonic_ns':started_ns}
    budget.setdefault('d_replacements',{})[cell_id] = token
    return token


def with_history(row, token):
    """Keep the complete old record on a new selected result or interruption."""
    if row.get('attempt_id') != token['attempt_id'] or row.get('attempt') != token['attempt']:
        raise RuntimeError('selected replacement returned another attempt identity')
    return dict(row, replacement_of=reference(token['selection']),
                prior_attempts=[copy.deepcopy(token['selection']['record'])])
