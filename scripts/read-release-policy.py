#!/usr/bin/env python3
"""Read the typed release policy from TOML and emit its exact JSON projection."""

import json
import pathlib
import sys
import tomllib


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: read-release-policy.py POLICY_TOML", file=sys.stderr)
        return 2
    with pathlib.Path(sys.argv[1]).open("rb") as policy_file:
        policy = tomllib.load(policy_file)
    release = policy.get("release")
    expected = {
        "version": str,
        "class": str,
        "contract": str,
        "publication_authorized": bool,
        "personhood_claim": bool,
        "scientific_truth_claim": bool,
        "product_fitness_claim": str,
    }
    if not isinstance(release, dict) or set(release) != set(expected):
        raise ValueError("[release] must contain exactly the declared release-policy keys")
    for key, kind in expected.items():
        if type(release[key]) is not kind:
            raise ValueError(f"[release].{key} has the wrong TOML type")
    print(json.dumps(release, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"release policy failure: {error}", file=sys.stderr)
        raise SystemExit(1)
