# Architecture Audit Notes

This file records generated-source cache schema changes that must invalidate
previous compiler output.

| **Purpose**: cache invalidation history | **Scope**: generated source and Dev IR schemas | **Audience**: compiler maintainers |

> The newest row is checked against `cache::CACHE_VERSION` by
> `cargo test -p nexa-cli --test cache_audit`. Bumping the constant without adding a
> row here fails that test, so this changelog cannot silently fall behind the compiler.
> The range is intentionally not contiguous: the version advances in a commit that
> changes cache-sensitive output, so intermediate numbers never existed.

## Quick verification

After changing cache-sensitive code generation, run the cache invalidation checks:

```bash
CARGO_TARGET_DIR=/tmp/nexa-verification-target cargo test -p nexa-cli --test dependency_cache
```

## Cache schema

| Generator Schema | Architectural Change / IR Lowering |
|---|---|
| `build-v285` | Uses a compact iOS keyboard detent so focused sheet content scrolls without expanding to full height. |
| `build-v284` | Animates keyboard-driven bottom-sheet detents and restores the prior detent after editing. |
| `build-v283` | Opens Android sheets at the large detent like iOS and fills the expanded sheet viewport. |
| `build-v282` | Aligns Compose title3 fields and bottom-sheet toolbar affordances with SwiftUI. |
| `build-v281` | Renders shared Android text fields without Material decoration or internal padding. |
| `build-v280` | Uses unpadded native Compose text fields so shared sheet inputs align with SwiftUI. |
| `build-v279` | Shares compact navigation-title typography with Theme and expands Compose text fields to their SwiftUI-proposed width. |
| `build-v278` | Positions an Android keyboard-expanded sheet below the system status bar. |
| `build-v277` | Lets Android bottom sheets fill the keyboard-reduced viewport while editing, matching iOS expansion. |
| `build-v276` | Expands Android modal sheets while the keyboard is visible and restores their partial detent after dismissal. |
| `build-v275` | Matches Android destination large-title behavior to iOS and keeps bottom-sheet toolbars, keyboard focus, and conditional grouped surfaces aligned. |
| `build-v274` | Matches Android navigation destination title scale and weight while retaining compact centered titles and grouped surfaces. |
| `build-v273` | Keeps Android navigation destination titles compact and centered while matching grouped screen surfaces. |
| `build-v272` | Extends Android grouped list and form surfaces behind matching navigation headers. |
| `build-v271` | Keeps Android navigation destination headers at the existing height while starting them below the status bar. |
| `build-v270` | Positions Android navigation-destination headers below the status bar to match iOS large-title placement. |
| `build-v269` | Matches Android destination title spacing to the expanded native iOS navigation header. |
| `build-v263` | Aligns SwiftUI native-list backgrounds, separators, and row insets with Android shared rows. |
| `build-v262` | Keeps Android large titles anchored and vertically centered while enabling scroll gestures only for overflowing content. |
| `build-v258` | Increases Android's collapsed app-tab title size to 17sp. |
| `build-v257` | Paints Android searchable-tab backgrounds behind keyboard inset padding. |
| `build-v256` | Increases the Android empty-state icon size from 55dp to 58dp. |
| `build-v255` | Places the Android Form top inset inside scrollable content to preserve the header. |
| `build-v254` | Sets the Android Form content top inset to align its first section with iOS. |
| `build-v253` | Aligns the Android Form content top inset with the native iOS Form margin. |
| `build-v252` | Adds the native-equivalent top content inset to Android Forms. |
| `build-v251` | Reduces shared Android large-title app-bar height to tighten title-to-content spacing. |
| `build-v250` | Uses one shared 28-point app-tab title size across SwiftUI and Compose. |
| `build-v249` | Sets iOS app-tab large-title font attributes for the native navigation bar. |
| `build-v248` | Reads optional native large-title text attributes before applying the iOS title font. |
| `build-v247` | Sets a compact native large-title font for iOS app-tab navigation bars. |
| `build-v246` | Emits explicit 28-point styling for inline large app-tab title text. |
| `build-v245` | Passes app-tab navigation titles as styled SwiftUI text on iOS 26. |
| `build-v239` | Uses modern top-bar placements for SwiftUI toolbar items. |
| `build-v231` | Detects Android native lists inside conditional tab content for grouped headers. |
| `build-v232` | Uses one shared Android large-title header size across screens. |
| `build-v233` | Applies one Android hairline color and thickness to automatic separators. |
| `build-v234` | Reduces the shared Android expanded large-title top padding. |
| `build-v235` | Reduces the shared Android expanded large-title height. |
| `build-v236` | Uses symmetrical horizontal insets for Android large-title toolbars. |
| `build-v237` | Gives iOS large titles their native full-width row alongside toolbar actions. |
| `build-v238` | Restores iOS inline large titles for alignment with native toolbar actions. |
| `build-v230` | Aligns Android native-list tab headers, surfaces, row spacing, and divider insets with iOS. |
| `build-v229` | Reduces the gap between Android Form large titles and their first section. |
| `build-v228` | Moves Android Form large titles closer to the status bar. |
| `build-v227` | Aligns Android Form row density, large-title placement, and divider insets with iOS. |
| `build-v226` | Renders Android grouped-form separators as consistent single-pixel hairlines. |
| `build-v225` | Prevents repeated Android back taps from popping the root screen during navigation transitions. |
| `build-v224` | Makes the label area of iOS switches toggle the setting while preserving the native switch control. |
| `build-v223` | Reduces the Android empty-state icon size from 60dp to 55dp. |
| `build-v222` | Renders Android toolbar navigation links as compact icon actions instead of full-width rows. |
| `build-v221` | Keeps Android large-title placement consistent across tabs and restores native large-title scrolling on iOS. |
| `build-v220` | Reduces Android large-title app bar height on tabs without toolbar actions. |
| `build-v219` | Collapses Android large navigation titles and centers them as tab content scrolls. |
| `build-v218` | Matches default Android Bordered button tint and disabled alpha to dynamic iOS system blue. |
| `build-v217` | Gives SwiftUI and Compose Form rows one shared 56-point/dp minimum height and removes native iOS vertical row insets. |
| `build-v216` | Sets Android control icons to 24dp and calibrates explicitly sized Material icons. |
| `build-v215` | Calibrates Android Material icon sizing against SwiftUI SF Symbols in controls and explicit icons. |
| `build-v214` | Aligns Android Material icon sizing with SwiftUI SF Symbols in controls and pickers. |
| `build-v213` | Aligns Android Settings geometry, shared icons, navigation cues, switches, footer, and Form row heights with SwiftUI defaults. |
| `build-v210` | Gives Android Form picker and switch labels the shared 17sp regular text metrics used by iOS defaults. |
| `build-v209` | Matches Android grouped Form section and separator colors to the native iOS dark and light palettes. |
| `build-v208` | Uses one shared icon-to-label gap for Android pickers and buttons, matching the Todo settings rows. |
| `build-v207` | Aligns Android grouped Form surfaces, section spacing, navigation links, and plain button colors and insets with iOS defaults. |
| `build-v206` | Removes Android Material button sizing that added a trailing inset to default pickers. |
| `build-v205` | Matches Android Form row minimum height and trailing picker inset to the native iOS Form defaults. |
| `build-v204` | Matches Android large-title bottom spacing to the native iOS navigation title before Form content. |
| `build-v203` | Replaces the Android Picker modal with an anchored dropdown. |
| `build-v202` | Adds the iOS-style double-chevron selection indicator to default Android pickers. |
| `build-v201` | Matches Android's default `FormSection` corner radius to the grouped iOS form appearance. |
| `build-v200` | Keeps Android edge-to-edge status-bar backdrop aligned with the active bottom-bar tab surface, including grouped `Form` tabs. |
| `build-v199` | Extends Android tab backgrounds through navigation titles when their content uses a grouped `Form` surface, in both AOT and DevRuntime. |
| `build-v198` | Aligns Android sheet titles and conditional toolbars with SwiftUI, keeps the sheet surface visible behind the toolbar, and lets row text fields leave room for trailing controls. |
| `build-v192` | Attaches Swift alerts to a live anchor and emits confirmation actions directly with native dialog styling. |
| `build-v190` | Matches Android large navigation title weight and scales the ContentUnavailable empty-state icon, typography, and insets to iOS. |
| `build-v189` | Renders Android confirmation-dialog actions as centered full-width text rows with separators to match the iOS action list. |
| `build-v188` | Maps Bordered buttons to matching outlined shapes, padding, tint, and disabled colors across SwiftUI and Compose. |
| `build-v186` | Sizes iOS horizontal FastList rows and emits sendable typed-throws Swift hot-reload row mappers compatible with SQLite’s error contract. |
| `build-v179` | Attaches SwiftUI toolbar items to their containing layout so sheet navigation actions remain visible. |
| `build-v178` | Keeps Android date picker calendar days aligned with local timestamps across timezone boundaries. |
| `build-v177` | Insets Android form rows and adds dividers between adjacent section controls to match native settings lists. |
| `build-v176` | Gives Android buttons compact default padding and single-line labels so horizontal action rows do not wrap a button vertically. |
| `build-v175` | Renders Android switches with their text labels in a full-width aligned control row. |
| `build-v174` | Gives Android virtualized lists full cross-axis width, row separators, and opaque resting content behind swipe actions. |
| `build-v173` | Combines leading and trailing Android toolbars into one aligned action row inside vertical layouts. |
| `build-v172` | Preserves circular geometry for icon-only circle buttons in Android Compose, including localized empty labels. |
| `build-v170` | Renders Android tab toolbar actions beside the navigation title and removes them from the screen body. |
| `build-v168` | Collects toolbar controls nested inside tab content and attaches them to the active iOS tab navigation bar. |
| `build-v167` | Attaches iOS tab titles, search, and toolbar actions to the tab container, and propagates the app tab tint to Android screen controls. |
| `build-v166` | Removes nested navigation stacks from tab content so route links can use the application navigation path. |
| `build-v165` | Vertically centers Compose page-indicator dots to match SwiftUI `HStack` alignment. |
| `build-v164` | Binds default iOS button and pager accents to dynamic system blue, matching the shared light and dark accent palette on Android. |
| `build-v163` | Aligns large-button geometry across iOS and Android and centers Compose pager indicators in a full-width layout. |
| `build-v162` | Standardizes onboarding visual defaults across DevRuntime hosts, uses the filled active-notification icon on both platforms, and fixes Swift lifecycle modifiers around conditional root views. |
| `build-v161` | Removes the legacy `@query` annotation system in favour of `Signal<T>` reactive primitives across the parser, IR, both AOT backends, and DevRuntime. |
| `build-v135` | Completes the Nexa v1 language and archetype set: new component grammar, state scopes, and generated-host metadata. |
| `build-v132` | Adds scoped imperative animation handles and their lowering to both AOT backends and DevRuntime. |
| `build-v131` | Adds typed foreground task launch/cancellation actions, native executor lowering, and task lifecycle cleanup in both AOT backends and DevRuntime. |
| `build-v130` | Adds typed optional array `first()` and `last()` utilities to both AOT backends and DevRuntime; advances the Dev IR format. |
| `build-v129` | Generates periodic OS background-task handlers and scheduling metadata for iOS and Android; advances the Dev IR format for task declarations. |
| `build-v128` | Adds typed screen orientation requests to both AOT backends and DevRuntime, with iOS scene geometry support and generated interface-orientation declarations. |
| `build-v127` | Preserves valid nullable values while decoding generic plugin returns in Android DevRuntime, including nullable elements in compound values. |
| `build-v125` | Adds configurable Pressable long-press timing to both native backends and DevRuntime, accepts `.onTap` as the tap-handler spelling, and advances the Dev IR format. |
| `build-v124` | Adds typed Pressable pinch scale callbacks to both native backends and DevRuntime, and advances the Dev IR format. |
| `build-v123` | Adds typed Pressable drag callbacks to both native backends and DevRuntime, and advances the Dev IR format. |
| `build-v122` | Adds typed app-private string storage to both native backends and DevRuntime, and emits iOS required-reason privacy metadata for generated UserDefaults and file-metadata APIs. |
| `build-v121` | Adds statically generated JSON codecs to AOT apps and typed JSON dispatch/codecs to both DevRuntime hosts. |
| `build-v120` | Decodes nullable generic Android plugin collection inputs before dispatch. |
| `build-v119` | Makes Android launcher orientation follow the user's current orientation policy at startup. |
| `build-v118` | Adds typed Haptics calls to both native backends and DevRuntime dispatch. |
| `build-v117` | Fixes recursive Kotlin DevRuntime decoding for optional generic plugin arguments. |
| `build-v116` | Adds the typed iOS and Android system text clipboard API and DevRuntime dispatch. |
| `build-v115` | Emits Swift and Kotlin read codecs for optional values nested inside compound generic plugin values; Kotlin tracks decode failure separately from a valid null value. |
| `build-v114` | Includes the SecureStorage native adapters in every DevRuntime host so the API remains available after hot reload. |
| `build-v113` | Adds typed core cryptographic helpers to both native backends and DevRuntime. |
| `build-v112` | Supports optional generic plugin input codecs and parses explicit type arguments on qualified service calls. |
| `build-v111` | Adds locale-aware `Number.formatCurrency` to the iOS and Android AOT backends and DevRuntime. |
| `build-v110` | Adds cross-platform generic plugin codecs for `Result` values with enum failures, including hot-reload value adapters. |
| `build-v109` | Generates keyboard autofill and return-key semantics, plus `Keyboard.dismiss()` support in release apps and DevRuntime; advances the Dev IR format. |
| `build-v108` | Adds optional-element array codecs to DevRuntime plugin adapters for methods, properties, events, component props, and component events. |
| `build-v107` | Adds typed conditional view transitions to the source IR, SwiftUI and Compose output, and both DevRuntime renderers; advances the Dev IR and development protocol versions. |
| `build-v104` | Adds Pressable double-tap actions to typed IR, both native backends, and both Dev renderers; advances the Dev IR format version. |
| `build-v103` | Returns success after rendering a matched Android DevRuntime plugin visual component, so the renderer no longer reports a rendered component as unsupported. |
| `build-v102` | Adds `Pair` and `Triple` support to generated plugin value codecs and the DevRuntime plugin bridge, and fixes tuple member access in async DevRuntime evaluation. |
| `build-v101` | Moves source accessibility annotations to optional named arguments on rendered built-ins, custom components, and native plugin components; updates typed lowering while retaining the existing accessibility IR node and Dev IR format. |
| `build-v100` | Adds static visual effects to typed layout and text styles, both native backends, and both Dev renderers; advances the Dev IR format version. |
| `build-v99` | Adds a native menu-style Picker and Dev IR/runtime rendering on both platforms; advances the development protocol and format versions. |
| `build-v98` | Adds native string-backed SegmentedControl lowering and Dev IR/rendering on both platforms; advances the development protocol and format versions. |
| `build-v97` | Adds typed native Dialog alerts and Dev IR/runtime rendering on both platforms; advances the development protocol and format versions. |
| `build-v96` | Adds native Float64 Slider and progress controls, extends both development runtimes with their state/expression handling, and advances Dev IR and protocol versions. |
| `build-v95` | Added typed arithmetic IR and native lowering for subtraction, multiplication, division, remainder, and unary negation; added string concatenation lowering and native collection `count`/`isEmpty` accessors; extended both dev interpreters and advanced the Dev IR and WebSocket protocol versions. |
| `build-v94` | Iterates Compose state collections through immutable snapshots. |
| `build-v93` | Binds Android runtime context before native plugin initialization. |
| `build-v92` | Includes development host File and Path helpers for hot reload. |
