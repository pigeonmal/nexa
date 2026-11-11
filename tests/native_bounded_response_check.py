#!/usr/bin/env python3
"""Run the generated native Swift response collector against private URLProtocol streams."""
import json
from pathlib import Path
import shutil
import sys
import tempfile

from native_ui_hotpaths import ROOT, run


def main():
    if sys.platform != "darwin" or not shutil.which("swiftc"):
        print(json.dumps({"swift": "unavailable: Apple Swift toolchain required"}))
        return
    source = (ROOT / "crates/nexa-backend-swift/src/generator/api/bounded_response.swift").read_text()
    fixture = (ROOT / "tests/fixtures/native-ui/BoundedResponseCheck.swift").read_text()
    with tempfile.TemporaryDirectory(prefix="nexa-native-bounded-response-") as directory:
        scratch = Path(directory)
        path = scratch / "Check.swift"
        path.write_text("import Foundation\nimport os\n" + source + "\n" + fixture)
        binary = scratch / "check"
        run(["swiftc", "-O", "-warnings-as-errors", "-swift-version", "6", "-parse-as-library", path, "-o", binary])
        output = json.loads(run([binary]))
    print(json.dumps({"scope": "Native Apple URLSession transport; not app frame timing", "swift": output}, indent=2))


if __name__ == "__main__":
    main()
