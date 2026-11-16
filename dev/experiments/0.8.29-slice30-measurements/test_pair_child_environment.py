"""Actual harmless child environment oracles; no model/GPU/native campaign qualification."""

import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from g_driver import run_program_step

UUID='GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b'
KEYS=['CUDA_VISIBLE_DEVICES','FATHOMDB_EMBED_DEVICE','HF_HOME','HF_HUB_OFFLINE',
      'TRANSFORMERS_OFFLINE','FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT']
PARENT={'CUDA_VISIBLE_DEVICES':'','FATHOMDB_EMBED_DEVICE':'cpu','HF_HOME':'/parent/cache',
        'HF_HUB_OFFLINE':'parent-offline','TRANSFORMERS_OFFLINE':'parent-transformers',
        'FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT':'parent-selector'}
CHILD="\n".join([
    'import json,os,pathlib,sys,time',
    'c=json.loads(pathlib.Path(sys.argv[1]).read_text());out=pathlib.Path(sys.argv[2]);out.mkdir()',
    'time.sleep(0.05)',
    f"seen={{key:os.environ.get(key) for key in {KEYS!r}}}",
    "(out/'result.json').write_text(json.dumps({'complete':False,'runner_only':True,'seen':seen,'config':c}))",
])


class PairChildEnvironmentTests(unittest.TestCase):
    def test_actual_paired_g_children_receive_uuid_mapping_and_keep_mode_selector(self):
        for family,mode,observed in [('G-R','winner',False),('G-R','eager',True),
                                     ('G-W','winner',True),('G-W','eager',False)]:
            with self.subTest(family=family,mode=mode),tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary)
                config={'family':family,'slice50_pair_mode':mode,'observe':observed,
                        'slice50_gpu':{'uuid':UUID,'ordinal':0,'hf_home':str(root/'owned-cache')}}
                original=copy.deepcopy(config)
                with patch.dict(os.environ,PARENT):
                    before=dict(os.environ)
                    step=run_program_step([sys.executable,'-c',CHILD],config,root/'child',time.monotonic_ns()+5*10**9)
                    self.assertEqual(dict(os.environ),before)
                self.assertTrue(step['result']['runner_only'])
                seen=step['result']['seen']
                self.assertEqual(seen['CUDA_VISIBLE_DEVICES'],UUID,
                                 'G native child inherited the frozen empty P visibility')
                self.assertEqual(seen['FATHOMDB_EMBED_DEVICE'],'cuda:0')
                self.assertEqual(seen['HF_HOME'],str(root/'owned-cache'))
                self.assertEqual(seen['HF_HUB_OFFLINE'],'1')
                self.assertEqual(seen['TRANSFORMERS_OFFLINE'],'1')
                self.assertEqual(seen['FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT'],'1' if mode=='eager' else None)
                self.assertEqual(step['child_environment'],{key:seen[key] for key in KEYS[:-1]})
                self.assertEqual(config,original)
                self.assertEqual(json.loads(Path(step['config']).read_text())['slice50_gpu'],original['slice50_gpu'])

    def test_unsupported_paired_g_bindings_refuse_before_any_child_launch(self):
        good={'uuid':UUID,'ordinal':0,'hf_home':'/owned/cache'}
        invalid=[None,dict(good,ordinal=1),dict(good,ordinal=True),dict(good,uuid=''),
                 dict(good,uuid='unselected-device'),dict(good,hf_home=''),{'uuid':UUID,'ordinal':0}]
        for binding in invalid:
            with self.subTest(binding=binding),tempfile.TemporaryDirectory() as temporary:
                output=Path(temporary)/'unlaunched'/'child'
                with self.assertRaisesRegex(ValueError,'paired G|GPU binding'):
                    run_program_step([sys.executable,'-c',CHILD],
                        {'family':'G-R','slice50_pair_mode':'winner','slice50_gpu':binding},
                        output,time.monotonic_ns()+5*10**9)
                self.assertFalse(output.parent.exists())

    def test_paired_p_and_legacy_g_keep_parent_environment(self):
        for family,mode in [('P1','winner'),('P2','eager'),('G-R',None)]:
            with self.subTest(family=family,mode=mode),tempfile.TemporaryDirectory() as temporary:
                config={'family':family}
                if mode is not None:config['slice50_pair_mode']=mode
                with patch.dict(os.environ,PARENT):
                    before=dict(os.environ)
                    step=run_program_step([sys.executable,'-c',CHILD],config,Path(temporary)/'child',time.monotonic_ns()+5*10**9)
                    self.assertEqual(dict(os.environ),before)
                expected=dict(PARENT)
                expected['FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT']=('1' if mode=='eager' else None) if mode else PARENT['FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT']
                self.assertEqual(step['result']['seen'],expected)
                self.assertNotIn('child_environment',step)

    def test_g_payload_keeps_its_environment_unchanged(self):
        from g_pair import FEATURES,plan
        from g_pair_runtime import payload
        cell=next(c for c in plan()['primary'] if c['family']=='G-W' and c['routes']==['text'])
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);attempt=root/'attempt';attempt.mkdir();config=root/'cell.json';config.write_text(json.dumps(cell))
            manifest={'schema':'slice50-paired-campaign/v1','scope':'primary','features':FEATURES,
                'plan':plan(),'root':str(root),'uuid':'source-control','hf_home':'/owned/cache',
                'gpu':{'uuid':UUID,'ordinal':0},'build':{'binaries':{'ordinary':{'path':'never-invoked'}}}}
            (root/'manifest.json').write_text(json.dumps(manifest));original=(root/'manifest.json').read_bytes()
            with patch.dict(os.environ,PARENT),patch('g_pair.worker_admission'),patch('g_pair_runtime.execute_native',return_value={'complete':False,'steps':[]}):
                before=dict(os.environ)
                result=payload(root,config,attempt,time.monotonic_ns()+5*10**9)
                self.assertEqual(dict(os.environ),before,'G worker environment changed outside its native child')
            self.assertFalse(result['complete'])
            self.assertEqual((root/'manifest.json').read_bytes(),original)


if __name__=='__main__':
    unittest.main()
