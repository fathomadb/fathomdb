"""Collector/transport boundaries only; no database integration or GPU work."""

import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from m_runtime import payload

PID=71


class RecordingSession:
    """Record the literal lazy transport and seal its existing clocks before close."""

    def __init__(self, command, root, *, result_format):
        self.root=Path(root);self.root.mkdir()
        (self.root/'stderr.raw').write_text('')
        self.child=SimpleNamespace(pid=PID);self.records={};self.failures=[]
        self.started_ns=time.monotonic_ns();self.handshake_complete_ns=self.started_ns+10
        self.closed=False;self.payload_done=threading.Event();self.sequence=[];self.close_delay=0
        self.timeout=45;self.searches=0;self.ready_ns=None

    def __enter__(self): return self

    def __exit__(self,*args):
        self.sequence.append('EOF');self.closed=True
        time.sleep(self.close_delay)
        self.exit={'code':0,'exit_ns':time.monotonic_ns(),'readers_joined':True}

    def record(self, method, *, name=None, phase='control', elapsed=10):
        rpc=len(self.records)+1;started=self.started_ns+rpc*100
        row={'rpc_id':rpc,'pid':PID,'request':{'method':method,'params':{'name':name}},
             'phase':phase,'status':'completed','ok':True,'elapsed_ns':elapsed,
             'send_start_ns':started,'arrival_ns':started+elapsed,'payload':{}}
        self.records[rpc]=row;return row

    def initialize(self):
        self.sequence.extend(('initialize','notifications/initialized','tools/list'))
        self.record('initialize');self.record('tools/list')

    def tool(self,name,arguments,*,phase='control',**facts):
        self.sequence.append(name)
        if name=='list_stores':return self.record('tools/call',name=name,elapsed=129395)
        self.searches+=1
        return self.record('tools/call',name=name,phase=phase,
                           elapsed=1559620616 if self.searches==1 else 84779048)

    def phase_boundary(self):
        self.ready_ns=time.monotonic_ns()+150000000
        self.payload_done.set()


