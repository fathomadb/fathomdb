"""Passive probe deadlines and unchanged non-lifecycle observation defaults."""

import json
from pathlib import Path
from types import SimpleNamespace
import tempfile
import threading
import unittest
from unittest.mock import patch

from m_runtime import payload


class LifecycleProbeTests(unittest.TestCase):
    def test_later_sample_shares_500ms_and_rejects_late_completion(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'prepared.json').write_text('{"files":{}}')
            query=root/'q';query.write_text('"alpha"\n');now=[1000000000];timeouts=[];done=threading.Event()
            def run(command,**kwargs):
                timeouts.append(kwargs['timeout']);baseline=len(timeouts)<=2
                now[0]+=50000000 if baseline else 300000000
                if '--query-gpu=' in ' '.join(command):
                    stdout=f'literal-gpu,NVIDIA GeForce RTX 3090,580.178.04,{2 if baseline else 407},{0 if baseline else 1}\n'
                else:
                    stdout='' if baseline else '71,literal-gpu,396\n';done.set() if not baseline else None
                return SimpleNamespace(stdout=stdout)
            def completed(*args,on_spawn,**kwargs):
                on_spawn(71);self.assertTrue(done.wait(1),'controlled metadata probe did not execute')
                return {'complete':True,'pid':71}
            cell={'family':'M-L','case':'lazy','search_mode':'hybrid','log':'warn'}
            manifest={'task':directory,'gpu':{'uuid':'literal-gpu','ordinal':0},'hf_home':directory,
                'inputs':{'queries':{'path':str(query)}},'uuid':'literal',
                'builds':{'ordinary':{'binary':{'path':'not-launched'}}}}
            with patch('m_runtime.worker_admission'),patch('m_runtime.execute',completed), \
                    patch('g_gpu.time.monotonic_ns',side_effect=lambda:now[0]), \
                    patch('g_gpu.subprocess.run',side_effect=run), \
                    patch('g_gpu.Path',return_value=SimpleNamespace(resolve=lambda:Path('/usr/local/cuda-12.6'))),patch.dict('os.environ'):
                result=payload(manifest,cell,root,20000000000)
            self.assertFalse(result['complete'],'late positive probe completion supplied GPU credit')
            self.assertTrue(any('TimeoutError' in failure for failure in result['gpu']['failures']))
            self.assertEqual(len(timeouts),4)
            self.assertLessEqual(timeouts[2],.5)
            self.assertLessEqual(timeouts[3],.2)

    def test_read_and_format_observation_defaults_stay_one_second_without_new_probe_options(self):
        for family in ('M-R','M-C'):
            with self.subTest(family=family),tempfile.TemporaryDirectory() as directory:
                root=Path(directory);(root/'prepared.json').write_text('{"files":{}}')
                query=root/'q';query.write_text('"alpha"\n');waits=[];constructor=[];sample_options=[];execution=[]
                class Event:
                    def __init__(self):self.stopped=False
                    def is_set(self):return self.stopped
                    def set(self):self.stopped=True
                    def wait(self,seconds):waits.append(seconds);self.stopped=True
                class Thread:
                    def __init__(self,*,target,daemon):self.target=target
                    def start(self):self.target()
                    def join(self,timeout):pass
                    def is_alive(self):return False
                class Observer:
                    def __init__(self,*args,**kwargs):constructor.append(kwargs)
                    def sample(self,pid,**kwargs):
                        sample_options.append(kwargs)
                        return {'pid':pid,'pid_memory_mib':10,'gpu_memory_mib':20,
                                'baseline_gpu_memory_mib':2,'utilization_percent':1}
                def execute(*args,on_spawn,**kwargs):
                    execution.append(kwargs);on_spawn(71);return {'complete':True,'pid':71}
                manifest={'task':directory,'gpu':{'uuid':'literal','ordinal':0},'hf_home':directory,
                    'inputs':{'queries':{'path':str(query)}},'uuid':'literal',
                    'builds':{'ordinary':{'binary':{'path':'not-launched'}}}}
                cell={'family':family,'search_mode':'hybrid','log':'warn'}
                with patch('m_runtime.worker_admission'),patch('m_runtime.execute',execute), \
                        patch('m_runtime.threading.Event',Event),patch('m_runtime.threading.Thread',Thread), \
                        patch('g_gpu.GpuObserver',Observer),patch.dict('os.environ'):
                    result=payload(manifest,cell,root,20000000000)
                self.assertTrue(result['complete'])
                self.assertEqual(waits,[1])
                self.assertEqual(constructor,[{}]);self.assertEqual(sample_options,[{}])
                self.assertEqual(set(execution[0]),{'queries','deadline_ns','files','control_path'})


if __name__=='__main__':unittest.main()
