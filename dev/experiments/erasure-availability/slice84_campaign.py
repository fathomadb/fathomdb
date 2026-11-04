"""Bounded Slice84 campaign: profile BASELINE PHASE82 MASTERS OUTPUT OWNED_ROOT; finish CANDIDATE OUTPUT."""
import json
from pathlib import Path
import statistics
import sys
import time
from campaign import digest, events_at, safe_child, save, remove_replica_files
from phase2_campaign import summarize


def cells():
    """Freeze54 cells without replacement attempts, with matched pair controls."""
    out=[]
    def add(phase, family, rep, arm, mode):
        out.append(dict(id=len(out),phase=phase,family=family,repetition=rep,arm=arm,mode=mode,write_hz=20 if family=='property' else 100))
    for phase,arm in [('baseline_profile','baseline'),('candidate_profile','candidate')]:
        for family in ('node','edge','property'): add(phase,family,0,arm,'erase')
    for family in ('node','edge','property'):
        for rep in range(3):
            for arm in (('candidate','baseline') if rep%2 else ('baseline','candidate')):
                for mode in ('erase','control'): add('core',family,rep,arm,mode)
    for phase,arms,mode in [('context',('phase82','candidate'),'erase'),('idle',('baseline','candidate'),'idle')]:
        for family in ('node','edge','property'):
            for arm in arms: add(phase,family,0,arm,mode)
    return out


def remaining(segments, current):
    """Carry every active segment into the single2700-second budget."""
    return max(0,2700-sum(s['elapsed_s'] for s in segments)-current)


def assess_family(records,family):
    """Apply all paired S84-6 gates without pooling protocols or replacing failures."""
    rows=[r for r in records if r['cell']['phase']=='core' and r['cell']['family']==family]
    reasons=[]
    durations={'baseline':[],'candidate':[]}
    writes={'baseline':[],'candidate':[]}
    for rep in range(3):
        pair={}
        for arm in ('baseline','candidate'):
            for mode in ('erase','control'):
                matches=[r for r in rows if r['cell']['repetition']==rep and r['cell']['arm']==arm and r['cell']['mode']==mode]
                if len(matches)!=1 or not matches[0].get('complete'):
                    reasons.append(f'{rep}/{arm}/{mode}: incomplete'); continue
                r=matches[0]
                if mode=='erase':
                    pair[arm]=r
                    if r['result'].get('native_error') is not None: reasons.append(f'{rep}/{arm}: native refusal')
                    durations[arm].append(r['result']['native_ns'])
                    writes[arm].append(r['streams']['write']['completed'])
                for stream,s in r['streams'].items():
                    if any(s[k] for k in ('failed','unfinished','unknown','accounting_errors')): reasons.append(f'{rep}/{arm}/{mode}/{stream}: invalid')
                    if mode=='control' and (s['completed']!=s['scheduled'] or s['engine_refused'] or s['generator_missed']): reasons.append(f'{rep}/{arm}/{stream}: failed control')
        if len(pair)==2:
            b,c=pair['baseline'],pair['candidate']
            if c['result']['native_ns']>=b['result']['native_ns']: reasons.append(f'{rep}: duration did not improve')
            for stream in ('read','write'):
                if c['streams'][stream]['completed']<b['streams'][stream]['completed']: reasons.append(f'{rep}: fewer {stream} completions')
                if c['streams'][stream]['engine_refused']>b['streams'][stream]['engine_refused']: reasons.append(f'{rep}: additional {stream} refusals')
    medians={arm:dict(duration_ns=statistics.median(durations[arm]),writes=statistics.median(writes[arm])) for arm in durations if len(durations[arm])==len(writes[arm])==3}
    if len(medians)==2:
        if medians['candidate']['duration_ns']>.75*medians['baseline']['duration_ns']: reasons.append('median duration reduction below25%')
        if medians['candidate']['writes']<=medians['baseline']['writes']: reasons.append('median writes did not increase')
    else: reasons.append('three valid pairs unavailable')
    return dict(qualified=not reasons,reasons=reasons,medians=medians)


def assess_campaign(records, segments):
    """Keep core comparisons distinct from complete, budget-valid qualification."""
    core={f:assess_family(records,f) for f in ('node','edge','property')}
    active=sum(s['elapsed_s'] for s in segments)
    complete=len(records)==54 and {r['cell']['id'] for r in records}==set(range(54)) and all(r.get('complete') and all(not stream[k] for k in ('failed','unfinished','unknown','accounting_errors')) for r in records for stream in r.get('streams',{}).values()) and all(set(r.get('streams',{}))=={'read','write'} for r in records)
    qualified=complete and active<=2700 and all(s['state']=='complete' for s in segments) and all(v['qualified'] for v in core.values())
    return dict(qualified=qualified,all54_complete=complete,active_seconds=active,core=core)


def freeze(path):
    """Bind an immutable binary or master to its exact bytes."""
    path=Path(path).resolve()
    return dict(path=str(path),sha256=digest(path))


