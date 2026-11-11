**Release app audit · 2026-11-09 · Android 12 ARM64 emulator · AOT + R8 · Simulator-only scope**

This pass profiles an installed Release app, including native CPU scheduling, callstack samples, frame timelines, and background behavior. It establishes an app-level baseline; it does not claim a frame-rate guarantee or a measured application speedup.

```bash
python3 tests/prepare_android_release_profile.py --out /tmp/nexa-release-profile-owned \
  --nexa /tmp/nexa-verification-target/debug/nexa --edge-fade 16
```

## Build and capture

The preparation tool copies the audit example into its own directory, changes its application ID to `dev.nexa.performanceprofile`, and uses a disposable signing key. Its APK is AOT Release with R8, not a DevRuntime host. The only native manifest instrumentation adds shell profiling permission; it does not enable debugging.

- `--out` is required: use an empty directory or one previously owned by the tool. Existing unrelated files are rejected.
- `--nexa` selects the compiler executable; its default is `nexa` on PATH.
- `--edge-fade` selects the controlled test variant, either `0` or `16` (default).
- The tool builds and prints an APK path. It never installs or launches the app.
- The output includes `profile-artifact.json`, source/APK hashes, and the generated Release native project.

Use the emulator explicitly; the physical phone is outside this audit's authorized scope.

```bash
profile_dir=/tmp/nexa-release-profile-owned
adb -s emulator-5554 install -r "$profile_dir/build/android/app/build/outputs/apk/release/app-release.apk"
adb -s emulator-5554 shell am start -W -n dev.nexa.performanceprofile/.MainActivity
# Allow startup work to settle before recording steady animation.
adb -s emulator-5554 shell perfetto --txt -c - \
  -o /data/misc/perfetto-traces/nexa-release.pftrace \
  < tests/fixtures/native-ui/release-profile.pbtxt
adb -s emulator-5554 pull /data/misc/perfetto-traces/nexa-release.pftrace /tmp/nexa-release.pftrace
```

The checked-in configuration records twelve seconds with scheduler events, 100 Hz app CPU sampling, frame timelines, and process-memory counters. It uses Android 12-compatible fields and enables no screenshot or screen-recording source.

The first survey used a newer `timestamp_clock` field rejected by Android 12, then a file-streaming configuration with diagnostics. Those captures are excluded from accepted measurements. Final captures use ring buffers with no reported error/data-loss statistics. The checked-in configuration also passed an independent native smoke capture.

## Analyze a saved trace

```bash
python3 -m venv /tmp/nexa-profile-tools
/tmp/nexa-profile-tools/bin/pip install perfetto
/tmp/nexa-profile-tools/bin/python tests/native_release_trace.py /tmp/nexa-release.pftrace \
  --package dev.nexa.performanceprofile --out /tmp/nexa-release-summary.json
```

`native_release_trace.py` takes a trace path, an optional `--package` filter (default `dev.nexa.performanceprofile`), and optional `--out` JSON path. Without `--out`, it prints JSON. It fails if the target process is missing or the trace reports error/data-loss statistics; missing data cannot silently become a zero-CPU success.

Perfetto is an optional developer-side dependency, never a Nexa app dependency. This run used Python package 0.59.1 and trace processor v59.1. Analysis records the trace SHA-256 so the extracted metrics can be associated with the original capture.

## Measured app behavior

The app includes a currency NumberFlow, timed text, an overflowing Marquee, a sectioned library, and a bounded 10,000-item FastList.

- In a **12.05-second foreground sample**, the main thread accumulated **604.8 ms** of scheduled CPU; RenderThread accumulated **5,151.6 ms**. These are per-thread CPU totals, not nested slice wall times.
- Sample leaves and long rendering slices point toward buffer transfer/submission work in the emulator rendering path. This is an attribution clue, not proof that the framework or emulator alone caused it.
- No app measure/layout slices appeared during steady animation. `Recomposer:recompose` bookkeeping markers appeared; these do not establish how many individual composables recomposed.
- The frame timeline reports missed deadlines and buffer stuffing. This is **not passing frame-performance acceptance**, and no physical-device 120 Hz result is claimed.
- During a separate **11.99-second settled background sample**, the target process remained present, with **zero app CPU samples and no scheduled app-thread runs**. RSS stayed constant across thirteen process-stat samples at approximately **129.2 MiB**. This short interval does not prove absence of leaks or battery efficiency over time.

Evidence: [foreground](release-app-2026-11-09/foreground.json), [background](release-app-2026-11-09/background.json), [artifact identity](release-app-2026-11-09/artifact.json).

## Fade comparison

An exploratory pair used the same source/configuration with only `edgeFade` changed, reinstalling each Release build and allowing eight seconds after cold launch before twelve-second captures.

- Fade `0`: RenderThread **5,235.6 ms**, main thread **891.9 ms**.
- Fade `16`: RenderThread **5,179.7 ms**, main thread **931.4 ms**.

This pair did not isolate the mask as the dominant cost. It is insufficient for a statistical or hardware-wide conclusion, so no feature removal or native rendering rewrite is justified by it. [Fade 0](release-app-2026-11-09/fade-0.json), [fade 16 repeat](release-app-2026-11-09/fade-16-repeat.json).

## iOS AOT Release simulator profile · 2026-11-10

The generated PerformanceAudit app was built with the iOS simulator Release configuration, installed and launched on the iPhone 18 Pro Max simulator (iOS 27.0), then sampled for about 13 seconds after launch settled. The workload includes NumberFlow, TextReveal, Marquee, sectioned content, and a 10,000-row FastList.

