"""Fail-closed filesystem reads for the local measurement helpers."""

import os
import stat
from pathlib import Path


def _reject_symlink_components(path, floor=None):
    absolute = Path(path).absolute()
    floor_parts = len(Path(floor).absolute().parts) if floor else 1
    current = Path(floor).absolute() if floor else Path(absolute.anchor)
    for component in absolute.parts[floor_parts:]:
        current /= component
        if current.is_symlink():
            raise RuntimeError(f'refusing symlink path component: {current}')


def _confined(path, workspace):
    if Path(path).is_symlink():
        raise RuntimeError(f'refusing symlink path: {path}')
    workspace_path = Path(workspace).absolute()
    _reject_symlink_components(workspace_path, workspace_path.parent)
    _reject_symlink_components(path, workspace_path)
    workspace_root = workspace_path.resolve(strict=True)
    resolved = Path(path).resolve(strict=True)
    resolved.relative_to(workspace_root)
    return resolved


def safe_files(workspace):
    workspace = Path(workspace)
    if workspace.is_symlink():
        raise RuntimeError(f'refusing symlink workspace: {workspace}')
    _reject_symlink_components(workspace, workspace.parent)
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


def read_regular(path, workspace):
    path = _confined(path, workspace)
    flags = os.O_RDONLY | os.O_NONBLOCK
    if hasattr(os, 'O_NOFOLLOW'):
        flags |= os.O_NOFOLLOW
    descriptor = os.open(path, flags)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode):
            raise RuntimeError(f'refusing non-regular measurement entry: {path}')
        chunks = []
        while True:
            chunk = os.read(descriptor, 1024 * 1024)
            if not chunk:
                return b''.join(chunks)
            chunks.append(chunk)
    finally:
        os.close(descriptor)
