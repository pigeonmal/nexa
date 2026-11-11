**Native animation audit · 2026-11-09 · SwiftUI/UIKit + Compose · AOT and DevRuntime**

This pass improves drawing and allocation work in compiled Nexa apps. The public TextReveal and Marquee APIs stay unchanged.

```bash
python3 tests/native_text_reveal_check.py --require-device
python3 tests/native_marquee_check.py --require-device
CARGO_TARGET_DIR=/tmp/nexa-verification-target cargo test -p nexa-cli --test native_ui_hotpaths
```

## TextReveal drawing on iOS

- Each highlighted line previously requested a draw of every glyph in the paragraph. Repeating that traversal made long captions expensive.
- Keep the existing rectangular clips and their order. Ask TextKit for the glyph range intersecting each clip, including a device-pixel fringe for antialiased edges, then draw that range.
- Use the already laid-out glyph cache. Playback does not request additional text layout or allocate a mask, rectangle array, or path for each frame.
- Apple's [glyph-range documentation](https://developer.apple.com/documentation/uikit/nslayoutmanager/glyphrange%28forboundingrectwithoutadditionallayout%3Ain%3A%29) describes the native bounded-drawing query used here.

The regression harness compares the emitted UIKit renderer against the previous per-line reference in private memory buffers. It creates no screenshot or image file.

- **170 pixel comparisons passed:** reveal start, partial positions, middle, ending, and completion; LTR and RTL; leading, center, and trailing alignment; automatic, small, integer, and fractional line heights; italic, underline, strikethrough, and translucent text; 1×/2×/3× rendering scales; CJK, emoji, and mixed-script text.
- The tests use exact pixel hashes on the same native layout, not a tolerance that could hide clipping changes.
- For 60 laid-out lines, median CPU drawing time fell from **53.80 ms to 10.05 ms per 20 draws**, approximately **5.3×** faster in this simulator run.
- For six lines, medians were **1.19 ms → 0.95 ms per 20 draws**. The two-line result was effectively flat: **0.318 ms → 0.322 ms**. No universal speedup is claimed.

Raw samples and cases: [UIKit rendering](native-animation-2026-11-09/uikit-rendering.json), [environment](native-animation-2026-11-09/environment.json).

## TextReveal paint and native clock lifetime

- **Reproduced correctness defect:** changing only color or dim opacity left the old attributed-string paint in place. The earlier native probe returned `colorUpdates: false`.
- Update foreground attributes independently of font/layout configuration. Native regression tests verify the new color and opacity while retaining timing geometry storage.
- SwiftUI dismantling now explicitly disposes the native display link. Disposal invalidates the link and prevents layout from restarting a disposed timeline.
- The native test mounts the view in a window, starts its actual CADisplayLink, disposes it, and checks that it stays stopped after layout.

These changes live in the common Swift native primitive used by both AOT output and DevRuntime.

## Marquee layout allocation on Android

- Layout and scrolling callbacks previously copied the immutable metrics snapshot even when size and visibility were unchanged. StateFlow suppressed the equal publication after the object had already been allocated.
- Check the existing typed fields first. Copy and publish only a real measurement or visibility change. Reactive child values are passed through the typed `revision` input, so same-size text changes can restart without publishing Compose state from a draw callback.
- The shared Kotlin primitive changes no viewport geometry, playback phase, timing, or accessibility behavior.

Measurement evidence:

- The compiled JVM probe measured **1,600,000 bytes → 0 bytes** for 20,000 unchanged measurement/visibility pairs after warm-up.
- The same generated geometry kernel ran on **Android ART, API 31**. Seven samples reported approximately **1.28 MB before** and **no allocation-counter increase after**, with no counter increase in the control samples.
- ART's [runtime allocation statistics](https://developer.android.com/reference/android/os/Debug#getRuntimeStat(java.lang.String)) are approximate and may lag individual allocations. The raw values are not an exact per-object count or a frame-time result.
- Regression checks preserve snapshot identity on unchanged input and verify changed dimensions, visibility transitions, and the compiler-generated revision path. The primitive source check guards against draw-phase StateFlow writes.

Raw evidence: [ART allocations](native-animation-2026-11-09/art-allocations.json), [JVM/native helper checks](native-animation-2026-11-09/native-helper-results.json).

## Scope and limits

- Native source cache schema `build-v504` invalidates older generated units even when app source is unchanged.
- Shared native primitive changes apply to AOT and DevRuntime. No interpreter, timer in Nexa, dynamic parameter map, or third-party runtime dependency was added.
- The performance fixture includes NumberFlow, TextReveal, Marquee, and a bounded virtualized 10,000-row list inside an explicit ScrollView. Previously approved parity routes are unchanged.
- Tests use the currently booted simulators; they do not create devices. The Android geometry probe touches no UI and removes its temporary dex archive afterward.
- Software bitmap timings isolate UIKit/TextKit work. They do not measure the complete SwiftUI frame pipeline, GPU work, playback, app startup, retained memory over time, battery use, or 120 Hz acceptance.
- This is a verified slice of the broader native-output audit; whole-app profiling and other native paths remain open.

## Verification