- The native profiler reported no CPU hotspot or UI hang in this steady-state capture. The raw CPU query contained 9,638 PerformanceAudit samples spanning 12.964 seconds.
- The capture did not exercise scrolling, changing values, startup, or a long-running memory workload. It therefore does not establish costs for those interactions.
- The profiler's leak export was unavailable because this trace had no Leaks track. Memory growth was not measured.
- This simulator result cannot establish hardware 120 Hz frame performance or battery use.

This adds the first usable un-injected iOS AOT Release steady-state trace; the interactive, startup, and longer memory captures remain open. Verification remains simulator-only at the user's request.

## Android AOT Release emulator profile · 2026-11-10

A fresh signed AOT Release build of the same PerformanceAudit fixture was installed on the Android 12 ARM64 QEMU emulator and sampled with Perfetto for 12.18 seconds after launch settled. The trace completed without Perfetto issues.

- The main thread accumulated **648 ms** CPU; RenderThread accumulated **4,679 ms**. Its most sampled leaf was `transfer_max_buffers` (315 of 556 process samples), in the emulator graphics buffer-transfer path.
- Compose `Recomposer:recompose` accounted for **13.5 ms** total. The trace had no Compose measure/layout slices. This does not indicate a recomposition or layout bottleneck in the fixture.
- Frame timelines contained app deadline misses and buffer-stuffing markers. This simulator result fails to establish frame performance; it is not a physical-device or 120 Hz acceptance result.
- RSS ranged from **134.3 MiB to 135.9 MiB** across twelve samples. That short capture cannot establish long-term memory stability.

The rendering samples point to the emulator's buffer-transfer pipeline, so this trace alone does not justify changing Nexa's drawing implementation. [Trace summary and artifact hashes](native-ui-2026-11-10/android-aot-profile.json).

## Android AOT FastList scroll · 2026-11-10

The same Release fixture was exercised with repeated vertical swipes over its 10,000-row `FastList` while a 12-second Perfetto trace ran. Android accessibility state advanced from visible row `0` to `101`; no screenshot was captured. Perfetto reported no trace errors or data loss.

- The main thread accumulated **603.5 ms** CPU across the trace; RenderThread accumulated **5,611.9 ms**. `transfer_max_buffers` remained the leading sampled frame (**318 of 619 app-process samples**), while the app main thread had only 66 CPU samples.
- Compose `Recomposer:recompose` markers totaled **11.7 ms** over 443 frames. No Compose measure or layout slices appeared. Main-thread sampling did not identify a Nexa FastList or visible-row selection hotspot.
- The emulator still reported app deadline misses and buffer stuffing, with long frame intervals. Because the graphics-transfer path dominates and this is QEMU, these results do not establish device scrolling performance or a 120 Hz result.
- RSS varied from **135.2 MiB to 136.7 MiB** and Java heap from **5.3 MiB to 6.2 MiB** during this short interval. This is not a leak or long-term memory test.

Evidence: [FastList scroll trace summary and hash](native-ui-2026-11-10/android-fastlist-scroll.json). The iOS steady-state profile did not exercise a gesture. An iOS XCTest issued repeated swipes and collected metrics, but did not assert that the first visible row changed; it is excluded as proof of active scrolling.

## iOS TextReveal layout cache · 2026-11-10

The generated UIKit renderer queried TextKit for each line's highlight glyph range on every display-link draw. The generator now computes a conservative glyph range once when text layout or timing geometry changes, then keeps the existing per-line clips and draw order during animation.

- The offscreen native bitmap harness passed all pixel checks, including LTR/RTL text, italic and decorated text, fractional line heights, and 1x/2x/3x scale. It creates no screenshot files.
- On the iPhone 18 Pro Max iOS 27 simulator, median cost for 20 draws changed from **1.177 ms to 0.934 ms** at six lines (**20.6% lower**) and from **53.030 ms to 9.176 ms** at sixty lines (**5.78× faster**). The two-line result was unchanged within measurement noise.
- This isolates renderer CPU work; it is not app frame timing, hardware performance, or a 120 Hz result.

The current parity app was regenerated from the same `.nx` source for both platforms and compiled as an AOT Debug app: Xcode `BUILD SUCCEEDED`, Gradle `BUILD SUCCESSFUL`. The previously materialized Android source under `examples/component-parity/build/parity-aot` was dated October 26, so it was excluded from current-source code review. [Raw TextReveal benchmark and pixel coverage](native-ui-2026-11-10/ios-text-reveal.json).

## Regression and remaining work

The native iOS lifecycle harness now also moves an ancestor between scroll owners within one window and verifies that scrolling the new owner offscreen suspends Marquee. The tested behavior already passed; this pass adds regression coverage rather than a speculative native fix. [Native result](release-app-2026-11-09/scroll-owner.json).

The preparation tool completed a fresh signed Release build, and its generated APK identity was recorded. [Builder validation](release-app-2026-11-09/builder-validation.json). The temporary profiling app and remote traces were removed from the emulator after capture; the existing dev app was reopened. No physical phone was modified.

Workspace check, Clippy, all **1,611 workspace tests**, and the temporary-directory collision gate passed. Formatting, generated parity-report freshness, and whitespace checks passed. The profiling tools also rejected a missing target process, rejected a trace with diagnostics, and preserved an unowned output directory. [Tool checks](release-app-2026-11-09/tool-validation.json).

Interactive workload profiling, representative startup samples, and longer memory trends remain open. Verification remains simulator-only at the user's request; hardware frame timing and battery acceptance are unverified.
