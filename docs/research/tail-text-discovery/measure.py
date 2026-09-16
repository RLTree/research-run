"""Reproducible local experiment; outputs contain operational metadata only."""
import argparse
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
    _reject_symlink_components,
)
from measurement_fixture import record_initialization, require_fixture

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


def validate_workspace(workspace):
    result = subprocess.run(
        [str(BINARIES['baseline']), 'validate', '--json'],
        cwd=workspace,
        capture_output=True,
    )
    if result.returncode != 0:
        raise RuntimeError('baseline validation rejected supplied workspace')
    try:
        receipt = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError('baseline validation returned non-JSON output') from error
    if not isinstance(receipt, dict) or receipt.get('valid') is not True:
        raise RuntimeError('baseline validation returned valid=false')


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
    if pathlib.Path(name).name != name or name in ('.', '..'):
        raise RuntimeError('synthetic fixture name must be one path component')
    _reject_symlink_components(EVIDENCE)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    workspace = EVIDENCE / name
    if workspace.is_symlink():
        raise RuntimeError(f'synthetic workspace is a symlink: {workspace}')
    if workspace.exists():
        require_reusable_workspace(workspace, count, size)
    else:
        invoke(BINARIES['baseline'], EVIDENCE,
               ['init', str(workspace), '--name', name, '--without-review-authority'])
        record_initialization(workspace, count, size)
        for index in range(count):
            path = workspace / '.research-run/knowledge' / f'note-{index:04}.json'
            with path.open('xb') as output:
                output.write(expected_record(index, size))
    require_reusable_workspace(workspace, count, size)
    fingerprint = digest_state(workspace)
    validate_workspace(workspace)
    if digest_state(workspace) != fingerprint:
        raise RuntimeError('synthetic workspace changed during validation')
    return workspace, dict(
        queries=dict(prefix='prefix-marker', tail='tail-needle', absent='absent-needle', common='common-marker'),
        target_id='note-0000',
        fingerprint=fingerprint,
    )


def require_reusable_workspace(workspace, count, size):
    if workspace.is_symlink() or not workspace.is_dir():
        raise RuntimeError(f'synthetic workspace is not a regular directory: {workspace}')
    require_fixture(workspace, count, size, expected_record)


def query_probe(binary, workspace, command, query):
    args = [command, query, '--limit', '256'] if command == 'search' else [command, '--query', query, '--limit', '256']
    result = subprocess.run([str(binary), *args], cwd=workspace, capture_output=True)
    if result.returncode != 0:
        raise RuntimeError(f'{command} probe failed')
    value = json.loads(result.stdout)
    items = value.get('items', value.get('matches', []))
    total = value.get('total_matches', len(items))
    return items, total, len(items) >= 256


def qualify_queries(workspace, queries, target_id):
    qualifications = {'search': {}, 'context': {}}
    for command in qualifications:
        for label, query in queries.items():
            baseline_items, baseline_total, baseline_saturated = query_probe(
                BINARIES['baseline'], workspace, command, query
            )
            candidate_items, candidate_total, candidate_saturated = query_probe(
                BINARIES['candidate'], workspace, command, query
            )
            baseline_ids = {item.get('id') for item in baseline_items}
            candidate_ids = {item.get('id') for item in candidate_items}
            status, reason = 'not_applicable', 'control did not separate baseline and candidate'
            if label == 'absent':
                status = 'qualified' if baseline_total == 0 and candidate_total == 0 else 'not_applicable'
                reason = 'both binaries returned zero matches' if status == 'qualified' else 'absent control matched a record'
            elif label == 'common':
                status = 'qualified' if baseline_total > 0 and candidate_total > 0 else 'not_applicable'
                reason = 'both binaries returned nonzero matches' if status == 'qualified' else 'common control had no matches'
            elif target_id:
                if label == 'tail' and not baseline_saturated and target_id not in baseline_ids and target_id in candidate_ids:
                    status, reason = 'qualified', 'target absent in baseline and present in candidate'
                elif label == 'prefix' and target_id in baseline_ids and target_id in candidate_ids:
                    status, reason = 'qualified', 'target present in both binaries'
                elif label == 'tail' and baseline_saturated:
                    reason = 'baseline result set saturated; absence is unknown'
                else:
                    reason = 'target result separation was not established'
            qualifications[command][label] = {
                'status': status,
                'reason': reason,
                'baseline_matches': baseline_total,
                'candidate_matches': candidate_total,
            }
    return qualifications


