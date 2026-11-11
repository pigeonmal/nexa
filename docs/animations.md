# Nexa Animations

> [!NOTE]
> **Scope:** native state, layout, and conditional-view motion | **Targets:** SwiftUI and Jetpack Compose | **Execution:** AOT and DevRuntime

Nexa exposes a typed animation API that maps to native platform animation systems. Springs are intended for interactive motion; tweens provide a shared duration and easing curve when timing must be predictable.

**10-second start:** wrap the state write in `withAnimation` and choose a spring or timed curve.

```nx
app AnimatedProgress {
    state progress: Float64 = 0.2

    body {
        Column(gap: 12, animation: Spring(response: 0.35, damping: 0.82)) {
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
| Conditional transition | `.transition(.fade)`, `.transition(.scale)`, `.transition(.slide(from: .bottom), animation: Spring(...))`, `.transition(.fadeSlide(from: .top), animation: Tween(...))` | Slide edges are `.bottom`, `.left`, and `.right`; `fadeSlide` combines opacity with a top-edge move; animation is optional and defaults to platform timing | Inserting and removing `if` / `when` content |
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

For changing onboarding copy or other short content, attach a combined fade and
vertical move directly to a `when` block. Each native backend animates the old
and new branch together; no timer or per-frame state is needed:

```nx
state page: Int32 = 0