- Native UIKit pixel, paint-invalidation, and display-link lifetime tests passed on the booted iOS simulator.
- Native geometry checks passed on the Android emulator's ART runtime and on the host JVM.
- Both AOT Release build audit statuses are `measured`: [Release audit](native-animation-2026-11-09/release-audit.json).
- Updated iOS and Android DevRuntime hosts built and connected.
- `cargo check --workspace --all-targets`: passed.
- `cargo clippy --workspace --all-targets`: passed without warnings.
- `cargo test --workspace`: 1,607 tests passed on the default stack; one existing ignored test.
- `cargo test -p nexa-testkit -- --ignored`: concurrent temporary-directory collision gate passed.
- Cache-schema history and dependency-cache tests passed after the `build-v504` bump.
- Formatting, generated parity-report freshness, and whitespace checks passed.

Follow-up: [native sectioned-list refresh audit](native-lists-2026-11-09.md).

Lifecycle follow-up: [terminal Marquee teardown](native-lifecycle-2026-11-09.md).

Android invalidation follow-up: the 2026-11-10 audit removed the Marquee draw modifier's `contentInvalidated()` StateFlow publication. Compose draw callbacks can run during graphics-layer recording; they should not publish state back into the coroutine that owns animation timing. Child state values already flow through `revision`, while changed measured geometry is published by the layout phase. This is an architectural fix and a regression guard; the available debug simulator CPU traces did not establish a frame-time improvement.

## Android marquee draw-feedback follow-up · 2026-11-10

- Regenerated and compiled the component parity app for iOS and Android DevRuntime, then generated and compiled an Android AOT Release profiling fixture with R8 minification and resource shrinking.
- The Android primitive no longer publishes `MutableStateFlow` from the graphics-layer source draw callback. Reactive child values arrive through the typed `revision` parameter; measured dimensions and visibility are published only when they change.
- Added codegen and runtime-helper regression coverage. The source guard checks that the cached content modifier does not write animation-control state from drawing, and the Kotlin codegen test checks that a changing `.nx` state is present in `revision`.
- On the Android 12 emulator (ART API 31), 20,000 unchanged geometry/visibility pairs allocated 0 bytes in six of seven samples and 16 KiB in one sample; the reference implementation allocated 950–967 KiB per sample. The runtime allocation counter is approximate, so this demonstrates the unchanged-geometry fast path rather than complete-frame allocation or CPU improvement. Raw values: [Android Marquee draw-feedback probe](native-animation-2026-11-09/android-marquee-draw-feedback.json).
- A release simulator CPU profile could not be taken: Android's `simpleperf` denied shell access to the profileable, non-debuggable app, and emulator `gfxinfo` returned inconsistent legacy frame statistics. No app-level frame-time or CPU speedup is claimed from this pass.

## Android TextReveal draw-work follow-up · 2026-11-10

Inspecting generated component-parity Kotlin exposed repeated paragraph work in
`NexaTextRevealPrimitive`: every visible line called `drawText` for the whole
laid-out paragraph. The component now records that paragraph into one native
`Picture` when its draw cache is built, then replays the recorded commands under
the existing per-line reveal clips. The `CanvasDrawScope` and `Picture` live for
the mounted component; animation progress is still read in the draw phase, so
playback does not rebuild layout or re-record the paragraph. DevRuntime imports
preload the same drawing dependencies, while AOT imports remain feature-gated.

An Android API 31 Compose `TextLayoutResult` probe compared the direct-draw
reference with Picture replay in memory. LTR, RTL, and mixed-script cases had
identical pixels. In this isolated software-canvas probe, three draws took
147.875 ms versus 7.728 ms for 72 LTR lines, 72.648 ms versus 7.150 ms for 40
RTL lines, and 72.368 ms versus 5.041 ms for 50 mixed-script lines. These are
paragraph-drawing measurements, not complete app frames, GPU timings, or a
120 Hz result.

The cached full-paragraph `Picture` is replayed under each visible line's own
clip. A follow-up tried combining the line rectangles into one reusable union
clip so the picture could be replayed once per frame. An in-memory Android
pixel comparison disproved equivalence: 16 pixels differed for an LTR partial
reveal with a fractional line height. The union clip was removed. Independent
line clips match the reference compositing, including overlapping fractional
line bounds, while still replaying cached draw commands instead of rerunning
text shaping and glyph drawing for every line.

The corrected Android emulator probe compared the independent-clip Picture
renderer with direct per-line `drawText` for 24 LTR, RTL, mixed-script, CJK,
emoji, fractional-line-height, tight-line-height, and reveal-position cases;
all pixels matched in memory. No screenshots were captured. An attempted
standalone `app_process` timing probe was invalid because that shell process
does not initialize the app's Typeface defaults; it was discarded. No
end-to-end frame-time improvement is claimed for the rejected union-clip
change.

The generated component-parity Swift and Kotlin apps compiled after the change.
Regression tests require recording to remain outside the animated line loop,
replaying the recorded paragraph inside independent per-line clips, and
emitting these imports for DevRuntime while omitting them from a plain-text AOT
target. AOT Android's first compile caught a DrawScope receiver mismatch during
development; the remembered native `CanvasDrawScope` fix is covered by the
successful generated build. The simulator profile remains inconclusive for
app-level frame cost, so this pass targets the directly reproduced repeated
text-drawing hot path only.
