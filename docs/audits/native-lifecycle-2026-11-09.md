**Native lifecycle audit · 2026-11-09 · SwiftUI/UIKit + Core Animation · Shared AOT/DevRuntime primitive**

Marquee teardown now permanently stops its native controller. Delayed work cannot restart an animation on a controller retained after SwiftUI dismantles its subtree.

```bash
python3 tests/native_marquee_lifecycle_check.py --require-device
CARGO_TARGET_DIR=/tmp/nexa-verification-target cargo test -p nexa-cli --test native_ui_hotpaths marquee_disposal_is_terminal_when_simulator_is_available
```

## Reproduced defect

The previous `dispose()` canceled current animation and removed observers, but left `updatePlayback()` callable. An already queued scroll callback could therefore schedule playback again while the dismantled controller remained retained and attached.

The native regression reproduced **animation active → dispose stops animation → later callback restarts animation**. [Before evidence](native-lifecycle-2026-11-09/marquee-before.json).

## Root fix

- Record disposal as a terminal controller state before canceling native work.
- Reject playback, layout, source-layout callbacks, and configuration updates after disposal.
- Keep repeated teardown harmless and continue releasing notification tokens, scroll observers, layout callbacks, and the edge-fade mask.
- Keep ordinary pause/resume separate from disposal; the public API and mounted-controller identity stay unchanged.
- Advance the generated-source cache to version 507 so existing app hosts receive the fix.

The check adds no per-frame SwiftUI state, timer, parameter map, or type erasure. Core Animation continues to drive mounted marquee motion. It prevents native work from being revived after teardown; battery and frame-time improvements have not been quantified.

## Native verification scope

The optimized Swift harness compiles the actual generic controller source for the iOS 16 simulator target. It mounts overflowing SwiftUI text inside a UIKit scroll hierarchy, verifies playback starts, queues a scroll callback, disposes the controller, and drains the run loop.

- Playback stays stopped after the queued callback and direct playback update.
- Repeated disposal releases observers, callbacks, and the fade mask.
- Late configuration and layout cannot revive the controller.
- A pending initial delay remains canceled after its deadline passes.

[After evidence](native-lifecycle-2026-11-09/marquee-after.json). The Cargo regression runs this harness when a simulator is booted; `--require-device` prevents a skipped local acceptance run.

The headless harness substitutes app-root environment keys and temporarily reports an active UIApplication within its own process. This is controlled lifecycle evidence, not a full scene/background acceptance test. Objective-C test plumbing is confined to the fixture and never emitted into Nexa apps. No screenshots are captured.

The Android implementation already owns animation through Compose's cancellable `LaunchedEffect`/`collectLatest` coroutine; this pass changes no Kotlin animation behavior. That is a source-level ownership review, not a measured Android lifecycle result.

## Build verification

The focused Cargo regression, workspace check, Clippy, and all **1,611 workspace tests** passed. The temporary-directory collision gate passed. Both updated DevRuntime hosts rebuilt and connected on the simulators. Both Release audit builds completed with status `measured`, no failure reason, and no audit warnings. [Release artifacts](native-lifecycle-2026-11-09/release-builds.json).

## Remaining acceptance

Whole-app startup, retained-memory trends, battery use, and physical-device 120 Hz performance remain open. Successful teardown regression and native compilation do not prove those broader properties.

App-level follow-up: [installed AOT Release profiling](release-app-2026-11-09.md).
