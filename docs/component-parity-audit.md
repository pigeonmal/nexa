# Nexa Component Parity Audit

**Status:** in progress · **Current human gate:** SegmentedCases · **Platforms:** iOS and Android

The generated route and capability matrix is maintained in [`PARITY.md`](../PARITY.md). Automated work on non-screen framework capabilities may continue while this screenshot review is pending. A screen does not advance to passed until a person reviews and approves it.

The catalog's 41 feature screens have automated route and return-navigation coverage. Eight components have been approved in the human screenshot review: Column, Row, Stack, Spacer, Title, Menu, Button, and Pressable. The per-screen parameter and state audit continues; a screen does not pass until its iOS and Android result is reviewed and approved.

**10-second check:**

```bash
CARGO_TARGET_DIR=/tmp/nexa-verification-target cargo test -p nexa-cli --test hot_reload_coverage
```

The paired run used the booted iOS 26.5 simulator and Android API 31 emulator with their existing profiles. No viewport, size, or density overrides were applied.

## Coverage

| Area | Check | Result |
|---|---|---|
| Component arguments | The example exercises every declared argument, shared accessibility argument, behavior modifier, and representative enum value. | Catalog presence check passed; per-component value audit in progress |
| Native generation | The catalog app compiles to SwiftUI and Compose; DevRuntime parity is checked where the component screen exposes it. | Build passed; per-screen review in progress |
| iOS / Android routes | Automated coverage opens all 41 feature screens and returns to the catalog, including the two nested title variants. | Route walk passed; human screenshot gates in progress |
| Runtime behavior | AOT/DevRuntime replay covers representative toolbar, control, text entry, list, refresh, paging, navigation, presentation, and gesture behavior. | Baseline replay passed; capability coverage still needs audit |
| Rust workspace | Full check and Clippy passed previously; the workspace suite has one known external SQLite analyzer fixture failure. | Re-run final gates after this sweep |

### Current human review gate: SegmentedControl

The iOS 26.5 simulator and Android API 31 emulator are both on `SegmentedCases`, reset to Day. The screen includes the default platform animation, a custom spring, a custom tween, and one-item, empty, and invalid-selection cases. AOT first-tap selection and rapid retargeting were checked on both platforms; Android indicator motion was recorded at 30 fps. Typed animation settings are covered through compiler lowering, Swift/Kotlin generation, and DevRuntime dispatch. Human screenshot approval is pending, so the screen-by-screen review remains on this route.

The shared animation API is documented in [`animations.md`](animations.md). It currently supports springs, tweens/easing, scoped state animation, conditional transitions, and content-size animation. Keyframe timelines and repeat-forever motion remain unimplemented.

### Previously audited component: Text

The iOS 26.5 simulator and Android API 31 emulator were rebuilt from the current compiler and opened on the three TextCases pages: typography, semantic font styles, and layout/effects. The fixture covers every declared Text argument, every alignment and semantic font-style value, representative font designs and weights, newline preservation, truncation, line height, spacing, padding, glyph shadow, opacity, transform, blur, clipping, z-order, selection, and one-tap page switching.

- Fixed Android AOT localization so explicit newlines are emitted as Android `\\n` resource escapes instead of being collapsed to spaces.
- Fixed iOS AOT and DevRuntime so `fontStyle: Headline` keeps its semibold default when paired with `fontSize`; explicit `fontWeight` still overrides it.
- Android and iOS AOT builds succeeded. Swift/Kotlin backend unit tests passed. Native text selection opened Copy actions on both platforms.
- The three AOT screenshot pairs were reviewed and compared with DevRuntime. Human approval remains pending; automated evidence does not advance the screen review gate.

The parameter gate proves every declared argument is present in a real parsed and generated example. It does not run every possible value combination for every argument; enum alternatives and interaction states are covered where the catalog screen or runtime replay provides them.

## Findings and fixes

- Android DevRuntime previously placed two `Dialog` button children in the confirm slot. It now maps the first child to dismissal and the second to confirmation, matching AOT. A source regression test protects the mapping.
- The component route walk exposed a native title difference for the selected Inbox tab. The Android smoke test now asserts the tab's actual visible title and still verifies that Back returns to the catalog.
- AOT/DevRuntime screenshot diffs on Android show scroll-offset drift in a few captures because the two runs do not land on the same content anchor. Stable, non-scrolling states and interaction results match; those offset captures are not treated as evidence of a rendering defect.
- App bottom navigation and native picker/slider controls retain their platform-specific controls. Shared content, spacing, and layout were compared while allowing those native controls to differ.

## Workspace test limitation

The unrestricted `cargo test --workspace` run stops at `nexa-compiler/tests/core_task_crud.rs::typed_task_crud_example_compiles_for_both_targets`. The separate SQLite plugin analyzer in the `plugins` submodule does not handle the newer `ChildBody::AppBottomBar` AST case. That plugin package is outside this framework audit, so it was left unchanged. The rest of the workspace suite passed with only that test skipped.
