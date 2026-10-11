| Metadata | Value |
|---|---|
| App | Nexa Component Parity |
| Targets | iOS SwiftUI and Android Jetpack Compose |
| Purpose | Focused visual and behavior checks for shared components |

This app gives each shared component or behavior its own route, with multiple parameter and state variants on that route. Use the same route and interaction state on both targets to find rendering or behavior drift.

## Quick start

```sh
cd examples/component-parity
nexa dev --all
```

## Coverage

| Area | Focused routes |
|---|---|
| Layout | Column, Row, Stack, Spacer, Divider |
| Text and controls | Text, Button, TextInput, Switch, Slider, SegmentedControl, Picker, DatePicker |
| Content and lists | Form/Section, ContentUnavailable, ProgressBar, ProgressRing, FastList, RefreshControl, KeyboardAware |
| Media and navigation | Icon, Image, LinearGradient, Link, NavigationBack, Toolbar, PagePager, AppBottomBar, NavigationSplitView |
| Presentation and environment | BottomSheet, Dialog, ConfirmationDialog, Appearance, StatusBar, accessibility semantics, screen lifecycle |
