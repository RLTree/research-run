"""Reproducible local experiment; outputs contain operational metadata only."""
import hashlib
import json
import pathlib
import statistics
import subprocess
import sys
import time

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
    result = {}
    for path in sorted((workspace / '.research-run').rglob('*')):
        if path.is_file():
            result[str(path.relative_to(workspace))] = hashlib.sha256(path.read_bytes()).hexdigest()
    return result


def synthetic(name, count, size):
    workspace = EVIDENCE / name
    if not workspace.exists():
        invoke(BINARIES['baseline'], EVIDENCE,
               ['init', str(workspace), '--name', name, '--without-review-authority'])
    if not (workspace / f'.research-run/knowledge/note-{count-1:04}.json').exists():
        for index in range(count):
            prefix, tail = 'prefix-marker common-marker ', ' tail-needle'
            body = prefix + 'x' * (size - len(prefix) - len(tail)) + tail
            assert len(body.encode()) == size
            record = dict(schema_version=1, kind='knowledge', id=f'note-{index:04}',
                          record_type='observation', title='Synthetic note', body=body,
                          occurred_at='2026-07-18T20:00:00Z', state='open', authorship='human')
            path = workspace / '.research-run/knowledge' / f'note-{index:04}.json'
            path.write_text(json.dumps(record))
    invoke(BINARIES['baseline'], workspace, ['validate', '--json'])
    return workspace, dict(prefix='prefix-marker', tail='tail-needle', absent='absent-needle', common='common-marker')


def measure(name, workspace, queries):
    before = digest_state(workspace)
    records = list((workspace / '.research-run/knowledge').glob('*.json'))
    output = dict(workload=name, knowledge_records=len(records),
                  canonical_bytes=sum(p.stat().st_size for p in (workspace / '.research-run').rglob('*') if p.is_file()),
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


if __name__ == '__main__':
    EVIDENCE.mkdir(exist_ok=True)
    if sys.argv[1] == 'synthetic':
        for name, count, size in [('many-short', 1000, 200), ('near-budget', 1000, 65536)]:
            workspace, queries = synthetic(name, count, size)
            measure(name, workspace, queries)
    else:
        workspace = pathlib.Path(sys.argv[1]).resolve()
        records = sorted((workspace / '.research-run/knowledge').glob('*.json'))
        body = max((json.loads(p.read_text())['body'] for p in records), key=len)
        queries = dict(prefix=body[:16].strip(), tail=body[-16:].strip(),
                       absent='rropp-synthetic-absent-needle', common='the')
        measure('current-workspace', workspace, queries)
