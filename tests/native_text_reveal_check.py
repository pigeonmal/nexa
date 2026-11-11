#!/usr/bin/env python3
"""Compile the emitted UIKit TextReveal renderer on an existing iOS simulator.

Offscreen pixel arrays compare native drawing against the previous per-line
reference. No screenshots or image files are created. No device is booted here.
"""
import argparse
import json
from pathlib import Path
import platform
import subprocess
import shutil
import tempfile

from native_ui_hotpaths import ROOT, run


def swift_check(device, scratch):
    source = (ROOT / "crates/nexa-backend-swift/src/generator/components/text_reveal.swift").read_text()
    native = "private struct NexaTextRevealWordTiming {" + source.split("private struct NexaTextRevealWordTiming {", 1)[1]
    fixture = (ROOT / "tests/fixtures/native-ui/TextRevealCheck.swift").read_text()
    path = scratch / "Check.swift"
    path.write_text("import UIKit\nimport QuartzCore\nimport CryptoKit\n" + native + "\n" + fixture)
    sdk = run(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"])
    binary = scratch / "swift-check"
    run(["xcrun", "--sdk", "iphonesimulator", "swiftc", "-O", "-swift-version", "6", "-parse-as-library", "-sdk", sdk,
         "-target", f"{platform.machine()}-apple-ios16.0-simulator", path, "-o", binary])
    return json.loads(run(["xcrun", "simctl", "spawn", device, binary]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ios-device")
    parser.add_argument("--require-device", action="store_true")
    args = parser.parse_args()
    device = args.ios_device
    if not device and shutil.which("xcrun"):
        available = subprocess.run(["xcrun", "simctl", "list", "devices", "booted", "-j"], capture_output=True, text=True)
        if available.returncode == 0:
            devices = json.loads(available.stdout)["devices"]
            device = next((entry["udid"] for entries in devices.values() for entry in entries if entry["state"] == "Booted"), None)
    if not device:
        if args.require_device:
            raise SystemExit("A booted iOS simulator is required")
        print(json.dumps({"ios": "unavailable: no booted iOS simulator"}))
        return
    results = {"scope": "Native UIKit offscreen rendering; not app frame timing or screen captures"}
    with tempfile.TemporaryDirectory(prefix="nexa-text-reveal-") as directory:
        results["ios"] = swift_check(device, Path(directory))
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
