# Nexa Agent Review

> **In progress** · 0 reviewed · 1 in progress · 40 pending · iOS + Android · native AOT + DevRuntime

Review shared behavior and appearance across iOS and Android. Build success and sampled device checks do not establish exhaustive parity.

```bash
python3 tests/render_component_parity_report.py --check
```

## Current work

- Route-by-route parity review restarted on 2026-11-01. Prior human approvals are cleared. Existing route and device notes are historical context only and do not count as current validation.
- Active route: ColumnCases, the first route after Catalog in examples/component-parity/App.nx. Review routes in source order across iOS and Android AOT and DevRuntime.
- ColumnCases first-pass finding: fractional layout dimensions now use the same app-window viewport source as ScreenWidth/ScreenHeight on both platforms. Focused dual-target generator coverage and iOS/Android DevRuntime host compilation pass; device and visual parity remain pending.
- ColumnCases clipping finding (2026-11-02): fixed-height Android children were measured at the parent’s shorter maximum; Kotlin AOT and DevRuntime now preserve top-aligned overflow. Backend regressions pass and both DevRuntime hosts rebuilt/connected; visual confirmation remains pending.
- ColumnCases clip-effect follow-up (2026-11-02): Android Compose now applies effect clipping outside fixed sizing and transforms in AOT and DevRuntime, matching SwiftUI's final-output clip. Kotlin backend tests pass (199); both DevRuntime hosts rebuilt and clients applied the module. Human visual confirmation remains pending.
- Use finite parameter coverage: defaults, enum and Boolean alternatives, meaningful boundaries and optional states, and pairwise interactions. Record grammar- or semantically inexpressible cases as not applicable with their source constraint.

## Route review

Active route: **ColumnCases**. Agent Review and human approval are tracked separately.

| Route | Human review | Agent Review |
|---|---|---|
| ColumnCases | Pending | In progress |
| RowCases | Pending | Pending |
| StackCases | Pending | Pending |
| SpacerCases | Pending | Pending |
| TitleCases | Pending | Pending |
| TitleLeadingCases | Pending | Pending |
| TitleNoToolbarCases | Pending | Pending |
| DividerCases | Pending | Pending |
| TextCases | Pending | Pending |
| ButtonCases | Pending | Pending |
| PressableCases | Pending | Pending |
| TextInputCases | Pending | Pending |
| SwitchCases | Pending | Pending |
| SliderCases | Pending | Pending |
| SegmentedCases | Pending | Pending |
| PickerCases | Pending | Pending |
| PickerDateCases | Pending | Pending |
| FormCases | Pending | Pending |
| EmptyCases | Pending | Pending |
| ProgressBarCases | Pending | Pending |
| ProgressRingCases | Pending | Pending |
| FastListCases | Pending | Pending |
| RefreshCases | Pending | Pending |
| KeyboardCases | Pending | Pending |
| IconCases | Pending | Pending |
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
