#!/usr/bin/env python3
"""Proposed operational controls; never evidence until actual approved runs."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import time
import a6_operator as op

def child_command(args,case,stop=None,stage='controls'):
    command=[sys.executable,str(op.PACKAGE/'a6_operator.py'),stage,'--authority',str(args.authority),'--venue',str(args.venue),'--candidate-sha',args.candidate_sha,'--source',str(args.source),'--parse',str(args.parse),'--probe',str(args.probe),'--binaries',str(args.binaries),'--case',case]
    if stop:command+=['--stop-mode',stop]
    return command

def start(args,case,stop=None,stage='controls'):
    p=subprocess.Popen(child_command(args,case,stop,stage),stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
    ready,_,_=select.select([p.stdout],[],[],20)
    if not ready:raise RuntimeError('controller initialization bound')
    first=json.loads(p.stdout.readline());root=Path(first['evidence'])
    if not root.is_relative_to(op.TASK/'a6-execution') or first['runId']!=root.name:raise RuntimeError('owned evidence identity')
    return p,root

def stop_owned(p):
    if p.poll() is None:
        if os.getpgid(p.pid)!=p.pid or os.getsid(p.pid)!=p.pid:raise RuntimeError('owned private session changed')
        os.killpg(p.pid,signal.SIGTERM)
        try:p.wait(timeout=2)
        except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait(timeout=2)

def ordinary(args,case,stop=None,stage='controls'):
    p,root=start(args,case,stop,stage)
    try:out,err=p.communicate(timeout=240)
    except BaseException:stop_owned(p);raise
    result=op.read_json(root/'result.json')
    expected='EXPECTED_CONTROLLED_STOP' if stop else 'COMPLETED'
    if stage=='launch-negative':
        if p.returncode==0 or result['status']!='UNKNOWN':raise RuntimeError('bad-launch hash was not refused')
        if 'launch script digest differs' not in (root/'observer.log').read_text(errors='replace'):raise RuntimeError('unrelated failure is not launch-refusal proof')
    elif p.returncode or result['status']!=expected:raise RuntimeError({'control':case,'exit':p.returncode,'stdout':out,'stderr':err,'result':result})
    return {'key':('launch-negative' if stage=='launch-negative' else case+(':'+stop if stop else '')),'case':case,'stop':stop,'stage':stage,'evidence':str(root),'resultSha256':op.digest(root/'result.json'),'status':'EXPECTED_OBSERVATION'}

def controller_death(args):
    p,root=start(args,'linger');ready=root/'controller-ready.json';deadline=time.monotonic()+180;record=None
    try:
        while time.monotonic()<deadline:
            if ready.exists():
                record=op.read_json(ready);marker=record['control']+'\\case\\child.entered'
                if op.command('if(Test-Path -LiteralPath '+op.quote(marker)+"){'yes'}else{'no'}")=='yes':break
            if p.poll() is not None:raise RuntimeError('controller exited before live-child witness')
            time.sleep(0.5)
        else:raise RuntimeError('real child witness unavailable')
        fields=Path('/proc/'+str(p.pid)+'/stat').read_text().split()
        if record['pid']!=p.pid or record['pgid']!=p.pid or record['sid']!=p.pid or record['startTicks']!=fields[21] or os.getpgid(p.pid)!=p.pid or os.getsid(p.pid)!=p.pid:raise RuntimeError('controller identity changed')
        # Abruptly kill only the created controller's private process group,
        # including its SSH transport. The driver never renews that guest lease.
        os.killpg(p.pid,signal.SIGKILL);p.wait(timeout=3);start_dead=time.monotonic();terminal=None
        while time.monotonic()-start_dead<110:
            path=record['control']+'\\termination.json'
            try:terminal=json.loads(op.command('if(Test-Path -LiteralPath '+op.quote(path)+"){Get-Content -Raw -Encoding UTF8 -LiteralPath "+op.quote(path)+"}else{'null'}"))
            except RuntimeError:terminal=None
            if terminal is not None:break
            time.sleep(1)
        if terminal is None or terminal['runId']!=record['runId'] or terminal['candidateSha']!=args.candidate_sha or terminal['phase']!='STOPPED' or not terminal['commands']:raise RuntimeError('controller death lacks terminal owned cleanup proof')
        if not all(x['authoritativeEmpty'] and x['counts']['ActiveProcesses']==0 and 'lease expired' in str(x['stopReason']) for x in terminal['commands']):raise RuntimeError('lease cause or authoritative empty missing')
        evidence={'case':'controller-group-death','terminal':terminal,'elapsed':time.monotonic()-start_dead,'physical_host_power_loss_tested':False}
        (root/'controller-death-observation.json').write_text(json.dumps(evidence,indent=2)+'\n')
        return {'key':'controller-group-death','case':'controller-group-death','observationFile':'controller-death-observation.json','evidence':str(root),'resultSha256':op.digest(root/'controller-death-observation.json'),'status':'EXPECTED_OBSERVATION'}
    finally:stop_owned(p)

def invalid_local(args):
    command=child_command(args,'normal');command[command.index('--candidate-sha')+1]='not-a-sha'
    p=subprocess.run(command,capture_output=True,text=True,timeout=10,start_new_session=True)
    if p.returncode==0 or 'run/source syntax' not in p.stderr:raise RuntimeError('invalid-source refusal not established')
    return {'key':'invalid-source','case':'invalid-source-before-VM','exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr,'status':'EXPECTED_REFUSAL'}

def main():
    p=argparse.ArgumentParser();p.add_argument('--authority',type=Path,required=True);p.add_argument('--venue',type=Path,required=True);p.add_argument('--candidate-sha',required=True);p.add_argument('--source',type=Path,required=True);p.add_argument('--parse',type=Path,required=True);p.add_argument('--probe',type=Path,required=True);p.add_argument('--binaries',type=Path,required=True)
    args=p.parse_args();package_hash,_=op.check_package();op.check_authority(args.authority,'controls',package_hash)
    results=[invalid_local(args)]
    for case in op.CASES:results.append(ordinary(args,case))
    for stop in ['expire','cancel','transport']:results.append(ordinary(args,'linger',stop))
    results.append(controller_death(args));results.append(ordinary(args,'normal',stage='launch-negative'))
    # Transfer fault entries have their own real positive/negative byte checks.
    for fault in ['digest','length']:
        cmd=child_command(args,'normal',stage='transfer-negative')+['--transfer-fault',fault]
        p=subprocess.run(cmd,capture_output=True,text=True,timeout=240,start_new_session=True)
        if p.returncode:raise RuntimeError({'transferFault':fault,'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
        folder=Path(json.loads(p.stdout.splitlines()[0])['evidence'])
        if not folder.is_relative_to(op.TASK/'a6-execution'):raise RuntimeError('transfer evidence path')
        results.append({'key':'transfer-'+fault,'case':'transfer-'+fault,'evidence':str(folder),'observationFile':'negative-transfer.json','resultSha256':op.digest(folder/'negative-transfer.json'),'stdout':p.stdout,'status':'EXPECTED_REFUSAL'})
    path=op.TASK/'a6-execution'/('controls-index-'+str(time.time_ns())+'.json')
    path.write_text(json.dumps({'packageManifestSha256':package_hash,'candidateSha':args.candidate_sha,'results':results,'reviewRequired':True,'a6Qualified':False},indent=2)+'\n');print(path)

if __name__=='__main__':main()
