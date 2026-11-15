#!/usr/bin/env python3
"""Operator-source proposal. Operational entry points require later authority."""
from __future__ import annotations
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import threading
import time
import uuid
import xml.etree.ElementTree as ET
import zipfile

PACKAGE=Path(__file__).resolve().parent
TASK=Path('/mnt/data/tmp2/fathomdb-0829-continuation/slice20')
IMAGES=Path('/mnt/data/tmp2/fathomdb-0829-continuation/slice20-windows-images-approved-20261113')
GUEST='R:\\fathomdb-s20-a6'
GIB=1024**3
KEY='/home/coreyt/local/gh-runner-wonl/data/windows11/memex-local-ci-ed25519'
SSH=['ssh','-F','/dev/null','-i',KEY,'-o','IdentitiesOnly=yes','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=5','-o','ControlMaster=no','-o','ProxyCommand=none','-o','ServerAliveInterval=5','-o','ServerAliveCountMax=1','runner@192.168.122.51']
CASES=['normal','orphan','output','breakaway','inherited-handle','cleanup-failure','diagnostic-overflow','diagnostic-truncation','death-before-create','death-after-create','death-after-resume']
TRANSFER_END=bytes.fromhex('fb29'*16)
FRAMEWORK='C:\\Windows\\Microsoft.NET\\Framework64\\v4.0.30319'

