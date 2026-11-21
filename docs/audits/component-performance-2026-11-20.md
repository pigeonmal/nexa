**Nexa performance audit · 2026-11-20 · iOS and Android**

The audit starts with generated source across 50 catalog entries, standalone Release packaging for PagePager and PickerDate, and runtime measurements for Column, Row, Stack, Spacer, Divider, Shape, Button, Pressable, Switch, ProgressRing, ProgressBar, SegmentedControl, Slider, Picker, Link, Icon, Image, and ContentUnavailable. The remaining runtime and retained memory coverage is in progress.

```bash
CARGO_TARGET_DIR=/tmp/nexa-verification-target cargo build -p nexa-cli --bin nexa
/tmp/nexa-verification-target/debug/nexa audit examples/performance-audit/App.nx --target all --release-sizes --out /tmp/nexa-performance-audit.json
```

## Coverage

- [Component codegen measurements](component-performance-2026-11-20/component-codegen.json) include a minimal Text reference and one runnable fixture per catalog entry. Some fixtures require supporting components or assets. The numbers describe the whole fixture's generated source.
- [API and feature codegen measurements](component-performance-2026-11-20/api-codegen.json) cover eight existing examples, including background tasks, network status and caching, file access, clipboard, typed storage, crypto, and JSON.
- Track additional coverage against the typed IR, core APIs, plugins, and shared framework paths. Platform coverage includes Release AOT and DevRuntime. Compare each supported parameter that can materially change work, memory use, or packaging.
- Runtime comparisons need equivalent native fixtures and repeated workloads. Measure CPU, frame times, and process memory. Use a longer repeated workload or heap evidence to investigate suspected retained memory.

## Shared render effects

The Release fixture animates 120 tiles with explicit unit scale, full opacity, and zero rotation, offset, and blur. Text, layout, and scroll views now normalize effects before typed IR enters the AOT or hot reload paths. [Fixture and paired samples](component-performance-2026-11-20/render-effects-architecture.json).

- **iOS** skips the redundant native modifiers. The isolated nine-second comparison measured **1.76 s** median CPU after versus **1.80 s** before. Start RSS was **168.1 MiB** after and **170.3–172.2 MiB** before. End RSS varied across the baseline runs. Display-link p95 stayed near **16.7 ms**. The unsigned device Release app measured **270,910 B**.
- **Android** keeps its original layer chain. Removing every neutral modifier reduced recent frame cadence from **49–54 fps to 25–28 fps**. Retaining only the explicit opacity layer gave **39–43 fps** versus **39–47 fps** in the eight-second retest, with overlapping CPU samples. The adopted version generates the original component source.
- **Compiler** now uses one owning fold for nested layouts. It preserves explicitly arranged rows and marquee layouts, and moves flattened children without cloning their subtrees. The AOT and DevRuntime style fixture confirms the same typed body for both modes within each platform.
- **Hot reload** applied the same edit on both simulators, changing a tile from zero to nonzero rotation while the fixture was running.
- **Android sizing experiment** removed the inner fixed-width and fixed-height modifiers from the tile. CPU and cadence stayed close to the baseline, and the p95 frame duration varied. The adopted version keeps the existing sizing path.

## Compose measurement reads

Nexa now marks state-driven Shape dimensions that can be read during measurement. Android AOT and DevRuntime use the same native primitive and unboxed dimension readers. Literal sizes use native Spacer sizing. Dimensions that call app functions stay in composition, and animation targets are still installed there. [Fixtures, counters, and packaging](component-performance-2026-11-20/shape-measurement-architecture.json).

- **120-tile Release fixture** delivered about **30–31 fps** in recent app frame records, up from **21–25 fps**. Median process CPU over 14 seconds fell from **2.72 to 2.53 s**. P95 frame duration was **103–116 ms** after and **82–109 ms** before.
- **Composition** recorded **16,440** tile scope commits before and **zero** after warm-up with the measured path in the separate five-second probe. Each sampled time window delivered a different number of input updates. This probe measures where work ran.
- **Memory and packaging** showed lower PSS at the five-second sample, with overlapping end-of-run samples. APK size stayed **759,507 B**, and the AAB grew **9,858 B**. The shared primitive removed **29,685 B** of growth from the inline implementation.
- **Native verification** matched the Android Release and DevRuntime bounds for width, height, circle, explicit animation, and function-sized shapes. The hot reload fixture switched one shape from measured to fixed sizing and back while retaining the current state. Both simulators applied both edits. Generated Swift app and types source matched the baseline.

## Virtual masonry geometry

iOS FastList records measured card heights and groups equal-height estimates into runs. The layout computes offscreen positions on demand and rebuilds the affected suffix after a height change. Fixed and adaptive grids keep their native compositional layout. [Release fixture and counters](component-performance-2026-11-20/masonry-geometry.json).

- **12,000 cards at 600 pt/s** delivered about **55–56 display-link callbacks/s** versus **18–19 before**. Median CPU over nine seconds fell from **7.29 to 2.52 s**. Callback p95 fell from **117–133 ms to 16.7 ms**. Display-link records measure callback scheduling. Compositor presentation remains unverified.
- **End RSS** fell from **240–244 MiB** to about **212 MiB**. Model verification matched a sequential implementation across varied heights, column counts, fractional gaps, width changes, appends, and distant queries in a million-card list.
- **Packaging** grew by **43,632 B** in the Masonry fixture's simulator Release executable. The Swift generator includes this implementation only when an AOT app uses Masonry.
- **Native verification** exercised column placement, item 4,000 jumps, card resizing, appending, and returning to the top in iOS AOT and DevRuntime. Generated Android source matched the baseline.
- **Standalone UIKit list axes** now receive the shared scroll reference helpers. The old Masonry-only app needed those helpers to compile, so both versions of the runtime benchmark include them.

