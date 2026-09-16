"""Fixture-specific custody for CLI-initialized, fixed synthetic workloads.

The receipt contains hashes and directories, not a copy of canonical content.
It binds this fixture's initialization (including its random workspace identity).
It is local experiment metadata, not product authority or an adversarial seal.
"""
import hashlib
import json
import os
import stat

from measurement_fs import MAX_RECORD_BYTES, ReadBudget, read_regular, safe_files

RECEIPT_NAME = '.tail-text-fixture.json'


def authority_layout(workspace):
    """Inspect the complete state in the documented cooperative workspace."""
    files = safe_files(workspace)
    directories = sorted(str(path.relative_to(workspace))
                         for path in (workspace / '.research-run').rglob('*')
                         if path.is_dir())
    budget = ReadBudget()
    hashes = {str(path.relative_to(workspace)):
              hashlib.sha256(read_regular(path, workspace, budget)).hexdigest()
              for path in files}
    return {'directories': directories, 'files': hashes}


def record_initialization(workspace, count, size):
    """Called only immediately after successful init of an absent fixture."""
    layout = authority_layout(workspace)
    if any((workspace / '.research-run/knowledge').iterdir()):
        raise RuntimeError('initialized synthetic knowledge collection is not empty')
    receipt = {'version': 1, 'count': count, 'body_bytes': size, 'initialization': layout}
    raw = json.dumps(receipt, indent=2).encode()
    if len(raw) > MAX_RECORD_BYTES:
        raise RuntimeError('synthetic initialization receipt exceeds its byte limit')
    path = workspace / RECEIPT_NAME
    with path.open('xb') as output:
        output.write(raw)


def _read_receipt(workspace):
    path = workspace / RECEIPT_NAME
    flags = os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW
    try:
        descriptor = os.open(path, flags)
        try:
            info = os.fstat(descriptor)
            if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_RECORD_BYTES:
                raise RuntimeError('invalid synthetic initialization receipt type or size')
            with os.fdopen(descriptor, 'rb', closefd=False) as source:
                raw = source.read(MAX_RECORD_BYTES + 1)
            if len(raw) > MAX_RECORD_BYTES:
                raise RuntimeError('synthetic initialization receipt grew beyond its limit')
            return json.loads(raw)
        finally:
            os.close(descriptor)
    except (OSError, ValueError) as error:
        raise RuntimeError('missing or invalid initialization receipt; fixture preserved') from error


def require_fixture(workspace, count, size, expected_record):
    """Reject any layout/content drift before validation or timed commands."""
    actual = authority_layout(workspace)
    if {path.name for path in workspace.iterdir()} != {'.research-run', RECEIPT_NAME}:
        raise RuntimeError('unexpected synthetic workspace entry; fixture preserved')
    receipt = _read_receipt(workspace)
    if (not isinstance(receipt, dict) or receipt.get('version') != 1
            or receipt.get('count') != count or receipt.get('body_bytes') != size
            or not isinstance(receipt.get('initialization'), dict)):
        raise RuntimeError('synthetic fixture parameters or receipt changed; fixture preserved')
    initial = receipt['initialization']
    if not isinstance(initial.get('files'), dict) or not isinstance(initial.get('directories'), list):
        raise RuntimeError('invalid synthetic initialization layout; fixture preserved')
    expected = dict(initial['files'])
    for index in range(count):
        name = f'.research-run/knowledge/note-{index:04}.json'
        if name in expected:
            raise RuntimeError('initialization receipt contains fixture records')
        expected[name] = hashlib.sha256(expected_record(index, size)).hexdigest()
    if actual['directories'] != initial['directories'] or actual['files'] != expected:
        raise RuntimeError('synthetic authority layout or content drifted; fixture preserved')
