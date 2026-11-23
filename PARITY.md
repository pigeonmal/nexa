# Nexa Agent Review

> **In progress** · 23 reviewed · 3 in progress · 18 pending · iOS + Android · native AOT + DevRuntime

Review shared behavior and appearance across iOS and Android. Build success and sampled device checks do not establish exhaustive parity.

```bash
python3 tests/render_component_parity_report.py --check
```

## Current work

- Route-by-route parity review restarted on 2026-11-01. Historical route notes remain context; current human validation is recorded explicitly below.
- ColumnCases first-pass finding: fractional layout dimensions now use the same app-window viewport source as ScreenWidth/ScreenHeight on both platforms. Focused dual-target generator coverage and iOS/Android DevRuntime host compilation pass; human approval recorded on 2026-11-02.
- ColumnCases clipping finding (2026-11-02): fixed-height Android children were measured at the parent’s shorter maximum; Kotlin AOT and DevRuntime now preserve top-aligned overflow. Backend regressions pass and both DevRuntime hosts rebuilt/connected; human approval recorded on 2026-11-02.
- ColumnCases clip-effect follow-up (2026-11-02): Android Compose now applies effect clipping outside fixed sizing and transforms in AOT and DevRuntime, matching SwiftUI's final-output clip. Kotlin backend tests pass (199); both DevRuntime hosts rebuilt and clients applied the module. Human approval recorded on 2026-11-02.
- Use finite parameter coverage: defaults, enum and Boolean alternatives, meaningful boundaries and optional states, and pairwise interactions. Record grammar- or semantically inexpressible cases as not applicable with their source constraint.
- Human verification approved (2026-11-02): the user validated ColumnCases on iOS and Android, including the animated Behavior sizing fix. This supersedes the earlier pending visual confirmation notes; Agent Review approved.
- RowCases Bounds follow-up (2026-11-02): the red child requests height 88 inside a centered Row capped at 68. iOS crops both rounded ends of the oversized child. The prior Android outer clip incorrectly rounded the constrained visible area. Android AOT and DevRuntime now apply corner styling inside fixed-size overflow and inherit the parent Row vertical alignment for overflow placement; Column keeps top-aligned overflow. Human verification was approved on 2026-11-03.
- RowCases blur finding (2026-11-02): Compose blur defaulted to rectangular edge clamping, keeping the solid surface boundary sharp while its text blurred. Android AOT and DevRuntime now request transparent unbounded blur for layout and text effects, matching SwiftUI surface-edge behavior. Focused tests cover blur outside rounded background styling. Human verification was approved on 2026-11-03.
- Row/Column collapse animation finding (2026-11-02): Android drew surface styling inside animateContentSize, so background and border snapped to the collapsed target while the layout wrapper still animated. Kotlin AOT and Android DevRuntime now place surface styling outside the size animation and keep content constraints inside it, matching SwiftUI. Regression coverage checks both sides of this modifier boundary. Human verification was approved on 2026-11-03. Both simulator hosts rebuilt and reconnected to DevRuntime; Android Row Behavior expansion and collapse were exercised through its toggle. Workspace check, clippy, and tests pass.
- Human verification approved (2026-11-03): the user approved StackCases on iOS and Android after the overflow-alignment fix; Agent Review approved.
- Human verification approved (2026-11-03): the user approved SpacerCases on iOS and Android after the two-spacer wrapping fix; Agent Review approved.
- Human verification approved (2026-11-03): the user approved TitleNoToolbarCases on iOS and Android; Agent Review approved. Active route: TextCases, next in App.nx route order.
- TextCases layout follow-up (2026-11-03): the iOS accessibility frame showed the UIKit-backed TextReveal occupying 29.6% of the screen height despite containing one line, pushing the numeric interpolation card down. The shared Swift primitive now applies fixed vertical sizing while retaining width-driven wrapping. A focused Swift backend regression passes; both DevRuntime hosts rebuilt and applied the updated module. The provided crop also starts below iOS “Monospaced design”; that element remains present in the iOS accessibility tree, so the supplied captures have different vertical anchors. Human visual verification of the sizing correction remains pending.
- TextCases demo clarification (2026-11-03): the TextReveal sample now explains that the app supplies per-word timings and playback position, states that it does not play audio, and offers three playback-position buttons to preview highlighting across a sentence. `nexa check` passes and iOS/Android DevRuntime clients applied the hot-reloaded module. Human visual and interaction verification remains pending.
- TextCases TextReveal preview correction (2026-11-03): the sample was marked playing with a forward target offset and also enabled strikethrough, so the 0.0 s control did not show the requested timestamp and the text looked crossed out. It now runs paused, removes the unrelated text decorations, and snaps to the selected timestamp. `nexa check` passes and both DevRuntime hosts applied the hot reload; human interaction verification remains pending.
- TextCases Android TextReveal update fix (2026-11-03): the preview-position buttons update screen state; Compose draws paused reveals directly from the current `position` parameter and uses Animatable only during playback. The first reinstall used a stale CLI binary, so it did not include the Compose change. Rebuilt the CLI with the same `/tmp/nexa-verification-target` used to launch it, confirmed the generated Android host contains the direct-position branch and the sample uses `isPlaying: false`, then reinstalled and relaunched both DevRuntime apps. Android accessibility state updates to 0.800000 s on selection. Human visual confirmation of the highlight remains pending.
- TextCases TextReveal Android DevRuntime constructor initialization fix (2026-11-03): the previous field-reader diagnosis was incomplete. Android evaluates initial app state before committing the incoming struct registry, so struct constructor calls lost their fields and the reveal timing data became invalid. Android now resolves struct field names from each constructor call's typed return type, which is available during initial installation. Both DevRuntime hosts rebuilt and relaunched; user-provided visual confirmation remains pending.
- TextCases NumberFlow parity case (2026-11-06): added reversible Add/Subtract €0.10 controls around a rolling EUR value starting at €42.99, covering carry and reverse digit rolls. The live iOS 27.0 and Android Ajami_Android12 DevRuntime hosts built, launched, and applied their first modules. Human verification was approved by the user on 2026-11-06; Agent Review approved.
- TextInputCases Agent Review (2026-11-10): reproduced Android DevRuntime focus jumping from the visible Task name input to an unrelated multiline field. Root cause: JSONObject.optString("focused") converted explicit JSON null into the shared string "null", binding every unbound field to one synthetic focus state. Android DevRuntime now treats absent/null focus bindings as unbound, with a runtime unit regression. Rebuilt DevRuntime hosts for iOS 27.0 and Android API 31; Android interactions verified Task name focus/value updates, clear without focus transfer, Done submission/keyboard dismissal, independent Search email focus, programmatic focus clearing, and interactive keyboard dismissal. Android AOT controlled-focus release and shared keyboard dismissal also received root fixes and regressions. Workspace tests, check, clippy, and generated iOS/Android host builds pass. Human visual and iOS interaction review remains pending; no screenshot image was inspected.
- TextInputCases Android clearable-height regression (2026-11-10): entering the first character made the row taller because its conditional Material IconButton introduced a 48 dp minimum touch target only after the value became nonempty. Android AOT and DevRuntime now reserve the same 48 dp clear-action slot while empty, with no hidden or disabled accessibility button. Rebuilt the CLI and both simulator DevRuntime hosts; iOS and Android clients reconnected. Agent Review reopened; Human review remains pending.
- TextInput rewrite (2026-11-10): shared native editor and edit policy for AOT/DevRuntime; direct scoped Boolean focus on iOS replaces the global focus map. iOS simulator interaction checks pass programmatic focus/blur, stable single-line height, minimum/maximum multiline height, newline rejection, deletion after rejection, and unbounded minimum-only input. The shared 28-case Unicode/edit corpus passes Swift and Android ART; 18 iOS native delegate/geometry checks pass. Android native accessibility confirms a fifth explicit line is rejected by the four-line field. maxLines limits explicit newline-separated lines; soft wrapping scrolls internally. Implementation and agent functional checks are complete; human route review remains pending. See docs/audits/text-input-2026-11-10.md. No screenshots were captured.
- Human verification approved (2026-11-11): the user approved TextInputCases on iOS and Android. Agent Review is approved; active route advances to SwitchCases.
- Human verification approved (2026-11-11): the user approved SwitchCases on iOS and Android after fixing the iOS white-on-white label color. Agent Review is approved; active route advances to SliderCases.
- Human verification approved (2026-11-11): the user approved SliderCases on iOS and Android after adding stepped slider indicators and continuous track-touch updates. Agent Review is approved; active route advances to SegmentedCases.
- Human verification approved (2026-11-11): the user approved SegmentedCases on iOS and Android. Agent Review is approved; active route advances to PickerCases.
- Human verification approved (2026-11-11): the user approved PickerCases on iOS and Android after keeping the label visible beside the menu and using primary text color. Agent Review is approved; active route advances to PickerDateCases.
- Human verification approved (2026-11-15): the user approved PickerDateCases on iOS and Android. Agent Review is approved; active route advances to FormCases.
- FormCases horizontal inset follow-up (2026-11-15): paired iOS 27.0 and Android API 31 DevRuntime measurements found Android section content at 48dp versus 32pt on iOS. The Compose Section surface added a 16dp inset before its existing 16dp row/header inset, on top of the sample Column padding. FORM_SECTION_HORIZONTAL_INSET is now 0dp, leaving the section content inset intact. Rebuilt both DevRuntime hosts from this checkout; screenshots and accessibility geometry show the row and heading starts aligned at about 32dp/pt. Focused Kotlin section tests pass; human review remains pending.
- Human verification approved (2026-11-15): the user approved FormCases on iOS and Android after confirming footer and section-label alignment. Agent Review is approved; active route advances to EmptyCases.
- Human verification approved (2026-11-15): the user approved EmptyCases on iOS and Android. Agent Review is approved; active route advances to ProgressBarCases.
- Human verification approved (2026-11-15): the user approved ProgressBarCases on iOS and Android. Agent Review is approved; active route advances to ProgressRingCases.
- Human verification approved (2026-11-15): the user approved ProgressRingCases on iOS and Android. Agent Review is approved; active route advances to RefreshCases.
- Human verification approved (2026-11-15): the user approved RefreshCases on iOS and Android. Agent Review is approved; active route advances to KeyboardCases.
- KeyboardCases test-route revision (2026-11-15): an added long Delivery notes editor places the final field below the keyboard's resting area. On iOS 27.0 its accessibility frame moves from y=0.727 to y=0.598 when focused, above the keyboard at y=0.637; Android API 31 DevRuntime visibly scrolls the field above the IME. Agent Review is in progress; human review is pending.
- KeyboardCases dismiss-button clipping follow-up (2026-11-15): the Done action was trailing content inside KeyboardAware's scroll viewport, so focusing the final field could leave it clipped at the viewport edge. It now sits beside the focus control outside the scrolling form. With the keyboard open, iOS accessibility reports the action fully at y=165pt and the focused field at y=572pt; Android reports the action in the top control area and the focused field above the IME. Human review remains pending.
- Human verification approved (2026-11-18): the user approved KeyboardCases parity on iOS and Android after the iOS focused-input avoidance and KeyboardToolbar joined the keyboard opening animation. Agent Review is approved; active route advances to IconCases.
- TextCases and IconCases follow-up (2026-11-18): the parity app now demonstrates discovered font weight fallbacks and system, prebuilt-font, and SVG-generated icons across component slots. Focused dual-target regressions pass; both route reviews remain in progress pending simulator and human verification.
- TextCases and IconCases DevRuntime follow-up (2026-11-20): both routes were opened on the iOS 27.0 simulator and Android API 31 emulator. Text font-weight fallback and custom system/font/SVG icon coverage pass focused tests; IconCases remains the active route. AOT simulator review and human approval remain pending.
- Human verification approved (2026-11-22): the user approved TextCases (TextRoute) on iOS and Android after the font-family algorithm follow-up. Agent Review is approved; the next unapproved route in App.nx order is FastListCases.

