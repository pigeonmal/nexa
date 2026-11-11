#!/usr/bin/env python3
"""Exercise generated section-header tracking on an existing iOS simulator.

No screenshots, device creation, or changes to the foreground app. The private
UITableViews cover pinned headers, empty sections, reuse and weak ownership.
"""
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
    with tempfile.TemporaryDirectory(prefix="nexa-section-headers-") as directory:
        scratch = Path(directory)
        source = (ROOT / "crates/nexa-backend-swift/src/generator/components/list_headers.swift").read_text()
        fixture = (ROOT / "tests/fixtures/native-ui/SectionHeadersCheck.swift").read_text()
        runtime = (ROOT / "crates/nexa-backend-swift/src/generator/components/list_runtime.rs").read_text()
        start = runtime.index("        func leadingVisibleRowPosition(in tableView: UITableView) -> Int? {")
        end = runtime.index("        func seedInitialScrollPosition", start)
        # Execute the emitted method, not a separately maintained optimization.
        method = runtime[start:end]
        source += "\n@MainActor private struct VisibilityProbe { var reverseLayout: Bool\n" + method + "}\n"
        collection = "indexPathsForVisibleItems.lazy.map(\\.item).min()"
        assert runtime.count(collection) == 12, "All collection observation paths must use lazy projection"
        source += "\nprivate func collectionLeading(_ paths: [IndexPath]) -> Int? { paths.lazy.map(\\.item).min() }\n"
        source += (ROOT / "tests/fixtures/native-ui/ListVisibilityCheck.swift").read_text()
        path = scratch / "Check.swift"
        path.write_text("import UIKit\nimport QuartzCore\n" + source + "\n" + fixture)
        sdk = run(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"])
        binary = scratch / "check"
        run(["xcrun", "--sdk", "iphonesimulator", "swiftc", "-O", "-swift-version", "6", "-parse-as-library", "-sdk", sdk,
             "-target", f"{platform.machine()}-apple-ios16.0-simulator", path, "-o", binary])
        result = json.loads(run(["xcrun", "simctl", "spawn", device, binary]))
    print(json.dumps({"scope": "Native UIKit header refresh; not complete app frame time", "ios": result}, indent=2))


if __name__ == "__main__":
    main()
