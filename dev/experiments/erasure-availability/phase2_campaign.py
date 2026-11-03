"""Frozen paired native cleanup campaign; run baseline/candidate serially."""
import hashlib
import json
import math
from pathlib import Path
import sys
import time
from campaign import child, save


def summarize(events, stream, complete, expected=1500):
    """Reconcile all offers and qualify only fully completed streams."""
    offer_rows = [e for e in events if e['kind'] == 'offer' and e['stream'] == stream]
    finish_rows = [e for e in events if e['kind'] == 'finish' and e['stream'] == stream]
    offers = {e['id']: e for e in offer_rows}
    finishes = {e['id']: e for e in finish_rows}
    invalid = len(offer_rows)-len(offers)+len(finish_rows)-len(finishes)
    invalid += sum(not isinstance(i, int) or not 0 <= i < expected or e['state'] not in ('issued', 'generator_missed') for i, e in offers.items())
    for i, event in finishes.items():
        offer = offers.get(i, {})
        if offer.get('state') != 'issued' or event['state'] not in ('completed', 'engine_refused', 'failed'):
            invalid += 1
            continue
        invalid += int(not (offer['due_ns'] <= offer['issued_ns'] <= event['call_start_ns'] <= event['finished_ns']))
        invalid += int(event['service_ns'] != event['finished_ns']-event['call_start_ns'])
        invalid += int(event['offer_latency_ns'] != event['finished_ns']-offer['due_ns'])
    counts = {key: sum(e['state'] == key for e in finishes.values()) for key in ('completed', 'engine_refused', 'failed')}
    counts.update(scheduled=expected, issued=sum(e['state'] == 'issued' for e in offers.values()),
                  generator_missed=sum(e['state'] == 'generator_missed' for e in offers.values()),
                  unfinished=sum(e['state'] == 'issued' and i not in finishes for i, e in offers.items()),
                  unknown=max(0, expected-len(offers)), accounting_errors=invalid)
    qualified = complete and counts['completed'] == expected and expected >= 1000 and invalid == 0
    good = [e for e in finishes.values() if e['state'] == 'completed']
    for label, field in [('service', 'service_ns'), ('offer', 'offer_latency_ns')]:
        ordered = sorted(e[field] for e in good)
        for suffix, fraction in [('p50', .5), ('p99', .99)]:
            counts[f'{label}_{suffix}_ns'] = ordered[math.ceil(len(ordered)*fraction)-1] if qualified else None
        counts[f'{label}_max_completed_ns'] = max(ordered, default=None)
    return counts


def main():
    """Freeze inputs and budget before timing; retain evidence, retire disposable DB copies."""
    baseline, candidate, masters, output = map(Path, sys.argv[1:])
    output.mkdir()
    cells = [dict(family=family, repetition=rep, arm=arm, mode='erase')
             for family in ('node', 'edge', 'property') for rep in range(3)
             for arm in (('baseline', 'candidate') if rep % 2 == 0 else ('candidate', 'baseline'))]
    cells += [dict(family=family, repetition=0, arm=arm, mode='control')
              for family in ('node', 'edge', 'property') for arm in ('baseline', 'candidate')]
    inputs = {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in [baseline, candidate, *[masters/f'{f}-default.sqlite' for f in ('node', 'edge', 'property')]]}
    save(output/'manifest.json', dict(inputs=inputs, cells=cells, budget=dict(total_seconds=2700, child_seconds=600, rss_bytes=8*1024**3, scratch_bytes=12*1024**3), workload=dict(window_seconds=15, offers_per_stream=1500, read_clients=8, write_clients=1, write_population=8, writes='plain doc survivor supersession; no property projection', edge_reads='endpoint node text'), baseline_metrics='unavailable: Phase82 has no stage seam', candidate_metrics='overlapping timers: never sum stage totals'))
    deadline = time.monotonic()+2700
    records = []
    for index, cell in enumerate(cells):
        target = output/f'{index:02d}-{cell["family"]}-{cell["arm"]}-{cell["mode"]}'
        if time.monotonic() >= deadline:
            records.append(dict(cell=cell, stop='not_run_budget'))
            continue
        binary = baseline if cell['arm'] == 'baseline' else candidate
        command = [str(binary), str(masters/f'{cell["family"]}-default.sqlite'), str(target), cell['family'], cell['mode']]
        record = child(command, output/f'{index:02d}.log', output, min(deadline, time.monotonic()+600))
        result_path = target/'result.json'
        result = json.loads(result_path.read_text()) if result_path.exists() else None
        event_path = target/'events.jsonl'
        events = [json.loads(line) for line in event_path.read_text().splitlines()] if event_path.exists() else []
        complete = record['exit_code'] == 0 and not record.get('stop') and bool(result and result['verified'])
        record.update(cell=cell, result=result, streams={stream: summarize(events, stream, complete) for stream in ('read', 'write')})
        records.append(record)
        save(output/'records.json', records)
        # Only this cell's generated replicas; masters and all raw event/results remain.
        for path in target.glob('replica.sqlite*'):
            path.unlink()
    save(output/'records.json', records)


if __name__ == '__main__':
    main()
