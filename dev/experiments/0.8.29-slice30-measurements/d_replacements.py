"""Explicit reviewed100k pair and single300k C continuation; no general retry policy."""

import copy
import json
from pathlib import Path

from d_controller import _qualified_record, cell_limit_ns


ORDER = ('d-t-overlap-n100000-r1', 'd-c-overlap-n100000-r1')
CURRENT = ('d-c-overlap-n100000-r0', 'd-t-overlap-n100000-r0')
HELD_C = 'd-c-held-n300000-r1'
HELD_CURRENT = ('d-c-held-n300000-r0', 'd-t-held-n300000-r0', 'd-t-held-n300000-r1')
HELD_CUMULATIVE_NS = 7500*10**9


def _held_raw(selection, root):
    identity = selection.get('old_attempt')
    if (not isinstance(identity,dict) or identity.get('attempt_id') != 'attempt-0001'
            or not isinstance(identity.get('attempt'),str)):
        return False
    original = Path(identity['attempt']).resolve()
    if original != Path(root).resolve()/HELD_C/'attempt-0001' or not original.is_dir():
        return False
    try:
        cell = json.loads((original/'cell.json').read_text())
    except (OSError,ValueError):
        return False
    return isinstance(cell,dict) and all(cell.get(key) == value for key,value in {
        'id':HELD_C,'n':300000,'mode':'held','repeat':1,'layer':'C',
        'cell_ns':4500*10**9,'attempt_id':'attempt-0001'}.items())


def guardian_limits(root, budget, active, prior_cells, whole_cell):
    """Return expanded cumulative and unchanged fresh limits only for owned C300k.

    Missing or mismatching consumed-token/raw evidence retains the ordinary
    cumulative guard. This never admits or consumes an attempt.
    """
    ordinary = (whole_cell,None)
    if (active.get('cell') != HELD_C or active.get('replacement') is not True
            or whole_cell != 4500*10**9):
        return ordinary
    token = budget.get('d_replacements',{}).get(HELD_C) or {}
    if not isinstance(token,dict):
        return ordinary
    selected = token.get('selection') or {}
    if not isinstance(selected,dict):
        return ordinary
    old = selected.get('record') or {}
    if not isinstance(old,dict):
        return ordinary
    prior = prior_cells.get(HELD_C)
    expected = str(Path(root).resolve()/'attempts'/HELD_C/'attempt-0002')
    if (type(prior) is not int or prior <= 0 or selected.get('charged_ns') != prior
            or type(selected.get('cumulative_limit_ns')) is not int
            or selected['cumulative_limit_ns'] != HELD_CUMULATIVE_NS
            or selected.get('next_attempt_id') != 'attempt-0002'
            or old.get('id') != HELD_C or old.get('status') != 'interrupted'
            or old.get('elapsed_ns') != prior or old.get('attempt_id') is not None
            or old.get('attempt') is not None
            or token.get('attempt_id') != 'attempt-0002' or token.get('attempt') != expected
            or active.get('attempt_id') != token['attempt_id'] or active.get('attempt') != expected
            or active.get('started_monotonic_ns') != token.get('started_monotonic_ns')
            or not _held_raw(selected,Path(root)/'attempts')):
        return ordinary
    return HELD_CUMULATIVE_NS,whole_cell


def reference(selection):
    """Return the immutable old-attempt reference retained on its replacement."""
    old = selection['record']
    identity = selection.get('old_attempt',old)
    return {'attempt_id':identity['attempt_id'], 'attempt':identity['attempt'], 'status':old['status'],
            'charged_ns':selection['charged_ns']}