def measure(name, workspace, specification):
    queries = specification['queries']
    target_id = specification.get('target_id')
    before = specification['fingerprint']
    if digest_state(workspace) != before:
        raise RuntimeError('measurement workspace changed before qualification')
    records = list((workspace / '.research-run/knowledge').glob('*.json'))
    qualifications = qualify_queries(workspace, queries, target_id)
    if digest_state(workspace) != before:
        raise RuntimeError('measurement workspace changed during qualification')
    byte_budget = ReadBudget()
    output = dict(workload=name, knowledge_records=len(records),
                  canonical_bytes=sum(len(read_regular(path, workspace, byte_budget)) for path in safe_files(workspace)),
                  qualifications=qualifications, cases=[])
    for command in ('search', 'context'):
        for label, query in queries.items():
            if qualifications[command][label]['status'] != 'qualified':
                continue
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
    bodies = []
    for path in records:
        record = json.loads(read_regular(path, workspace, budget).decode())
        if not isinstance(record, dict) or not isinstance(record.get('body'), str):
            raise RuntimeError(f'knowledge record body is not text: {path}')
        bodies.append(record['body'])
    target, body = max(zip(records, bodies), key=lambda pair: len(pair[1]))
    if len(body) <= PROJECTION_CHARS:
        raise RuntimeError('measurement workspace has no body tail beyond the projection')
    prefix = body[:16].strip()
    tail = body[-16:].strip()
    tail_source = body[PROJECTION_CHARS:]
    if not prefix or not tail or not tail_source.strip() or tail not in tail_source:
        raise RuntimeError('derived measurement queries do not prove a tail-only control')
    return dict(queries=dict(prefix=prefix, tail=tail,
                             absent='rropp-synthetic-absent-needle', common='the'),
                target_id=target.stem)


def select_knowledge_files(workspace, files):
    knowledge = workspace / '.research-run/knowledge'
    return [path for path in files if path.parent == knowledge and path.suffix == '.json']


def self_test():
    from measurement_fixture_controls import run_controls as fixture_controls
    from measurement_read_controls import run_controls as read_controls
    read_controls()
    fixture_controls()

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
                if b'symlink' not in result.stderr:
                    raise AssertionError('entry failed after the required symlink preflight')
                after = {path: path.read_bytes() for path in EVIDENCE.glob('latency-*.json')}
                if after != evidence:
                    raise AssertionError('rejected entry path wrote measurement evidence')
    print('measurement entry controls passed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', help='self-test, entry-test, integration-test, synthetic, or workspace path')
    parser.add_argument('--evidence-dir', type=pathlib.Path,
                        help='fixture/output directory; retained baseline/candidate paths stay unchanged')
    options = parser.parse_args()
    if options.evidence_dir is not None:
        EVIDENCE = options.evidence_dir.absolute()
    if options.action == 'self-test':
        self_test()
    elif options.action == 'entry-test':
        entry_self_test()
    elif options.action == 'integration-test':
        from measurement_integration import run_controls
        run_controls()
    elif options.action == 'synthetic':
        for name, count, size in [('many-short', 1000, 200), ('near-budget', 1000, 65536)]:
            workspace, queries = synthetic(name, count, size)
            measure(name, workspace, queries)
    else:
        workspace = pathlib.Path(options.action).absolute()
        files = safe_files(workspace)
        before = digest_state(workspace)
        validate_workspace(workspace)
        files = safe_files(workspace)
        records = select_knowledge_files(workspace, files)
        specification = derive_queries(workspace, records)
        if digest_state(workspace) != before:
            raise RuntimeError('supplied workspace changed during validation and query derivation')
        specification['fingerprint'] = before
        _reject_symlink_components(EVIDENCE)
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        measure('current-workspace', workspace, specification)
