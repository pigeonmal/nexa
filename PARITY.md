# Nexa Agent Review

> **Agent Review complete** · 41/41 routes · iOS + Android · native AOT + DevRuntime

Review shared behavior and appearance across iOS and Android. Build success and sampled device checks do not establish exhaustive parity.

```bash
python3 tests/render_component_parity_report.py --check
```

## Review summary

- Agent Review complete: 41/41 component routes checked across catalog parameters, paired iOS/Android AOT builds, DevRuntime mappings, and recorded device evidence. Human approval remains separate.
- Finite parameter rule: defaults, enum and Boolean alternatives, meaningful boundaries/optional states, and pairwise interactions; grammar- or semantically inexpressible cases are recorded as not applicable with their source constraint.

## Route review

All routes have completed Agent Review. Human approval is tracked separately.

| Route | Human review | Agent Review |
|---|---|---|
| ColumnCases | Approved | Reviewed |
| RowCases | Approved | Reviewed |
| StackCases | Approved | Reviewed |
| SpacerCases | Approved | Reviewed |
| TitleCases | Approved | Reviewed |
| TitleLeadingCases | Approved | Reviewed |
| TitleNoToolbarCases | Approved | Reviewed |
| DividerCases | Pending | Reviewed |
| TextCases | Pending | Reviewed |
| ButtonCases | Pending | Reviewed |
| PressableCases | Approved | Reviewed |
| TextInputCases | Pending | Reviewed |
| SwitchCases | Pending | Reviewed |
| SliderCases | Pending | Reviewed |
| SegmentedCases | Approved | Reviewed |
| PickerCases | Pending | Reviewed |
| PickerDateCases | Approved | Reviewed |
| FormCases | Approved | Reviewed |
| EmptyCases | Approved | Reviewed |
| ProgressBarCases | Approved | Reviewed |
| ProgressRingCases | Approved | Reviewed |
| FastListCases | Pending | Reviewed |
| RefreshCases | Pending | Reviewed |
| KeyboardCases | Pending | Reviewed |
| IconCases | Pending | Reviewed |
| ImageCases | Pending | Reviewed |
| GradientCases | Pending | Reviewed |
| LinkCases | Pending | Reviewed |
| NavigationBackCases | Pending | Reviewed |
| ToolbarCases | Pending | Reviewed |
| PagerCases | Pending | Reviewed |
| BottomBarCases | Pending | Reviewed |
| SplitViewCases | Pending | Reviewed |
| SheetCases | Pending | Reviewed |
| DialogCases | Pending | Reviewed |
| ConfirmationCases | Pending | Reviewed |
| AppearanceCases | Pending | Reviewed |
| StatusBarCases | Pending | Reviewed |
| AccessibilityCases | Pending | Reviewed |
| LifecycleCases | Pending | Reviewed |
| DirectionCases | Pending | Reviewed |

## Records

- [Status fixture](tests/fixtures/component-parity-status.json): Agent Review notes, device evidence, and human approvals.
- [Parity app](examples/component-parity/App.nx): route and argument fixtures.
- [Report generator](tests/render_component_parity_report.py): regenerate after changing the fixture or route inventory.