## Shared iOS image loads

Overlapping requests for one file or HTTPS URL share a transfer and ImageIO decode in iOS AOT and DevRuntime. Canceling the last waiter cancels the shared task. Completed images leave the loader, so later requests read the file again or use the existing URLSession cache policy. [Release fixture and counters](component-performance-2026-11-20/image-request-coalescing.json).

- **24 copies of one 4096 × 3072 JPEG** took a median **0.78 s → 0.18 s** to load. Process CPU over that interval fell from **4.49 s to 0.19 s**, and the views received **one decoded object instead of 24**.
- **Peak process RSS** fell from a median **570 MiB to 254 MiB**. Resident memory 400 ms after the final load rose from **190 MiB to 195 MiB**. The simulator executable grew by **67,904 B**.
- **Native checks** matched the original JPEG and PNG thumbnails, including EXIF orientation and alpha. They also verified invalid and oversized files, off-main work, independent waiter cancellation, and retry after the last waiter cancels. Generated Android source matched the baseline.

## Display-sized iOS thumbnails

iOS file and HTTPS images in `Fit` mode use the authored height and SwiftUI display scale to choose a thumbnail resolution. ImageIO metadata accounts for orientation. Fill, parent-filling, unconstrained, and shared-element images retain the existing 2048-pixel longest-edge limit. Overlapping sizes still share one read or transfer. [Native display scale](https://developer.apple.com/documentation/swiftui/environmentvalues/displayscale), [Release samples](component-performance-2026-11-20/image-fit-decoding.json).

- **12 distinct JPEGs**, with maximum heights of 32, 65 and 95 points, used **144 MiB → 2.6 MiB** of decoded pixel storage. Baseline images had a 2048-pixel longest edge. The new images decoded to **128 × 96**, **260 × 195**, and **380 × 285** pixels.
- **Paired Release runs** used **2.26 s → 1.77 s** of median process CPU to load all images. Median load time fell from **458 ms to 301 ms**, and median peak process RSS fell from **685 MiB to 337 MiB**. RSS 400 ms after the final load was about **203 MiB** in both builds.
- **Packaging and behavior**. The simulator executable grew by **59,008 B**. Native tests verified both thumbnail and full-resolution pixels for PNG and rotated JPEG, cancellation, and a larger view arriving during a smaller decode without a second file read.

## iOS file image scheduling

The shared iOS image loader runs up to six local file reads and decodes concurrently, bounded by the active processor count. Pending requests share the same URL entry and preserve arrival order. Views that cancel before loading remove their requests from the queue. An active load releases its slot after native work exits. HTTPS continues through the shared URLSession. [Release samples](component-performance-2026-11-20/image-file-scheduling.json).

- **12 simultaneous covers** used **338 → 280 MiB** median peak RSS. Median process CPU was **1.71 → 1.66 s**, and all images loaded in **293 → 298 ms**.
- **Five batch replacements 50 ms apart** used **415 → 287 MiB** median peak RSS and **4.47 → 3.14 s** of process CPU. The final batch loaded in **328 → 281 ms** after its URLs changed.
- **Default selection** balanced the memory and completion-time results. Four slots brought the steady fixture's peak to about **253 MiB** in the prototype, with median completion time of **379 ms**. Six slots preserved more throughput. The Release executable grew by **3,264 B**.
- **Native checks** covered FIFO order, independent HTTPS dispatch, coalescing while queued, canceling pending and active loads, and holding the slot until a canceled task returns. Both DevRuntime hosts loaded replacement covers and applied a live edit. The iOS test also verified the selected album and replacement image after reloading.

## Android FastList observation lifetime

The generated end observer stops after delivery and rearms when the item count changes. DevRuntime skips viewport observation for lists without listeners and drops end detection once it is delivered. Position binding and scroll callbacks continue to observe the leading item. Both modes use the same end-visibility test for padded viewports, grids, masonry, and sticky headers. [Release samples](component-performance-2026-11-20/android-fastlist-observation.json).

- **5,000 rows over eight seconds**, after the first end callback, went from **239–487** viewport checks to **zero**. Each of the eight Release runs delivered the callback once. The signed APK stayed at **956,543 B**.
- **Process timings** varied across emulator runs. CPU ranged from **1.09–2.56 s before** and **1.86–2.57 s after**. Recent frame cadence ranged from approximately **27–60** to **31–60 fps**. The measurements establish eliminated checks and preserved delivery.
- **Native behavior** covered sticky headers, scroll-position binding, grid, masonry, and snap in Android AOT and DevRuntime. Matching item counts in separate `if` and `when` branches now keep independent delivery markers. Hot reload retained the current branch and its delivered count. The iOS simulator also passed the sticky-header and count-growth scenario.

## iOS window metrics

Release apps that use native layout without reading window metrics omit the UIKit probe and its state updates. Apps with viewport or inset expressions, safe-area inset content, or status bar styling retain the probe. [Release samples](component-performance-2026-11-20/ios-window-root.json).

- **An isolated eight-second FastList scroll** used **1.01–1.24 s** of process CPU before and **1.22–1.24 s** after. The preliminary batch showed a decrease during workspace verification, so the two batches disagree on CPU. Display-link p95 stayed near **16.7 ms**. Isolated end RSS was about **189–190 MiB** in all runs.
- **Executable size** fell from **387,176 B to 336,184 B** in the instrumented simulator fixture.
- **Native verification** confirmed that a navigation screen using viewport-relative width and height still laid out its content on iOS.
- **DevRuntime** now uses the same window probe as AOT. The iOS simulator rendered viewport-relative content before and after a live edit. The root and hosted marquee views dropped two unused window environment values.

## Android window metrics

Compose Release builds use the same reachability check as Swift to omit unused window metrics. DevRuntime retains the provider for live modules. [Release samples](component-performance-2026-11-20/android-window-root.json).

- **An eight-second grid scroll** used **1.54–1.59 s** of CPU before and **1.30–1.71 s** after. Recent frame cadence ranged from **32–33 fps before** to **28–36 fps after**. Both builds reached the same item, and PSS samples overlapped.
- **Packaging** removed **8,648 B** from uncompressed `classes.dex`. The signed APK stayed at **991,135 B**.
- **Native verification** built an icon-free navigation app with the core back-arrow dependency. Its viewport-sized label measured **469 × 54 px** in the Android Release accessibility tree.

## iOS FastList scroll observation

Release vertical lists query the leading row during scrolling only when a binding or callback needs its position. DevRuntime passes an optional position observer to the shared UIKit list primitives. End visibility remains independent in those primitives. [Release samples](component-performance-2026-11-20/ios-fastlist-observation.json).

- **An eight-second scroll in a 5,000-row list** produced **480 leading-row lookups before** and **zero after**. All four runs recorded 480 display-link intervals at roughly **60 callbacks per second**.
- **Process CPU** ranged from **0.74–1.11 s before** and **1.06–1.11 s after**. End RSS was about **193–194 MiB** in all runs.
- **DevRuntime** passed the iOS simulator test for end delivery. A second visit to the same end kept the count at one. Growing the list and visiting its new end delivered a second callback.
- **Callback scope** matches AOT in both DevRuntimes. A fixture with an outer `index = 42` and a row parameter named `index` delivered `42` to both the scroll and end actions on both simulators. iOS removed the position state that previously fed those actions, and Android stopped building a separate locals map for each scroll callback.

## Android image resource IDs

Android AOT links project image assets and remote placeholders through `R.drawable`. Names supplied outside the asset catalog and names from live modules still resolve at runtime. Both paths use the same Compose painter. [Release samples](component-performance-2026-11-20/android-drawable-ids.json).

- **5,000 cells over eight seconds** went from **84 startup lookups and 236 scrolling lookups to zero**. The baseline spent **6.4–7.4 ms** in scrolling lookups. Each run ended at the same grid item.
- **Recent frame records** stayed at about **60 fps**. Process CPU was **1.40–1.45 s before** and **1.22–1.46 s after**. End PSS was about **42.8 MiB before** and **42.5 MiB after**.
- **Packaging** kept the referenced drawable through its generated resource ID. Both builds removed the unused drawable. The signed Release APK measured **991,147 B before** and **991,135 B after**. Generated Swift output was identical.
- **Native verification** covered a staged asset, a separately supplied drawable, and a remote error placeholder in Android Release. Both simulators changed the displayed bundled image during hot reload.

## Android Form and confirmation actions

DevRuntime passes button presentation directly to the shared Compose renderer. A direct Form row, a section row, or a button in a confirmation dialog now uses its original module node, eliminating a JSON serialization and parse on each applicable render.

- **Native verification** opened a confirmation dialog from a plain row and a section row, activated a default action and an explicitly styled action, and kept the selected state through a live row-label edit.

## Android toolbar rendering

DevRuntime extracts toolbars and their remaining screen content in one pass. Compose retains the result for the current module and reuses unchanged nodes. Compact toolbar pickers copy only the path they modify, with no serialization or parsing.

- **Native verification** showed one copy of each toolbar control, delivered button actions and a picker selection, and preserved both values after a live toolbar change.

## Android marquee motion settings

Android marquees share system motion and accessibility listeners while any marquee is mounted. Each marquee still owns its animation, geometry, delay, and lifecycle. [Release fixture and counts](component-performance-2026-11-20/android-marquee-motion.json).

- **Eight marquees** registered **one animator-scale observer after the change**. The baseline registered eight. Removing all eight marquees unregistered the observer, and remounting registered it once again.
- **Motion settings** switched the shared state to stationary when animations were disabled, then back to animated when they were re-enabled. The test restored the emulator’s original setting afterward.
- **Instrumented packaging** grew by **668 B of uncompressed DEX** and **16 KiB in the signed APK**.

## PagePager

The Android template for PickerDate lacked a codegen section marker. PagePager inherited the date implementation and failed to compile when compiled by itself. The two components now resolve their helpers independently.

- Generated Android source fell from **113,490 B to 45,356 B**, a **60.0%** reduction for a one-page fixture.
- The fixed Android Release app measured **964,703 B APK** and **2,374,883 B AAB**. The original fixture failed to compile, so an APK size delta is unavailable.
- [Original output](component-performance-2026-11-20/pager-controlled-before.json) and [fixed output](component-performance-2026-11-20/pager-after.json) use the same checkout and fixture, with the section marker toggled.

## PickerDate

The Swift date picker had the same missing codegen marker. The Android date picker also relied on imports and the Compose icon dependency supplied by other components. Both platforms now include the graphical date picker in a standalone Release app.

- Android APK **1,534,643 B** and AAB **3,665,902 B**. These sizes include the app host, Material date picker, and dependencies. [Android Release report](component-performance-2026-11-20/date-verified.json).
- iOS executable **611,432 B** and unsigned `iphoneos` app bundle **612,190 B**. The SDK build measures packaging independently of runtime profiling. [iOS Release report](component-performance-2026-11-20/date-ios-after.json).
- The Todo fixture does not use PickerDate. Its generated Swift fell from **724,592 B to 703,071 B** after the date picker received its own codegen section. The Todo regression now checks the actual navigation modifier and verifies that the calendar is absent.

## Text and FastList workload

An Android 12 emulator ran three scroll cycles in a Release fixture with 10,000 virtualized Text rows. Whole-process CPU was **0.81–2.65 seconds** per cycle at 100 ticks per second, and PSS samples ranged from **23.9 to 27.1 MiB**. The workload includes FastList, Compose, and the host. [Raw cycle counters](component-performance-2026-11-20/android-text-baseline.json).

The emulator reported zero modern janky frames while its legacy counter reported 32–53 per cycle. Run time also varied from 2.3 to 6.3 seconds. This capture is inconclusive for FPS, leaks, or an optimization decision.

## Column parameter probes

A fixture with two text labels measured **21,823 B iOS** and **36,660 B Android** of generated source with default parameters. Explicit gap or alignment changed source by at most 217 B on either target. Edge insets added 5,472 B on iOS and 2,506 B on Android. Parent-relative width added 2,548 B on iOS and 2,499 B on Android. The exact inputs are in the [codegen measurements](component-performance-2026-11-20/component-codegen.json).

## Column runtime comparison

The Release layout stress fixture changes a child shape's width in each of 192 Columns on every frame callback. The control replaces the cell Columns with centered SwiftUI or Compose layout calls at the same spacing. Each comparison uses two runs per variant, in alternating order, with two seconds of warmup and five seconds of measurement. [Source and raw samples](component-performance-2026-11-20/column-native-comparison.json).

- **iOS native reference** used 1.28 s median process CPU per run. The original Nexa fixture used 1.45 s in the same comparison. Both had about 60 display link callbacks per second. Callback cadence describes main thread scheduling. A compositor trace would be needed to measure presented FPS.
- **iOS optimization** applies the width frame and environment reset only at a Column that receives the safe area fill request. In a separate paired run, median CPU went from **1.33 s to 1.11 s (16.5% less)**. Display link p95 remained 16.68 ms. End RSS rose by about 0.7 MiB and the simulator executable grew by 144 B. The optimized result and the native reference were captured in separate comparisons with different baseline conditions.
- **iOS geometry** reported 28 × 52 pt at intrinsic size and 200 × 52 pt at the safe area root under the same 200 × 160 pt proposal. The child probes verified the inherited trailing text alignment and that the fill request stopped at the parent Column.
- **Android** median CPU was 4.50 s for Nexa and 4.71 s for the direct Compose call. Completed frame records gave roughly 41 fps for Nexa and 20–30 fps for the reference, with large variation across the two native runs. Both APKs were **743,123 B**. The samples are too few and noisy to establish an Android performance advantage.

The short captures also serve as initial memory measurements. They leave long duration retention and leak diagnosis open.

## Row runtime comparison

A Release fixture updates the first shape width in each of 192 Rows on every app frame callback. In the native control, the same shapes and host use direct SwiftUI or Compose calls for the cell Rows. Each sample has two seconds of warmup and five seconds of measurement. [Fixture, package sizes, and raw samples](component-performance-2026-11-20/row-native-comparison.json).

- **iOS** reuses one `HStack` and applies the width frame and environment reset only for an explicit horizontal arrangement or safe area fill. Median process CPU fell from **1.414 s to 1.272 s (10.1%)**. The direct `HStack` control used **1.112 s** in the same comparison. All three held a display link p95 near 16.68 ms. The optimized simulator executable was 160 B smaller and median end RSS was 0.82 MiB lower than the original Nexa build.
- **iOS geometry** matched the original in 48 combinations of intrinsic or safe area fill, horizontal arrangement, vertical alignment, and layout direction. The check compared rasterized child positions in memory and verified inherited alignment and fill behavior.
- **Android** median CPU was 3.32 s for Nexa and 3.53 s for the direct Compose call. Retained frame records yielded roughly 35–39 fps and 38–39 fps, respectively. End PSS was 30.2 MiB and 28.6 MiB. Both APKs measured **743,119 B**. These samples leave Android runtime differences inconclusive.
- **Parameter coverage** includes generated source probes for every Row parameter, with border color and width tested together and Marquee as a separate animation case. Insets added about 5.4 KB of Swift source or 2.3 KB of Kotlin source. Marquee was the largest addition in this fixture at 18.5 KB of Swift and 14.0 KB of Kotlin.

The frame data describes simulator and emulator behavior. Five-second memory windows leave leak diagnosis open.

## Stack runtime comparison

The Release fixture changes the first shape width in each of 192 Stacks on every app frame callback. The control swaps only the cell layout call for a centered SwiftUI `ZStack` or Compose `Box`. Comparisons use two samples per variant, with two seconds of warmup and five seconds of measurement per sample. [Fixture and raw measurements](component-performance-2026-11-20/stack-native-comparison.json).

- **iOS** median process CPU was 1.590 s for Nexa, 1.605 s for direct `ZStack`, and 1.674 s for the candidate that skips inactive safe area modifiers. Display link p95 was about 16.68 ms for all three. The candidate matched the original in 36 combinations of alignment, layout direction, and safe area fill. Its timings did not justify adopting it. The simulator executable was 541,336 B for Nexa and 538,072 B for the control.
- **Android** median process CPU was 3.315 s for Nexa and 3.379 s for direct `Box`. Completed frame records gave 36–40 fps and 37–38 fps, respectively. Median end PSS was 29.6 MiB and 27.6 MiB. The short samples do not establish a clear performance difference.
- **Android packaging** measured 759,507 B APK for Nexa and 743,123 B for the control. Compressed entry payload increased by 648 B, including 698 B in compressed DEX. That moved the first native library from byte 557,056 to byte 573,440 at the next 16 KiB alignment boundary. The AAB increased by 1,441 B. The APK step depends on where the app's other bytes fall relative to that boundary.
- **Memory endurance** sampled both simulators every five seconds during 45 seconds of active width updates. After the initial drop, iOS RSS stayed near 172 MiB. Android PSS fluctuated between 27.1 and 32.9 MiB across the interval samples and ended at 30.6 MiB. The sampled workloads show no sustained growth. Object lifetime and longer sessions remain unverified.
- **Parameters** were measured for generated source across all 25 Stack parameters on each target. Insets produced the largest Swift increase at about 5.4 KB. Relative width and width constraints produced the largest Kotlin increase at about 2.5 KB. These deltas describe the generated source of the parameter probes.

Instruments rejected both Animation Hitches and Core Animation FPS on this simulator. The iOS samples report display link cadence. Presented FPS remains unverified.

## Spacer runtime comparison

The Release test repeats 192 fixed Spacers alongside shapes that change width each frame. Each pair uses the same host and children. The baseline stacks separate native width and height modifiers. The new lowering uses a single SwiftUI frame or Compose size. [Fixture and raw measurements](component-performance-2026-11-20/spacer-native-comparison.json).

- **iOS** median process CPU fell from **1.520 s to 1.371 s (9.8%)** in two samples per variant. Display link p95 stayed near 16.68 ms. The simulator executable shrank by 272 B and median end RSS fell by 0.92 MiB. Dimensions and placement matched in all 36 native geometry cases with varying parent constraints.
- **Android** median process CPU went from 3.530 s to 3.319 s, while the measured completed frame cadence dropped from about 40–42 fps to 36–38 fps. Two samples per variant are inconclusive for a speed claim. Both APKs were 743,123 B. The AAB grew by 101 B.
- **Memory** showed no sustained growth in 45 seconds of active updates with the new lowering. iOS RSS settled near 165 MiB. Android PSS varied from 27.1 to 30.1 MiB across interval samples. These are process measurements for the fixture. Object retention remains unverified.
- **Parameters** cover 25 combinations of omitted, fixed, parent-relative, viewport-relative, and fill dimensions across both targets. Two resolved dimensions share one native modifier. Flexible, fill, and parent-relative sizing keep their existing behavior.
- **DevRuntime** avoids the extra Swift view wrapper for directly sized spacers and skips the Android window metrics lookup when neither dimension uses the viewport. The minimal iOS and Android debug hosts compiled. The Android build also exposed, and verified the fix for, a missing focus extension import when the initial app omits TextInput.
- **Flexible Spacer follow-up** fixed an Android Release build failure caused by emitting weight inside a `Stack`. Only Row and Column owners supply weight. Single children of plain `if` and `when` branches inherit that owner, while layout and transition wrappers keep their own scope. AOT and DevRuntime rendered the standalone and nested Stack cases and reported identical relative gaps of 288 px for a conditional Row, 277 px for a conditional Column, and 227 px for a direct Row on the same emulator. The regression fixture produced an 828,827 B APK. The failing baseline produced no APK for that fixture.

## Divider runtime comparison

The Release fixture draws 192 two-point or two-dp horizontal dividers while their parent widths change each frame. Each comparison used two runs per variant, with two seconds of warmup and five seconds of measurements. [Fixture and raw measurements](component-performance-2026-11-20/divider-native-comparison.json).

- **Visible iOS divider** used 0.857 s median process CPU. A one-frame candidate with a visibility branch used 1.001 s. It matched all 90 native geometry and pixel cases and made the simulator executable 16,576 B smaller. The short CPU comparison did not justify adopting that rendering change. Display link p95 ranged from 16.68 to 18.16 ms across the runs. Presented FPS remains unverified.
- **Visible Android divider** used 3.077 s median process CPU. A Compose `Spacer` painted with the same modifier chain used 2.963 s. Completed frame records gave 33–41 fps and 36–39 fps respectively. Both APKs were 743,127 B. The candidate remained experimental. The [AndroidX Material divider source](https://raw.githubusercontent.com/androidx/androidx/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Divider.kt) uses a stroke, whereas the fixture compares two ways to fill the measured bounds.
- **Zero thickness** now skips native view construction on both targets and skips color resolution in both debug renderers. In Swift geometry checks, the original zero divider made a two-child column 100 × 36 pt. The fixed and omitted versions both measured 12 × 28 pt, including through the type-erased debug path. The zero-only AOT fixture removed 355 B of generated Swift and 493 B of Kotlin. It packaged at 180,864 B for the unsigned iOS app and 734,935 B for the Android APK.
- **Memory** showed no sustained growth in the original visible-divider fixture over 45 seconds. iOS RSS decreased from about 168 to 133 MiB while both simulators and workspace compilation shared the host. Android interval PSS ranged from 25.4 to 28.6 MiB and ended at 27.7 MiB. Whole-process memory samples do not establish object lifetime.
- **Parameters** covered 30 combinations of color format and thickness on both targets, including adaptive color, theme spacing, and fractions. Both debug hosts compiled a mixed fixture with zero and visible dividers.

## Shape runtime comparison

The Release fixture updates the widths of 192 rounded shapes on each app frame callback. Android was measured with two five-second samples per sizing path following two seconds of warmup. Swift AOT already emits a direct SwiftUI shape with a single frame, so its measurements serve as the native baseline. [Fixture and raw measurements](component-performance-2026-11-20/shape-native-comparison.json).

- **Android** combines width and height in one Compose size modifier for nonanimated shapes. Median process CPU fell from **3.362 s to 3.084 s (8.3%)** in this fixture. Completed frame records gave about 36 fps before and 36–38 fps after. The APK stayed at 743,123 B and the AAB shrank by 316 B. The animation layout still reads the native state in its layout phase.
- **iOS** measured 0.637 s median CPU for the native SwiftUI path with display link p95 near 16.69 ms. The simulator executable was 514,536 B. Hot reload now builds the same framed view when a Shape omits animation and uses one size value for the explicit animation trigger. The AOT timings do not measure this hot reload change.
- **Circle geometry** now matches for wide, tall and square frames. A 40 × 16 frame has a centered 16 × 16 circle on both platforms and retains its 40 × 16 layout size. The new Compose outline was checked using Compose 1.12.1 for both layout directions, and SwiftUI's native path bounds were measured on the simulator. Capsule reuses the native rounded outline that fills the frame. [AndroidX rounded shape source](https://raw.githubusercontent.com/androidx/androidx/androidx-main/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/shape/RoundedCornerShape.kt).
- **Memory** showed no sustained growth during 45 seconds of updates. iOS RSS stayed near 165 MiB. Android PSS ranged from 25.3 to 28.1 MiB on the interval samples. Process samples leave object retention and longer sessions unverified.
- **Parameters** covered 36 combinations of shape, literal or adaptive fill, fixed dimensions, expression-driven dimensions, tween, and spring. The rectangular Circle fix adds 841 B of generated Kotlin to the fixed Circle probe. Both hot reload hosts compiled with a mixed fixture.
- **Mixed Release fixture** measured 439,671 B for the unsigned iOS app and 896,235 B for the Android APK. The Android fixture includes a Button for resizing. The previous compiler failed to build it because the shared icon helper needed the Compose semantics extension import. The updated import gate now follows the features that use the helper.

## Button runtime comparison

The Release fixture alternates the labels of 40 visible small buttons on each app frame callback. It exercises repeated updates of each platform's native Button with Nexa's existing style. [Fixtures and raw measurements](component-performance-2026-11-20/button-native-comparison.json).

- **Android baseline** used **2.03–2.48 s** of process CPU per five-second run. Recent completed frame records showed **45–48 fps**, and the 20-second baseline sampled **31.2–39.7 MiB** PSS with no sustained rise. The APK was **903,995 B** and the AAB was **2,153,252 B**.
- **Android experiment** reused shape and padding values and skipped unused theme reads. Short runs used **2.51–2.68 s** of process CPU and produced **40–44 fps**. APK size stayed flat. Host builds overlapped the 20-second pair, limiting confidence in its CPU and frame results. The original primitive was restored.
- **iOS baseline** used **3.77 s** of process CPU over 15 seconds. RSS stayed near **186 MiB**. Display-link p95 was **16.68 ms** at approximately 60 callbacks per second. These samples measure callback cadence, with compositor FPS and object retention still unverified. The unsigned Release app was **456,918 B**.
- **Option coverage** exercised code generation for all four styles, explicit and omitted size and shape, tint, loading, disabled, an accessibility label, icon, and glass. Both debug hosts compiled during the experiment. Runtime tap and option transitions and a separate hand-written native comparison remain unmeasured.

## Pressable runtime baseline

The Release fixture alternates the child text in 40 Pressable controls for 12 seconds after warmup. The workload measures rendering updates without touch injection. [Fixture and raw measurements](component-performance-2026-11-20/pressable-native-baseline.json).

- **Android** used **8.39 s** of process CPU. Recent completed-frame records averaged **40.1 fps**. PSS decreased from **39.1 to 34.2 MiB**. The APK was **857,811 B** and the AAB was **2,066,561 B**.
- **iOS** used **1.67 s** of process CPU. Display-link p95 was **16.68 ms** and RSS stayed near **183 MiB**. The unsigned Release app was **322,236 B**. Compositor FPS and object retention remain unmeasured.
- **Hot reload** removes the unused interaction source for Android tap-only controls and skips empty action dispatch and unrequested haptic lookups. iOS evaluates the long-press duration only for controls with a long-press handler. The Release baseline does not quantify these debug-only changes. Gesture-heavy workloads and a before-and-after hot reload profile remain pending.

## Switch runtime baseline

Twelve visible switches share a Boolean that the driver alternates on every app frame callback. This measures rendering under continuous updates. [Fixture and raw measurements](component-performance-2026-11-20/switch-native-baseline.json).

- **Android** used **2.49 s** of CPU in 12 seconds and recently completed **33.7 fps**. Process PSS sampled **28.2–43.5 MiB** and ended at **31.4 MiB**. The APK was **904,935 B** and the AAB was **2,139,895 B**.
- **iOS** ran two eight-second samples per variant. The new path used **1.46–1.77 s** of CPU, within the **0.91–1.87 s** baseline range. Display-link p95 stayed near **16.7 ms** and RSS was **312–318 MiB** across all runs. The unsigned Release app was **272,714 B**.
- **State transitions** now use a single guarded write and optional callback in the shared Swift primitive. The label tap skips constructing a temporary binding, and DevRuntime skips the duplicate store read and empty action dispatch. The driver writes state directly, so these samples do not measure the cost of handling a tap or establish object retention.
- **Interaction** passed an iOS AOT XCTest that tapped the native switch and its label. Each tap invoked one callback, and the callback observed the updated state. Both DevRuntime hosts compiled. Android interaction and DevRuntime timing remain separate coverage.

## ProgressRing runtime comparison

The Release fixture alternates progress values across 36 visible determinate rings. iOS samples used two eight-second runs per version in before/after/after/before order. [Fixture and raw measurements](component-performance-2026-11-20/progress-ring-native-comparison.json).

- **iOS** now initializes repeating rotation and reads the Reduce Motion setting only when the spinner is mounted. Median process CPU for determinate updates was **2.42 s before and 2.43 s after**. Display-link p95 was **16.68 ms** in both variants. The unsigned Release app was **290,666 B**.
- **Android** used **2.09 s** of CPU over 12 seconds and recently completed **36.4 fps**. The APK was **759,507 B** and the AAB was **1,745,221 B**.
- **Memory** rose early in the short runs but showed no sustained rise over 45 seconds. Android PSS peaked at **49.0 MiB** and ended at **27.0 MiB**. iOS RSS declined from **176.4 to 93.4 MiB** over the longer run. Builds and the two simulators ran concurrently during those samples, so use them for memory trend only. Object-level retention remains unverified.

## ProgressBar runtime comparison

The Release fixture alternates progress values across 24 default determinate bars. [Fixture and raw measurements](component-performance-2026-11-20/progress-bar-native-comparison.json).

- **iOS** keeps custom track state out of the native determinate view. Median CPU over eight seconds was **1.398 s before and 1.404 s after**. Each version delivered about **60 display-link callbacks/s**. RSS stayed within roughly **177–182 MiB** across the short runs. The unsigned Release app was **301,862 B**.
- **Android** used **1.30 s** of process CPU in 12 seconds. Recent completed frames ran at about **28.3 fps** on the emulator. The APK was **759,503 B** and the AAB was **1,744,151 B**.
- **Android memory** rose to **47.0 MiB** PSS and ended at **28.8 MiB** in the 45-second run, close to the **26.3 MiB** start. The host was building concurrently. Heap retention remains unverified.

## SegmentedControl runtime comparison

Twelve controls alternate between two of three choices on each app frame. Both backends share these primitives between AOT and hot reload. [Fixture and raw measurements](component-performance-2026-11-20/segmented-control-native-comparison.json).

- **iOS** reuses installed UIKit titles and skips repeated default styling. Median CPU was **1.391 s before and 1.285 s after**, about **7.6% less** in the eight-second fixture. All four runs delivered about **60 display-link callbacks/s**, with RSS around **182–184 MiB**. The unsigned Release app was **301,361 B**.
- **Android** reuses the first selection lookup for indicator visibility. The APK stayed at **859,027 B**, and the AAB decreased by **63 B**. CPU and frame comparisons were inconclusive. Frame records were missing in the first runs and sparse on retry. PSS rose during the short runs, so heap retention remains open.
- **Interaction** passed an iOS XCTest that changed the selection, replaced the items, and checked the retained selection and new choices. Both hot reload hosts compiled with default and custom styling.

## Slider runtime baseline

The Release fixture alternates the value across 14 continuous sliders. [Fixture and raw measurements](component-performance-2026-11-20/slider-native-baseline.json).

- **Android** now selects the same Material 3 beta as DevRuntime when an app reaches Slider. The original standalone build selected an alpha version whose Slider overload was incompatible. The shared editing observer now mounts only when a callback is supplied and reads the current callback across recomposition.
- **iOS runtime** used **3.06 s** of CPU over 12 seconds. Display-link p95 was **16.68 ms**, and RSS stayed near **176 MiB**. The unsigned Release app was **300,438 B**. The iOS implementation is unchanged.
- **Android runtime** on Material 3 beta completed about **23–25 fps** in recent frame records. Short-run process CPU ranged from **0.58–1.39 s** before and **0.62–1.24 s** after over six seconds. Builds overlapped these runs, so the samples cannot establish a CPU speedup. The APK stayed at **808,663 B** and the AAB decreased by **1,131 B** to **1,906,435 B**.
- **Memory** increased from about **27 to 34 MiB** PSS in the short Android beta runs. A discarded alpha experiment returned near its starting PSS after 45 seconds. Long-run beta retention and object-level retention remain unverified.
- **Interaction** passed on Android DevRuntime. A native drag on the stepped slider with indicators hidden delivered exactly two editing transitions and ended with editing false. Both DevRuntime hosts compiled with the beta dependency. The native build fixture now includes continuous and stepped sliders.

## Picker runtime comparison

Twelve closed labeled pickers alternate their selected value on each app frame. This measures trigger rendering. Opening and choosing a menu item is covered separately. [Fixture and raw measurements](component-performance-2026-11-20/picker-native-comparison.json).

- **Android** caches the chevron path and stroke across draws. Median CPU fell from **1.384 to 1.246 s** over six seconds, while recent completed frame cadence fell from **28.7 to 27.4 fps**. Two samples per variant leave the net throughput effect inconclusive. The APK stayed at **992,883 B**, and the AAB grew by **1,644 B**.
- **iOS** uses the original array indices for option identity and icon lookup, avoiding the temporary enumerated array. Eight-second CPU ranges overlapped (**1.01–1.42 s before**, **1.30–1.37 s after**). Display-link p95 stayed near **16.69 ms** and RSS near **190 MiB**. The unsigned Release app was **405,481 B**.
- **Memory** peaked at **57.0 MiB** Android PSS and ended at **35.1 MiB** in 45 seconds, from **30.6 MiB** initially. Host builds overlapped this run. Heap retention remains unverified.
- **Interaction** passed an iOS XCTest that opened the labeled picker, verified the label stayed visible, and selected an icon-bearing option.

## Link runtime baseline

Forty visible links alternate between two child labels. The fixture measures rendering and leaves URL dispatch out of the timed workload. [Fixture and raw measurements](component-performance-2026-11-20/link-native-baseline.json).

- **Android** shares one intent launcher between AOT and hot reload and avoids creating a scheme set per hot reload activation. Generated Kotlin in the fixture decreased by **677 B**. The APK stayed at **857,811 B** and the AAB fell by **212 B** to **2,063,437 B**.
- **Android runtime** used **1.34–1.81 s** of CPU before and **1.50–1.84 s** after over six seconds. Recent completed frames ranged from **27–34 fps** across all runs. PSS fluctuated between **32.6 and 45.5 MiB**. These short samples leave performance and heap retention inconclusive.
- **iOS runtime** used **3.07 s** of CPU over 12 seconds, with display-link p95 at **16.68 ms** and RSS near **187 MiB**. The unsigned Release app was **297,178 B**. The component uses SwiftUI Link and keeps its typed label content.

## Icon runtime comparison

Twelve rows change the tint of system symbols, prebuilt Mazicon glyphs, SVG-generated Brand glyphs, and a custom button icon. This measures the composed screen and its dynamic color updates. [Fixture and raw measurements](component-performance-2026-11-20/icon-native-comparison.json).

- **iOS** reuses the standalone font icon renderer in component slots, reducing generated Swift by **645 B** in the fixture. Eight-second CPU samples overlapped (**1.53–2.42 s before** and **1.53–2.04 s after**). Display-link p95 remained near **16.68 ms** and RSS near **193 MiB**.
- **Android** skips conversion of the unused base code point for layered glyphs. Median process CPU was **1.203 s before and 1.174 s after** over six seconds. Recent completed-frame cadence ranged from **24–28 fps** across both variants. The APK stayed at **964,903 B** and the AAB was **2,249,254 B**. The small sample leaves the runtime effect inconclusive.
- **Font assets** were present in the simulator app and APK. The SVG-generated Brand font was **1,000 B** and the prebuilt Mazicon font was **59,832 B** before packaging compression.
- **Packaging and memory** measured **622,755 B** for the unsigned iOS app. Android PSS peaked at **48.3 MiB** and leveled near **32–33 MiB** in the 45-second run. Builds overlapped the endurance sample. Heap retention and visual quality were outside this measurement.

## Image runtime comparison

Thirty-six copies of a bundled PNG are displayed in stacks with changing opacity. This exercises asset rendering and parent compositing. [Fixture and raw measurements](component-performance-2026-11-20/image-native-comparison.json).

- **Android Release** dropped the benchmark drawable because Nexa resolves it by name. Generated keep rules now cover referenced image and placeholder names while leaving unused names eligible for shrinking. Gradle marked the sample reachable, and the rebuilt APK included its **991 B** processed PNG. APK size went from **793,587 B** with the missing drawable to **795,011 B** with the asset restored. [Android shrinker documentation](https://developer.android.com/topic/performance/app-optimization/customize-which-resources-to-keep).
- **Android runtime** with the image packaged recorded **1.19 s** of CPU over **13.77 s**, about **20.9 fps** in recent frame records, and **30.8–32.5 MiB** PSS. Host builds overlapped the sample. Earlier captures rendered a transparent fallback and cannot serve as an image performance baseline.
- **iOS** skips an unconstrained fit frame. Eight-second CPU ranges overlapped (**0.24–0.27 s before**, **0.23–0.28 s after**) and display-link p95 stayed near **16.68 ms**. RSS stayed near **175–176 MiB**, and the unsigned app was **314,372 B**.
- **Verification** covered referenced and unused drawable names, stale keep-rule removal, both hot reload builds, and iOS accessibility labels and fixed-container size. Android also remembers the transparent fallback painter per composition. Remote decode, network caching, and object retention remain open.

## ContentUnavailable runtime comparison

Six empty-state cards alternate titles on each app frame with a stable icon and description. [Fixture and raw measurements](component-performance-2026-11-20/content-unavailable-native-comparison.json).

- **Android** remembers the bold title style for the current theme typography. CPU was **0.95–1.59 s before** and **0.88–0.92 s after**. One baseline sample ran nearly a second longer than requested. Recent completed frames ranged from **19–28 fps before** and **27–30 fps after**, so the small sample cannot establish a reliable gain. The APK stayed at **827,135 B**, and the AAB decreased by **277 B**.
- **iOS** shares the title label between the native empty state and the older-iOS fallback. CPU ranges overlapped (**1.28–1.55 s before**, **1.31–1.50 s after**) and display-link p95 stayed near **16.68 ms**. The unsigned Release app was **409,492 B**. RSS ranged from **183 to 193 MiB**.
- **Android memory** rose from **28.1 MiB** to a **53.8 MiB** peak and fell to **35.0 MiB** by the end of the 35-second run. Build activity overlapped the capture. Heap retention remains unverified.
