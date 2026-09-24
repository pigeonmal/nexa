#!/usr/bin/env python3
"""Fail when a release package contains Nexa's debug-only development runtime."""

from __future__ import annotations

import sys
import zipfile
from pathlib import Path


FORBIDDEN_MARKERS = (
    b"NexaDevRuntime",
    b"NexaDevRenderer",
    b"NexaDevServer",
    b"nexa_dev_runtime",
    b"nexa.dev.runtime",
)


def contents(path: Path):
    if path.is_dir():
        for child in path.rglob("*"):
            if child.is_file():
                yield child.relative_to(path).as_posix(), child.read_bytes()
        return
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            for name in archive.namelist():
                if not name.endswith("/"):
                    yield name, archive.read(name)
        return
    raise ValueError(f"unsupported or missing release artifact: {path}")


def main(arguments: list[str]) -> int:
    if not arguments:
        print("usage: verify-release-artifacts.py <archive-or-package>...", file=sys.stderr)
        return 2
    failures = []
    for argument in arguments:
        artifact = Path(argument)
        try:
            matches = [
                (name, marker.decode())
                for name, data in contents(artifact)
                for marker in FORBIDDEN_MARKERS
                if marker in data
            ]
        except (OSError, ValueError, zipfile.BadZipFile) as error:
            print(f"{artifact}: {error}", file=sys.stderr)
            return 2
        if matches:
            failures.extend(f"{artifact}!{name}: found {marker}" for name, marker in matches)
    if failures:
        print("Debug-only Nexa runtime found in release artifacts:", file=sys.stderr)
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(f"verified {len(arguments)} release artifact(s): no Nexa DevRuntime content found")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
