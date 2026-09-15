"""Reproducible local experiment; outputs contain operational metadata only."""
import hashlib
import json
import pathlib
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

from measurement_fs import read_regular, safe_files

ROOT = pathlib.Path(__file__).resolve().parents[3]
EVIDENCE = ROOT / 'target/tail-text-evidence'
BINARIES = {name: EVIDENCE / name for name in ('baseline', 'candidate')}


def invoke(binary, workspace, args):
    started = time.perf_counter_ns()
    result = subprocess.run([str(binary), *args], cwd=workspace, capture_output=True)
    elapsed = (time.perf_counter_ns() - started) / 1e6
    if result.returncode:
        raise RuntimeError(f'command failed: {args[0]}, exit {result.returncode}')
    return elapsed, result.stdout


def digest_state(workspace):
    return {
        str(path.relative_to(workspace)): hashlib.sha256(read_regular(path, workspace)).hexdigest()
        for path in safe_files(workspace)
    }


def expected_body(size):
    prefix, tail = 'prefix-marker common-marker ', ' tail-needle'
    body = prefix + 'x' * (size - len(prefix) - len(tail)) + tail
    assert len(body.encode()) == size
    return body


def expected_record(index, size):
    return json.dumps(dict(
        schema_version=1, kind='knowledge', id=f'note-{index:04}',
        record_type='observation', title='Synthetic note', body=expected_body(size),
        occurred_at='2026-07-18T20:00:00Z', state='open', authorship='human'
    )).encode()


def corpus_is_expected(workspace, count, size):
    records = sorted((workspace / '.research-run/knowledge').glob('note-*.json'))
    if len(records) != count:
        return False
    return all(
        read_regular(path, workspace) == expected_record(index, size)
        for index, path in enumerate(records)
    )


def synthetic(name, count, size):
    workspace = EVIDENCE / name
    if workspace.exists() and (workspace.is_symlink() or not workspace.is_dir()):
        raise RuntimeError(f'synthetic workspace is not a regular directory: {workspace}')
    if workspace.exists():
        safe_files(workspace)
    if workspace.exists() and not corpus_is_expected(workspace, count, size):
        shutil.rmtree(workspace)
    if not workspace.exists():
        invoke(BINARIES['baseline'], EVIDENCE,
               ['init', str(workspace), '--name', name, '--without-review-authority'])
    if not corpus_is_expected(workspace, count, size):
        for index in range(count):
            record = dict(schema_version=1, kind='knowledge', id=f'note-{index:04}',
                          record_type='observation', title='Synthetic note', body=expected_body(size),
                          occurred_at='2026-07-18T20:00:00Z', state='open', authorship='human')
            path = workspace / '.research-run/knowledge' / f'note-{index:04}.json'
            path.write_text(json.dumps(record))
    if not corpus_is_expected(workspace, count, size):
        raise RuntimeError(f'synthetic corpus did not converge: {workspace}')
    invoke(BINARIES['baseline'], workspace, ['validate', '--json'])
    return workspace, dict(prefix='prefix-marker', tail='tail-needle', absent='absent-needle', common='common-marker')


def measure(name, workspace, queries):
    before = digest_state(workspace)
    records = list((workspace / '.research-run/knowledge').glob('*.json'))
    output = dict(workload=name, knowledge_records=len(records),
                  canonical_bytes=sum(len(read_regular(path, workspace)) for path in safe_files(workspace)),
                  cases=[])
    for command in ('search', 'context'):
        for label, query in queries.items():
            args = [command, query] if command == 'search' else [command, '--query', query]
            case = dict(command=command, query_class=label, observations={})
            for binary_name, binary in BINARIES.items():
                cold, raw = invoke(binary, workspace, args)
                parsed = json.loads(raw)
                case['observations'][binary_name] = dict(
                    first_ms=cold, output_bytes=len(raw),
                    returned=len(parsed.get('items', parsed.get('matches', []))),
                    total_matches=parsed.get('total_matches'), warm_ms=[])
            for _ in range(3):
                for binary in BINARIES.values():
                    invoke(binary, workspace, args)
            for pair in range(15):
                order = list(BINARIES) if pair % 2 == 0 else list(reversed(BINARIES))
                for binary_name in order:
                    elapsed, _ = invoke(BINARIES[binary_name], workspace, args)
                    case['observations'][binary_name]['warm_ms'].append(elapsed)
            for value in case['observations'].values():
                value['median_ms'] = statistics.median(value['warm_ms'])
                value['p95_ms'] = sorted(value['warm_ms'])[14]
            old, new = (case['observations'][n] for n in BINARIES)
            case['review_trigger'] = any(new[m] - old[m] > absolute and new[m] > old[m] * ratio
                                         for m, absolute, ratio in [('median_ms', 25, 1.2), ('p95_ms', 50, 1.25)])
            # Separate fresh Python parent: RUSAGE_CHILDREN covers only this CLI.
            # On macOS ru_maxrss is bytes; this avoids time -l's denied sysctl.
            memory_probe = ("import resource,subprocess,sys; "
                            "r=subprocess.run(sys.argv[1:],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL); "
                            "print(resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss); "
                            "sys.exit(r.returncode)")
            for binary_name, binary in BINARIES.items():
                result = subprocess.run([sys.executable, '-c', memory_probe, str(binary), *args],
                                        cwd=workspace, capture_output=True)
                assert result.returncode == 0
                case['observations'][binary_name]['peak_rss_bytes'] = int(result.stdout)
            output['cases'].append(case)
            (EVIDENCE / f'latency-{name}.json').write_text(json.dumps(output, indent=2))
            print(name, command, label, 'done', flush=True)
    output['canonical_unchanged'] = before == digest_state(workspace)
    assert output['canonical_unchanged']
    (EVIDENCE / f'latency-{name}.json').write_text(json.dumps(output, indent=2))


