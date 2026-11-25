# AOT icon FastList benchmark

| Targets | Build | Workload |
|---|---|---|
| iOS simulator and Android emulator | Native Release, compiled from Nexa AOT output | 600 fixed-height rows with two single-color SVG icons, one layered SVG icon, and one prebuilt font glyph per row |

Build both variants from the same fixture. Pass a saved CLI binary from before
the change and one from the current checkout.

```sh
bash tests/fixtures/icon-fastlist/build.sh "$before_cli" "$benchmark_dir" before "$ios_udid"
bash tests/fixtures/icon-fastlist/build.sh "$after_cli" "$benchmark_dir" after "$ios_udid"
```

The script injects the same display-link probe into both benchmark iOS hosts.
The Android APK uses Release optimization with the standard local debug key
for simulator installation.

## Measurement

- **iOS:** Install each Release simulator app. Launch it fresh for each run
  and wait for `Saved product 0`. Use Argent for four upward and four
  downward 600 ms swipes with momentum disabled, from 79% to 23% of viewport
  height at x = 51%. Take process CPU time and RSS from `ps -o time=,rss=`
  before and after the gesture sequence. Read `tmp/icon-fastlist-frames.json`
  inside the app container, and calculate display-link intervals from the
  start of the first swipe to the end of the last swipe.
- **Android:** Install each Release APK in turn. Run
  `python3 tests/fixtures/icon-fastlist/measure_android.py "$android_serial"`.
  The script forces native compilation, does one warmup, and records three
  runs with the same swipe distance and duration. It reads process CPU ticks,
  post-scroll PSS, and the frame-duration histogram from `gfxinfo`.
- Report CPU per gesture sequence, memory after the sequence, and frame-time
  percentiles. Android frame durations and iOS display-link intervals are
  different measurements. Compare each platform against its own baseline.

## Observed results

These are simulator measurements from the iPhone 18 Pro Max on iOS 27.0 and
the API 31 arm64 Android emulator. Each cell reports the median of three iOS
or six Android runs per build. CPU includes the gesture sequence and settling.

| Metric | iOS before → after | Android before → after |
|---|---:|---:|
| Process CPU per sequence | 5.03 s → 5.42 s | 705 ms → 555 ms |
| Post-scroll memory | 407.02 MiB → 398.58 MiB RSS | 34.17 MiB → 34.00 MiB PSS |
| p50 frame interval or duration | 16.67 ms → 16.67 ms | 34.5 ms → 37 ms |
| p95 frame interval or duration | 16.67 ms → 16.67 ms | 93 ms → 93 ms |
| p99 frame interval or duration | 16.67 ms → 16.67 ms | 135.5 ms → 101 ms |

Android CPU ranged from 600–920 ms before and 480–770 ms after. Median
CPU fell in the first batch and rose in the repeat batch. Rendered frame
counts varied from 87–310 before and 98–119 after, so the benchmark does
not establish a CPU speedup. iOS CPU ranged from 5.00–5.06 s before and
5.17–5.60 s after. The current runs show a small iOS CPU increase and stable
display-link cadence.

The staged fonts shrank from **61,212 B to 1,900 B** on both platforms.
The same fonts occupy **27,070 B → 1,096 B** compressed in the Android
Release APK. Brand went from 1,380 to 1,092 uncompressed bytes and Mazicon
from 59,832 to 808.

## Layer color evaluation

The SVG compiler currently stores each layer as a glyph alias and an RGBA
value. A probe containing `fill="currentColor"` alongside `fill="#000000"`
produced two opaque black layers. The metadata cannot distinguish their
paint sources after SVG normalization.

Per-layer customization can add an optional, typed palette keyed to SVG layer
order. Sparse overrides should leave the original color in place. If layer
colors must survive SVG reordering, named layers are a better contract than
positions. Explicit whole-icon tint already uses `color:` and should retain
its existing behavior.

Selective `currentColor` tinting needs an explicit paint source in the
compiled layer metadata. Record it before the SVG parser resolves CSS, keep
authored opacity separate, and preserve the source through single-layer
lowering and native menu images. This allows runtime tinting of theme layers
while retaining fixed artwork colors.
