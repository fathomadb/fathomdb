import json, os, pathlib, signal, socket, subprocess, sys, time

binary, result_dir = sys.argv[1:]
root = pathlib.Path(result_dir).resolve()
env = dict(os.environ, CUDA_VISIBLE_DEVICES='', HF_HUB_OFFLINE='1')
bins = root / 'tools/node_modules/.bin'
for fmt, port in [('text', 18765), ('structured', 18766)]:
    cfg = root / f'{fmt}.json'
    cfg.write_text(json.dumps({'stores': {'demo': {'path': str(root/'demo.sqlite')}}, 'default_store':'demo', 'result_format':fmt, 'idle_timeout_secs':0}))
    log = (root / f'bridge-{fmt}.log').open('w')
    bridge_cmd = [str(bins/'supergateway'), '--stdio', f'{binary} --config {cfg}', '--outputTransport', 'streamableHttp', '--stateful', '--host','127.0.0.1','--port',str(port),'--logLevel','debug']
    bridge = subprocess.Popen(bridge_cmd, stdout=log, stderr=subprocess.STDOUT, env=env, start_new_session=True)
    try:
        for _ in range(100):
            if bridge.poll() is not None: raise RuntimeError('bridge exited')
            try:
                with socket.create_connection(('127.0.0.1',port), timeout=.2): break
            except OSError: time.sleep(.1)
        cmd = [str(bins/'conformance'),'server','--url',f'http://127.0.0.1:{port}/mcp','--suite','all','-o',str(root/f'official-{fmt}'),'--verbose']
        (root/f'official-{fmt}-command.json').write_text(json.dumps(cmd))
        with (root/f'official-{fmt}.log').open('w') as out:
            result = subprocess.run(cmd, env=env, stdout=out, stderr=subprocess.STDOUT, timeout=600)
        (root/f'official-{fmt}.exit').write_text(str(result.returncode))
        print(fmt, 'official exit', result.returncode, flush=True)
    finally:
        os.killpg(bridge.pid, signal.SIGTERM)
        try: bridge.wait(timeout=8)
        except subprocess.TimeoutExpired: os.killpg(bridge.pid, signal.SIGKILL); bridge.wait()
        log.close()
    cases = [('initialize',None,None), ('tools/list',None,None), *[('tools/call', n, a) for n,a in [
        ('list_stores',{}),('use_store',{'store':'demo'}),('orient',{}),('search',{'query':'alpha','max_tokens':10000}),
        ('fetch',{'refs':['l:note:1','l:missing','bad','l:note:1'],'max_tokens':10000}),('expand',{'refs':['l:note:1'],'max_depth':2,'max_tokens':10000}),
        ('feedback',{'response_id':'r1-expired','used':[]}),('search',{'store':'missing','query':'alpha'}),('search',{'query':123}),('drop_store',{})]]]
    for i,(method,tool,args) in enumerate(cases):
        cmd=[str(bins/'mcp-inspector'),'--cli','--method',method,'--format','json']
        if method=='tools/list': cmd+=['--strict']
        if tool: cmd+=['--tool-name',tool,'--tool-args-json',json.dumps(args)]
        cmd+=['--',binary,'--config',str(cfg)]
        with (root/f'inspector-{fmt}-{i:02}.stdout').open('w') as out, (root/f'inspector-{fmt}-{i:02}.stderr').open('w') as err:
            result=subprocess.run(cmd,env=env,stdout=out,stderr=err,timeout=35)
        (root/f'inspector-{fmt}-{i:02}.exit').write_text(str(result.returncode))
        print(fmt,'inspector',i,method,tool,result.returncode,flush=True)