def main():
    """Run baseline diagnostics then pause for code/review; finish never retries a cell."""
    begun=time.monotonic()
    command=sys.argv[1]
    if command=='profile':
        baseline,context,masters,root,owned_root=map(Path,sys.argv[2:])
        if not root.resolve().is_relative_to(owned_root.resolve()): raise SystemExit("output must be within owned scratch root")
        root.mkdir()
        manifest=dict(cells=cells(),artifacts={'baseline':freeze(baseline),'phase82':freeze(context)},masters={f:freeze(masters/f'{f}-default.sqlite') for f in ('node','edge','property')},segments=[],budget_seconds=2700,cell_seconds=90,resource_root=str(owned_root.resolve()),controller_sha256=digest(Path(__file__)),helper_sha256={name:digest(Path(__file__).with_name(name)) for name in ("campaign.py","phase2_campaign.py")},observer='same instrumented baseline/candidate; overlapping counters are not additive')
        records=[]
        phases={'baseline_profile'}
    elif command=='finish':
        candidate,root=map(Path,sys.argv[2:])
        manifest=json.loads((root/'manifest.json').read_text())
        if any(s.get('state')!='complete' for s in manifest['segments']): raise SystemExit('interrupted segment: retain evidence; no automatic resume')
        if 'candidate' in manifest['artifacts']: raise SystemExit('finish already attempted; no replacement run')
        if digest(Path(__file__))!=manifest['controller_sha256']: raise SystemExit('controller changed after profile')
        for name,checksum in manifest['helper_sha256'].items():
            if digest(Path(__file__).with_name(name))!=checksum: raise SystemExit('helper changed after profile')
        manifest['artifacts']['candidate']=freeze(candidate)
        records=json.loads((root/'records.json').read_text())
        phases={'candidate_profile','core','context','idle'}
    else: raise SystemExit(__doc__)
    segment=dict(command=command,started_unix_s=time.time(),elapsed_s=0,state='running')
    previous=list(manifest['segments'])
    manifest['segments'].append(segment)
    save(root/'manifest.json',manifest)
    save(root/'records.json',records)
    try:
        for artifact in list(manifest['artifacts'].values())+list(manifest['masters'].values()):
            if digest(Path(artifact['path']))!=artifact['sha256']: raise ValueError('frozen artifact changed: '+artifact['path'])
        stopped=None
        for cell in manifest['cells']:
            if cell['phase'] not in phases: continue
            allowance=remaining(previous,time.monotonic()-begun)
            if stopped or allowance<=0:
                records.append(dict(cell=cell,complete=False,stop=stopped or 'not_run_budget'))
                save(root/'records.json',records); continue
            target=root/f"{cell['id']:02d}-{cell['phase']}-{cell['family']}-{cell['arm']}-{cell['mode']}"
            args=[manifest['artifacts'][cell['arm']]['path'],manifest['masters'][cell['family']]['path'],str(target),cell['family'],cell['mode'],str(cell['write_hz'])]
            record=safe_child(args,root/f"{cell['id']:02d}.log",Path(manifest['resource_root']),time.monotonic()+min(90,allowance))
            record.update(cell=cell,complete=False,command=args,raw=str(target))
            try:
                result_path=target/'result.json'
                result=json.loads(result_path.read_text()) if result_path.exists() else None
                events=events_at(target/'events.jsonl')
                complete=record['exit_code']==0 and not record.get('stop') and record.get('deadline_overshoot_s',0)==0 and bool(result and result['verified'])
                streams={s:summarize(events,s,complete,0 if cell['mode']=='idle' else (1500 if s=='read' else 15*cell['write_hz'])) for s in ('read','write')}
                for s,summary in streams.items():
                    good=[e for e in events if e.get('kind')=='finish' and e.get('stream')==s and e.get('state')=='completed']
                    summary['completed_service_median_ns']=statistics.median(e['service_ns'] for e in good) if good else None
                record.update(cell=cell,complete=complete,result=result,streams=streams,command=args,raw=str(target))
            except Exception as error:
                record.update(complete=False,stop='result_accounting_error',accounting_error=repr(error))
            records.append(record)
            save(root/'records.json',records)
            remove_replica_files(target)
            if record.get('stop') in ('rss_limit','disk_limit','monitor_error','result_accounting_error'): stopped=record['stop']
            segment['elapsed_s']=time.monotonic()-begun
            save(root/'manifest.json',manifest)
        for artifact in list(manifest['artifacts'].values())+list(manifest['masters'].values()):
            if digest(Path(artifact['path']))!=artifact['sha256']: raise ValueError('frozen artifact changed: '+artifact['path'])
        segment['state']='complete'
        save(root/'assessment.json',dict(core={f:assess_family(records,f) for f in ('node','edge','property')},qualification='pending final budget validation'))
    finally:
        segment['elapsed_s']=time.monotonic()-begun
        segment['budget_exceeded']=remaining(previous,segment['elapsed_s'])==0
        save(root/'manifest.json',manifest)
        save(root/'assessment.json',assess_campaign(records,manifest['segments']))

if __name__=='__main__': main()
