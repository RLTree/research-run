"""Fail-closed reads for quiescent, cooperative local measurements.

Static symlinks and special files are rejected before reads; regular files are
opened no-follow and nonblocking, then checked with ``fstat``. This helper does
not prevent an adversarial concurrent parent-directory replacement between its
path checks and descriptor acquisition. Per-file limits follow canonical record
budgets (1 MiB ordinary, 32 MiB inventory) and each pass has a 64 MiB aggregate
budget. Measurement evidence therefore excludes uncooperative concurrent
writers and makes no universal confinement claim.
"""

import os
import stat
import sys
from pathlib import Path

MAX_RECORD_BYTES = 1 * 1024 * 1024
MAX_INVENTORY_RECORD_BYTES = 32 * 1024 * 1024
MAX_SNAPSHOT_BYTES = 64 * 1024 * 1024


class ReadBudget:
    def __init__(self, limit=MAX_SNAPSHOT_BYTES):
        self.limit = limit
        self.used = 0

    def consume(self, amount):
        if self.used + amount > self.limit:
            raise RuntimeError(f'measurement snapshot exceeds {self.limit} bytes')
        self.used += amount


def _allowed_os_alias(path):
    if sys.platform != 'darwin':
        return False
    return path in (Path('/var'), Path('/tmp')) and path.resolve(strict=True) in (
        Path('/private/var'), Path('/private/tmp')
    )


def _reject_symlink_components(path):
    absolute = Path(path).absolute()
    current = Path(absolute.anchor)
    for component in absolute.parts[1:]:
        current /= component
        if current.is_symlink() and not _allowed_os_alias(current):
            raise RuntimeError(f'refusing symlink path component: {current}')


def _confined(path, workspace):
    if Path(path).is_symlink():
        raise RuntimeError(f'refusing symlink path: {path}')
    workspace_path = Path(workspace).absolute()
    _reject_symlink_components(workspace_path)
    _reject_symlink_components(path)
    workspace_root = workspace_path.resolve(strict=True)
    resolved = Path(path).resolve(strict=True)
    resolved.relative_to(workspace_root)
    return resolved


def safe_files(workspace):
    workspace = Path(workspace)
    if workspace.is_symlink():
        raise RuntimeError(f'refusing symlink workspace: {workspace}')
    _reject_symlink_components(workspace)
    root = workspace / '.research-run'
    if root.is_symlink():
        raise RuntimeError(f'refusing symlink research state: {root}')
    _confined(root, workspace)
    if not root.is_dir():
        raise RuntimeError(f'research state is not a directory: {root}')
    files = []
    pending = [root]
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as entries:
            for entry in entries:
                path = Path(entry.path)
                info = entry.stat(follow_symlinks=False)
                if stat.S_ISLNK(info.st_mode):
                    raise RuntimeError(f'refusing symlink in research workspace: {path}')
                _confined(path, workspace)
                if stat.S_ISDIR(info.st_mode):
                    pending.append(path)
                elif stat.S_ISREG(info.st_mode):
                    files.append(path)
                else:
                    raise RuntimeError(f'refusing non-regular measurement entry: {path}')
    return sorted(files)


def file_limit(path, workspace):
    state = Path(workspace).resolve(strict=True) / '.research-run'
    relative = Path(path).resolve(strict=True).relative_to(state)
    return MAX_INVENTORY_RECORD_BYTES if relative.parts[:1] == ('inventories',) else MAX_RECORD_BYTES


def read_regular(path, workspace, budget=None):
    path = _confined(path, workspace)
    flags = os.O_RDONLY | os.O_NONBLOCK
    if hasattr(os, 'O_NOFOLLOW'):
        flags |= os.O_NOFOLLOW
    descriptor = os.open(path, flags)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode):
            raise RuntimeError(f'refusing non-regular measurement entry: {path}')
        limit = file_limit(path, workspace)
        if info.st_size > limit:
            raise RuntimeError(f'measurement file exceeds {limit} bytes: {path}')
        budget = budget or ReadBudget()
        chunks = bytearray()
        remaining = limit + 1
        total_read = 0
        while True:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                return bytes(chunks)
            budget.consume(len(chunk))
            chunks.extend(chunk)
            remaining -= len(chunk)
            total_read += len(chunk)
            if total_read > limit:
                raise RuntimeError(f'measurement file grew beyond {limit} bytes: {path}')
    finally:
        os.close(descriptor)
