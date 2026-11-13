"""Contained operator controls: all host/guest command boundaries are simulated."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
import subprocess
import sys
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import uuid
import xml.etree.ElementTree as ET

TOOL = Path(__file__).with_name('capacity_only.py')
GIB = 1024**3
VMUUID = '3a928939-57ed-4c62-999b-b610f3e44d42'
ORIGINAL = {'Number': 0, 'UniqueId': 'original', 'SerialNumber': 'QM00001',
            'Size': 120*GIB, 'IsBoot': True, 'IsSystem': True, 'PartitionStyle': 'GPT'}
FRESH = {'Number': 1, 'UniqueId': 'scratch', 'SerialNumber': 'FDB29S20A6',
         'Size': 64*GIB, 'IsBoot': False, 'IsSystem': False, 'PartitionStyle': 'RAW'}
XML = """<domain><name>gh-runner-wonl-win11</name><uuid>UUID</uuid><devices>
<disk type='file' device='disk'><source file='/var/lib/libvirt/images/gh-runner-wonl-win11.qcow2'/><target dev='sda' bus='sata'/></disk>
<controller type='usb' index='0' model='qemu-xhci' ports='15'/></devices></domain>""".replace('UUID', VMUUID)


class FakeProcess:
    name = '123'

    def __truediv__(self, name):
        return self

    def read_bytes(self):
        return b'/usr/bin/qemu-system-x86_64\0-uuid\0' + VMUUID.encode() + b'\0'

    def stat(self):
        return SimpleNamespace(st_uid=64055)


class CapacityControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='s20-capacity-control-', dir=os.environ['TMPDIR'])
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        spec = importlib.util.spec_from_file_location('capacity_' + uuid.uuid4().hex, TOOL)
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)
        self.module.PACKAGE = self.root/'package'
        self.module.TASK = self.root/'task'
        self.module.SOURCE = self.root/'source'
        self.module.IMAGES = self.root/'images'
        self.module.IMAGE = self.module.IMAGES/'scratch.qcow2'
        for path in [self.module.PACKAGE, self.module.TASK, self.module.SOURCE]:
            path.mkdir()
        (self.module.PACKAGE/'operator.py').write_bytes(TOOL.read_bytes())
        (self.module.PACKAGE/'manifest.json').write_text(json.dumps({'files': {'operator.py': self.module.digest(TOOL)}}))
        self.authority = self.root/'authority.json'
        self.authority_data = {'approved': True, 'operation': 'capacity-only-create-attach',
                               'packetManifestSha256': self.module.digest(self.module.PACKAGE/'manifest.json'),
                               'nativeWindowReleased': True, 'rulingReference': 'seq-370'}
        self.authority.write_text(json.dumps(self.authority_data))
        self.live = ET.fromstring(XML)
        self.inactive = ET.fromstring(XML)
        self.commands = []
        self.guest_requests = []
        self.after_disks = [ORIGINAL.copy(), FRESH.copy()]
        self.attach_timeout = False
        self.change_inactive = False
        # Only external OS/process inventory is replaced; main's policy and filesystem branches run.
        def fixture_path(value):
            if str(value) == '/proc':
                return SimpleNamespace(iterdir=lambda: [FakeProcess()])
            return Path(value)
        self.real_run = self.module.run
        fixture_path.cwd = Path.cwd
        self.addCleanup(patch.stopall)
        patch.object(self.module, 'Path', fixture_path).start()
        patch.object(self.module, 'sample', lambda: None).start()
        patch.object(self.module, 'run', self.run_command).start()
        patch.object(self.module, 'ps', self.guest).start()

    def run_command(self, argv, timeout=10):
        self.commands.append(list(argv))
        if argv[0] == 'qemu-img':
            if argv[1] == 'create':
                self.module.IMAGE.write_bytes(b'contained qcow2 fixture')
                return ''
            return json.dumps({'format': 'qcow2', 'virtual-size': 64*GIB})
        if argv[0] in ['setfacl', 'getfacl']:
            return ''
        if argv[0] == 'virsh':
            if 'dumpxml' in argv:
                return ET.tostring(self.inactive if '--inactive' in argv else self.live, encoding='unicode')
            if 'attach-device' in argv:
                if self.attach_timeout:
                    raise TimeoutError('contained attach timeout')
                self.live.find('devices').append(ET.parse(argv[-2]).getroot())
                if self.change_inactive:
                    self.inactive.find('name').text = 'unexpected'
                return ''
        raise AssertionError(f'unhandled command boundary: {argv}')

    def guest(self, script, timeout=10):
        self.guest_requests.append(script)
        for forbidden in ['Initialize-Disk', 'New-Partition', 'Format-Volume', 'provision-guest', 'csc.exe']:
            self.assertNotIn(forbidden, script, 'capacity-only must never submit a guest mutator')
        disks = [ORIGINAL.copy()] if len(self.guest_requests) == 1 else self.after_disks
        return json.dumps({'disks': disks, 'drives': [{'Name': 'C', 'Free': 6*GIB}]})

    def invoke(self):
        with patch('sys.argv', [str(TOOL), '--approval', str(self.authority)]), contextlib.redirect_stdout(io.StringIO()):
            self.module.main()
        return json.loads((self.module.EVIDENCE/'result.json').read_text())

    def mutations(self):
        return [c for c in self.commands if c[0] == 'setfacl' or c[:2] == ['qemu-img', 'create'] or 'attach-device' in c]

    def assert_refused_before_mutation(self):
        with self.assertRaises((RuntimeError, ValueError)):
            self.invoke()
        self.assertEqual(self.mutations(), [])

    def test_healthy_capacity_authority_attaches_raw_disk_without_guest_mutation(self):
        result = self.invoke()
        self.assertEqual(result['status'], 'ATTACHED_RAW_CAPACITY_ONLY')
        self.assertEqual(result['guestDisk']['UniqueId'], 'scratch')
        self.assertFalse(result['guestMutations'])
        self.assertFalse(result['compilerHelperTests'])
        self.assertEqual(len(self.mutations()), 4)
        self.assertEqual(len(self.guest_requests), 2)
        attach = next(c for c in self.commands if 'attach-device' in c)
        self.assertEqual(attach[-1], '--live')
        self.assertNotIn('--config', attach)
        self.assertNotIn('--persistent', attach)

    def test_changed_operator_bytes_refuse_before_mutation(self):
        (self.module.PACKAGE/'operator.py').write_text('modified')
        self.assert_refused_before_mutation()

    def test_wrong_manifest_authority_refuses_before_mutation(self):
        self.authority_data['packetManifestSha256'] = 'wrong'
        self.authority.write_text(json.dumps(self.authority_data))
        self.assert_refused_before_mutation()

    def test_broad_or_false_authority_refuses_before_mutation(self):
        for key, value in [('operation', 'provision-new-scratch-disk'), ('approved', False), ('rulingReference', 'other')]:
            with self.subTest(key=key):
                original = self.authority_data[key]
                self.authority_data[key] = value
                self.authority.write_text(json.dumps(self.authority_data))
                self.assert_refused_before_mutation()
                self.authority_data[key] = original

    def test_occupied_destination_refuses_before_mutation(self):
        self.module.IMAGES.mkdir()
        self.assert_refused_before_mutation()

    def test_wrong_vm_identity_refuses_before_mutation(self):
        self.live.find('uuid').text = 'other'
        self.assert_refused_before_mutation()

    def test_occupied_port_target_or_serial_refuses_before_mutation(self):
        for kind in ['port', 'target', 'serial']:
            with self.subTest(kind=kind):
                self.live = ET.fromstring(XML)
                device = ET.SubElement(self.live.find('devices'), 'disk')
                ET.SubElement(device, 'target', {'dev': 'sdg' if kind == 'target' else 'sdz'})
                if kind == 'port':
                    ET.SubElement(device, 'address', {'type': 'usb', 'bus': '0', 'port': '10'})
                if kind == 'serial':
                    ET.SubElement(device, 'serial').text = 'FDB29S20A6'
                self.assert_refused_before_mutation()

    def test_ambiguous_or_wrong_new_disk_is_unknown_without_followup(self):
        self.after_disks = [ORIGINAL.copy(), FRESH.copy(), dict(FRESH, UniqueId='second')]
        with self.assertRaises(RuntimeError):
            self.invoke()
        result = json.loads((self.module.EVIDENCE/'result.json').read_text())
        self.assertEqual(result['status'], 'UNKNOWN')
        self.assertEqual(len(self.mutations()), 4)
        self.assertEqual(len(self.guest_requests), 2)

    def test_changed_inactive_xml_is_unknown_without_guest_followup(self):
        self.change_inactive = True
        with self.assertRaises(RuntimeError):
            self.invoke()
        self.assertEqual(len(self.mutations()), 4)
        self.assertEqual(len(self.guest_requests), 1)

    def test_attach_timeout_is_unknown_and_stops_requests(self):
        self.attach_timeout = True
        with self.assertRaises(TimeoutError):
            self.invoke()
        result = json.loads((self.module.EVIDENCE/'result.json').read_text())
        self.assertEqual(result['status'], 'UNKNOWN')
        self.assertEqual(len(self.mutations()), 4)
        self.assertEqual(len(self.guest_requests), 1)
        self.assertIn('attach-device', self.commands[-1])


    def test_wrong_qemu_uid_refuses_before_mutation(self):
        with patch.object(FakeProcess, 'stat', lambda _: SimpleNamespace(st_uid=64054)):
            self.assert_refused_before_mutation()

    def test_mismatched_new_disk_is_unknown_without_followup(self):
        self.after_disks = [ORIGINAL.copy(), dict(FRESH, SerialNumber='other')]
        with self.assertRaises(RuntimeError):
            self.invoke()
        result = json.loads((self.module.EVIDENCE/'result.json').read_text())
        self.assertEqual(result['status'], 'UNKNOWN')
        self.assertEqual(len(self.mutations()), 4)
        self.assertEqual(len(self.guest_requests), 2)

    def test_changed_original_guest_disk_is_unknown_without_followup(self):
        self.after_disks = [dict(ORIGINAL, Size=119*GIB), FRESH.copy()]
        with self.assertRaises(RuntimeError):
            self.invoke()
        result = json.loads((self.module.EVIDENCE/'result.json').read_text())
        self.assertEqual(result['status'], 'UNKNOWN')
        self.assertEqual(len(self.mutations()), 4)
        self.assertEqual(len(self.guest_requests), 2)

    def test_real_local_timeout_retains_initiation_and_partial_output(self):
        self.module.EVIDENCE = self.root/'real-run-logs'
        self.module.EVIDENCE.mkdir()
        command = [sys.executable, '-c',
                   "import sys,time;print('partial-out',flush=True);print('partial-err',file=sys.stderr,flush=True);time.sleep(5)"]
        with self.assertRaises(subprocess.TimeoutExpired):
            self.real_run(command, timeout=0.2)
        records = [json.loads(line) for line in (self.module.EVIDENCE/'commands.jsonl').read_text().splitlines()]
        self.assertEqual([r['event'] for r in records], ['COMMAND_START', 'COMMAND_TIMEOUT'])
        self.assertEqual(records[0]['token'], records[1]['token'])
        self.assertIn('partial-out', records[1]['stdout'])
        self.assertIn('partial-err', records[1]['stderr'])
        self.assertEqual(records[1]['termination'], 'DIRECT_CHILD_KILLED_REAPED_BY_SUBPROCESS_RUN;DESCENDANTS_AND_GUEST_UNKNOWN')


if __name__ == '__main__':
    unittest.main()
