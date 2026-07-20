#!/usr/bin/env python3
"""Read the root package version from Cargo.toml with TOML semantics."""

import pathlib
import sys
import tomllib


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: read-cargo-package-version.py CARGO_TOML", file=sys.stderr)
        return 2
    with pathlib.Path(sys.argv[1]).open("rb") as manifest_file:
        manifest = tomllib.load(manifest_file)
    package = manifest.get("package")
    if not isinstance(package, dict):
        raise ValueError("Cargo manifest must contain a [package] table")
    version = package.get("version")
    if not isinstance(version, str) or not version:
        raise ValueError("[package].version must be a non-empty string")
    print(version)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"Cargo version failure: {error}", file=sys.stderr)
        raise SystemExit(1)
