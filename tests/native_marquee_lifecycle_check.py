#!/usr/bin/env python3
"""Exercise the emitted Marquee controller on a booted iOS simulator, without screenshots."""
import argparse
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

from native_ui_hotpaths import ROOT, run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--require-device", action="store_true")
    parser.add_argument("--expect-disposal-bug", action="store_true")
    args = parser.parse_args()
    device = None
    if shutil.which("xcrun"):
        result = subprocess.run(["xcrun", "simctl", "list", "devices", "booted", "-j"], capture_output=True, text=True)
        if result.returncode == 0:
            device = next((entry["udid"] for entries in json.loads(result.stdout)["devices"].values() for entry in entries if entry["state"] == "Booted"), None)
    if not device:
        if args.require_device:
            raise SystemExit("A booted iOS simulator is required")
        print(json.dumps({"ios": "unavailable: no booted iOS simulator"}))
        return
    source = (ROOT / "crates/nexa-backend-swift/src/generator/components/marquee.swift").read_text()
    fixture = (ROOT / "tests/fixtures/native-ui/MarqueeLifecycleCheck.swift").read_text()
    with tempfile.TemporaryDirectory(prefix="nexa-marquee-lifecycle-") as directory:
        scratch = Path(directory)
        path = scratch / "Check.swift"
        path.write_text("import SwiftUI\nimport UIKit\nimport QuartzCore\nimport ObjectiveC\n" + source + "\n" + fixture)
        sdk = run(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"])
        binary = scratch / "check"
        run(["xcrun", "--sdk", "iphonesimulator", "swiftc", "-O", "-warnings-as-errors", "-swift-version", "6", "-parse-as-library", "-sdk", sdk,
             "-target", f"{platform.machine()}-apple-ios16.0-simulator", path, "-o", binary])
        arguments = ["xcrun", "simctl", "spawn", device, binary]
        if args.expect_disposal_bug:
            arguments.append("--expect-disposal-bug")
        output = json.loads(run(arguments))
    print(json.dumps({"scope": "Native Marquee controller lifecycle; no app frame-time claim", "ios": output}, indent=2))


if __name__ == "__main__":
    main()
