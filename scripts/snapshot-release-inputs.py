#!/usr/bin/env python3
"""Snapshot bounded release inputs through no-follow directory handles."""

import os
import stat
import sys


def read_regular_nofollow(path: str, limit: int) -> bytes:
    if not os.path.isabs(path):
        raise ValueError("release authorization inputs must use absolute paths")
    parts = [part for part in path.split(os.sep) if part]
    if not parts:
        raise ValueError("release authorization input path is empty")
    directory = os.open(os.sep, os.O_RDONLY | os.O_DIRECTORY)
    try:
        for part in parts[:-1]:
            next_directory = os.open(
                part,
                os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                dir_fd=directory,
            )
            os.close(directory)
            directory = next_directory
        descriptor = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory)
        try:
            before = os.fstat(descriptor)
            if not stat.S_ISREG(before.st_mode) or before.st_size > limit:
                raise ValueError("release authorization input is not a bounded regular file")
            chunks: list[bytes] = []
            remaining = limit + 1
            while remaining:
                chunk = os.read(descriptor, min(65536, remaining))
                if not chunk:
                    break
                chunks.append(chunk)
                remaining -= len(chunk)
            data = b"".join(chunks)
            after = os.fstat(descriptor)
            identity = lambda value: (value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns)
            if len(data) > limit or identity(before) != identity(after) or len(data) != before.st_size:
                raise ValueError("release authorization input changed while being read")
            return data
        finally:
            os.close(descriptor)
    finally:
        os.close(directory)


def write_exclusive(path: str, data: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        view = memoryview(data)
        while view:
            written = os.write(descriptor, view)
            view = view[written:]
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def main() -> int:
    if len(sys.argv) != 5:
        print("usage: snapshot-release-inputs.py DEST REQUEST SIGNATURE PUBLIC_KEY", file=sys.stderr)
        return 2
    destination, request, signature, public_key = sys.argv[1:]
    if not os.path.isdir(destination) or os.path.islink(destination):
        raise ValueError("snapshot destination must be a regular directory")
    for name, path, limit in (
        ("request.json", request, 64 * 1024),
        ("request.sig", signature, 64 * 1024),
        ("owner.pub", public_key, 16 * 1024),
    ):
        write_exclusive(os.path.join(destination, name), read_regular_nofollow(path, limit))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as error:
        print(f"release authorization snapshot failure: {error}", file=sys.stderr)
        raise SystemExit(2)
