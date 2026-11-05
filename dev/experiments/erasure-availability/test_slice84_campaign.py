import unittest
from slice84_campaign import cells, assess_family, remaining

class ProtocolTests(unittest.TestCase):
    def test_fixed_grid(self):
        grid = cells()
        self.assertEqual(len(grid), 54)
        self.assertEqual(len({c['id'] for c in grid}), 54)
        self.assertEqual([sum(c['phase'] == p for c in grid) for p in ('baseline_profile','candidate_profile','core','context','idle')], [3,3,36,6,6])
        for family in ('node','edge','property'):
            for rep in range(3):
                pair = [c for c in grid if c['phase']=='core' and c['family']==family and c['repetition']==rep]
                self.assertEqual(len(pair),4)
                self.assertEqual(pair[0]['arm'], 'candidate' if rep==1 else 'baseline')
        self.assertTrue(all(c['write_hz']==20 for c in grid if c['family']=='property'))

    def test_budget_carries_segments(self):
        self.assertEqual(remaining([{'elapsed_s':600},{'elapsed_s':100}],50),1950)
        self.assertEqual(remaining([{'elapsed_s':2800}],0),0)

    def records(self):
        rows=[]
        for c in cells():
            if c['phase']!='core' or c['family']!='node': continue
            n=1500 if c['mode']=='control' else (1000 if c['arm']=='baseline' else 1200)
            streams={s:dict(scheduled=1500,completed=n if s=='write' else 1500,engine_refused=0,failed=0,unfinished=0,unknown=0,accounting_errors=0,generator_missed=1500-n if s=='write' else 0) for s in ('read','write')}
            rows.append(dict(cell=c,complete=True,result=dict(native_ns=100 if c['arm']=='baseline' else 70,native_error=None),streams=streams))
        return rows

    def test_gates_and_invalid_controls(self):
        rows=self.records()
        self.assertTrue(assess_family(rows,'node')['qualified'])
        rows[0]['result']['native_error']='incomplete'
        self.assertFalse(assess_family(rows,'node')['qualified'])
        rows=self.records()
        next(r for r in rows if r['cell']['mode']=='control')['streams']['write']['completed']=1499
        self.assertFalse(assess_family(rows,'node')['qualified'])

    def test_missing_pair_and_read_regression_fail(self):
        rows=self.records()
        self.assertFalse(assess_family(rows[:-1],'node')['qualified'])
        next(r for r in rows if r['cell']['arm']=='candidate' and r['cell']['mode']=='erase')['streams']['read']['completed']=1499
        self.assertFalse(assess_family(rows,'node')['qualified'])

class PersistenceTests(unittest.TestCase):
    def run_finish(self, prior_count=53, elapsed=1, malformed=False):
        import json
        from pathlib import Path
        import tempfile
        from unittest.mock import patch
        import slice84_campaign as controller

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cell = dict(id=53, phase='idle', family='property', repetition=0,
                        arm='candidate', mode='idle', write_hz=20)
            manifest = dict(cells=[cell], artifacts={}, masters={'property':
                            dict(path=str(root/'master'), sha256='fixture')},
                            segments=[dict(state='complete', elapsed_s=600)],
                            resource_root=str(root), controller_sha256='fixture',
                            helper_sha256={})
            (root/'manifest.json').write_text(json.dumps(manifest))
            (root/'records.json').write_text(json.dumps([
                dict(cell=dict(id=i), complete=True, streams={stream:dict(failed=0,unfinished=0,unknown=0,accounting_errors=0) for stream in ('read','write')}) for i in range(prior_count)]))

            def child(args, log, owned_root, deadline):
                self.assertEqual(owned_root, root)
                self.assertEqual(deadline, 90)
                target = Path(args[2])
                target.mkdir()
                (target/'result.json').write_text('{' if malformed else '{"verified":true}')
                return dict(exit_code=0, stop=None, deadline_overshoot_s=0)

            with patch.object(controller.sys, 'argv', ['controller', 'finish', str(root/'candidate'), str(root)]), \
                 patch.object(controller, 'digest', return_value='fixture'), \
                 patch.object(controller, 'safe_child', side_effect=child), \
                 patch.object(controller, 'assess_family', return_value={'qualified':True}), \
                 patch.object(controller.time, 'monotonic', side_effect=[0,0,0,0,elapsed]):
                controller.main()
            return (json.loads((root/'assessment.json').read_text()),
                    json.loads((root/'records.json').read_text()),
                    json.loads((root/'manifest.json').read_text()))

    def test_final_validation_time_counts_against_shared_budget(self):
        valid, _, _ = self.run_finish(elapsed=2099)
        self.assertTrue(valid['qualified'])
        late, _, manifest = self.run_finish(elapsed=2101)
        self.assertTrue(late['all54_complete'])
        self.assertEqual(late['active_seconds'], 2701)
        self.assertFalse(late['qualified'])
        self.assertTrue(manifest['segments'][-1]['budget_exceeded'])

    def test_missing_context_cell_prevents_global_qualification(self):
        assessment, _, _ = self.run_finish(prior_count=52)
        self.assertFalse(assessment['all54_complete'])
        self.assertFalse(assessment['qualified'])
        self.assertTrue(all(row['qualified'] for row in assessment['core'].values()))

    def test_unknown_context_offers_prevent_global_qualification(self):
        from unittest.mock import patch
        import slice84_campaign as controller
        _, records, manifest = self.run_finish()
        records[0]['streams']['read']['unknown'] = 1
        with patch.object(controller, 'assess_family', return_value={'qualified':True}):
            assessment = controller.assess_campaign(records, manifest['segments'])
        self.assertFalse(assessment['all54_complete'])
        self.assertFalse(assessment['qualified'])

    def test_malformed_child_result_remains_an_attempt(self):
        assessment, records, _ = self.run_finish(malformed=True)
        self.assertEqual(len(records),54)
        attempted = records[-1]
        self.assertEqual(attempted['cell']['id'],53)
        self.assertEqual(attempted['exit_code'],0)
        self.assertEqual(attempted['stop'],'result_accounting_error')
        self.assertIn('JSONDecodeError',attempted['accounting_error'])
        self.assertFalse(attempted['complete'])
        self.assertFalse(assessment['qualified'])

if __name__=='__main__': unittest.main()
