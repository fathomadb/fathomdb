"""Build normal G and retain actual Cargo feature closure before manual fixtures."""

import hashlib,json,os,subprocess,sys,time
from pathlib import Path
task=Path(os.environ['S30_GD_TASK']); label=sys.argv[1] if len(sys.argv)>1 else str(time.time_ns())
log=task/'logs'/f'gd-bge-template-{label}-compiler.jsonl'
assert not log.exists()
environment = dict(os.environ)
environment.update(CUDA_HOME="/usr/local/cuda-12.6", CUDA_PATH="/usr/local/cuda-12.6")
environment["PATH"] = "/usr/local/cuda-12.6/bin:" + environment["PATH"]
with log.open('w') as output:
    result=subprocess.run(['cargo','build','--locked','--release','-p','fathomdb-sdk','--example','slice30_g_mean','--features','embed-cuda','--message-format=json'],stdout=output,env=environment)
rows=[json.loads(line) for line in log.read_text().splitlines() if line.strip()]
for row in rows:
    if row.get('reason')=='compiler-message': print(row['message']['rendered'],end='')
if result.returncode: raise SystemExit(result.returncode)
names={'fathomdb_engine','fathomdb_embedder','fathomdb_sdk','slice30_g_mean'}
artifacts=[{key:row[key] for key in ['package_id','target','features','filenames']} for row in rows if row.get('reason')=='compiler-artifact' and row['target']['name'] in names]
engine=[row for row in artifacts if row['target']['name']=='fathomdb_engine']
assert len(engine)==1 and 'embed-cuda' in engine[0]['features']
assert not {'test-hooks','slice30-measurement'}.intersection(engine[0]['features'])
binary=task/'target/release/examples/slice30_g_mean'
proof={'normal_gn':True,'artifacts':artifacts,'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'compiler_json_sha256':hashlib.sha256(log.read_bytes()).hexdigest(),'lock_sha256':hashlib.sha256(Path('Cargo.lock').read_bytes()).hexdigest(),'source':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'delta':subprocess.check_output(['git','status','--porcelain'],text=True)}
encoded=json.dumps(proof,indent=2)+'\n'
(task/'proofs'/f'gd-gn-feature-closure-{label}.json').write_text(encoded)
(task/'proofs/gd-gn-feature-closure.json').write_text(encoded)
print(json.dumps({'binary_sha256':proof['binary_sha256'],'normal_gn':True,'engine_features':engine[0]['features']}),flush=True)
