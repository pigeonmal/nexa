#!/usr/bin/env python3
"""Run the shared TextInput edit corpus on Android ART without touching UI."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import uuid
import zipfile

from native_ui_hotpaths import ROOT, kotlin_jars, run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--android-device")
    parser.add_argument("--require-device", action="store_true")
    args = parser.parse_args()
    sdk = Path(os.environ.get("ANDROID_HOME", os.environ.get("ANDROID_SDK_ROOT", Path.home() / "Library/Android/sdk")))
    adb = sdk / "platform-tools/adb"
    device = args.android_device
    dependencies = kotlin_jars()
    if not device and adb.is_file():
        result = subprocess.run([str(adb), "devices"], capture_output=True, text=True)
        device = next((line.split()[0] for line in result.stdout.splitlines()[1:] if len(line.split()) == 2 and line.split()[1] == "device"), None)
    if not device or not dependencies or not shutil.which("java"):
        if args.require_device:
            raise SystemExit("A connected Android device and cached Kotlin compiler are required")
        print(json.dumps({"android": "unavailable: device or toolchain missing"}))
        return
    with tempfile.TemporaryDirectory(prefix="nexa-text-input-native-") as directory:
        scratch = Path(directory)
        android = sorted((sdk / "platforms").glob("android-*/android.jar"))[-1]
        d8 = sorted((sdk / "build-tools").glob("*/lib/d8.jar"))[-1]
        source = (ROOT / "crates/nexa-backend-kotlin/src/generator/components/text_input_edit.kt").read_text()
        path = scratch / "Check.kt"
        corpus = (ROOT / "tests/fixtures/text-input-edits.json").read_text()
        fixture = r'''
object Check {
    @JvmStatic fun main(args: Array<String>) {
        val cases = org.json.JSONArray("""CORPUS""")
        val limiter = NexaTextInputEditLimiter()
        for (index in 0 until cases.length()) {
            val item = cases.getJSONObject(index)
            val actual = limiter.edit(item.getString("previous"), item.getString("proposed"),
                item.optInt("maxLength", Int.MAX_VALUE), item.optInt("maxLines", Int.MAX_VALUE), item.getBoolean("multiline"))
            check(actual == item.getString("expected")) { "Failed: " + item.getString("name") + " actual=" + actual }
        }
        println("{\"passed\":" + cases.length() + "}")
    }
}
'''.replace("CORPUS", corpus)
        path.write_text(source + fixture)
        classpath = os.pathsep.join(str(path) for path in dependencies)
        classes = scratch / "check.jar"
        run(["java", "-cp", classpath, "org.jetbrains.kotlin.cli.jvm.K2JVMCompiler", "-no-stdlib", "-no-reflect", "-classpath", str(android) + os.pathsep + classpath, path, "-d", classes])
        dex = scratch / "dex"
        dex.mkdir()
        runtime = [path for path in dependencies if path.name.startswith(("kotlin-stdlib-", "kotlinx-coroutines-core-jvm-"))]
        run(["java", "-cp", d8, "com.android.tools.r8.D8", "--lib", android, "--min-api", "24", "--output", dex, classes, *runtime])
        archive = scratch / "check-dex.jar"
        with zipfile.ZipFile(archive, "w") as output:
            for path in dex.glob("*.dex"):
                output.write(path, path.name)
        remote = "/data/local/tmp/nexa-text-input-" + uuid.uuid4().hex + ".jar"
        run([adb, "-s", device, "push", archive, remote])
        try:
            result = run([adb, "-s", device, "shell", f"CLASSPATH={remote} app_process /system/bin Check"])
            print(json.dumps(json.loads(result.splitlines()[-1]), indent=2))
        finally:
            run([adb, "-s", device, "shell", "rm", remote])


if __name__ == "__main__":
    main()
