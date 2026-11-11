**Nexa audit · 2026-11-09 · compiler and native generators**

This audit fixes reproduced failures and measures compiler code generation. It does not certify that every possible runtime performance issue has been eliminated.

```bash
CARGO_TARGET_DIR=/tmp/nexa-verification-target cargo test --workspace
python3 tests/benchmark-suite.py /tmp/nexa-verification-target/debug/nexa /tmp/nexa-benchmark --runs 9
```

## Fixed findings

- **Compiler stack exhaustion:** the built-in lowering dispatcher retained a large debug stack frame during recursive child lowering. Split component handlers into focused modules and prevent recursive dispatcher/handler inlining. The previously overflowing adaptive-layout, compiler, and locale suites now run on the default test stack. LLDB disassembly on this arm64 debug build shows the dispatcher frame fell from approximately 449 KB to 106 KB (about 76%); this is a compiler debug-stack measurement, not an app-memory result.
- **Repeated source-section scans:** replace repeated full-section fixed-point scans with an indexed identifier traversal. Visit each reachable section once, preserve original emission order and conservative reference semantics, and preallocate output capacity. Regression tests cover cycles, duplicate symbols, non-identifier labels, and equivalence to the original algorithm.
- **Unnecessary IR copies:** move owned native-call codecs and marquee playback expressions during folding instead of cloning them.
- **Android host compilation:** shared status-bar background and slider primitives require extension imports even when the initial source module does not use those components. Import these with shared native primitives and assert their availability in the minimal-host test.
- **Coverage failures:** update stale structural assertions to match current native hit targets and resolved child arrays. Add a separate HapticCases route to cover Light, Medium, and Heavy without changing approved routes.
- **Lint findings:** group Swift renderer arguments into typed options and reuse RenderScope. This changes generator organization rather than app behavior.

## Measurements

Nine runs per fixture, using the debug CLI on the same macOS arm64 host. The baseline already includes the stack-frame refactor; this comparison isolates the source-section traversal change.

- Counter codegen median: iOS 5,540 → 4,506 μs; Android 4,305 → 4,004 μs.
- Todo codegen median: iOS 6,140 → 4,544 μs; Android 4,830 → 4,493 μs.
- Virtual-list codegen median: iOS 13,248 → 13,617 μs; Android 5,885 → 5,011 μs. The iOS result is about 3% slower; no universal speedup is claimed.
- Showcase codegen median: iOS 7,544 → 5,393 μs; Android 7,908 → 5,696 μs.
- Generated source byte counts match in every pair. These measurements concern compilation, not device frame rate.

Raw samples and environment metadata: [before](performance-2026-11-09/codegen-before.json), [after](performance-2026-11-09/codegen-after.json).

## Scope and limits

- Inspected compiler lowering, IR folding, native source reachability, shared primitive imports, generated UI type erasure and primitive-state patterns, and AOT/DevRuntime coverage gates.
- No production backend occurrences were found by the targeted scan for AnyView, explicitly boxed numeric mutableStateOf, Class.forName, or javaClass. This scan is evidence about those patterns, not a proof of absence of all allocations.
- Device battery use, retained-memory trends, production startup, and 120 Hz frame acceptance require separate physical-device measurements. Simulator builds and compiler timings cannot establish those results.
- Follow-up work on compiled NumberFlow and FastList native code is recorded separately in [the native output audit](native-ui-2026-11-09.md). Compiler timings here are not app runtime measurements.
- Device verification used textual accessibility trees. Argent describe returned automatically saved screen paths; no screenshot was opened or visually inspected.

## Verification

- `cargo check --workspace --all-targets`: passed.
- `cargo clippy --workspace --all-targets`: passed with no warnings.
- `cargo test --workspace`: passed on the default stack, with no filtered tests or enlarged stack override. Native-toolchain integration tests also ran.
- `python3 tests/render_component_parity_report.py --check`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- Performance-audit DevRuntime hosts: iOS and Android native builds passed. The first Android launch attempt found no connected emulator; the existing AVD was then booted for the parity app.
- Existing native integration tests compile both AOT and DevRuntime hosts. The final parity hosts built and connected on both simulators; accessibility inspection confirmed Marquee content and Source mounts: 1 on each platform. Physical-device profiling remains outside this verification.
