#!/usr/bin/env python3
"""Exercise generated UITableView and FastList grid code on an iOS simulator.

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
        native_template = runtime.split("#[cfg(test)]", 1)[0]
        assert collection in native_template, "Collection observers must project indexes lazily"
        for eager_projection in ("indexPathsForVisibleItems.map(", "indexPathsForVisibleItems.compactMap("):
            assert eager_projection not in native_template, "Collection observers must not materialize projected indexes"
        source += "\nprivate func collectionLeading(_ paths: [IndexPath]) -> Int? { paths.lazy.map(\\.item).min() }\n"
        source += (ROOT / "tests/fixtures/native-ui/ListVisibilityCheck.swift").read_text()
        grid_start = native_template.index("private final class NexaFastFixedGridLayout:")
        grid_end = native_template.index("// <nexa:list-runtime-masonry:begin>", grid_start)
        source += native_template[grid_start:grid_end]
        source += (ROOT / "tests/fixtures/native-ui/FixedGridCheck.swift").read_text()
        horizontal_start = native_template.index("private final class NexaFastHorizontalListView:")
        horizontal_end = native_template.index("private struct NexaFastHorizontalList<RowContent:", horizontal_start)
        source += native_template[horizontal_start:horizontal_end].removesuffix("@available(iOS 16.0, *)\n")
        offset_start = native_template.index("@MainActor\nprivate func nexaScrollFastListToOffset(")
        offset_end = native_template.index("@MainActor\nprivate func nexaScrollFastList(", offset_start)
        source += native_template[offset_start:offset_end]
        source += (ROOT / "tests/fixtures/native-ui/HorizontalListViewportCheck.swift").read_text()
        scroll_path = native_template.index("        func scrollPath(for row: Int, in tableView: UITableView)")
        report_position = native_template.index("        private func reportProgrammaticScrollPosition", scroll_path)
        restore_start = native_template.index("        func restoreBoundScrollPositionIfNeeded", report_position)
        restore_end = native_template.index("\n    }\n}\n// <nexa:list-runtime-vertical:end>", restore_start)
        source += """
@MainActor
private final class VerticalListRestoreProbe: NSObject, UITableViewDataSource {
    var rowCount = 24
    var nativeRowCount = 24
    var reverseLayout = true
    var scrollPosition: Int32? = 10
    var restoreRevision = 1
    var isApplyingProgrammaticScrollPosition = false
    private var pendingScrollPositionRestore: DispatchWorkItem?

    func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int { nativeRowCount }
    func tableView(_ tableView: UITableView, cellForRowAt path: IndexPath) -> UITableViewCell { UITableViewCell() }
    func leadingVisibleRowPosition(in tableView: UITableView) -> Int? { nil }
    func reportProgrammaticScrollPosition(_ position: Int) {}
    func reportEndReachedIfVisible(in tableView: UITableView) {}
"""
        source += native_template[scroll_path:report_position] + native_template[restore_start:restore_end] + "\n}\n"
        source += (ROOT / "tests/fixtures/native-ui/VerticalListRestoreCheck.swift").read_text()
        path = scratch / "Check.swift"
        path.write_text("import UIKit\nimport QuartzCore\n" + source + "\n" + fixture)
        sdk = run(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"])
        binary = scratch / "check"
        run(["xcrun", "--sdk", "iphonesimulator", "swiftc", "-O", "-swift-version", "6", "-parse-as-library", "-sdk", sdk,
             "-target", f"{platform.machine()}-apple-ios16.0-simulator", path, "-o", binary])
        result = json.loads(run(["xcrun", "simctl", "spawn", device, binary]))
    print(json.dumps({"scope": "Native UIKit FastList geometry and header refresh", "ios": result}, indent=2))


if __name__ == "__main__":
    main()
