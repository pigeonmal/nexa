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
| `build-v447` | Fixes SwiftUI range-picker label compilation and constrains inline Material 3 range calendars in scrolling parents while keeping dialogs platform-sized. |
| `build-v446` | Rebuilds PickerDate on native SwiftUI and Material 3 controls, removing forced full-screen dialog sizing. |
| `build-v445` | Bounds Material 3 calendar height for safe layout inside scrolling parents and compact dialogs. |
| `build-v444` | Rebuilds DatePicker output with native Material 3 controls and full-width compact calendar dialogs. |
| `build-v442` | Scales the compact Material 3 date calendar to its measured dialog width. |
| `build-v441` | Maps the shared PickerDate `Input` style to the native compact SwiftUI picker because `.field` is unavailable on iOS. |
| `build-v440` | Applies label hiding to the native SwiftUI DatePicker before selecting a style so the generated picker compiles on iOS. |
| `build-v439` | Uses Material 3's native `DatePickerDialog` for compact Android dates, retaining its standard sizing, touch targets, shape, and actions. |
| `build-v438` | Centers the compact Material date dialog at its native 360 dp width to preserve native calendar sizing and date-cell hit targets on standard phones. |
| `build-v437` | Insets Android's compact Material date dialog by 16 dp and scales its calendar to the measured width, keeping all seven columns visible without a full-bleed surface. |
| `build-v436` | Keeps Material 3 time input unfocused until the user opens it, avoiding keyboard-driven layout shifts when a date-picker screen first appears. |
| `build-v435` | Replaces Android's fixed-width Material 3 date dialog with a width-aware Material dialog so compact calendars fit narrow screens without clipping the final date column. |
| `build-v434` | Scales Android's compact Material 3 date dialog into the available screen insets so the final calendar column remains visible. |
| `build-v433` | Uses Material 3 `DatePickerDialog` for compact Android dates, preserving its 360 dp calendar, 48 dp date targets, mode toggle, scrim, and confirmation action. |
| `build-v432` | Gives Android's compact Material 3 calendar its full 360 dp width on standard phones, so all seven 48 dp date targets remain visible. |
| `build-v431` | Measures Android compact Material 3 date content at 360 dp inside the inset popup, preserving 48 dp touch cells and the full Saturday column. |
| `build-v430` | Uses Material 3's supported 40 dp compact date-cell size so all seven dates fit inside the inset Android popup. |
| `build-v429` | Measures the Android compact calendar inside its visible popup width and scales Material 3 through density, keeping the Saturday date cell fully round and interactive. |
| `build-v428` | Preserves the full 384 dp Android compact calendar measure through both popup content boxes before scaling, so Material 3 retains all seven date cells. |
| `build-v427` | Measures compact Android Material 3 calendar content at 384 dp, including its horizontal insets, before uniformly scaling it into the popup viewport. |
| `build-v426` | Measures compact Android calendar popups against the full display bounds so the Material 3 360 dp minimum width is not reduced by the visible-window inset. |
| `build-v425` | Gives compact Android calendars the full Material 3 360 dp minimum width on standard phones, scaling only when the screen itself is narrower. |
| `build-v424` | Accounts for the full Material 3 compact calendar inner and outer horizontal insets before uniformly scaling Android popup content. |
| `build-v423` | Includes Material 3’s horizontal calendar padding in Android compact popup measurement before scaling, preserving all seven full-width date cells. |
| `build-v422` | Renders Android compact date calendars at Material 3’s native 360 dp width and scales the complete popup card to the available inset width. |
| `build-v421` | Sizes Android compact date-picker touch cells to the popup’s available calendar width. |
| `build-v420` | Scales Material 3 calendars inside Android compact picker popups to prevent the selected Saturday pill from clipping at the popup edge. |
| `build-v419` | Scales Android Material 3 date-calendar density to fit constrained parent widths without losing the Saturday column. |
| `build-v418` | Uses a simple fill-width container for Android inline date calendars so narrow parents do not clip Saturday dates. |
| `build-v417` | Lets Android inline date calendars honor their parent width so inset containers do not clip the Saturday column. |
| `build-v416` | Measures the Android compact picker card to fit a 12 dp inset while keeping all calendar columns inside its viewport. |
| `build-v415` | Keeps Android compact picker windows screen-clipped so they reposition instead of being vertically cut off. |
| `build-v414` | Expands Android compact picker surfaces to the viewport so Material 3 date columns stay visible. |
| `build-v413` | Keeps Android Material 3 calendars within the measured parent width to prevent clipped date columns. |
| `build-v412` | Gives inline Android Material 3 calendars enough width for all seven date columns inside inset layouts. |
| `build-v405` | Derives the pending range anchor from shared start/end values so selection survives SwiftUI view reconstruction. |
| `build-v403` | Uses a single controlled SwiftUI month grid for date ranges, avoiding multi-date toggles while keeping shared start/end semantics. |
| `build-v394` | Gives narrow Android graphical date pickers enough centered width for all seven Material 3 cells, with 48 dp touch targets on wider layouts. |
| `build-v393` | Keeps all seven Android Material 3 date cells visible in narrow form rows while retaining 48 dp touch targets when the full calendar width is available. |
| `build-v392` | Keeps all seven Android Material 3 date cells visible in narrow form rows by matching the date cells' 40 dp visual size. |
| `build-v391` | Applies a custom Nexa date-picker tint to the Android Material 3 current-day text and outline as well as the selected day and range. |
| `build-v390` | Starts Android input-style date pickers in editable mode after the user opens them, while keeping the keyboard closed during screen entry and scrolling. |
| `build-v389` | Opens Android Material 3 input-mode date/time controls only after the compact trigger is tapped, preventing automatic keyboard focus on screen entry. |
| `build-v388` | Prevents Material 3 time-input focus from opening the Android keyboard when a Nexa date/time picker screen first appears. |
| `build-v387` | Uses a color-aware SwiftUI compact pill when explicit text or background colors are requested, preserving native date-picker interaction. |
| `build-v386` | Uses a filled Material 3 Expressive pill for Android compact date pickers and refreshes picker state when locale or time zone changes. |
| `build-v385` | Uses a filled Material 3 Expressive pill for Android compact date-picker controls and keeps native locale-aware date/time formatting. |
| `build-v384` | Adds shared PickerDate locale and IANA time-zone settings through AOT and DevRuntime generation. |
| `build-v383` | Constrains Android DateTime selections to exact minimum and maximum timestamps, including changes made in the time picker. |
| `build-v382` | Prevents Android graphical date/time pickers from writing unchanged initial values, preserves hidden time when a Date picker changes its day, and displays selected local range endpoints. |
| `build-v381` | Restores the iOS native Menu presentation with a SwiftUI Picker for icon-bearing choices, keeping the label and selected value in the trigger and option icons neutral. |
| `build-v380` | Keeps the iOS Picker label outside its trailing-anchored native SwiftUI selection popover, preserves whole-row activation, and leaves option icons neutral. |
| `build-v379` | Restores the shared SwiftUI Picker accent color and full-row menu hit target while keeping the label outside the native selection choices. |
| `build-v378` | Moves the labeled SwiftUI Picker label outside its selection list and anchors the selection menu to a compact trailing trigger. |
| `build-v377` | Keeps a labeled SwiftUI Picker's label outside the menu on the leading side while anchoring the native selection menu to a compact trailing trigger. |
| `build-v376` | Shows the labeled SwiftUI Picker label while its options are open, using native label visibility on iOS 18+ and a menu heading on older iOS. |
| `build-v375` | Unifies all iOS SegmentedControl styles through one UISegmentedControl, using native selection tint, title attributes, and control background while preserving the system capsule. |
| `build-v374` | Routes every iOS SegmentedControl style through one native `UISegmentedControl`; applies track, selected-pill, and text colors through UIKit without a parallel SwiftUI renderer. |
| `build-v373` | Uses native `UISegmentedControl.selectedSegmentTintColor` for tinted iOS segments because SwiftUI Picker `.tint()` does not recolor the iOS 26 selected pill; untinted iOS and Android paths are unchanged. |
| `build-v372` | Routes iOS tint-only segmented Pickers through SwiftUI `.tint()` so the system retains its native glass pill; Android output is unchanged. |
| `build-v371` | Restores Android's existing Nexa segmented-control renderer while retaining the native SwiftUI Picker and custom pill geometry on iOS. |
| `build-v367` | Adds shared tween delays and custom cubic Bézier easing to AOT and DevRuntime animation lowering; advances Dev IR and development protocol versions. |
| `build-v366` | Centers Android full-screen sheet content and the drag handle within their presentation slots. |
| `build-v365` | Keeps Android adaptive split-view content top-aligned and horizontally centered like the compact SwiftUI layout. |
| `build-v364` | Centers compact split-view content and each adaptive column consistently with the iOS layout. |
| `build-v363` | Enables vertical bounce on SwiftUI RefreshControl content so short content can trigger pull-to-refresh like Android. |
| `build-v362` | Accounts for the Android navigation-bar inset when centering partial and expanded sheet content. |
| `build-v361` | Uses Material 3's explicit enabled sheet states so partial sheets keep a half-expanded detent even with short content. |
| `build-v360` | Sizes and animates partial Android sheet content within the visible detent while preserving full-height expansion anchors. |
| `build-v359` | Centers partial Android sheet content inside the measured full-height sheet while keeping the keyboard-safe body content-sized. |
| `build-v358` | Keeps Android partial-sheet content full-height so Material 3 retains its half-expanded anchor for short content. |
| `build-v357` | Regenerates Swift status-bar backgrounds from the active window safe area so navigation chrome cannot hide the configured band. |
| `build-v356` | Estimates fixed-height UIKit list rows with the same per-row spacing as their measured height, so reverse-list initial positioning does not skip content. |
| `build-v355` | Reapplies reversed FastList's initial scroll position after UIKit receives its final viewport, keeping the first logical rows inside the list below the screen header. |
| `build-v354` | Disables UIKit's automatic safe-area inset for reversed FastLists so the first logical rows remain visible below Nexa's own header. |
| `build-v353` | Makes reverse FastList scrollPosition report the visually leading row on Android, matching iOS. |
| `build-v350` | Keeps nested Android appearance scopes content-sized and prevents them from mutating app-wide system bars. |
| `build-v349` | Keeps default alert actions plain while preserving explicit styles for buttons in dialogs and sheets. |
| `build-v348` | Maps two Android alert actions to Material 3 dismiss and confirm slots so they sit in one action row. |
| `build-v347` | Centers full-screen Android sheet content to match SwiftUI full-screen presentation layout. |
| `build-v346` | Uses the shared app background for Android sheets and centers their expanded content to match SwiftUI presentation layout. |
| `build-v345` | Launches Android external links without a package-visibility preflight that can incorrectly hide installed browsers. |
| `build-v344` | Bounds SwiftUI images with explicit `maxHeight` to the same clipped viewport used by Compose, preventing fill images from drawing beyond their requested height. |
| `build-v343` | Stops a horizontal child list at its own edge for the current drag, so one Android gesture cannot scroll the list and advance its containing page together. |
| `build-v342` | Lets nested horizontal FastLists own their drag while preserving page swipes outside the list, matching SwiftUI pager behavior. |
| `build-v341` | Matches Android native FastList row height to the measured iOS inset-grouped list height. |
| `build-v340` | Aligns Android native-list row content and separators with the iOS grouped-list insets, and top-aligns SwiftUI pager pages to match Compose. |
| `build-v339` | Keeps Android native FastList rows at the same minimum height as native iOS table rows. |
| `build-v338` | Exposes determinate progress values as accessible ranges on Android, matching iOS progress semantics. |
| `build-v337` | Gives Android segmented controls a 32 dp visual track with a 48 dp accessible tap target to match iOS control height. |
| `build-v336` | Sets the Android determinate progress track to a neutral theme color matching the iOS default. |
| `build-v335` | Makes Android segmented controls fill their container with equal-width choices and matches the iOS neutral track and selected surface. |
| `build-v334` | Generates Android pickers and context menus with Material 3 Expressive groups and per-item shapes; selects the API 24+ artifact only when those menus are used. |
| `build-v332` | Gives SwiftUI tab pages a full-height top-aligned content frame so their first content starts below the native title bar like Android. |
| `build-v331` | Routes tab titles and actions into the owning native app bar when tabs are nested inside a navigation destination. |
| `build-v330` | Uses native SwiftUI navigation titles and toolbars for tab roots, Material 3 top app bars for Android tab roots, and removes the extra first-form-section inset below those bars. |
| `build-v323` | Uses native SwiftUI navigation titles and Material 3 Expressive large app bars for generated screen navigation. |
| `build-v322` | Maps SwiftUI Stack springs to the same stiffness and damping as Compose so content-size animation timing matches. |
| `build-v321` | Replaces the title-bar implementation with a shared navigation-chrome primitive and scopes iOS scroll observation to the current content subtree. |
| `build-v320` | Applies SwiftUI content-size animations explicitly when measured Stack dimensions change, while respecting Reduce Motion in AOT and DevRuntime output. |
| `build-v319` | Animates only SwiftUI's measured Stack frame while keeping child layout final, so expansion reveals clipped content smoothly without moving or fading it. |
| `build-v318` | Keeps SwiftUI conditional content at full opacity while its animated Stack parent expands, matching Compose's default insertion behavior. |
| `build-v317` | Animates SwiftUI layout changes in one subtree transaction, so sibling Stack children expand together like Compose instead of serializing preference-driven size updates. |
| `build-v316` | Calls the generated navigation body with its required Compose `ColumnScope` receiver inside the header content box. |
| `build-v315` | Animates Swift measured content-size frame changes through a value-driven animation, so Stack expansion works reliably in AOT and DevRuntime output. |
| `build-v314` | Rebuilds Swift title and content-size primitives, matching Compose Material easing curves and preserving stable child content during Stack expansion. |
| `build-v313` | Regenerates the Swift title bar primitive after correcting the UIKit controller's title property collision. |
| `build-v312` | Uses a pre-concurrency AnimatableModifier conformance at the SwiftUI actor boundary while keeping measured-frame interpolation on the main actor. |
| `build-v311` | Isolates the SwiftUI animatable measured size frame on the main actor for Swift 6 concurrency while retaining smooth Stack expansion. |
| `build-v310` | Interpolates the measured SwiftUI content frame through an explicit animatable size pair while keeping changing child content free of inherited animation. |
| `build-v309` | Keeps SwiftUI child content from disabling the parent’s measured size transaction, so Stack and other content-size animations expand on iOS in AOT and DevRuntime output. |
| `build-v308` | Interpolates the SwiftUI size frame and clips its child during content-size animation, preventing content from vanishing or escaping while matching Compose expansion. |
| `build-v306` | Animates SwiftUI content size independently of its changing children and interpolating frame, matching Compose expansion behavior. |
| `build-v301` | Gives Swift page-snap rows the measured viewport height despite automatic cell sizing, so each feed item occupies one page. |
| `build-v300` | Makes Swift paged FastLists fill their available viewport, matching native full-screen paging behavior. |
| `build-v299` | Adds shared FastList spacing across AOT and DevRuntime and accounts for grid gaps in responsive AutoGrid column sizing. |
| `build-v298` | Expands Swift remote and asset fill images to their proposed cell bounds, matching Compose image sizing. |
| `build-v295` | Composes iOS Pressable tap, double-tap, and long-press actions exclusively so long presses do not fall through to tap callbacks. |
| `build-v292` | Emits per-Pressable Android gesture imports and removes Material 3 ripple overlays from bottom-sheet toolbar controls. |
| `build-v291` | Uses deployment-compatible system font constructors for cross-platform text font design. |
| `build-v290` | Adds cross-platform text font design in AOT, widgets, and DevRuntime rendering. |
| `build-v289` | Hides inactive Android bottom-sheet toolbar slots when conditional content removes their buttons. |
| `build-v288` | Adds shared italic and underline text styles across AOT, DevRuntime, and widget output. |
| `build-v287` | Draws Android status-bar backgrounds in the edge-to-edge Compose tree and removes the deprecated window color setter. |
| `build-v286` | Aligns Swift text line-height with Compose and refreshes generated navigation, icon, and pager output. |
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
| `build-v238` | Restores iOS inline large titles for alignment with native toolbar actions. |
| `build-v237` | Gives iOS large titles their native full-width row alongside toolbar actions. |
| `build-v236` | Uses symmetrical horizontal insets for Android large-title toolbars. |
| `build-v235` | Reduces the shared Android expanded large-title height. |
| `build-v234` | Reduces the shared Android expanded large-title top padding. |
| `build-v233` | Applies one Android hairline color and thickness to automatic separators. |
| `build-v232` | Uses one shared Android large-title header size across screens. |
| `build-v231` | Detects Android native lists inside conditional tab content for grouped headers. |
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
