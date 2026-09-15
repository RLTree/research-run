"""Reproducible local experiment; outputs contain operational metadata only."""
import hashlib
import json
import pathlib
import statistics
import subprocess
import sys
import tempfile
import time

from measurement_fs import (
    MAX_INVENTORY_RECORD_BYTES,
    MAX_RECORD_BYTES,
    ReadBudget,
    read_regular,
    safe_files,
)

ROOT = pathlib.Path(__file__).resolve().parents[3]
EVIDENCE = ROOT / 'target/tail-text-evidence'
BINARIES = {name: EVIDENCE / name for name in ('baseline', 'candidate')}
PROJECTION_CHARS = 512


def invoke(binary, workspace, args):
    started = time.perf_counter_ns()
    result = subprocess.run([str(binary), *args], cwd=workspace, capture_output=True)
    elapsed = (time.perf_counter_ns() - started) / 1e6
    if result.returncode:
        raise RuntimeError(f'command failed: {args[0]}, exit {result.returncode}')
    return elapsed, result.stdout


def digest_state(workspace):
    budget = ReadBudget()
    return {
        str(path.relative_to(workspace)): hashlib.sha256(read_regular(path, workspace, budget)).hexdigest()
        for path in safe_files(workspace)
    }


def expected_body(size):
    prefix, tail = 'prefix-marker common-marker ', ' tail-needle'
    body = prefix + 'x' * (size - len(prefix) - len(tail)) + tail
    if len(body.encode()) != size:
        raise ValueError(f'synthetic body size mismatch: expected {size}')
    return body


def expected_record(index, size):
    return json.dumps(dict(
        schema_version=1, kind='knowledge', id=f'note-{index:04}',
        record_type='observation', title='Synthetic note', body=expected_body(size),
        occurred_at='2026-07-18T20:00:00Z', state='open', authorship='human'
    )).encode()


def corpus_is_expected(workspace, count, size):
    knowledge = workspace / '.research-run/knowledge'
    if knowledge.is_symlink() or not knowledge.is_dir():
        return False
    entries = sorted(path.name for path in knowledge.iterdir())
    expected = [f'note-{index:04}.json' for index in range(count)]
    if entries != expected:
        return False
    records = [knowledge / name for name in expected]
    budget = ReadBudget()
    return all(
        read_regular(path, workspace, budget) == expected_record(index, size)
        for index, path in enumerate(records)
    )


def synthetic(name, count, size):
    workspace = EVIDENCE / name
    if workspace.is_symlink():
        raise RuntimeError(f'synthetic workspace is a symlink: {workspace}')
    if workspace.exists():
        require_reusable_workspace(workspace, count, size)
    else:
        invoke(BINARIES['baseline'], EVIDENCE,
               ['init', str(workspace), '--name', name, '--without-review-authority'])
        safe_files(workspace)
    knowledge = workspace / '.research-run/knowledge'
    if knowledge.is_symlink() or not knowledge.is_dir():
        raise RuntimeError(f'synthetic knowledge directory is not a regular directory: {knowledge}')
    if not corpus_is_expected(workspace, count, size):
        for index in range(count):
            record = dict(schema_version=1, kind='knowledge', id=f'note-{index:04}',
                          record_type='observation', title='Synthetic note', body=expected_body(size),
                          occurred_at='2026-07-18T20:00:00Z', state='open', authorship='human')
            path = workspace / '.research-run/knowledge' / f'note-{index:04}.json'
            if path.exists() or path.is_symlink():
                raise RuntimeError(f'synthetic record path already exists: {path}')
            path.write_text(json.dumps(record))
    if not corpus_is_expected(workspace, count, size):
        raise RuntimeError(f'synthetic corpus did not converge: {workspace}')
    invoke(BINARIES['baseline'], workspace, ['validate', '--json'])
    return workspace, dict(prefix='prefix-marker', tail='tail-needle', absent='absent-needle', common='common-marker')


def require_reusable_workspace(workspace, count, size):
    if workspace.is_symlink() or not workspace.is_dir():
        raise RuntimeError(f'synthetic workspace is not a regular directory: {workspace}')
    safe_files(workspace)
    if not corpus_is_expected(workspace, count, size):
        raise RuntimeError(f'reused synthetic corpus drifted; preserving workspace: {workspace}')