class LifecycleGpuTests(unittest.TestCase):
    def run_payload(self,mode,*,seconds=5):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'prepared.json').write_text('{"files":{}}')
            q=root/'queries.jsonl';q.write_text('"literal query"\n')
            cell={'id':'M-L-1-lazy','family':'M-L','case':'lazy','search_mode':'hybrid',
                  'result_format':'text','log':'warn'}
            manifest={'task':directory,'gpu':{'uuid':'literal-gpu','ordinal':0},'hf_home':directory,
                'inputs':{'queries':{'path':str(q)}},'uuid':'literal-freeze',
                'builds':{'ordinary':{'binary':{'path':'not-launched'}}}}
            sessions=[];samples=[];good=[False]
            def session(*args,**kwargs):
                value=RecordingSession(*args,**kwargs)
                if mode=='post-close':value.close_delay=1.1
                if mode=='later-error':value.close_delay=.25
                sessions.append(value);return value
            class Observer:
                def __init__(self,*args,**kwargs):self.baseline={'gpu_memory_mib':2,'applications':[]}
                def sample(self,pid,**kwargs):
                    s=sessions[0] if sessions else None
                    late=s is not None and s.payload_done.is_set() and time.monotonic_ns()>=s.ready_ns
                    positive=(late and not s.closed and mode in ('late','later-error','wrong-pid')) or (s is not None and s.closed and mode=='post-close')
                    if late and mode=='error':raise RuntimeError('literal observer error')
                    if good[0] and mode=='later-error':raise RuntimeError('literal late observer error')
                    if positive:good[0]=True
                    row={'pid':pid+1 if mode=='wrong-pid' else pid,'gpu_memory_mib':407,
                        'pid_memory_mib':396,'baseline_gpu_memory_mib':2,
                        'utilization_percent':1 if positive else 0,'monotonic_ns':time.monotonic_ns()}
                    samples.append(dict(row,closed=s.closed if s else None));return row
            start=time.monotonic()
            with patch('m_runtime.worker_admission'),patch('m_cell.McpSession',side_effect=session), \
                    patch('g_gpu.GpuObserver',Observer),patch.dict('os.environ'):
                result=payload(manifest,cell,root,time.monotonic_ns()+int(seconds*10**9))
            wall=time.monotonic()-start;s=sessions[0]
            self.assertTrue(s.closed,'observer failure escaped without closing its MCP transport')
            self.assertEqual(s.sequence,['initialize','notifications/initialized','tools/list',
                                         'list_stores','search','search','EOF'])
            self.assertEqual(len(s.records),5)
            self.assertEqual(result['lifecycle']['first_call_ns'],1559620616)
            self.assertEqual(result['lifecycle']['immediate_call_ns'],84779048)
            self.assertEqual(result['whole_process']['requests'],5)
            return result,wall,samples

    def test_late_true_same_pid_witness_is_observed_before_eof_without_extra_rpcs(self):
        result,wall,rows=self.run_payload('late')
        self.assertTrue(result['complete'],'existing execute closed before the late genuine witness')
        self.assertTrue(any(r['utilization_percent']>0 and r['closed'] is False for r in rows))
        self.assertLess(wall,1.5)

    def test_post_close_positive_sample_cannot_rescue_missing_preclose_witness(self):
        result,wall,rows=self.run_payload('post-close')
        self.assertFalse(result['complete'],'post-EOF samples rescued an unqualified lifecycle')
        self.assertLess(wall,4)

    def test_missing_or_wrong_pid_witness_is_bounded_and_unqualified(self):
        for mode in ('none','wrong-pid'):
            with self.subTest(mode=mode):
                result,wall,_=self.run_payload(mode)
                self.assertFalse(result['complete'])
                self.assertLess(wall,2.8)

    def test_observer_error_closes_transport_and_retains_failure(self):
        result,wall,_=self.run_payload('error')
        self.assertFalse(result['complete'])
        self.assertIn('RuntimeError: literal observer error',result['gpu']['failures'])
        self.assertLess(wall,1.5)

    def test_later_error_invalidates_a_preclose_witness(self):
        result,wall,_=self.run_payload('later-error')
        self.assertFalse(result['complete'])
        self.assertIn('RuntimeError: literal late observer error',result['gpu']['failures'])
        self.assertLess(wall,1.5)

    def test_work_deadline_clips_tail_without_consuming_external_cleanup_reserve(self):
        result,wall,_=self.run_payload('none',seconds=.12)
        self.assertFalse(result['complete'])
        self.assertLess(wall,.7)


class ProbeDeadlineTests(unittest.TestCase):
    def test_lifecycle_baseline_shares_500ms_and_rejects_a_late_successful_query(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'prepared.json').write_text('{"files":{}}')
            q=root/'q';q.write_text('"alpha"\n');now=[1000000000];timeouts=[]
            def run(command,**kwargs):
                timeouts.append(kwargs['timeout']);now[0]+=300000000
                output='literal-gpu,NVIDIA GeForce RTX 3090,580.178.04,2,0\n' if '--query-gpu=' in ' '.join(command) else ''
                return SimpleNamespace(stdout=output)
            manifest={'task':directory,'gpu':{'uuid':'literal-gpu','ordinal':0},'hf_home':directory,
                'inputs':{'queries':{'path':str(q)}},'uuid':'literal',
                'builds':{'ordinary':{'binary':{'path':'not-launched'}}}}
            cell={'family':'M-L','case':'lazy','search_mode':'hybrid','log':'warn'}
            with patch('m_runtime.worker_admission'),patch('g_gpu.time.monotonic_ns',side_effect=lambda:now[0]), \
                    patch('g_gpu.subprocess.run',side_effect=run), \
                    patch('g_gpu.Path',return_value=SimpleNamespace(resolve=lambda:Path('/usr/local/cuda-12.6'))), \
                    patch('m_runtime.execute',return_value={'complete':True,'pid':PID}) as execute,patch.dict('os.environ'):
                result=payload(manifest,cell,root,20000000000)
            self.assertFalse(execute.called,'late baseline result was accepted before launching payload')
            self.assertFalse(result['complete'])
            self.assertEqual(len(timeouts),2)
            self.assertLessEqual(timeouts[0],.5)
            self.assertLessEqual(timeouts[1],.2)


if __name__=='__main__': unittest.main()
