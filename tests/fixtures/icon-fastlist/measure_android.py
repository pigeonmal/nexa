#!/usr/bin/env python3
"""Measure the installed AOT IconFastList benchmark on an Android emulator."""

import argparse
import json
import re
import subprocess
import time


PACKAGE = "dev.nexa.iconfastlist"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("serial")
    parser.add_argument("--trials", type=int, default=3)
    args = parser.parse_args()
    if not args.serial.startswith("emulator-"):
        parser.error("this benchmark is scoped to an emulator")

    def adb(*command):
        return subprocess.check_output(["adb", "-s", args.serial, *command], text=True)

    sizes = re.findall(r"(\d+)x(\d+)", adb("shell", "wm", "size"))
    width, height = map(int, sizes[-1])
    ticks_per_second = int(adb("shell", "getconf", "CLK_TCK"))
    x, bottom, top = int(width * 0.51), int(height * 0.79), int(height * 0.23)
    down = f"input swipe {x} {bottom} {x} {top} 600"
    up = f"input swipe {x} {top} {x} {bottom} 600"
    gesture = "; ".join([down] * 4 + [up] * 4)

    def cpu_ticks(pid):
        stats = adb("shell", "cat", f"/proc/{pid}/stat").rsplit(")", 1)[1].split()
        return int(stats[11]) + int(stats[12])

    adb("shell", "cmd", "package", "compile", "-m", "speed", "-f", PACKAGE)
    results = []
    for trial in range(args.trials + 1):
        adb("shell", "am", "force-stop", PACKAGE)
        adb("shell", "am", "start", "-W", "-n", f"{PACKAGE}/.MainActivity")
        time.sleep(2)
        pid = adb("shell", "pidof", PACKAGE).strip()
        adb("shell", "dumpsys", "gfxinfo", PACKAGE, "reset")
        t0 = cpu_ticks(pid)
        start = time.monotonic()
        adb("shell", gesture)
        time.sleep(0.5)
        elapsed = time.monotonic() - start
        t1 = cpu_ticks(pid)
        frames = adb("shell", "dumpsys", "gfxinfo", PACKAGE)
        memory = adb("shell", "dumpsys", "meminfo", "-s", PACKAGE)

        def metric(pattern, source):
            return int(re.search(pattern, source).group(1))

        if trial:
            results.append({
                "seconds": round(elapsed, 3),
                "cpu_ms": (t1 - t0) * 1000 / ticks_per_second,
                "pss_kib": metric(r"TOTAL PSS:\s+(\d+)", memory),
                "frames": metric(r"Total frames rendered: (\d+)", frames),
                "p50_ms": metric(r"50th percentile: (\d+)ms", frames),
                "p95_ms": metric(r"95th percentile: (\d+)ms", frames),
                "p99_ms": metric(r"99th percentile: (\d+)ms", frames),
            })
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
