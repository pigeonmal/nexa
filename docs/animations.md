# Nexa Animations

> [!NOTE]
> **Scope:** native state, layout, and conditional-view motion | **Targets:** SwiftUI and Jetpack Compose | **Execution:** AOT and DevRuntime

Nexa exposes a typed animation API that maps to native platform animation systems. Springs are intended for interactive motion; tweens provide a shared duration and easing curve when timing must be predictable.

**10-second start:** wrap the state write in `withAnimation` and choose a spring or timed curve.

```nx
app AnimatedProgress {
    state progress: Float64 = 0.2

    body {
        Column(spacing: 12, animation: Spring(response: 0.35, damping: 0.82)) {
            ProgressBar(progress: progress)
            Button("Advance") {
                withAnimation(Tween(duration: 0.24, easing: EaseOut)) {
                    progress = 0.8
                }
            }
        }
    }
}
```

## Animation options

| Option | Syntax | Defaults and limits | Use |
|---|---|---|---|
| Spring | `Spring` or `Spring(response: 0.35, damping: 0.82)` | Default response `0.5` seconds and damping ratio `0.825`; custom response and damping values must be positive | Interruptible state changes and layout motion |
| Tween | `Tween(duration: 0.24, delay: 0.08, easing: EaseOut)` or `Tween(0.24, EaseOut)` | Duration is seconds, rounded to the nearest millisecond, from `1 ms` through `60 s`; delay defaults to `0` and is limited to `0–60 s`; easing defaults to `EaseInOut` | Motion that must finish on a known timeline |
| Easing | `EaseIn`, `EaseOut`, `EaseInOut`, `Linear`, or `CubicBezier(x1, y1, x2, y2)` | Available in `Tween` and as a standalone preset; standalone presets use `300 ms`; Bézier x control points must be between `0` and `1` | Use standard curves or tune the curve to match a design |
| Conditional transition | `.transition(.fade)`, `.transition(.scale)`, `.transition(.slide(from: .bottom), animation: Spring(...))` | Slide edges are `.bottom`, `.left`, and `.right`; animation is optional and defaults to platform timing | Inserting and removing `if` / `when` content |
| Layout animation | `Column(animation: Spring)`; also `Row` and `Stack` | Animates the container's content-size changes | Expanding and collapsing layout content |

Tween easing values are `EaseIn`, `EaseOut`, `EaseInOut`, `Linear`, and a cubic Bézier curve. The named form requires `duration`; `delay` and `easing` are optional. The positional forms accept a duration and optional preset easing. The delay applies before the curve begins.

```nx
withAnimation(Tween(
    duration: 0.32,
    delay: 0.06,
    easing: CubicBezier(x1: 0.2, y1: 0.0, x2: 0.2, y2: 1.0)
)) {
    progress = 1.0
}
```

`CubicBezier(x1, y1, x2, y2)` is also accepted positionally. Nexa keeps the same control points and delay in SwiftUI and Compose, so AOT and DevRuntime use the same curve.

Standalone easing presets can be used in the same places as springs and tweens:

```nx
app AnimatedDetails {
    state detailsVisible: Bool = false

    body {
        Button("Show details") {
            detailsVisible = !detailsVisible
        }

        if detailsVisible {
            Text("Your saved preferences")
        }.transition(.slide(from: .bottom), animation: Spring(response: 0.35, damping: 0.82))
    }
}
```

`withAnimation(spec) { ... }` scopes the native animation to mutable `Float32` and `Float64` state assignments, including control flow around those assignments. Integer, Boolean, string, and collection updates are not accepted there because they do not map to the same interpolated value on both platforms. Conditional `if` and `when` views accept the same optional spec on `.transition(...)`, so apps can tune both entrance and exit motion. For collection-driven content-size changes, put `animation:` on its `Column`, `Row`, or `Stack`; use `.transition(...)` for conditional insertion and removal. This keeps the animation spec attached to the native view whose layout actually changes.

Segmented controls animate selection without extra state or callbacks. Android moves the selection pill with Material 3 Expressive's default spatial motion spec; pass `animation:` to tune its spring or use a timed curve. iOS keeps its native segmented picker and applies the requested spec to the selection transaction.

```nx
state range: String = "Day"
state ranges: Array<String> = ["Day", "Week", "Month"]

SegmentedControl(
    items: ranges,
    selected: range,
    animation: Spring(response: 0.35, damping: 0.82)
)
```

## Native behavior and performance

| Target | State animation | Segmented-control selection |
|---|---|---|
| iOS | SwiftUI `withAnimation` and `Animation` | Native segmented `Picker`; optional `animation:` parameter sets its value animation |
| Android | Compose animation state and `AnimationSpec` | The selection indicator uses `MotionScheme.expressive().defaultSpatialSpec()` by default; optional `animation:` supplies a shared spring, tween, delay, or custom cubic Bézier curve |

Generated apps animate with SwiftUI and Compose directly. There is no JavaScript animation loop or per-frame Nexa interpreter work. Android's segmented indicator reads its animated offset during layout, so changing the selection does not rebuild the control's text every frame. Its default follows the official Material 3 Expressive motion scheme, and DevRuntime uses the same primitive as AOT.

Scoped `withAnimation` updates on iOS follow the system Reduce Motion setting. Compose animations follow Android's system animator duration scale, including the segmented-control indicator.

Nexa uses native animation engines directly, with no frame-by-frame Nexa runtime work. It supports spring and timed curves, per-tween delays, custom cubic Bézier easing, conditional view transitions, and content-size animation for layout containers. Keyframe timelines and repeat-forever animations are not part of the API yet.

For component signatures and layout-style options, see the [component guide](components.md) and [syntax audit](syntax-audit.md).