when page {
    0: { Text("Listen to millions of songs") }
    1: { Text("Find music that fits your taste") }
    else: { Text("Share playlists with friends") }
}.transition(.fadeSlide(from: .top), animation: Tween(duration: 0.2, easing: Linear))
```

`scale` on `Row`, `Column`, `Stack`, and `Text` accepts a numeric state
expression. `scaleY` independently controls vertical scale and defaults to
`scale` when both are supplied, or `1` when it is used alone. A
`withAnimation` write animates either native transform on both targets, and
each tween can set a different stage for a short pulse:

```nx
app FollowArtistButton {
    state iconScale: Float64 = 1.0

    body {
        Column(scale: iconScale) {
            Icon(fontFamily: "Mazicon", glyph: 61696, description: "Follow artist", size: 21, tint: "#FFFFFF")
        }
        Button("Follow") {
            withAnimation(Tween(duration: 0.07, easing: Linear)) {
                iconScale = 0.5
            }
            Task.launch(executor: TaskExecutor.Main) {
                try {
                    await Time.sleep(milliseconds: 70)
                    withAnimation(Tween(duration: 0.15, easing: Linear)) {
                        iconScale = 1.2
                    }
                    await Time.sleep(milliseconds: 150)
                    withAnimation(Tween(duration: 0.1, easing: Linear)) {
                        iconScale = 1.0
                    }
                } catch {
                }
            }
        }
    }
}
```

SwiftUI applies `scaleEffect`; Compose reads the animated value inside a graphics
layer. DevRuntime uses the same state animation specification and transform.
Keep the transform on the small icon view so its layout and hit target do not
change during the pulse.

For a vertical squash, apply `scaleY` to the small icon container while leaving
its horizontal scale at `1`:

```nx
app ShuffleControl {
    state iconScaleY: Float64 = 1.0

    body {
        Pressable(accessibilityLabel: "Shuffle", accessibilityRole: Button) {
            Row(scaleY: iconScaleY) {
                Icon(system: "music", description: "", size: 18, tint: "#FFFFFF")
            }
        }
        .onTap {
            Task.launch(executor: TaskExecutor.Main) {
                try {
                    withAnimation(Tween(duration: 0.13, easing: EaseInOut)) {
                        iconScaleY = 0.0
                    }
                    await Time.sleep(milliseconds: 130)
                    withAnimation(Tween(duration: 0.13, easing: EaseInOut)) {
                        iconScaleY = 1.0
                    }
                } catch {
                }
            }
        }
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

## Marquee container motion

`Marquee` is a general display-content preset for `animation:`. A `Row` moves horizontally toward the logical leading edge (leftward in LTR, rightward in RTL); a `Column` moves upward. There is no `axis` argument. To move an overlay composition, place its `Stack` inside a Row or Column.

```nx
Row(width: Fill, height: 40, gap: 12,
    animation: Marquee(speed: 32, gap: 24, delay: 1, edgeFade: 16, playing: marqueePlaying)) {
    Text(songTitle, lineLimit: 1)
    Icon(system: "music", size: 20, tint: "#047DF6", description: "Music")
}

Column(height: 64, gap: 12, animation: Marquee(speed: 24, edgeFade: 12)) {
    Text("Discover independent artists")
    Text("Listen to new releases")
    Stack { Text("Explore live performances") }
}
```

Declare `marqueePlaying` as Boolean state and `songTitle` as String state in the owning screen. Speed is constant across content lengths. Travel distance is content length plus marquee gap; a cycle lasts distance divided by speed, with linear motion and no pause between cycles.

<!-- nexadoc:begin MARQUEE_PARAMETERS -->
| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `edgeFade` | nonnegative finite numeric literal | 0 | Alpha fade length at each viewport edge in points/dp; clamped to half the viewport; disabled for fitting content or accessibility-stationary motion |
| `speed` | positive finite numeric literal | 32 | Logical units per second; points on iOS and dp on Android |
| `gap` | nonnegative finite numeric literal | 24 | Logical distance between repeated renderings; independent of container gap |
| `delay` | nonnegative finite numeric literal | 1 | Seconds before first motion or after a content/layout restart; not between cycles |
| `playing` | Bool expression | true | Pauses and resumes the current position; lifecycle and accessibility restrictions still apply |
<!-- nexadoc:end MARQUEE_PARAMETERS -->

### Viewport and content

- Set a bounded width for Row or height for Column, either through its parent or explicitly. Content that fits, empty content, and an unbounded moving axis remain stationary.
- Dimensions, padding, margin, background, border, and clipping describe the stationary viewport. The source receives an unconstrained moving axis and a constrained cross axis.
- Container `gap` separates children; marquee `gap` separates repeated renderings. Marquee adds no implicit padding or margin.
- The preset supports display-only text, icons, images, shapes, conditionals, custom components, and a Stack inside the moving container. Controls, navigation, gestures, selectable text, scrolling containers, native surfaces, and nested marquees are compile errors, including through custom components.
- `Marquee` is not a timing curve: it cannot be used in `withAnimation`, transitions, NumberFlow, or directly on Stack. Spring and Tween retain their existing semantics.

### Edge fades

`edgeFade: 16` fades the source alpha over 16 logical units at **both** viewport ends: left/right for Row, top/bottom for Column. The center remains opaque, and each fade is clamped to half the viewport length. `0` disables the mask.

Fades appear only for overflowing content. They remain during the initial delay and explicit pause, but disappear when Reduce Motion, disabled system animations, or a screen reader makes the source stationary. RTL mirrors motion while retaining the same symmetric mask.

The mask affects moving content only; padding, background, borders, layout bounds, and accessibility remain unchanged. Changing only `edgeFade` through hot reload preserves the current phase and delay. Like speed and gap, it accepts a compile-time numeric literal.

iOS caches a viewport gradient layer; Android caches its brush and paint and composites the source into an isolated layer only while a fade is needed. Neither platform rebuilds the mask on animation ticks or mounts additional source views.

### Lifecycle and accessibility

`playing: false`, offscreen placement, and backgrounding pause the current phase and remaining initial delay. Resume continues from that position. Content/layout/configuration changes restart at the logical start with the initial delay. Compatible hot reload preserves native identity and phase.

Reduce Motion, disabled system animations, and an active screen reader display one stationary source at the logical start. Accessibility retains the full source content; repeated visuals add no accessibility elements.

### Native rendering

| Target | Rendering and ownership |
|---|---|
| iOS | One generic SwiftUI hosting controller retains its environment and source tree. A clipped CAReplicatorLayer repeats that source using native Core Animation translation. |
| Android | One Compose subtree is recorded in a graphics layer and drawn at two translated positions. Native animation progress is read during drawing; geometry changes go through a native flow instead of composition-phase state reads. |

One source is mounted even while two visual instances are drawn, so component state, lifecycle callbacks, and image requests are not duplicated. Resources belong to the mounted primitive and are released on disposal. Motion uses no per-frame Nexa interpreter updates or per-frame SwiftUI state writes. Measure frame performance on target hardware before claiming a refresh-rate guarantee.

## Native behavior and performance

| Target | State animation | Segmented-control selection |
|---|---|---|
| iOS | SwiftUI `withAnimation` and `Animation` | Native segmented `Picker`; optional `animation:` parameter sets its value animation |
| Android | Compose animation state and `AnimationSpec` | The selection indicator uses `MotionScheme.expressive().defaultSpatialSpec()` by default; optional `animation:` supplies a shared spring, tween, delay, or custom cubic Bézier curve |

Generated apps animate with SwiftUI and Compose directly. There is no JavaScript animation loop or per-frame Nexa interpreter work. Android's segmented indicator reads its animated offset during layout, so changing the selection does not rebuild the control's text every frame. Conditional `fadeSlide` uses SwiftUI transitions and Compose `AnimatedContent` enter/exit transitions; DevRuntime uses the matching native primitives.

Animated transforms on Compose `Row`, `Column`, `Stack`, and `Text` are read inside `graphicsLayer` update blocks. Per-frame scale and rotation changes update the native layer without invalidating the composable tree. Keep animated transform state on the smallest view that needs it.

`opacity:` and `offsetY:` also accept a `Float32` or `Float64` state. Nexa
reads animated alpha and vertical translation in the Compose graphics layer,
so a fade or slide does not recompose the surrounding screen on every frame.
On iOS these values map to native SwiftUI opacity and offset modifiers.

Scoped `withAnimation` updates on iOS follow the system Reduce Motion setting. Compose animations follow Android's system animator duration scale, including the segmented-control indicator.

Nexa uses native animation engines directly, with no frame-by-frame Nexa runtime work. It supports spring and timed curves, per-tween delays, custom cubic Bézier easing, conditional fade, slide, fade-slide, and scale transitions, and content-size animation for layout containers. General keyframe timelines and repeat-forever timing curves are not part of the API yet. `Marquee` supplies continuous container motion with its own lifecycle.

For component signatures and layout-style options, see the [component guide](components.md) and [syntax audit](syntax-audit.md).
