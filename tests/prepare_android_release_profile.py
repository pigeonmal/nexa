#!/usr/bin/env python3
"""Build an isolated, signed AOT Release profiling APK; never installs it.

Copies examples/performance-audit with a separate application ID and disposable
signing credentials. Adds only the shell-profileable manifest flag after native
generation. Requires Nexa, Android SDK, Gradle dependencies, Java and keytool.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = "dev.nexa.performanceprofile"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, type=Path, help="Empty directory or an output previously owned by this script")
    parser.add_argument("--nexa", default="nexa", help="Nexa CLI executable")
    parser.add_argument("--edge-fade", type=int, choices=[0, 16], default=16, help="Controlled marquee A/B variant")
    args = parser.parse_args()
    root = args.out.resolve()
    marker = root / ".nexa-profile-fixture"
    root.mkdir(parents=True, exist_ok=True)
    if any(root.iterdir()) and not marker.is_file():
        raise SystemExit("Refusing to overwrite a directory not owned by this profiling fixture")
    marker.write_text("Nexa Release profiling fixture\n")
    for name in ["App.nx", "nexa.config.nx"]:
        shutil.copyfile(ROOT / "examples/performance-audit" / name, root / name)
    config = root / "nexa.config.nx"
    config.write_text(config.read_text().replace("dev.nexa.performanceaudit", PACKAGE))
    source = root / "App.nx"
    text = source.read_text()
    if "edgeFade: 16" not in text:
        raise SystemExit("Performance fixture changed; review the A/B substitution before profiling")
    source.write_text(text.replace("edgeFade: 16", f"edgeFade: {args.edge_fade}"))
    env = os.environ.copy()
    env.update(NEXA_ANDROID_KEYSTORE=str(root / "profiling.keystore"), NEXA_ANDROID_KEY_ALIAS="profiling",
               NEXA_ANDROID_STORE_PASSWORD="android", NEXA_ANDROID_KEY_PASSWORD="android")
    if not Path(env["NEXA_ANDROID_KEYSTORE"]).exists():
        subprocess.run(["keytool", "-genkeypair", "-noprompt", "-dname", "CN=Nexa Profiling Only", "-validity", "30",
                        "-keyalg", "RSA", "-keysize", "2048", "-keystore", env["NEXA_ANDROID_KEYSTORE"],
                        "-alias", "profiling", "-storepass", "android", "-keypass", "android"], check=True)
    subprocess.run([args.nexa, "release", "--android"], cwd=root, env=env, check=True)
    android = root / "build/android"
    manifest = android / "app/src/main/AndroidManifest.xml"
    namespace = "http://schemas.android.com/apk/res/android"
    ET.register_namespace("android", namespace)
    tree = ET.parse(manifest)
    application = tree.getroot().find("application")
    if application is None:
        raise SystemExit("Generated manifest has no application")
    profile = application.find("profileable")
    if profile is None:
        profile = ET.SubElement(application, "profileable")
    profile.set("{" + namespace + "}shell", "true")
    tree.write(manifest, encoding="utf-8", xml_declaration=True)
    subprocess.run([str(android / "gradlew"), ":app:assembleRelease", "--console=plain"], cwd=android, env=env, check=True)
    apk = android / "app/build/outputs/apk/release/app-release.apk"
    metadata = {"package": PACKAGE, "edgeFade": args.edge_fade, "apkBytes": apk.stat().st_size,
                "apkSha256": hashlib.sha256(apk.read_bytes()).hexdigest(),
                "sourceSha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                "scope": "AOT Release fixture with profiling manifest and disposable signing identity"}
    (root / "profile-artifact.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(apk)


if __name__ == "__main__":
    main()
