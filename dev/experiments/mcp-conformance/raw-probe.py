import json, os, pathlib, queue, signal, subprocess, sys, threading

binary, root = sys.argv[1:]
root=pathlib.Path(root)
results=[]
class Peer:
    def __init__(self, fmt):
        self.p=subprocess.Popen([binary,'--config',str(root/f'{fmt}.json')],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
        self.q=queue.Queue(); self.rows=[];self.err=[]
        def output():
            for line in self.p.stdout:
                self.rows.append({'receive':line.rstrip('\n')})
                try:self.q.put(json.loads(line))
                except Exception:self.q.put({'unparseable':line})
        threading.Thread(target=output,daemon=True).start()
        threading.Thread(target=lambda:self.err.extend(self.p.stderr.readlines()),daemon=True).start()
    def send(self, value):
        line=value if isinstance(value,str) else json.dumps(value)
        self.rows.append({'send':line});self.p.stdin.write(line+'\n');self.p.stdin.flush()
    def recv(self):return self.q.get(timeout=3)
    def request(self, method, params=None, id=1):
        d={'jsonrpc':'2.0','id':id,'method':method}
        if params is not None:d['params']=params
        self.send(d);return self.recv()
    def init(self,version='2025-11-25'):
        r=self.request('initialize',{'protocolVersion':version,'capabilities':{},'clientInfo':{'name':'raw-probe','version':'1'}},'init')
        self.send({'jsonrpc':'2.0','method':'notifications/initialized'});return r
    def close(self):
        if not self.p.stdin.closed:self.p.stdin.close()
        try:code=self.p.wait(timeout=4)
        except subprocess.TimeoutExpired:os.killpg(self.p.pid,signal.SIGKILL);code=self.p.wait()
        return code

for fmt in ['text','structured']:
    cases=[]
    def run(name,fn):
        p=Peer(fmt)
        try:detail=fn(p);cases.append({'name':name,'pass':True,'detail':detail})
        except Exception as e:cases.append({'name':name,'pass':False,'error':repr(e)})
        finally:
            code=p.close();cases[-1].update(exit=code,transcript=p.rows,stderr=p.err)
    def equal(a,b):assert a==b,(a,b)
    for version in ['2025-03-26','2025-06-18','1999-01-01']:
        def version_check(p,v=version):
            r=p.init(v);equal(r['result']['protocolVersion'],'2025-11-25' if v=='1999-01-01' else v)
        run('version '+version,version_check)
    run('EOF before initialize',lambda p:equal(p.close(),0))
    run('EOF after initialize',lambda p:(p.init(),equal(p.close(),0)))
    run('garbage then EOF',lambda p:(p.send('not-json'),equal(p.close(),0)))
    run('string request ID',lambda p:(p.init(),equal(p.request('ping',id='string-id'),{'jsonrpc':'2.0','id':'string-id','result':{}})))
    run('preinitialize ping',lambda p:equal(p.request('ping')['result'],{}))
    run('preinitialize tools/list inline lifecycle refusal',lambda p:equal(p.request('tools/list')['error']['code'],-32602))
    def malformed_line(p):
        p.init();p.send('not-json');r=p.request('ping',id='after-garbage');equal(r['id'],'after-garbage');equal(r['result'],{});return 'Malformed line ignored; no -32700 response; session survives.'
    run('malformed JSON retained rmcp behavior',malformed_line)
    for params in [{},{'name':'search','arguments':[]}]:
        def malformed_params(p,args=params):p.init();equal(p.request('tools/call',args)['error']['code'],-32601)
        run('malformed tools/call '+json.dumps(params),malformed_params)
    run('omitted arguments valid',lambda p:(p.init(),equal(p.request('tools/call',{'name':'list_stores'})['result']['isError'],False)))
    def notifications(p):
        p.init();p.send({'jsonrpc':'2.0','method':'unknown/notification'});p.send({'jsonrpc':'2.0','method':'notifications/cancelled','params':{'requestId':'missing'}});equal(p.request('ping')['result'],{})
    run('unknown notification and cancellation ignored',notifications)
    def batch(p):
        p.init();p.send([{'jsonrpc':'2.0','id':1,'method':'ping'}]);r=p.recv();equal(r['error']['code'],-32600);equal(p.request('ping',id=2)['result'],{})
    run('batch rejected and session survives',batch)
    def cursor(p):p.init();r=p.request('tools/list',{'cursor':'invalid'});equal(len(r['result']['tools']),7);return 'Unknown cursor ignored for fixed unpaginated tool list.'
    run('invalid tools cursor retained behavior',cursor)
    def methods(p):
        p.init()
        for method in ['logging/setLevel','resources/read','prompts/get']:
            params={'level':'debug'} if method=='logging/setLevel' else {'uri':'file:///nope'} if method=='resources/read' else {'name':'nope'}
            equal(p.request(method,params)['error']['code'],-32601)
        for method,key in [('resources/list','resources'),('prompts/list','prompts')]:equal(p.request(method)['result'][key],[])
    run('undeclared capabilities refused or empty',methods)
    def no_sockets(p):
        p.init();links=[]
        for f in pathlib.Path(f'/proc/{p.p.pid}/fd').iterdir():
            try:target=os.readlink(f)
            except OSError:continue
            if target.startswith('socket:'):links.append(target)
        equal(links,[])
    run('stdio child has no socket',no_sockets)
    results.append({'format':fmt,'passed':sum(c['pass'] for c in cases),'total':len(cases),'cases':cases})
    (root/f'raw-{fmt}.json').write_text(json.dumps(results[-1],indent=2))
print(json.dumps([{k:v for k,v in r.items() if k!='cases'} for r in results],indent=2))
if any(r['passed']!=r['total'] for r in results):sys.exit(1)