def digest(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def read_json(path):return json.loads(Path(path).read_text())
def quote(value):return "'"+value.replace("'","''")+"'"
def script_prefix(script):
    if len(base64.b64encode(script.encode('utf-16le')))+200<4096:return b''
    payload=base64.b64encode(script.encode('utf-8'))
    if not 0<len(payload)<=32768:raise ValueError('stdin script length bound')
    return payload

def encoded(script):
    prefix=script_prefix(script)
    if prefix:
        script=(PACKAGE/'stdin-prefix.ps1').read_text().replace('__A6_STDIN_BYTES__',str(len(prefix))).replace('__A6_STDIN_SHA256__',hashlib.sha256(prefix).hexdigest())
    code=base64.b64encode(script.encode('utf-16le')).decode()
    if len(code)+200>=4096:raise ValueError('remote command length bound')
    return SSH+['C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe','-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand',code]

def start_script(script,**kwargs):
    prefix=script_prefix(script)
    p=subprocess.Popen(encoded(script),stdin=subprocess.PIPE if prefix else subprocess.DEVNULL,**kwargs)
    try:
        if prefix:p.stdin.write(prefix);p.stdin.close()
    except BaseException:stop_ssh(p);raise
    return p
def script_invocation(path,expected,arguments):
    return "$ErrorActionPreference='Stop';$p="+quote(path)+";if((Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant() -cne '"+expected+"'){throw 'launch script digest differs'};& ([ScriptBlock]::Create((Get-Content -Raw -Encoding UTF8 -LiteralPath $p))) -ScriptPath $p "+arguments

def stop_ssh(p):
    if p.poll() is None:
        p.terminate()
        try:p.wait(timeout=1)
        except subprocess.TimeoutExpired:p.kill();p.wait(timeout=1)
def command(script,timeout=6,input_bytes=None):
    # SSH has no ProxyCommand/LocalCommand and inherits the controller's private
    # process group. Timeout kills this exact Popen child, never a shared group.
    p=subprocess.Popen(encoded(script),stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    try:out,err=p.communicate(script_prefix(script)+(input_bytes or b''),timeout=timeout)
    except BaseException:stop_ssh(p);raise
    if p.returncode:raise RuntimeError({'ssh_exit':p.returncode,'stdout':out.decode(errors='replace'),'stderr':err.decode(errors='replace')})
    return out.decode('utf-8-sig').strip()
def valid_digest(value):return isinstance(value,str) and re.fullmatch('[0-9a-f]{64}',value) is not None

def check_package():
    manifest=read_json(PACKAGE/'manifest.json')
    for name,expected in manifest['files'].items():
        if '/' in name or '\\' in name or not valid_digest(expected) or digest(PACKAGE/name)!=expected:raise ValueError('frozen package differs')
    return digest(PACKAGE/'manifest.json'),manifest

def check_authority(path,stage,package_hash):
    a=read_json(path)
    if a.get('capacityApproved') is not True or not valid_digest(a.get('capacityReceiptSha256')) or a.get('approved') is not True or a.get('nativeWindowReleased') is not True or a.get('packageManifestSha256')!=package_hash or stage not in a.get('allowedStages',[]):raise PermissionError('HELD: phase-specific execution authority/review/native window absent; capacity-only is insufficient')
    return a

def validate_source_run(sha,run_id):
    if not isinstance(sha,str) or not isinstance(run_id,str) or not re.fullmatch('[0-9a-f]{40}',sha) or not re.fullmatch('[0-9a-f]{32}',run_id):raise ValueError('run/source syntax')

def venue_config(path,sha,run):
    validate_source_run(sha,run)
    v=read_json(path)
    if not isinstance(v.get('diskUniqueId'),str) or not v['diskUniqueId'] or len(v['diskUniqueId'])>512:raise ValueError('actual disk identity missing')
    guid=str(uuid.UUID(v['partitionGuid'].strip('{}')))
    for key in ['cargoExe','rustcExe']:
        if not isinstance(v.get(key),str) or not re.fullmatch(r'[A-Z]:\\[^\r\n"]+',v[key]) or not v[key].lower().endswith('\\bin\\'+('cargo.exe' if key=='cargoExe' else 'rustc.exe')):raise ValueError('actual absolute toolchain backend path missing')
    return {'root':GUEST,'runId':run,'candidateSha':sha,'diskUniqueId':v['diskUniqueId'],'partitionGuid':guid,'cargoExe':v['cargoExe'],'rustcExe':v['rustcExe']}

class Session:
    def __init__(self,args,stage):
        self.run_id=uuid.uuid4().hex
        validate_source_run(args.candidate_sha,self.run_id)
        self.package_hash,self.package=check_package();self.authority=check_authority(args.authority,stage,self.package_hash)
        if self.authority.get('candidateSha')!=args.candidate_sha:raise PermissionError('authority source SHA differs')
        if os.getpid()!=os.getpgrp() or os.getpid()!=os.getsid(0):raise RuntimeError('invoke through setsid --wait; controller must own its session/group')
        self.args=args;self.stage=stage;self.lease_generation=-1;self.cfg=venue_config(args.venue,args.candidate_sha,self.run_id)
        self.root=TASK/'a6-execution'/self.run_id;self.root.mkdir(parents=True,mode=0o700,exist_ok=False)
        self.cfg['packagePath']=GUEST+'\\packages\\'+self.package_hash;self.cfg['packageFiles']=self.package['files'];self.cfg['packageManifestSha256']=self.package_hash
        self.samples=[];self.cancelled=False;self.last_swap=None;self.swap_over=0;self.owned_start=None
        signal.signal(signal.SIGINT,self.signal);signal.signal(signal.SIGTERM,self.signal)
        print(json.dumps({'evidence':str(self.root),'runId':self.run_id}),flush=True)
        self.log('controller',{'pid':os.getpid(),'pgid':os.getpgrp(),'sid':os.getsid(0),'start_ticks':Path('/proc/self/stat').read_text().split()[21],'interpreter':str(Path(sys.executable).resolve()),'interpreterSha256':digest(Path(sys.executable).resolve()),'packageManifestSha256':self.package_hash})
    def signal(self,sig,frame):self.cancelled=True
    def log(self,kind,value):
        with (self.root/'host.jsonl').open('a') as stream:stream.write(json.dumps({'utc':time.time(),'kind':kind,'value':value})+'\n')
    def du(self,path):
        if not path.exists():return 0
        for _ in range(3):
            r=subprocess.run(['du','-sb',str(path)],capture_output=True,text=True,timeout=5)
            if r.returncode==0:return int(r.stdout.split()[0])
            if not r.stderr.splitlines() or any('No such file or directory' not in x for x in r.stderr.splitlines()):raise RuntimeError(r.stderr)
        raise RuntimeError('owned size measurement failed')
    def sample(self):
        owned=max(self.du(TASK)+self.du(self.args.source),128*GIB)+max(64*GIB,self.du(IMAGES))
        mem={x.split(':',1)[0]:x.split(':',1)[1].split()[0] for x in Path('/proc/meminfo').read_text().splitlines()};vm=dict(x.split() for x in Path('/proc/vmstat').read_text().splitlines())
        f={'monotonic':time.monotonic(),'rootFree':os.statvfs('/').f_bavail*os.statvfs('/').f_frsize,'taskFree':os.statvfs(TASK).f_bavail*os.statvfs(TASK).f_frsize,'memory':int(mem['MemAvailable'])*1024,'ownedReserved':owned,'swap':int(vm['pswpin'])+int(vm['pswpout'])}
        if self.owned_start is None:
            if f['rootFree']<32*GIB or f['taskFree']<250*GIB or f['memory']<16*GIB:raise RuntimeError('host start floor')
            self.owned_start=owned
        if self.last_swap and f['monotonic']-self.last_swap['monotonic']>=30:
            old=self.last_swap;rate=(f['swap']-old['swap'])*os.sysconf('SC_PAGE_SIZE')/max(1,f['monotonic']-old['monotonic']);self.swap_over=self.swap_over+1 if rate>64*1024**2 else 0
            self.last_swap=f
        if self.last_swap is None:self.last_swap=f
        self.samples.append(f);self.log('resource',f)
        if f['rootFree']<20*GIB or f['taskFree']<50*GIB or f['memory']<8*GIB or owned>=200*GIB or owned-self.owned_start>=200*GIB or self.swap_over>=3:raise RuntimeError('host stop floor')
        if self.cancelled:raise RuntimeError('host cancellation')
        return f
    def identity(self):
        r=subprocess.run(['virsh','-c','qemu:///system','domuuid','gh-runner-wonl-win11'],check=True,capture_output=True,text=True,timeout=5)
        if r.stdout.strip()!='3a928939-57ed-4c62-999b-b610f3e44d42':raise RuntimeError('VM identity')
        r=subprocess.run(['virsh','-c','qemu:///system','dumpxml','gh-runner-wonl-win11'],check=True,capture_output=True,text=True,timeout=5)
        disks=[d for d in ET.fromstring(r.stdout).findall('./devices/disk') if d.findtext('serial')=='FDB29S20A6']
        if len(disks)!=1:raise RuntimeError('new disk identity ambiguous')
        d=disks[0];t=d.find('target');a=d.find('address');src=d.find('source')
        if t is None or a is None or src is None or t.get('dev')!='sdg' or t.get('bus')!='usb' or a.get('type')!='usb' or a.get('bus')!='0' or a.get('port')!='10' or src.get('file')!=str(IMAGES/'windows-a6-64g.qcow2'):raise RuntimeError('live attachment differs')
        if command('[Environment]::MachineName')!='MEMEX-AFE8V4I00':raise RuntimeError('guest hostname')
    def inline(self,name,call):return (PACKAGE/'common.ps1').read_text()+'\n'+(PACKAGE/name).read_text()+'\n'+call
    def upload(self,path,relative,wrong_digest=False,extra_length=0):
        self.sample()
        if not wrong_digest and not extra_length:
            destination=GUEST+'\\'+relative.replace('/', '\\')
            existing=command("$p="+quote(destination)+";if(Test-Path -LiteralPath $p -PathType Leaf){@{sha256=(Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant();length=(Get-Item -LiteralPath $p).Length}|ConvertTo-Json -Compress}else{'null'}")
            current=json.loads(existing)
            if current is not None:
                if current.get('sha256')!=digest(path) or current.get('length')!=path.stat().st_size:raise RuntimeError('occupied remote destination differs')
                self.log('read-only verified reuse',{'path':destination,'sha256':current['sha256']});return
        if path.stat().st_size>1024**2 and not wrong_digest and not extra_length:return self.upload_sftp(path,relative)
        c={key:self.cfg[key] for key in ['root','runId','candidateSha','diskUniqueId','partitionGuid']};c.update({'relative':relative,'length':path.stat().st_size+extra_length,'expectedSha256':('0'*64 if wrong_digest else digest(path))})
        b64=base64.b64encode(json.dumps(c,separators=(',',':')).encode()).decode();script=self.inline('receive.ps1','Invoke-A6Receive '+quote(b64))
        p=subprocess.Popen(encoded(script),stdin=subprocess.PIPE,stdout=(self.root/'transfer.stdout').open('ab'),stderr=(self.root/'transfer.stderr').open('ab'))
        errors=[]
        def writer():
            try:
                p.stdin.write(script_prefix(script))
                with path.open('rb') as stream:
                    while data:=stream.read(65536):p.stdin.write(data)
                p.stdin.write(TRANSFER_END);p.stdin.close()
            except BaseException as error:errors.append(repr(error))
        thread=threading.Thread(target=writer,daemon=True);thread.start();start=time.monotonic()
        try:
            while p.poll() is None:
                self.sample()
                if time.monotonic()-start>185:raise TimeoutError('transfer185s bound')
                time.sleep(0.5)
            thread.join(timeout=1)
            if p.returncode or errors:raise RuntimeError({'transfer_exit':p.returncode,'writer':errors})
        finally:stop_ssh(p)
    def upload_sftp(self,path,relative):
        if relative not in ('inputs/'+self.run_id+'/source.zip','control/'+self.run_id+'/run.json'):raise ValueError('SFTP is only for allocated source archive/config')
        destination=GUEST+'\\'+relative.replace('/', '\\');partial=destination+'.partial-'+uuid.uuid4().hex;done=partial+'.done'
        c={key:self.cfg[key] for key in ['root','runId','candidateSha','diskUniqueId','partitionGuid']}
        b64=base64.b64encode(json.dumps(c,separators=(',',':')).encode()).decode()
        prelude=(PACKAGE/'common.ps1').read_text()+"\n$c=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('"+b64+"'))|ConvertFrom-Json;Assert-A6Start $c|Out-Null;$p=Assert-A6Path "+quote(partial)+" $c.root;"
        command(prelude+"[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($p))|Out-Null;$f=[IO.File]::Open($p,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None);$f.Dispose()",timeout=20)
        receiver=(PACKAGE/'receive.ps1').read_text();watch=receiver[receiver.index('    $s=[hashtable]'):receiver.index('    $partial=$destination')]
        monitor=prelude+watch+"\nwhile(-not(Test-Path -LiteralPath "+quote(done)+")){[Threading.Thread]::Sleep(200)};$s.Done=$true"
        guard=None;p=None;start=time.monotonic();failure=None
        try:
            with (self.root/'sftp-monitor.stdout').open('ab') as out,(self.root/'sftp-monitor.stderr').open('ab') as err:
                guard=start_script(monitor,stdout=out,stderr=err);self.log('owned-sftp-monitor',{'pid':guard.pid,'pgid':os.getpgid(guard.pid),'partial':partial})
                with (self.root/'sftp.stdout').open('ab') as transfer_out,(self.root/'sftp.stderr').open('ab') as transfer_err:
                    p=subprocess.Popen(['sftp',*SSH[1:-1],'-b','-',SSH[-1]],stdin=subprocess.PIPE,stdout=transfer_out,stderr=transfer_err)
                    self.log('owned-sftp',{'pid':p.pid,'pgid':os.getpgid(p.pid),'partial':partial})
                    if any(x in str(path)+partial for x in ['"','\n','\r']):raise ValueError('SFTP path syntax')
                    p.stdin.write(('put "'+str(path)+'" "'+'/'+partial.replace('\\','/')+'"\n').encode());p.stdin.close()
                    while p.poll() is None:
                        self.sample()
                        if guard.poll() is not None:raise RuntimeError('independent guest transfer monitor stopped')
                        if time.monotonic()-start>=180:raise TimeoutError('SFTP180s bound')
                        time.sleep(0.5)
                    if p.returncode:raise RuntimeError('SFTP transfer failed')
                command(prelude+"$d=Assert-A6Path "+quote(destination)+" $c.root;if((Get-Item -LiteralPath $p).Length -ne "+str(path.stat().st_size)+" -or (Get-A6Hash $p) -cne '"+digest(path)+"'){throw 'transfer truncated or digest differs'};if(Test-Path -LiteralPath $d){throw 'transfer destination raced'};[IO.File]::Move($p,$d)",timeout=20)
                self.log('sftp-source-verified',{'path':destination,'bytes':path.stat().st_size,'sha256':digest(path)})
        except BaseException as error:failure=repr(error);self.log('sftp-UNKNOWN',failure);raise
        finally:
            if p is not None:stop_ssh(p)
            try:command("[IO.File]::WriteAllText("+quote(done)+",'done')",timeout=6)
            finally:
                if guard is not None:
                    try:guard.wait(timeout=6)
                    finally:stop_ssh(guard)

    def renew_lease(self):
        self.lease_generation+=1
        self.publish('lease.json',{'runId':self.run_id,'candidateSha':self.cfg['candidateSha'],'generation':self.lease_generation})
    def extract(self):
        self.sample();self.renew_lease();last_renewal=time.monotonic()
        script=self.cfg['packagePath']+'\\extract-source.ps1'
        invocation=script_invocation(script,self.package['files']['extract-source.ps1'],'-Control '+quote(self.control)+' -ConfigSha256 '+self.config_hash)
        start=time.monotonic();p=None;failure=None
        try:
            with (self.root/'extract.stdout').open('xb') as out,(self.root/'extract.stderr').open('xb') as err:
                p=start_script(invocation,stdout=out,stderr=err)
                self.log('owned-extract-observer',{'pid':p.pid,'pgid':os.getpgid(p.pid)})
                while p.poll() is None:
                    self.sample()
                    now=time.monotonic()
                    if now-start>=185:raise TimeoutError('extraction185s bound')
                    if now-last_renewal>=10:
                        self.renew_lease();last_renewal=time.monotonic()
                    time.sleep(0.5)
                if p.returncode:raise RuntimeError({'extraction_exit':p.returncode})
            receipt=self.remote_text('source-extracted.json')
            if receipt is None or receipt.get('status')!='EXACT_SOURCE_VERIFIED' or receipt.get('archiveSha256')!=self.cfg['archiveSha256'] or receipt.get('workspace')!=self.cfg['workspace'] or receipt.get('files')!=len(self.cfg['sourceFiles']):raise RuntimeError('exact extraction receipt missing/different')
            self.log('source-extracted',receipt)
        except BaseException as error:
            failure=repr(error);self.log('extraction-UNKNOWN',failure)
            # Stopping only this created SSH PID does not establish guest cleanup.
            # The guest's independent monotonic monitor retains its own stop bound.
            if p is not None:stop_ssh(p)
            try:self.publish('cancel.json',{'runId':self.run_id,'candidateSha':self.cfg['candidateSha'],'reason':failure})
            except BaseException as secondary:self.log('extract-cancel-unavailable',repr(secondary))
            raise
        finally:
            if p is not None:stop_ssh(p)
            (self.root/'extract-observer.json').write_text(json.dumps({'runId':self.run_id,'candidateSha':self.cfg['candidateSha'],'actualSshExit':None if p is None else p.returncode,'failure':failure,'guestCleanupProven':False,'partialFilesRetained':True})+'\n')
    def stage_package(self):
        for name in self.package['files']:
            self.upload(PACKAGE/name,'packages/'+self.package_hash+'/'+name)
    def remote_text(self,name,timeout=6):
        path=self.control+'\\'+name
        result=command("$ErrorActionPreference='Stop';$p="+quote(path)+";if(Test-Path -LiteralPath $p){Get-Content -Raw -Encoding UTF8 -LiteralPath $p}else{'null'}",timeout)
        r=json.loads(result)
        if r is not None and (r.get('runId')!=self.run_id or r.get('candidateSha')!=self.cfg['candidateSha']):raise ValueError('receipt identity')
        return r
    def publish(self,name,payload):
        raw=json.dumps(payload,separators=(',',':'));b64=base64.b64encode(raw.encode()).decode();path=self.control+'\\'+name
        command("$ErrorActionPreference='Stop';$p="+quote(path)+";$t=$p+'.host-'+[Guid]::NewGuid().ToString('N');$b=[Convert]::FromBase64String('"+b64+"');$f=[IO.File]::Open($t,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None);try{$f.Write($b,0,$b.Length);$f.Flush($true)}finally{$f.Dispose()};if([IO.File]::Exists($p)){[IO.File]::Replace($t,$p,[NullString]::Value)}else{[IO.File]::Move($t,$p)}")
    def probe(self):
        self.identity();self.sample();b64=base64.b64encode(json.dumps(self.cfg).encode()).decode()
        value=json.loads(command(self.inline('probe.ps1','Invoke-A6Probe '+quote(b64)),timeout=20));(self.root/'probe.json').write_text(json.dumps(value,indent=2)+'\n');return value
    def manifest(self):
        self.control=GUEST+'\\control\\'+self.run_id
        path=self.root/'run.json';path.write_text(json.dumps(self.cfg,sort_keys=True,separators=(',',':'))+'\n');self.config_hash=digest(path)
        self.upload(path,'control/'+self.run_id+'/run.json');return path
    def run(self,stop_mode=None,bad_launcher=False):
        self.manifest();last_renewal=None;p=None;terminal=None;failure=None;cut=False
        try:
            self.sample();self.renew_lease();last_renewal=time.monotonic()
            launcher='launch-supervisor.ps1' if self.stage=='a6' else 'launch-stage.ps1';remote=self.cfg['packagePath']+'\\'+launcher
            expected='0'*64 if bad_launcher else self.package['files'][launcher]
            script=script_invocation(remote,expected,'-Control '+quote(self.control)+' -ConfigSha256 '+self.config_hash)
            if self.stage=='a6':script+=' -WrapperSha256 '+self.cfg['wrapperSha256']
            observer=(self.root/'observer.log').open('wb');p=start_script(script,stdout=observer,stderr=subprocess.STDOUT);self.log('owned_observer',{'pid':p.pid,'pgid':os.getpgid(p.pid)})
            (self.root/'controller-ready.json').write_text(json.dumps({'runId':self.run_id,'candidateSha':self.cfg['candidateSha'],'control':self.control,'pid':os.getpid(),'pgid':os.getpgrp(),'sid':os.getsid(0),'startTicks':Path('/proc/self/stat').read_text().split()[21]})+'\n')
            start=time.monotonic();limit=1800 if self.stage=='a6' else 600
            while time.monotonic()-start<limit:
                cycle=time.monotonic();self.sample()
                if p.poll() is not None and p.returncode!=0 and not (cut and stop_mode):raise RuntimeError('launch/transport/exit observer failed')
                if stop_mode and not cut:
                    child_path=self.control+'\\case\\child.entered'
                    ready=command('if(Test-Path -LiteralPath '+quote(child_path)+"){'yes'}else{'no'}")=='yes'
                    if ready:
                        cut=True;self.log('control-cut',{'mode':stop_mode,'at':time.monotonic(),'lastRenewal':last_renewal})
                        if stop_mode=='cancel':self.publish('cancel.json',{'runId':self.run_id,'candidateSha':self.cfg['candidateSha'],'reason':'planned explicit-cancel control'})
                        if stop_mode=='transport':stop_ssh(p)
                if not cut:
                    self.renew_lease();last_renewal=time.monotonic()
                terminal=self.remote_text('termination.json')
                if terminal is not None:break
                while time.monotonic()-cycle<10 and not self.cancelled:time.sleep(0.2)
            else:raise TimeoutError('host phase deadline')
            if stop_mode:
                if not cut:raise RuntimeError('control never reached real child')
                commands=terminal.get('commands',[])
                reason=' '.join(str(x.get('stopReason')) for x in commands)
                if terminal['phase']!='STOPPED' or not commands or not all(x.get('authoritativeEmpty') for x in commands):raise RuntimeError('controlled stop lacks authoritative zero')
                if stop_mode in ('expire','transport') and 'lease expired' not in reason:raise RuntimeError('lease cause not observed')
                if stop_mode=='cancel' and 'cancellation' not in reason:raise RuntimeError('explicit cancellation cause not observed')
                result='EXPECTED_CONTROLLED_STOP'
            else:
                p.wait(timeout=15);exit_record=self.remote_text('supervisor-exit.json');self.log('exit',exit_record)
                if p.returncode or exit_record is None or exit_record['actualExitCode']!=0 or terminal['phase']!='COMPLETED':raise RuntimeError('normal completion/exit unproven')
                verify_terminal(self,terminal)
                result='COMPLETED'
            self.log('termination',terminal)
        except BaseException as error:
            failure=repr(error);result='UNKNOWN';self.log('failure',failure)
            if not cut:
                try:self.publish('cancel.json',{'runId':self.run_id,'candidateSha':self.cfg['candidateSha'],'reason':failure})
                except BaseException as e:self.log('cancel-unavailable',repr(e))
            if p is not None:stop_ssh(p)
            deadline=(last_renewal or time.monotonic())+110
            while time.monotonic()<deadline:
                try:
                    terminal=self.remote_text('termination.json',timeout=min(6,max(0.1,deadline-time.monotonic())))
                    if terminal is not None:break
                except BaseException as e:self.log('receipt-unavailable',repr(e))
                time.sleep(min(2,max(0,deadline-time.monotonic())))
        finally:
            if p is not None:stop_ssh(p)
        record={'status':result,'stage':self.stage,'runId':self.run_id,'candidateSha':self.cfg['candidateSha'],'packageManifestSha256':self.package_hash,'configSha256':self.config_hash,'failure':failure,'terminal':terminal,'controlRoot':self.control,'resources':self.samples,'a6Qualified':False}
        (self.root/'result.json').write_text(json.dumps(record,indent=2)+'\n');return record

def verify_terminal(session,terminal):
    commands=terminal.get('commands',[]);expected=5 if session.stage=='a6' else 3 if session.stage in ('compile','launch-negative') else 1
    if len(commands)!=expected:raise ValueError('missing command receipts')
    for row in commands:
        d=row.get('diagnostics',{})
        if row.get('authoritativeEmpty') is not True or row.get('rootExit')!=0 or row.get('counts',{}).get('ActiveProcesses')!=0 or d.get('lostOrOverflowed') is not False or d.get('cleanupFailure') is not False or not isinstance(d.get('entries'),list):raise ValueError('missing/failed cleanup evidence')
    if session.stage=='controls':
        c=terminal.get('controlResult',{});mode=session.cfg['controlCase']
        expected_verdict='EXPECTED_REFUSAL_NONZERO_ROOT' if mode=='nonzero-root' else 'EXPECTED_OWNED_TERMINATION' if mode.startswith('death-') else 'EXPECTED_REFUSAL_UNKNOWN' if mode in ['inherited-handle','cleanup-failure','diagnostic-overflow','diagnostic-truncation'] else 'EXPECTED_OBSERVATION'
        if c.get('mode')!=mode or c.get('exit')!=0 or c.get('verdict')!=expected_verdict:raise ValueError('missing/wrong real control observation')
        if mode=='nonzero-root':
            observed=c.get('nonzeroRoot') or {}
            if observed.get('actualRootExit')!=2 or observed.get('authoritativeEmpty') is not True or observed.get('activeProcesses')!=0 or observed.get('classification')!='STOPPED':raise ValueError('nonzero root/empty/refusal evidence missing')

def source_archive(session):
    a=session.args;sha=a.candidate_sha
    if subprocess.check_output(['git','rev-parse','HEAD'],cwd=a.source,text=True).strip()!=sha or subprocess.check_output(['git','status','--porcelain','--untracked-files=all'],cwd=a.source,text=True):raise ValueError('exact clean source required')
    if getattr(a,'reuse_source_run',None):
        previous=Path(a.reuse_source_run)
        if not previous.is_absolute() or not previous.is_relative_to(TASK/'a6-execution') or previous.name!='run.json':raise ValueError('owned source reuse record required')
        old=read_json(previous);archive=previous.parent/'source.zip'
        if old.get('candidateSha')!=sha or old.get('runId')!=previous.parent.name or digest(archive)!=old.get('archiveSha256'):raise ValueError('frozen source reuse identity differs')
        observed=json.loads(command("$p="+quote(old['sourceArchive'])+";@{sha256=(Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant();bytes=(Get-Item -LiteralPath $p).Length}|ConvertTo-Json -Compress",timeout=20))
        if observed['sha256']!=old['archiveSha256'] or observed['bytes']!=archive.stat().st_size:raise ValueError('verified guest source archive differs')
        session.cfg.update({k:old[k] for k in ['sourceArchive','archiveSha256','workspace','sourceFiles','sourceArchiveExcludedPaths']})
        session.cfg.update(resumeSource=True,reusedSourceEvidence={'configPath':str(previous),'configSha256':digest(previous),'productSha':sha},controllerSha=subprocess.check_output(['git','rev-parse','HEAD'],cwd=PACKAGE,text=True).strip())
        session.log('frozen-product-source-reused',{'productSha':sha,'controllerSha':session.cfg['controllerSha'],'archiveSha256':old['archiveSha256']});return
    excluded='dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot/'
    raw=session.root/'git-source.zip';archive=session.root/'source.zip'
    subprocess.run(['git','archive','--format=zip','--output',str(raw),sha],cwd=a.source,check=True)
    with zipfile.ZipFile(raw) as source,zipfile.ZipFile(archive,'w') as delivery:
        for info in source.infolist():
            if not info.filename.startswith(excluded):delivery.writestr(info,source.read(info))
    entries={};seen=set()
    with zipfile.ZipFile(archive) as z:
        for info in z.infolist():
            if info.is_dir():continue
            n=info.filename
            if n.startswith('/') or '\\' in n or ':' in n or '..' in n.split('/') or n.casefold() in seen or stat.S_ISLNK(info.external_attr>>16):raise ValueError('unsafe source archive')
            if any(x.endswith(('.',' ')) or re.fullmatch(r'(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\..*)?',x) for x in n.split('/')):raise ValueError('Windows-ambiguous source path')
            seen.add(n.casefold());entries[n]=hashlib.sha256(z.read(info)).hexdigest()
    session.cfg.update({'sourceArchive':GUEST+'\\inputs\\'+session.run_id+'\\source.zip','archiveSha256':digest(archive),'workspace':GUEST+'\\source\\'+sha,'sourceFiles':entries,'sourceArchiveExcludedPaths':[excluded]})
    session.upload(archive,'inputs/'+session.run_id+'/source.zip')

def load_observed(session,args):
    probe=read_json(args.probe);files=probe['files']
    if probe['hostname']!='MEMEX-AFE8V4I00' or probe.get('diskUniqueId')!=session.cfg['diskUniqueId'] or str(uuid.UUID(probe.get('partitionGuid','').strip('{}')))!=session.cfg['partitionGuid']:raise ValueError('probe venue/disk identity')
    for path,h in files.items():
        if not valid_digest(h):raise ValueError('actual observed hash missing')
    session.cfg['compilerFiles']=files
    session.cfg['powershellSha256']=files['C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe']
    session.cfg['cargoSha256']=files[session.cfg['cargoExe']];session.cfg['rustcSha256']=files[session.cfg['rustcExe']]

def verify_readiness(session,ready):
    reference=ready.get('controlsIndex',{})
    path=Path(reference.get('path',''))
    if not path.is_absolute() or not path.is_relative_to(TASK/'a6-execution') or not valid_digest(reference.get('sha256')) or digest(path)!=reference['sha256']:raise ValueError('actual controls index identity missing')
    index=read_json(path)
    if index.get('packageManifestSha256')!=session.package_hash or index.get('candidateSha')!=session.cfg['candidateSha']:raise ValueError('controls source/package differs')
    required=set(CASES+['invalid-source','linger:expire','linger:cancel','linger:transport','controller-group-death','launch-negative','transfer-digest','transfer-length'])
    keys=[x.get('key') for x in index.get('results',[])]
    if set(keys)!=required or len(keys)!=len(required) or any(x.get('status') not in ('EXPECTED_OBSERVATION','EXPECTED_REFUSAL') for x in index['results']):raise ValueError('missing/duplicate/unqualified operational controls')
    for row in index['results']:
        if 'evidence' in row:
            folder=Path(row['evidence']);name=row.get('observationFile','result.json');record=folder/name
            if not folder.is_relative_to(TASK/'a6-execution') or not valid_digest(row.get('resultSha256')) or digest(record)!=row['resultSha256']:raise ValueError('control raw receipt identity differs')

def main():
    p=argparse.ArgumentParser();p.add_argument('stage',choices=['probe','parse','compile','controls','a6','transfer-negative','launch-negative'])
    p.add_argument('--authority',type=Path,required=True);p.add_argument('--venue',type=Path,required=True);p.add_argument('--candidate-sha',required=True);p.add_argument('--source',type=Path,default=Path('/mnt/data/worktrees/fathomdb-0.8.29-s20-a6-current-20261115'));p.add_argument('--probe',type=Path);p.add_argument('--parse',type=Path);p.add_argument('--binaries',type=Path);p.add_argument('--readiness',type=Path);p.add_argument('--case',choices=CASES+['linger','nonzero-root']);p.add_argument('--reuse-source-run',type=Path);p.add_argument('--stop-mode',choices=['expire','cancel','transport']);p.add_argument('--transfer-fault',choices=['digest','length'])
    args=p.parse_args();s=Session(args,args.stage);s.identity();s.sample()
    if args.stage=='probe':s.probe();print(s.root);return 0
    s.stage_package()
    if args.stage=='parse':
        b64=base64.b64encode(json.dumps(s.cfg).encode()).decode();value=json.loads(command(s.inline('parse-package.ps1','Invoke-A6Parse '+quote(b64)),timeout=30));(s.root/'powershell-parse.json').write_text(json.dumps(value,indent=2)+'\n');print(s.root);return 0
    if args.stage=='transfer-negative':
        if not args.transfer_fault:raise ValueError('explicit transfer fault required')
        path=s.root/'transfer-control.bin';path.write_bytes(b'A6 owned transfer control\n')
        relative='packages/'+s.package_hash+'/transfer-control-'+s.run_id+'.bin'
        error_log=s.root/'transfer.stderr';offset=error_log.stat().st_size if error_log.exists() else 0
        try:s.upload(path,relative,wrong_digest=args.transfer_fault=='digest',extra_length=1 if args.transfer_fault=='length' else 0)
        except RuntimeError as e:
            raw=error_log.read_bytes()[offset:].decode(errors='replace')
            if 'transfer truncated or digest differs' not in raw:raise RuntimeError('unrelated failure is not transfer-refusal proof') from e
            target=GUEST+'\\'+relative.replace('/', '\\')
            if command('if(Test-Path -LiteralPath '+quote(target)+"){ 'present' }else{ 'absent' }")!='absent':raise RuntimeError('failed transfer materialized final artifact')
            (s.root/'negative-transfer.json').write_text(json.dumps({'expectedRefusal':True,'raw':raw,'finalDestinationAbsent':True,'runtimeQualification':False})+'\n');print(s.root);return 0
        raise RuntimeError('negative transfer unexpectedly succeeded')
    if args.probe is None:raise ValueError('actual observed tool/interpreter hashes required')
    parsed=read_json(args.parse) if args.parse else None
    if not parsed or parsed.get('valid') is not True or parsed.get('packageManifestSha256')!=s.package_hash:raise ValueError('actual matching PowerShell parse receipt required')
    load_observed(s,args);s.cfg['stage']='compile' if args.stage in ('compile','launch-negative') else args.stage
    if args.stage in ('controls','a6'):
        b=read_json(args.binaries)
        if b.get('approved') is not True or b.get('packageManifestSha256')!=s.package_hash:raise ValueError('independent compiled-binary review missing')
        s.cfg['reviewedBinaries']=b['binaries'];by_name={Path(k.replace('\\','/')).name:k for k in b['binaries']}
        if len(b['binaries'])!=3 or len(by_name)!=3:raise ValueError('compiled binary role mapping ambiguous')
        for name in ['Supervisor.exe','Controls.exe','OwnedChild.exe']:
            if name not in by_name or not valid_digest(b['binaries'][by_name[name]]):raise ValueError('actual reviewed binary/hash missing')
        s.cfg['controlsExe']=by_name['Controls.exe'];s.cfg['ownedChildExe']=by_name['OwnedChild.exe'];s.cfg['wrapperPath']=by_name['Supervisor.exe'];s.cfg['wrapperSha256']=b['binaries'][s.cfg['wrapperPath']]
    if args.stage=='controls':
        if not args.case:raise ValueError('control case required')
        s.cfg['controlCase']=args.case
    if args.stage=='a6':
        ready=read_json(args.readiness)
        if ready.get('approved') is not True or ready.get('packageManifestSha256')!=s.package_hash or ready.get('reviewedBinaries')!=s.cfg['reviewedBinaries']:raise ValueError('actual prerequisite controls/readiness approval missing')
        verify_readiness(s,ready)
        source_archive(s);s.manifest()
        s.extract()
        result=s.run()
    else:result=s.run(stop_mode=args.stop_mode,bad_launcher=args.stage=='launch-negative')
    print(json.dumps({'status':result['status'],'evidence':str(s.root)}))
    return 0 if result['status'] in ('COMPLETED','EXPECTED_CONTROLLED_STOP') else 125

if __name__=='__main__':raise SystemExit(main())
