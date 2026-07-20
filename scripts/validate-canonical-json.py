#!/usr/bin/env python3
"""Reject duplicate keys, extra documents, and noncanonical JSON bytes."""

import json
import sys


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: validate-canonical-json.py REQUEST", file=sys.stderr)
        return 2
    raw = open(sys.argv[1], "rb").read()
    try:
        text = raw.decode("utf-8")
        value = json.loads(
            text,
            object_pairs_hook=unique_object,
            parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)),
        )
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        print(f"release authorization JSON failure: {error}", file=sys.stderr)
        return 1
    canonical = (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()
    if raw != canonical:
        print("release authorization request must be one canonical JSON document", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
