"""Cross-reader and baseline/candidate discrimination through real executables."""
import hashlib
import json
import pathlib
import subprocess

from measurement_fs import ReadBudget, read_regular, safe_files

ROOT = pathlib.Path(__file__).resolve().parents[3]
OUT = ROOT / 'target/tail-text-evidence'
WORKSPACE = OUT / 'red-workspace'


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def call(name, args, cwd=WORKSPACE):
    result = subprocess.run([str(OUT / name), *args], cwd=cwd, capture_output=True)
    if result.returncode != 0:
        raise RuntimeError(result.stderr.decode())
    return json.loads(result.stdout)


def canonical():
    budget = ReadBudget()
    return {
        str(path.relative_to(WORKSPACE)): hashlib.sha256(read_regular(path, WORKSPACE, budget)).hexdigest()
        for path in safe_files(WORKSPACE)
    }


before = canonical()
result = {}
for name in ['baseline', 'candidate']:
    result[name] = {}
    for command in ['search', 'context']:
        args = [command, 'tail-needle'] if command == 'search' else [command, '--query', 'tail-needle']
        value = call(name, args)
        items = value['items' if command == 'search' else 'matches']
        if name == 'baseline':
            require(not items, f'{name} {command} unexpectedly matched tail text')
        else:
            require(len(items) == 1, f'{name} {command} match count changed')
            item = items[0]
            require(item['kind'] == 'knowledge' and item['id'] == 'synthetic-method',
                    f'{name} {command} returned the wrong record')
            require(item['matched_by'] == ['summary'], f'{name} {command} match reason changed')
            require('tail-needle' in item['summary'] and len(item['summary']) <= 512,
                    f'{name} {command} did not return a bounded tail excerpt')
        result[name][command] = len(items)
    handoff = call(name, ['handoff', 'create', '--id', f'{name}-handoff',
                         '--generated-at', '2026-07-18T21:00:00Z', '--query', 'tail-needle'])
    path = OUT / f'{name}-handoff-v2.json'
    path.write_text(json.dumps(handoff, ensure_ascii=False))
    other = 'candidate' if name == 'baseline' else 'baseline'
    require(call(other, ['handoff', 'inspect', '--input', str(path)], OUT) == handoff,
            'cross-reader v2 handoff mismatch')
    result[name]['v2_accepted_by_other_reader'] = True
    legacy = dict(handoff, schema_version=1)
    del legacy['validation']
    path = OUT / f'{name}-handoff-v1.json'
    path.write_text(json.dumps(legacy, ensure_ascii=False))
    require(call(other, ['handoff', 'inspect', '--input', str(path)], OUT) == legacy,
            'cross-reader v1 handoff mismatch')
    result[name]['v1_accepted_by_other_reader'] = True
    call(name, ['validate', '--json'])
for args in [['list'], ['recent'], ['timeline'], ['context'],
             ['show', '--kind', 'knowledge', '--id', 'synthetic-method'],
             ['search', 'prefix-marker']]:
    require(call('baseline', args) == call('candidate', args), f'projection changed: {args}')
result['unchanged_projections_and_prefix'] = True
result['canonical_unchanged'] = before == canonical()
require(result['canonical_unchanged'], 'compatibility verification changed canonical state')
result['binary_sha256'] = {name: hashlib.sha256((OUT/name).read_bytes()).hexdigest()
                           for name in ['baseline', 'candidate']}
(OUT / 'compatibility.json').write_text(json.dumps(result, indent=2))
print(json.dumps(result, indent=2))