def self_test():
    with tempfile.TemporaryDirectory() as temporary:
        workspace = pathlib.Path(temporary)
        knowledge = workspace / '.research-run/knowledge'
        knowledge.mkdir(parents=True)
        for index in range(2):
            (knowledge / f'note-{index:04}.json').write_bytes(expected_record(index, 200))
        assert corpus_is_expected(workspace, 2, 200)
        (knowledge / 'note-0001.json').unlink()
        assert not corpus_is_expected(workspace, 2, 200)
        (knowledge / 'note-0001.json').write_bytes(expected_record(1, 200).replace(b'tail-needle', b'changed-text'))
        assert not corpus_is_expected(workspace, 2, 200)
        (knowledge / 'note-0001.json').write_bytes(expected_record(1, 200))
        (knowledge / 'note-0002.json').write_bytes(expected_record(2, 200))
        assert not corpus_is_expected(workspace, 2, 200)
        (knowledge / 'note-0002.json').unlink()
        assert corpus_is_expected(workspace, 2, 200)
        (knowledge / 'linked').symlink_to(knowledge / 'note-0000.json')
        try:
            safe_files(workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink file was accepted')
        (knowledge / 'linked').unlink()
        outside_directory = workspace / 'outside-directory'
        outside_directory.mkdir()
        (knowledge / 'linked-directory').symlink_to(outside_directory, target_is_directory=True)
        try:
            safe_files(workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink directory was accepted')
        (knowledge / 'linked-directory').unlink()
        linked_workspace = workspace.parent / f'linked-workspace-{workspace.name}'
        if linked_workspace.exists() or linked_workspace.is_symlink():
            linked_workspace.unlink()
        linked_workspace.symlink_to(workspace, target_is_directory=True)
        try:
            safe_files(linked_workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink workspace was accepted')
        linked_workspace.unlink()
        linked_state_workspace = workspace.parent / f'linked-state-{workspace.name}'
        linked_state_workspace.mkdir()
        (linked_state_workspace / '.research-run').symlink_to(knowledge, target_is_directory=True)
        try:
            safe_files(linked_state_workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink research state was accepted')
        (linked_state_workspace / '.research-run').unlink()
        linked_state_workspace.rmdir()
        (knowledge / 'fifo').parent.mkdir(exist_ok=True)
        import os
        os.mkfifo(knowledge / 'fifo')
        try:
            safe_files(workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('FIFO was accepted')
    print('measurement helper controls passed')


if __name__ == '__main__':
    EVIDENCE.mkdir(exist_ok=True)
    if sys.argv[1] == 'self-test':
        self_test()
    elif sys.argv[1] == 'synthetic':
        for name, count, size in [('many-short', 1000, 200), ('near-budget', 1000, 65536)]:
            workspace, queries = synthetic(name, count, size)
            measure(name, workspace, queries)
    else:
        workspace = pathlib.Path(sys.argv[1]).resolve()
        files = safe_files(workspace)
        records = [path for path in files if path.parent.name == 'knowledge']
        body = max((json.loads(read_regular(path, workspace).decode())['body'] for path in records), key=len)
        queries = dict(prefix=body[:16].strip(), tail=body[-16:].strip(),
                       absent='rropp-synthetic-absent-needle', common='the')
        measure('current-workspace', workspace, queries)
