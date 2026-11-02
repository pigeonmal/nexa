#!/usr/bin/env python3
"""Machine-readable environment and capture manifest generator for Nexa Parity tests."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Optional


REQUIRED_FIELDS = {
    "fixture",
    "platform",
    "mode",
    "source_sha256",
    "compiler_build",
    "host_capability_hash",
    "module_revision",
    "device",
    "os_version",
    "ui_locale",
    "font_scale",
    "density",
    "appearance",
    "event_sequence",
    "capture_phase",
}


def compute_sha256(path: Path) -> str:
    if not path.is_file():
        return "missing"
    hasher = hashlib.sha256()
    hasher.update(path.read_bytes())
    return hasher.hexdigest()


def create_manifest(
    fixture: str,
    platform: str,
    mode: str,
    source_path: Optional[Path] = None,
    source_sha256: Optional[str] = None,
    compiler_build: str = "0.1.0",
    host_capability_hash: str = "unknown",
    module_revision: str = "initial",
    device: str = "generic",
    os_version: str = "unknown",
    ui_locale: str = "en-US",
    font_scale: float = 1.0,
    density: float = 1.0,
    appearance: str = "light",
    event_sequence: Optional[list[str]] = None,
    capture_phase: str = "initial",
) -> dict[str, Any]:
    if source_sha256 is None:
        if source_path is not None:
            source_sha256 = compute_sha256(source_path)
        else:
            source_sha256 = "0" * 64

    return {
        "fixture": fixture,
        "platform": platform,
        "mode": mode,
        "source_sha256": source_sha256,
        "compiler_build": compiler_build,
        "host_capability_hash": host_capability_hash,
        "module_revision": module_revision,
        "device": device,
        "os_version": os_version,
        "ui_locale": ui_locale,
        "font_scale": font_scale,
        "density": density,
        "appearance": appearance,
        "event_sequence": event_sequence or ["launch"],
        "capture_phase": capture_phase,
    }


def validate_manifest(data: dict[str, Any]) -> tuple[bool, list[str]]:
    errors = []
    missing = REQUIRED_FIELDS - set(data.keys())
    if missing:
        errors.append(f"Missing required manifest fields: {sorted(missing)}")
    if data.get("platform") not in ("ios", "android"):
        errors.append(f"Invalid platform: {data.get('platform')}")
    if data.get("mode") not in ("aot", "dev", "dev_cold"):
        errors.append(f"Invalid mode: {data.get('mode')}")
    return len(errors) == 0, errors


def main() -> None:
    parser = argparse.ArgumentParser(description="Nexa Parity Capture Manifest Utility")
    subparsers = parser.add_subparsers(dest="subcommand", required=True)

    write_parser = subparsers.add_parser("write", help="Write a capture manifest JSON")
    write_parser.add_argument("--out", type=Path, required=True, help="Destination JSON path")
    write_parser.add_argument("--fixture", required=True, help="Fixture name")
    write_parser.add_argument("--platform", choices=["ios", "android"], required=True)
    write_parser.add_argument("--mode", choices=["aot", "dev", "dev_cold"], required=True)
    write_parser.add_argument("--source", type=Path, default=None, help="Path to source file")
    write_parser.add_argument("--source-sha256", default=None)
    write_parser.add_argument("--compiler-build", default="0.1.0")
    write_parser.add_argument("--host-hash", default="default")
    write_parser.add_argument("--revision", default="initial")
    write_parser.add_argument("--device", default="simulator")
    write_parser.add_argument("--os-version", default="unknown")
    write_parser.add_argument("--locale", default="en-US")
    write_parser.add_argument("--font-scale", type=float, default=1.0)
    write_parser.add_argument("--density", type=float, default=1.0)
    write_parser.add_argument("--appearance", choices=["light", "dark"], default="light")
    write_parser.add_argument("--phase", required=True, help="Capture phase name")
    write_parser.add_argument("--events", nargs="*", default=["launch"])

    validate_parser = subparsers.add_parser("validate", help="Validate a capture manifest JSON")
    validate_parser.add_argument("manifest", type=Path, help="Path to manifest JSON")

    args = parser.parse_args()

    if args.subcommand == "write":
        manifest = create_manifest(
            fixture=args.fixture,
            platform=args.platform,
            mode=args.mode,
            source_path=args.source,
            source_sha256=args.source_sha256,
            compiler_build=args.compiler_build,
            host_capability_hash=args.host_hash,
            module_revision=args.revision,
            device=args.device,
            os_version=args.os_version,
            ui_locale=args.locale,
            font_scale=args.font_scale,
            density=args.density,
            appearance=args.appearance,
            event_sequence=args.events,
            capture_phase=args.phase,
        )
        valid, errors = validate_manifest(manifest)
        if not valid:
            print(f"Error creating manifest: {errors}", file=sys.stderr)
            sys.exit(1)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(manifest, indent=2) + "\n")
        print(f"Wrote capture manifest to {args.out}")

    elif args.subcommand == "validate":
        if not args.manifest.is_file():
            print(f"Manifest not found: {args.manifest}", file=sys.stderr)
            sys.exit(1)
        try:
            data = json.loads(args.manifest.read_text())
        except Exception as e:
            print(f"Invalid JSON in manifest {args.manifest}: {e}", file=sys.stderr)
            sys.exit(1)
        valid, errors = validate_manifest(data)
        if not valid:
            for err in errors:
                print(f"Validation error: {err}", file=sys.stderr)
            sys.exit(1)
        print(f"Valid manifest: {args.manifest}")


if __name__ == "__main__":
    main()
