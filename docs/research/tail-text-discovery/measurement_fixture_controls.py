"""Binary-independent controls for synthetic fixture ownership and reuse."""
import json
import pathlib
import secrets
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import measure


def fake_initialize(binary, parent, args):
    """Model only init's layout; this is not a domain schema validator."""
    if args[0] != 'init':
        raise AssertionError('unexpected CLI call in unit test')
    state = pathlib.Path(args[1]) / '.research-run'
    for name in ('knowledge', 'sources', 'claims', 'relationships', 'inventories',
                 'evidence', 'experiments', 'reviews', 'review-authorities',
                 'migrations', 'contribution-protocols'):
        (state / name).mkdir(parents=True)
    (state / 'manifest.json').write_text(json.dumps({'workspace_id': secrets.token_hex(32)}))
    (state / 'write.lock').write_bytes(b'')
    (state / 'contribution-protocols/agent-contribution.json').write_text('{"unit":true}')
    return 0, b''


def retained_tree(root):
    """Include empty directories so rejected layout drift stays inspectable."""
    return {str(p.relative_to(root)): None if p.is_dir() else p.read_bytes()
            for p in root.rglob('*')}


class FixtureControls(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = pathlib.Path(self.temporary.name).resolve()
        self.addCleanup(patch.stopall)
        patch.object(measure, 'EVIDENCE', self.root / 'missing/parents/evidence').start()
        self.init = patch.object(measure, 'invoke', side_effect=fake_initialize).start()
        self.validate = patch.object(measure, 'validate_workspace').start()

    def create(self, name='unit-corpus'):
        return measure.synthetic(name, 3, 200)[0]

    def test_complete_reuse_and_fresh_identities(self):
        first = self.create()
        before = retained_tree(first)
        measure.synthetic('unit-corpus', 3, 200)
        self.assertEqual(retained_tree(first), before)
        self.assertEqual(self.init.call_count, 1)
        second = self.create('another-corpus')
        self.assertNotEqual((first / '.research-run/manifest.json').read_bytes(),
                            (second / '.research-run/manifest.json').read_bytes())
        measure.require_reusable_workspace(second, 3, 200)

    def test_every_authority_collection_and_empty_directory_are_bound(self):
        for extra in ('sources/extra.json', 'claims/extra.json', 'relationships/extra.json',
                      'review-authorities/extra.json', 'inventories/extra.json',
                      'contribution-protocols/extra.json', 'extra.json', 'empty-directory/'):
            with self.subTest(extra=extra):
                workspace = self.create('fixture-' + str(self.init.call_count))
                path = workspace / '.research-run' / extra
                if extra.endswith('/'):
                    path.mkdir()
                else:
                    path.write_text('{"extra":true}')
                before = retained_tree(workspace)
                with self.assertRaises(RuntimeError):
                    measure.require_reusable_workspace(workspace, 3, 200)
                self.assertEqual(retained_tree(workspace), before)

    def test_manifest_protocol_and_knowledge_drift_are_preserved(self):
        cases = ('manifest.json', 'contribution-protocols/agent-contribution.json',
                 'knowledge/note-0001.json', 'knowledge/extra.json')
        for relative in cases:
            with self.subTest(path=relative):
                workspace = self.create('fixture-' + str(self.init.call_count))
                (workspace / '.research-run' / relative).write_text('{"drift":true}')
                before = retained_tree(workspace)
                with self.assertRaises(RuntimeError):
                    measure.require_reusable_workspace(workspace, 3, 200)
                self.assertEqual(retained_tree(workspace), before)
        workspace = self.create('missing-middle')
        (workspace / '.research-run/knowledge/note-0001.json').unlink()
        before = retained_tree(workspace)
        with self.assertRaises(RuntimeError):
            measure.require_reusable_workspace(workspace, 3, 200)
        self.assertEqual(retained_tree(workspace), before)

    def test_count_size_and_unbound_reuse_are_rejected(self):
        workspace = self.create()
        for count, size in ((2, 200), (3, 201)):
            with self.assertRaises(RuntimeError):
                measure.require_reusable_workspace(workspace, count, size)
        unbound = self.root / 'unbound'
        fake_initialize(None, None, ['init', str(unbound)])
        before = retained_tree(unbound)
        with self.assertRaises(RuntimeError):
            measure.require_reusable_workspace(unbound, 3, 200)
        self.assertEqual(retained_tree(unbound), before)


class ValidationControls(unittest.TestCase):
    def test_real_execution_requires_zero_exit_and_valid_true(self):
        cases = ((1, b'{"valid":true}'), (0, b'not JSON'),
                 (0, b'{"valid":false}'), (0, b'[]'), (0, b'{}'))
        for code, raw in cases:
            with self.subTest(code=code, raw=raw):
                with patch.object(measure.subprocess, 'run', return_value=
                                  subprocess.CompletedProcess([], code, stdout=raw)):
                    with self.assertRaises(RuntimeError):
                        measure.validate_workspace(pathlib.Path('.'))
        with patch.object(measure.subprocess, 'run', return_value=
                          subprocess.CompletedProcess([], 0, stdout=b'{"valid":true}')):
            measure.validate_workspace(pathlib.Path('.'))


def run_controls():
    suite = unittest.TestSuite(unittest.defaultTestLoader.loadTestsFromTestCase(case)
                               for case in (FixtureControls, ValidationControls))
    if not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful():
        raise RuntimeError('measurement fixture controls failed')


if __name__ == '__main__':
    run_controls()
