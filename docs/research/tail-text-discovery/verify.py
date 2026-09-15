"""Cross-reader and baseline/candidate discrimination through real executables."""
import hashlib
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
OUT = ROOT / 'target/tail-text-evidence'
WORKSPACE = OUT / 'red-workspace'


def call(name, args, cwd=WORKSPACE):
    result = subprocess.run([str(OUT / name), *args], cwd=cwd, capture_output=True)
    assert result.returncode == 0, result.stderr.decode()
    return json.loads(result.stdout)


def canonical():
    return {str(p.relative_to(WORKSPACE)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in (WORKSPACE / '.research-run').rglob('*') if p.is_file()}


before = canonical()
result = {}
for name in ['baseline', 'candidate']:
    result[name] = {}
    for command in ['search', 'context']:
        args = [command, 'tail-needle'] if command == 'search' else [command, '--query', 'tail-needle']
        value = call(name, args)
        items = value['items' if command == 'search' else 'matches']
        assert len(items) == (name == 'candidate')
        result[name][command] = len(items)
    handoff = call(name, ['handoff', 'create', '--id', f'{name}-handoff',
                         '--generated-at', '2026-07-18T21:00:00Z', '--query', 'tail-needle'])
    path = OUT / f'{name}-handoff-v2.json'
    path.write_text(json.dumps(handoff, ensure_ascii=False))
    other = 'candidate' if name == 'baseline' else 'baseline'
    assert call(other, ['handoff', 'inspect', '--input', str(path)], OUT) == handoff
    result[name]['v2_accepted_by_other_reader'] = True
    legacy = dict(handoff, schema_version=1)
    del legacy['validation']
    path = OUT / f'{name}-handoff-v1.json'
    path.write_text(json.dumps(legacy, ensure_ascii=False))
    assert call(other, ['handoff', 'inspect', '--input', str(path)], OUT) == legacy
    result[name]['v1_accepted_by_other_reader'] = True
    call(name, ['validate', '--json'])
for args in [['list'], ['recent'], ['timeline'], ['context'],
             ['show', '--kind', 'knowledge', '--id', 'synthetic-method'],
             ['search', 'prefix-marker']]:
    assert call('baseline', args) == call('candidate', args)
result['unchanged_projections_and_prefix'] = True
result['canonical_unchanged'] = before == canonical()
assert result['canonical_unchanged']
result['binary_sha256'] = {name: hashlib.sha256((OUT/name).read_bytes()).hexdigest()
                           for name in ['baseline', 'candidate']}
(OUT / 'compatibility.json').write_text(json.dumps(result, indent=2))
print(json.dumps(result, indent=2))
