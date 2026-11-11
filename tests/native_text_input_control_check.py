#!/usr/bin/env python3
"""Exercise the shared native TextInput controls on an existing iOS simulator.

Native delegate events verify editing buffers, callbacks and measured line heights. No screenshots or image files are created. No device is booted here.
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
    native = "\n".join((ROOT / ("crates/nexa-backend-swift/src/generator/components/" + name)).read_text()
        for name in ["text_input.swift", "text_input_native.swift", "text_input_edit.swift"])
    types = (ROOT / "crates/nexa-backend-swift/src/generator/engine/types.rs").read_text()
    error_type = types.split("public enum NexaComponentRefError", 1)[1].split("@MainActor", 1)[0]
    ref_type = types.split("public final class NexaTextInputRef", 1)[1].split("@MainActor", 1)[0]
    native += "\npublic enum NexaComponentRefError" + error_type
    native += "\n@MainActor public final class NexaTextInputRef" + ref_type
    fixture = (ROOT / "tests/fixtures/native-ui/TextInputControlCheck.swift").read_text()
    path = scratch / "Check.swift"
    path.write_text("import Foundation\nimport SwiftUI\nimport UIKit\n" + native + "\n" + fixture)
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
    results = {"scope": "Native UIKit text input delegate and geometry checks; no screenshots"}
    with tempfile.TemporaryDirectory(prefix="nexa-text-input-controls-") as directory:
        results["ios"] = swift_check(device, Path(directory))
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