def plan(records, budget, review):
    """Admit only the named continuations, preserving consumption and current pins."""
    def held(reason):
        return {'status':'not_run_stage_review', 'reason':reason}
    selections = review.get('replacements')
    if not isinstance(selections,dict):
        return held('explicit named ordinary replacement selection required')
    if (review.get('n'),review.get('mode'),set(selections)) == (100000,'overlap',set(ORDER)):
        n,mode,order,current,statuses = 100000,'overlap',ORDER,CURRENT,('failed','interrupted')
    elif (review.get('n'),review.get('mode'),set(selections)) == (300000,'held',{HELD_C}):
        n,mode,order,current,statuses = 300000,'held',(HELD_C,),HELD_CURRENT,('interrupted',)
    else:
        return held('only the exact100k pair or single300k held C continuation may be selected')
    fresh_limit = cell_limit_ns(n,budget.get('cell_limits_ns'))
    pins = review.get('qualified_current_attempts')
    if not isinstance(pins,dict) or set(pins) != set(current):
        return held('qualified current attempt identities are unavailable')
    for key in current:
        row = records.get(key)
        if (not _qualified_record(row,n,'C' if key.startswith('d-c-') else 'T',mode)
                or pins[key] != {name:row.get(name) for name in ('attempt_id','attempt')}):
            return held('qualified current evidence differs')
    pending, remaining, cumulative_limits = [], {}, {}
    tokens = budget.get('d_replacements',{})
    for key,status in zip(order,statuses):
        selected = selections[key]
        old = selected.get('record') if isinstance(selected,dict) else None
        spent = budget.get('cell_used_ns',{}).get(key)
        if (not isinstance(old,dict) or old.get('id') != key or old.get('status') != status
                or selected.get('next_attempt_id') != 'attempt-0002'
                or type(selected.get('charged_ns')) is not int or selected['charged_ns'] <= 0
                or type(spent) is not int or spent < 0):
            return held('original interrupted/failed attempt and cumulative charge must be explicit')
        cumulative_limit = fresh_limit
        if key == HELD_C:
            if (fresh_limit != 4500*10**9 or type(selected.get('cumulative_limit_ns')) is not int
                    or selected['cumulative_limit_ns'] != HELD_CUMULATIVE_NS
                    or old.get('attempt_id') is not None or old.get('attempt') is not None
                    or old.get('elapsed_ns') != selected['charged_ns']):
                return held('single300k C continuation requires exact fresh/cumulative limits and old TERM row')
            paths = [records[name].get('attempt') for name in current]
            if any(not isinstance(path,str) for path in paths):
                return held('qualified held raw paths are unavailable')
            roots = {Path(path).resolve().parent.parent for path in paths}
            if len(roots) != 1 or not _held_raw(selected,roots.pop()):
                return held('original interrupted C300k raw identity/configuration differs')
            cumulative_limit = HELD_CUMULATIVE_NS
            cumulative_limits[key] = cumulative_limit
        elif (old.get('attempt_id') != 'attempt-0001' or not isinstance(old.get('attempt'),str)
                or 'old_attempt' in selected or 'cumulative_limit_ns' in selected):
            return held('original100k attempt identity and unchanged allowance required')
        token = tokens.get(key)
        current_row = records.get(key)
        if token is None:
            if current_row != old or spent != selected['charged_ns']:
                return held('original failed/interrupted attempt or charge differs')
            pending.append(key)
            remaining[key] = min(fresh_limit,cumulative_limit-spent)
        elif (token.get('selection') != selected or current_row is None
                or current_row.get('attempt_id') != 'attempt-0002'
                or current_row.get('attempt') != token.get('attempt')
                or current_row.get('replacement_of') != reference(selected)
                or current_row.get('prior_attempts') != [old]
                or not _qualified_record(current_row,n,'T' if key.startswith('d-t-') else 'C',mode)
                or not 0 <= current_row.get('elapsed_ns',-1) < fresh_limit
                or spent < selected['charged_ns']+current_row.get('elapsed_ns',0)
                or spent >= cumulative_limit):
            return held('selected replacement is already consumed and does not qualify; reassessment required')
    result = {'status':'admit', 'pending_cells':pending, 'remaining_ns':remaining,
              'replacement_selection':copy.deepcopy(selections)}
    if cumulative_limits:
        result['cumulative_limits_ns'] = cumulative_limits
    return result


def consume(cell_id, records, budget, selection, root, started_ns):
    """Consume one fresh attempt before payload entry under the caller's custody."""
    if cell_id not in (*ORDER,HELD_C) or cell_id in budget.get('d_replacements',{}):
        raise RuntimeError('ordinary replacement is already consumed or outside the named pair')
    old = selection['record']
    directory = Path(root).resolve()/cell_id
    identity = selection.get('old_attempt',old)
    if (records.get(cell_id) != old or Path(identity['attempt']).resolve() != directory/'attempt-0001'
            or sorted(path.name for path in directory.glob('attempt-*')) != ['attempt-0001']
            or not (directory/'attempt-0001').is_dir()):
        raise RuntimeError('original replacement raw identity is unavailable or next attempt already exists')
    if cell_id == HELD_C and not _held_raw(selection,root):
        raise RuntimeError('original interrupted C300k raw configuration differs')
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