def measure(name, workspace, queries):
    before = digest_state(workspace)
    records = list((workspace / '.research-run/knowledge').glob('*.json'))
    byte_budget = ReadBudget()
    output = dict(workload=name, knowledge_records=len(records),
                  canonical_bytes=sum(len(read_regular(path, workspace, byte_budget)) for path in safe_files(workspace)),
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
                if result.returncode != 0:
                    raise RuntimeError(f'memory probe failed: {binary}')
                case['observations'][binary_name]['peak_rss_bytes'] = int(result.stdout)
            output['cases'].append(case)
            print(name, command, label, 'done', flush=True)
    output['canonical_unchanged'] = before == digest_state(workspace)
    if not output['canonical_unchanged']:
        raise RuntimeError('measurement changed canonical workspace state')
    (EVIDENCE / f'latency-{name}.json').write_text(json.dumps(output, indent=2))


def derive_queries(workspace, records):
    if not records:
        raise RuntimeError('measurement workspace has no knowledge records')
    budget = ReadBudget()
    bodies = [json.loads(read_regular(path, workspace, budget).decode())['body'] for path in records]
    body = max(bodies, key=len)
    if len(body) <= PROJECTION_CHARS:
        raise RuntimeError('measurement workspace has no body tail beyond the projection')
    prefix = body[:16].strip()
    tail = body[-16:].strip()
    tail_source = body[PROJECTION_CHARS:]
    if not prefix or not tail or not tail_source.strip() or tail not in tail_source:
        raise RuntimeError('derived measurement queries do not prove a tail-only control')
    return dict(prefix=prefix, tail=tail,
                absent='rropp-synthetic-absent-needle', common='the')


def self_test():
    with tempfile.TemporaryDirectory() as temporary:
        workspace = pathlib.Path(temporary) / 'corpus-workspace'
        knowledge = workspace / '.research-run/knowledge'
        knowledge.mkdir(parents=True)
        measurement_files = workspace / '.research-run/measurements'
        measurement_files.mkdir()
        budget = workspace / '.research-run/inventories'
        budget.mkdir()
        exact = measurement_files / 'exact.bin'
        exact.touch()
        exact.write_bytes(b'')
        with exact.open('r+b') as stream:
            stream.truncate(MAX_RECORD_BYTES)
        read_regular(exact, workspace)
        oversized = measurement_files / 'oversized.bin'
        oversized.touch()
        with oversized.open('r+b') as stream:
            stream.truncate(MAX_RECORD_BYTES + 1)
        try:
            read_regular(oversized, workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('oversized regular file was accepted')
        oversized.unlink()
        aggregate_a = measurement_files / 'aggregate-a.bin'
        aggregate_b = measurement_files / 'aggregate-b.bin'
        for path in (aggregate_a, aggregate_b):
            path.touch()
            with path.open('r+b') as stream:
                stream.truncate(MAX_RECORD_BYTES // 2)
        aggregate_budget = ReadBudget(MAX_RECORD_BYTES)
        read_regular(aggregate_a, workspace, aggregate_budget)
        read_regular(aggregate_b, workspace, aggregate_budget)
        overflow_a = measurement_files / 'overflow-a.bin'
        overflow_b = measurement_files / 'overflow-b.bin'
        for path in (overflow_a, overflow_b):
            path.touch()
            with path.open('r+b') as stream:
                stream.truncate((MAX_RECORD_BYTES * 3) // 5)
        overflow_budget = ReadBudget(MAX_RECORD_BYTES)
        read_regular(overflow_a, workspace, overflow_budget)
        try:
            read_regular(overflow_b, workspace, overflow_budget)
        except RuntimeError:
            pass
        else:
            raise AssertionError('aggregate over-budget files were accepted')
        inventory_exact = budget / 'exact.bin'
        inventory_exact.touch()
        with inventory_exact.open('r+b') as stream:
            stream.truncate(MAX_INVENTORY_RECORD_BYTES)
        read_regular(inventory_exact, workspace)
        inventory_over = budget / 'oversized.bin'
        inventory_over.touch()
        with inventory_over.open('r+b') as stream:
            stream.truncate(MAX_INVENTORY_RECORD_BYTES + 1)
        try:
            read_regular(inventory_over, workspace)
        except RuntimeError:
            pass
        else:
            raise AssertionError('oversized inventory file was accepted')
        inventory_over.unlink()
        for index in range(2):
            (knowledge / f'note-{index:04}.json').write_bytes(expected_record(index, 200))
        if not corpus_is_expected(workspace, 2, 200):
            raise AssertionError('complete corpus was rejected')
        (knowledge / 'note-0001.json').unlink()
        if corpus_is_expected(workspace, 2, 200):
            raise AssertionError('missing record was accepted')
        (knowledge / 'note-0001.json').write_bytes(expected_record(1, 200).replace(b'tail-needle', b'changed-text'))
        if corpus_is_expected(workspace, 2, 200):
            raise AssertionError('changed record was accepted')
        (knowledge / 'note-0001.json').write_bytes(expected_record(1, 200))
        (knowledge / 'note-0002.json').write_bytes(expected_record(2, 200))
        if corpus_is_expected(workspace, 2, 200):
            raise AssertionError('extra record was accepted')
        (knowledge / 'note-0002.json').unlink()
        if not corpus_is_expected(workspace, 2, 200):
            raise AssertionError('restored corpus was rejected')
        (knowledge / 'note-0001.json').write_bytes(expected_record(1, 200).replace(b'tail-needle', b'changed-text'))
        before = digest_state(workspace)
        try:
            require_reusable_workspace(workspace, 2, 200)
        except RuntimeError:
            pass
        else:
            raise AssertionError('drifted corpus was accepted for reuse')
        if digest_state(workspace) != before:
            raise AssertionError('drifted corpus was changed during rejection')
        short_workspace = workspace.parent / 'short-workspace'
        short_knowledge = short_workspace / '.research-run/knowledge'
        short_knowledge.mkdir(parents=True)
        short_record = short_knowledge / 'short.json'
        short_record.write_text(json.dumps({'body': 'short body'}))
        try:
            derive_queries(short_workspace, [short_record])
        except RuntimeError:
            pass
        else:
            raise AssertionError('short body was accepted for tail measurement')
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
        actual_parent = workspace.parent / f'actual-parent-{workspace.name}'
        actual_workspace = actual_parent / 'nested'
        (actual_workspace / '.research-run').mkdir(parents=True)
        linked_parent = workspace.parent / f'linked-parent-{workspace.name}'
        linked_parent.symlink_to(actual_parent, target_is_directory=True)
        try:
            safe_files(linked_parent / 'nested')
        except RuntimeError:
            pass
        else:
            raise AssertionError('symlink ancestor was accepted')
        linked_parent.unlink()
        (actual_workspace / '.research-run').rmdir()
        actual_workspace.rmdir()
        actual_parent.rmdir()
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


def entry_self_test():
    with tempfile.TemporaryDirectory() as temporary:
        root = pathlib.Path(temporary)
        actual_parent = root / 'actual-parent'
        workspace = actual_parent / 'workspace'
        knowledge = workspace / '.research-run/knowledge'
        knowledge.mkdir(parents=True)
        (knowledge / 'record.json').write_text(json.dumps({'body': 'synthetic entry control'}))
        linked_workspace = root / 'workspace-link'
        linked_workspace.symlink_to(workspace, target_is_directory=True)
        linked_parent = root / 'parent-link'
        linked_parent.symlink_to(actual_parent, target_is_directory=True)
        dangling = root / 'dangling-workspace'
        dangling.symlink_to(root / 'missing-workspace', target_is_directory=True)
        evidence = {path: path.read_bytes() for path in EVIDENCE.glob('latency-*.json')}
        for interpreter in ([sys.executable], [sys.executable, '-O']):
            for candidate in (linked_workspace, linked_parent / 'workspace', dangling):
                result = subprocess.run(
                    interpreter + [str(pathlib.Path(__file__)), str(candidate)],
                    capture_output=True,
                )
                if result.returncode == 0:
                    raise AssertionError('symlink entry path was accepted')
                if any(path.read_bytes() != content for path, content in evidence.items()):
                    raise AssertionError('rejected entry path wrote measurement evidence')
    print('measurement entry controls passed')


if __name__ == '__main__':
    EVIDENCE.mkdir(exist_ok=True)
    if sys.argv[1] == 'self-test':
        self_test()
    elif sys.argv[1] == 'entry-test':
        entry_self_test()
    elif sys.argv[1] == 'synthetic':
        for name, count, size in [('many-short', 1000, 200), ('near-budget', 1000, 65536)]:
            workspace, queries = synthetic(name, count, size)
            measure(name, workspace, queries)
    else:
        workspace = pathlib.Path(sys.argv[1]).absolute()
        files = safe_files(workspace)
        records = [path for path in files if path.parent.name == 'knowledge']
        queries = derive_queries(workspace, records)
        measure('current-workspace', workspace, queries)
