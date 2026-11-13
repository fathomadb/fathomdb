#!/usr/bin/env python3
"""Capacity-only derivative: create, narrow ACL, live attach, read-only identity; no guest mutation."""
import argparse,base64,hashlib,json,os,stat,subprocess,time,uuid,xml.etree.ElementTree as ET
from pathlib import Path
PACKAGE=Path(__file__).resolve().parent
PREDECESSOR_MANIFEST_SHA256='5122aa082974461cb8a6f04e2f96accd645c19554844419cd43131355171aafa'
PREDECESSOR_HOST_SHA256='30f3c681ce3a4fa6da1a1fdea9ac3e07545655b127c79b6aed56ffa6ff756075'
TASK=Path('/mnt/data/tmp2/fathomdb-0829-continuation/slice20')
SOURCE=Path('/mnt/data/worktrees/fathomdb-0.8.29-s20-approved-20261113')
IMAGES=TASK.parent/'slice20-windows-images-approved-20261113'
IMAGE=IMAGES/'windows-a6-64g.qcow2'
VM='gh-runner-wonl-win11';VMUUID='3a928939-57ed-4c62-999b-b610f3e44d42'
OSIMAGE='/var/lib/libvirt/images/gh-runner-wonl-win11.qcow2'
GIB=1024**3
SSH=['ssh','-F','/dev/null','-i','/home/coreyt/local/gh-runner-wonl/data/windows11/memex-local-ci-ed25519','-o','IdentitiesOnly=yes','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=5','-o','ControlMaster=no','-o','ProxyCommand=none','runner@192.168.122.51']
def digest(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(argv,timeout=10):
 def record(value):
  with (EVIDENCE/'commands.jsonl').open('a') as f:
   f.write(json.dumps(value)+'\n');f.flush();os.fsync(f.fileno())
 def output(value):return value.decode(errors='replace') if isinstance(value,bytes) else value or ''
 token=uuid.uuid4().hex
 record({'event':'COMMAND_START','token':token,'utc':time.time(),'argv':argv,'cwd':str(Path.cwd()),'timeoutSeconds':timeout})
 try:r=subprocess.run(argv,capture_output=True,text=True,timeout=timeout)
 except subprocess.TimeoutExpired as e:
  record({'event':'COMMAND_TIMEOUT','token':token,'utc':time.time(),'argv':argv,'stdout':output(e.stdout),'stderr':output(e.stderr),'error':repr(e),'termination':'DIRECT_CHILD_KILLED_REAPED_BY_SUBPROCESS_RUN;DESCENDANTS_AND_GUEST_UNKNOWN'})
  raise
 except BaseException as e:
  record({'event':'COMMAND_EXCEPTION','token':token,'utc':time.time(),'argv':argv,'error':repr(e),'termination':'UNKNOWN'})
  raise
 record({'event':'COMMAND_EXIT','token':token,'utc':time.time(),'argv':argv,'returncode':r.returncode,'stdout':r.stdout,'stderr':r.stderr})
 if r.returncode:raise RuntimeError('command failed; original diagnostics retained')
 return r.stdout.strip()
def ps(script,timeout=10):
 enc=base64.b64encode(script.encode('utf-16le')).decode()
 if len(enc)+200>=30000:raise ValueError('encoded command bound')
 return run(SSH+['C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe','-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand',enc],timeout)
def no_symlinks(p):
 for x in [p,*p.parents]:
  if x.is_symlink():raise RuntimeError('symlink path component')
def sample():
 mem={x.split(':',1)[0]:x.split(':',1)[1].split()[0] for x in Path('/proc/meminfo').read_text().splitlines()}
 def du(p):return int(run(['du','-sb',str(p)],10).split()[0]) if p.exists() else 0
 f={'rootFree':os.statvfs('/').f_bavail*os.statvfs('/').f_frsize,'taskFree':os.statvfs(TASK).f_bavail*os.statvfs(TASK).f_frsize,'memory':int(mem['MemAvailable'])*1024,'ownedReserved':max(128*GIB,du(TASK)+du(SOURCE))+max(64*GIB,du(IMAGES))}
 with (EVIDENCE/'resources.jsonl').open('a') as out:out.write(json.dumps(f)+'\n')
 if f['rootFree']<32*GIB or f['taskFree']<250*GIB or f['memory']<16*GIB or f['ownedReserved']>=200*GIB:raise RuntimeError('provisioning start/resource floor')
def xml(inactive=False):return ET.fromstring(run(['virsh','-c','qemu:///system','dumpxml',VM]+(['--inactive'] if inactive else [])))
def disk_inventory(tree):return [(d.findtext('serial'),d.find('source').attrib if d.find('source') is not None else {},d.find('target').attrib) for d in tree.findall('./devices/disk')]
def check_vm(tree):
 if tree.findtext('uuid')!=VMUUID or tree.findtext('name')!=VM:raise RuntimeError('VM identity')
 osdisk=[d for d in tree.findall('./devices/disk') if d.find('target').get('dev')=='sda']
 if len(osdisk)!=1 or osdisk[0].find('source').get('file')!=OSIMAGE:raise RuntimeError('original OS disk differs')
 controllers=tree.findall('./devices/controller')
 if not any(c.get('type')=='usb' and c.get('index')=='0' and c.get('model')=='qemu-xhci' and int(c.get('ports','0'))>=10 for c in controllers):raise RuntimeError('USB controller differs')
def main():
 global EVIDENCE
 a=argparse.ArgumentParser();a.add_argument('--approval',type=Path,required=True);args=a.parse_args()
 no_symlinks(PACKAGE);no_symlinks(TASK);no_symlinks(IMAGES)
 own=json.loads((PACKAGE/'manifest.json').read_text())
 for name,h in own['files'].items():
  if name.startswith('/') or '..' in name.split('/') or digest(PACKAGE/name)!=h:raise RuntimeError('capacity derivative differs')
 authority=json.loads(args.approval.read_text())
 if authority.get('approved') is not True or authority.get('operation')!='capacity-only-create-attach' or authority.get('packetManifestSha256')!=digest(PACKAGE/'manifest.json') or authority.get('rulingReference')!='seq-370' or authority.get('nativeWindowReleased') is not True:raise RuntimeError('actual disk approval/review/window absent')
 if IMAGES.exists() or IMAGE.exists():raise RuntimeError('owned image destination occupied; no retry')
 EVIDENCE=TASK/('capacity-only-'+uuid.uuid4().hex);EVIDENCE.mkdir(mode=0o700)
 (EVIDENCE/'approval.json').write_bytes(args.approval.read_bytes())
 status={'status':'UNKNOWN','image':str(IMAGE),'automaticRollback':False,'guestMutations':False,'compilerHelperTests':False}
 try:
  sample();before=xml();check_vm(before);inactive=ET.tostring(xml(True));old_disks=disk_inventory(before)
  if any(d.find('target').get('dev')=='sdg' or d.findtext('serial')=='FDB29S20A6' for d in before.findall('./devices/disk')) or any(x.get('type')=='usb' and x.get('bus')=='0' and x.get('port')=='10' for x in before.findall('.//address')):raise RuntimeError('disk target/serial/port occupied')
  qemu=[]
  for p in Path('/proc').iterdir():
   if not p.name.isdigit():continue
   try:
    argv=(p/'cmdline').read_bytes().split(b'\0')
    if b'qemu-system' in argv[0] and b'-uuid' in argv and argv[argv.index(b'-uuid')+1].decode()==VMUUID:qemu.append(p.stat().st_uid)
   except (OSError,ValueError,IndexError):pass
  if qemu!=[64055]:raise RuntimeError('QEMU UID differs; stop for review')
  guest=json.loads(ps("$ErrorActionPreference='Stop';if([Environment]::MachineName -cne 'MEMEX-AFE8V4I00'){throw 'hostname'};if(Get-PSDrive R -ErrorAction SilentlyContinue){throw 'R occupied'};$os=Get-CimInstance Win32_OperatingSystem -OperationTimeoutSec 2;if(1024L*$os.FreePhysicalMemory -lt 16GB){throw 'guest floor'};if(@(Get-Process cargo,rustc,cl,ninja -ErrorAction SilentlyContinue).Count){throw 'native window busy'};@{disks=@(Get-Disk|Select-Object Number,UniqueId,SerialNumber,Size,IsBoot,IsSystem,@{Name='PartitionStyle';Expression={$_.PartitionStyle.ToString()}});drives=@(Get-PSDrive -PSProvider FileSystem|Select-Object Name,Free)}|ConvertTo-Json -Depth 5 -Compress"))
  (EVIDENCE/'guest-before.json').write_text(json.dumps(guest,indent=2)+'\n');(EVIDENCE/'live-before.xml').write_bytes(ET.tostring(before));(EVIDENCE/'inactive-before.xml').write_bytes(inactive)
  sample();os.umask(0o077);IMAGES.mkdir(mode=0o700)
  run(['qemu-img','create','-f','qcow2',str(IMAGE),'68719476736'],30)
  info=json.loads(run(['qemu-img','info','--output=json',str(IMAGE)]))
  if info['format']!='qcow2' or info['virtual-size']!=64*GIB or info.get('backing-filename'):raise RuntimeError('created image differs')
  sample();run(['setfacl','-m','u:64055:--x',str(IMAGES)]);run(['setfacl','-m','u:64055:rw-',str(IMAGE)])
  run(['getfacl','--absolute-names',str(IMAGES),str(IMAGE)])
  device=EVIDENCE/'device.xml';device.write_text("<disk type='file' device='disk'><driver name='qemu' type='qcow2' cache='none'/><source file='"+str(IMAGE)+"'/><target dev='sdg' bus='usb'/><serial>FDB29S20A6</serial><address type='usb' bus='0' port='10'/></disk>\n")
  # Live attachment only. Timeout/transport loss stops subsequent actions; attachment outcome stays UNKNOWN.
  sample();run(['virsh','-c','qemu:///system','attach-device',VM,str(device),'--live'],30)
  after=xml();check_vm(after)
  new=[d for d in after.findall('./devices/disk') if d.findtext('serial')=='FDB29S20A6']
  if len(new)!=1:raise RuntimeError('attachment ambiguous')
  d=new[0]
  if d.find('source').get('file')!=str(IMAGE) or d.find('target').attrib!={'dev':'sdg','bus':'usb'} or d.find('address').attrib!={'type':'usb','bus':'0','port':'10'}:raise RuntimeError('attachment identity differs')
  others=[x for x in disk_inventory(after) if x[0]!='FDB29S20A6']
  # libvirt source index is runtime metadata; compare all existing full entries from the same live VM.
  if others!=old_disks or ET.tostring(xml(True))!=inactive:raise RuntimeError('existing disk/persistent configuration changed')
  (EVIDENCE/'live-after.xml').write_bytes(ET.tostring(after))
  # Read-only inventory only. No Initialize-Disk/New-Partition/Format-Volume or helper.
  after_guest=json.loads(ps("$ErrorActionPreference='Stop';if([Environment]::MachineName -cne 'MEMEX-AFE8V4I00'){throw 'hostname'};@{disks=@(Get-Disk|Select-Object Number,UniqueId,SerialNumber,Size,IsBoot,IsSystem,@{Name='PartitionStyle';Expression={$_.PartitionStyle.ToString()}});drives=@(Get-PSDrive -PSProvider FileSystem|Select-Object Name,Free)}|ConvertTo-Json -Depth 5 -Compress"))
  (EVIDENCE/'guest-after-readonly.json').write_text(json.dumps(after_guest,indent=2)+'\n')
  old_ids={d['UniqueId'] for d in guest['disks']}
  fresh=[d for d in after_guest['disks'] if d['UniqueId'] not in old_ids]
  if len(fresh)!=1:raise RuntimeError('new guest disk identity absent/ambiguous; no further action')
  disk=fresh[0]
  if disk['Number']==0 or not disk['UniqueId'] or disk['SerialNumber'].strip()!='FDB29S20A6' or disk['Size']!=64*GIB or disk['IsBoot'] or disk['IsSystem'] or disk['PartitionStyle']!='RAW':raise RuntimeError('new guest disk differs; no further action')
  old_after=[d for d in after_guest['disks'] if d['UniqueId'] in old_ids]
  if sorted(old_after,key=lambda d:d['UniqueId'])!=sorted(guest['disks'],key=lambda d:d['UniqueId']):raise RuntimeError('prior guest disks differ; no further action')
  sample();status.update({'status':'ATTACHED_RAW_CAPACITY_ONLY','guestDisk':disk,'runtimeQualification':False,'formatAuthority':'HELD','compilerHelperTestAuthority':'HELD'})
 except BaseException as e:status['error']=repr(e);raise
 finally:
  (EVIDENCE/'result.json').write_text(json.dumps(status,indent=2)+'\n');print(EVIDENCE)
if __name__=='__main__':main()