## Route review

Active route: **FastListCases**. Human approval also approves Agent Review; Agent Review can be completed before human approval.

| Route | Human review | Agent Review |
|---|---|---|
| ColumnCases | Approved | Approved |
| RowCases | Approved | Approved |
| StackCases | Approved | Approved |
| SpacerCases | Approved | Approved |
| TitleCases | Approved | Approved |
| TitleLeadingCases | Approved | Approved |
| TitleNoToolbarCases | Approved | Approved |
| DividerCases | Approved | Approved |
| TextCases | Approved | Approved |
| ButtonCases | Approved | Approved |
| PressableCases | Approved | Approved |
| TextInputCases | Approved | Approved |
| SwitchCases | Approved | Approved |
| SliderCases | Approved | Approved |
| SegmentedCases | Approved | Approved |
| PickerCases | Approved | Approved |
| PickerDateCases | Approved | Approved |
| FormCases | Approved | Approved |
| EmptyCases | Approved | Approved |
| ProgressBarCases | Approved | Approved |
| ProgressRingCases | Approved | Approved |
| FastListCases | Pending | In progress |
| ScrollViewCases | Pending | Pending |
| HapticCases | Pending | Pending |
| MarqueeCases | Pending | In progress |
| RefreshCases | Approved | Approved |
| KeyboardCases | Approved | Approved |
| IconCases | Pending | In progress |
| ImageCases | Pending | Pending |
| GradientCases | Pending | Pending |
| LinkCases | Pending | Pending |
| NavigationBackCases | Pending | Pending |
| ToolbarCases | Pending | Pending |
| PagerCases | Pending | Pending |
| BottomBarCases | Pending | Pending |
| SplitViewCases | Pending | Pending |
| SheetCases | Pending | Pending |
| DialogCases | Pending | Pending |
| ConfirmationCases | Pending | Pending |
| AppearanceCases | Pending | Pending |
| StatusBarCases | Pending | Pending |
| AccessibilityCases | Pending | Pending |
| LifecycleCases | Pending | Pending |
| DirectionCases | Pending | Pending |

## Records

- [Status fixture](tests/fixtures/component-parity-status.json): Agent Review notes, device evidence, and human approvals.
- [Parity app](examples/component-parity/App.nx): route and argument fixtures.
- [Report generator](tests/render_component_parity_report.py): regenerate after changing the fixture or route inventory.
