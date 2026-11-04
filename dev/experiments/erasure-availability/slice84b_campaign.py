"""Screen both Slice84b targets once: BASELINE CANDIDATE MASTERS NEW_OUTPUT."""
import json
from pathlib import Path
import sys
import time
from campaign import digest, events_at, remove_replica_files, safe_child, save
from phase2_campaign import summarize
from slice84_campaign import assess_family, freeze


def cells():
    """Retain screening as repetition zero; no diagnostics are needed after VM attribution."""
    rows = []
    for rep in range(3):
        for family in ('node', 'edge', 'property'):
            for arm in (('candidate', 'baseline') if rep % 2 else ('baseline', 'candidate')):
                for mode in ('erase', 'control'):
                    rows.append(dict(id=len(rows), phase='core', family=family, repetition=rep,
                                     arm=arm, mode=mode, write_hz=20 if family == 'property' else 100))
    for family in ('node', 'edge', 'property'):
        for arm in ('baseline', 'candidate'):
            rows.append(dict(id=len(rows), phase='idle', family=family, repetition=0,
                             arm=arm, mode='idle', write_hz=20 if family == 'property' else 100))
    return rows


def screen(records, family):
    """A valid first pair improves duration and preserves every read/write service guard."""
    reasons = []
    pair = {}
    for arm in ('baseline', 'candidate'):
        for mode in ('erase', 'control'):
            matches = [r for r in records if r['cell']['family'] == family and r['cell']['arm'] == arm
                       and r['cell']['mode'] == mode and r['cell']['phase'] == 'core' and r['cell']['repetition'] == 0]
            if len(matches) != 1 or not matches[0].get('complete'):
                reasons.append(f'{arm}/{mode}: missing or incomplete')
                continue
            row = matches[0]
            if mode == 'erase':
                pair[arm] = row
                if row['result'].get('native_error') is not None:
                    reasons.append(f'{arm}: native refusal')
            for stream in ('read', 'write'):
                s = row.get('streams', {}).get(stream)
                if s is None or any(s[k] for k in ('failed', 'unfinished', 'unknown', 'accounting_errors')):
                    reasons.append(f'{arm}/{mode}/{stream}: invalid')
                elif mode == 'control' and (s['completed'] != s['scheduled'] or s['engine_refused'] or s['generator_missed']):
                    reasons.append(f'{arm}/{stream}: failed control')
    if len(pair) == 2:
        b, c = pair['baseline'], pair['candidate']
        if c['result']['native_ns'] >= b['result']['native_ns']:
            reasons.append('duration did not improve')
        for stream in ('read', 'write'):
            if c['streams'][stream]['completed'] < b['streams'][stream]['completed']:
                reasons.append(f'fewer {stream} completions')
            if c['streams'][stream]['engine_refused'] > b['streams'][stream]['engine_refused']:
                reasons.append(f'additional {stream} refusals')
    return dict(qualified=not reasons, reasons=reasons)


def valid_cell(row):
    """Physical recovery after a refused public call cannot qualify any erase cell."""
    return (row.get('complete') and set(row.get('streams', {})) == {'read', 'write'}
            and all(not stream[k] for stream in row['streams'].values()
                    for k in ('failed', 'unfinished', 'unknown', 'accounting_errors'))
            and (row['cell']['mode'] == 'control' or row['result'].get('native_error') is None))


def main():
    """Freeze and execute each declared cell at most once within one1800-second allowance."""
    begun = time.monotonic()
    baseline, candidate, masters, root = map(Path, sys.argv[1:])
    root.mkdir()  # Existing evidence cannot be replaced or resumed.
    owned = root.parent
    artifacts = {a: freeze(p) for a, p in [('baseline', baseline), ('candidate', candidate)]}
    inputs = {f: freeze(masters / f'{f}-default.sqlite') for f in ('node', 'edge', 'property')}
    manifest = dict(cells=cells(), artifacts=artifacts, masters=inputs, budget_seconds=1800,
                    rss_limit_bytes=8 << 30, scratch_limit_bytes=12 << 30, cell_seconds=90,
                    started_unix_s=time.time(), source_controller=digest(Path(__file__)))
    records = []
    stopped = None
    save(root / 'manifest.json', manifest)
    for cell in manifest['cells']:
        if stopped or time.monotonic() - begun >= 1800:
            records.append(dict(cell=cell, complete=False, stop=stopped or 'budget'))
        else:
            target = root / f"{cell['id']:02d}-{cell['family']}-{cell['arm']}-{cell['mode']}"
            args = [artifacts[cell['arm']]['path'], inputs[cell['family']]['path'], str(target),
                    cell['family'], cell['mode'], str(cell['write_hz'])]
            row = safe_child(args, root / f"{cell['id']:02d}.log", owned,
                             min(begun + 1800, time.monotonic() + 90))
            row.update(cell=cell, command=args, raw=str(target), complete=False)
            try:
                result_path = target / 'result.json'
                result = json.loads(result_path.read_text()) if result_path.exists() else None
                events = events_at(target / 'events.jsonl')
                complete = row['exit_code'] == 0 and not row.get('stop') and not row.get('deadline_overshoot_s', 0) and bool(result and result['verified'])
                streams = {s: summarize(events, s, complete, 0 if cell['mode'] == 'idle' else
                                       1500 if s == 'read' else 15 * cell['write_hz']) for s in ('read', 'write')}
                row.update(result=result, streams=streams, complete=complete)
            except Exception as error:
                row.update(stop='accounting_error', error=repr(error))
            records.append(row)
            remove_replica_files(target)
            if not valid_cell(row):
                stopped = 'incomplete cell'
            elif cell['phase'] == 'core' and cell['repetition'] == 0 and cell['arm'] == 'candidate' and cell['mode'] == 'control':
                if not screen(records, cell['family'])['qualified']:
                    stopped = 'failed screening'
        save(root / 'records.json', records)
        manifest['active_seconds'] = time.monotonic() - begun
        save(root / 'manifest.json', manifest)
    for a in list(artifacts.values()) + list(inputs.values()):
        if digest(Path(a['path'])) != a['sha256']:
            stopped = 'frozen artifact changed'
    assessment = dict(stopped=stopped, active_seconds=time.monotonic() - begun,
                      screen={f: screen(records, f) for f in inputs},
                      core={f: assess_family(records, f) for f in inputs})
    assessment['qualified'] = not stopped and assessment['active_seconds'] <= 1800 and all(valid_cell(r) for r in records) and all(v['qualified'] for v in assessment['core'].values())
    save(root / 'assessment.json', assessment)
    print(json.dumps(assessment))


if __name__ == '__main__':
    main()
