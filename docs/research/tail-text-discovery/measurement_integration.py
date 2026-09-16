"""Explicit binary-dependent controls; prerequisites are documented in README."""
import json
import pathlib
import tempfile
from unittest.mock import patch

import measure


def run_controls():
    for name, binary in measure.BINARIES.items():
        if not binary.is_file():
            raise RuntimeError(f'build the documented {name} executable before integration-test')
    with tempfile.TemporaryDirectory() as temporary:
        root = pathlib.Path(temporary).resolve()
        with patch.object(measure, 'EVIDENCE', root / 'new/fixture-parent'):
            first, _ = measure.synthetic('first', 3, 200)
            before = measure.digest_state(first)
            measure.synthetic('first', 3, 200)
            if measure.digest_state(first) != before:
                raise RuntimeError('complete fixture reuse changed canonical bytes')
            for extra in ('source', 'relationship'):
                workspace, _ = measure.synthetic('with-' + extra, 3, 200)
                for binary in measure.BINARIES.values():
                    measure.invoke(binary, workspace, ['validate', '--json'])
                _, raw = measure.invoke(measure.BINARIES['baseline'], workspace,
                                        ['search', 'common-marker'])
                initial_total = json.loads(raw)['total_matches']
                if extra == 'source':
                    measure.invoke(measure.BINARIES['baseline'], workspace,
                                   ['source', 'add', '--id', 'extra-source', '--citation',
                                    'common-marker', '--locator', 'local:unit', '--provenance', 'human'])
                else:
                    record = dict(schema_version=1, kind='relationship', id='extra-link',
                                  relationship='related-to', authorship='human',
                                  rationale='common-marker', occurred_at='2026-07-18T20:00:00Z',
                                  **{'from': {'kind': 'knowledge', 'id': 'note-0000'},
                                     'to': {'kind': 'knowledge', 'id': 'note-0001'}})
                    input_path = root / 'relationship.json'
                    input_path.write_text(json.dumps(record))
                    measure.invoke(measure.BINARIES['baseline'], workspace,
                                   ['relationship', 'add', '--input', str(input_path)])
                measure.validate_workspace(workspace)
                _, raw = measure.invoke(measure.BINARIES['baseline'], workspace,
                                        ['search', 'common-marker'])
                if json.loads(raw)['total_matches'] != initial_total + 1:
                    raise RuntimeError('valid extra record did not discriminate the workload')
                changed = measure.digest_state(workspace)
                try:
                    measure.synthetic('with-' + extra, 3, 200)
                except RuntimeError:
                    pass
                else:
                    raise RuntimeError('valid additional state was accepted for fixed fixture reuse')
                if measure.digest_state(workspace) != changed:
                    raise RuntimeError('rejected fixture was changed')
                print(f'valid {extra} changes query count; reuse rejected; bytes preserved: PASS')
            second, _ = measure.synthetic('second', 3, 200)
            a = json.loads((first / '.research-run/manifest.json').read_text())['workspace_id']
            b = json.loads((second / '.research-run/manifest.json').read_text())['workspace_id']
            if a == b:
                raise RuntimeError('independent fixture identities unexpectedly match')
            print('two initialized fixture identities independently accepted: PASS')


if __name__ == '__main__':
    run_controls()
