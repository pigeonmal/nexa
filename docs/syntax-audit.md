# Syntax Audit

| **Scope**: accepted `.nx` component grammar | **Source**: compiler catalog | **Status**: generated and parse-checked |

This reference lists every built-in component's accepted arguments, child blocks, and event or style modifiers. The entries are generated from the parser catalog and checked by parser probes.

> Generated from `crates/nexa-syntax/src/catalog.rs` — do not edit by hand.
> Run `cargo test -p nexa-syntax` with `NEXA_UPDATE_SNAPSHOTS=1` to regenerate.
> Every entry mirrors a parser production: the catalog test suite parses
> each component probe, so an audit entry without a working probe fails.

Each component lists its canonical signature, argument requirements,
child-block model, trailing modifiers, and manual documentation anchor.
Required options are bare names; optional options carry a trailing colon.

## Quick start

This valid `.nx` app shows the basic component and state syntax covered by this catalog.

```nx
app SyntaxAuditQuickStart {
state savedItems: Int32 = 0

body {
Column(spacing: 8, padding: 16) {
Text("Saved items: \(savedItems)")
Button("Save an item") {
savedItems = savedItems + 1
}
}
}
}
```

Sections:

- [Layout](#layout)
- [Typography](#typography)
- [Controls](#controls)
- [Interactivity](#interactivity)
- [Media](#media)
- [Navigation](#navigation)
- [Lifecycle](#lifecycle)
- [Lists](#lists)
- [Tabs](#tabs)
- [Refresh](#refresh)
- [Theming](#theming)
- [Accessibility](#accessibility)
- [Overlays](#overlays)
- [Composition](#composition)

## Layout

### `Column`

Vertical layout container with optional spacing

Signature: `Column(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, scale:, rotation:, shadow:, blur:, clip:, zIndex:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#column

### `ContentUnavailable`

Shows a native empty or unavailable content state

Signature: `ContentUnavailable(title, icon, description, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `title`, `icon`, `description`

Optional options: `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#content-unavailable

### `Direction`

Layout direction override (LTR or RTL)

Signature: `Direction(value)`

Required options: `value`

Children: none

Modifiers: none

Reference: components.md#direction

### `Divider`

Separates content with a native divider

Signature: `Divider(color, thickness, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `color`, `thickness`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#divider

### `Form`

Native grouped settings form

Signature: `Form(accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#form

### `KeyboardAware`

Adjusts layout for the software keyboard

Signature: `KeyboardAware(dismiss:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `dismiss`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#keyboardaware

### `Row`

Horizontal layout container with optional spacing

Signature: `Row(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, scale:, rotation:, shadow:, blur:, clip:, zIndex:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#row

### `Section`

A titled or footnoted group inside a Form

Signature: `Section(title:, footer:, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `title`, `footer`, `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#section

### `Spacer`

Expands along the parent layout axis

Signature: `Spacer(accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#spacer

### `Stack`

Overlapping layout container

Signature: `Stack(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, scale:, rotation:, shadow:, blur:, clip:, zIndex:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#stack

## Typography

### `ContentUnavailable`

Shows a native empty or unavailable content state

Signature: `ContentUnavailable(title, icon, description, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `title`, `icon`, `description`

Optional options: `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#content-unavailable

### `Text`

Displays formatted text

Signature: `Text(value, comment:, color:, alignment:, fontSize:, fontWeight:, padding:, opacity:, scale:, rotation:, shadow:, blur:, clip:, zIndex:, lineLimit:, lineHeight:, letterSpacing:, strikethrough:, selectable:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Optional options: `comment`, `color`, `alignment`, `fontSize`, `fontWeight`, `padding`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `lineLimit`, `lineHeight`, `letterSpacing`, `strikethrough`, `selectable`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#text

## Controls

### `Button`

Interactive button with click action handler

Signature: `Button(value, comment:, icon:, loading:, disabled:, style:, size:, shape:, tint:, glass:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)` with optional trailing `{ ... }` actions

Optional options: `comment`, `icon`, `loading`, `disabled`, `style`, `size`, `shape`, `tint`, `glass`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: optional action block

Modifiers: none

Reference: components.md#button

### `DatePicker`

Presents the native date and optional time picker

Signature: `DatePicker(timestamp, hasTime, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `timestamp`, `hasTime`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#datepicker

### `Picker`

Selects one string option from a native menu

Signature: `Picker(items, selected, icon:, label:, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `items`, `selected`

Optional options: `icon`, `label`, `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#picker

### `ProgressBar`

Shows linear progress for a Float64 value

Signature: `ProgressBar(progress, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `progress`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#progressbar

### `ProgressRing`

Shows circular progress for a Float64 value

Signature: `ProgressRing(progress, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `progress`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#progressring

### `SegmentedControl`

Selects one string option from a compact segmented control

Signature: `SegmentedControl(items, selected, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `items`, `selected`

Optional options: `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#segmentedcontrol

### `Slider`

Adjusts a Float64 state value within a stepped range

Signature: `Slider(value, min, max, step, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `value`, `min`, `max`, `step`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#slider

### `Switch`

Boolean toggle with a label

Signature: `Switch(value, label, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `value`, `label`

Optional options: `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#switch

### `TextInput`

Text input control bound to mutable state

Signature: `TextInput(value, placeholder, comment:, keyboardType:, isSecure:, autofill:, returnKeyType:, multiline:, autocorrect:, capitalization:, focused:, maxLength:, font:, minLines:, maxLines:, searchable:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)` with optional trailing `{ ... }` actions

Required options: `value`, `placeholder`

Optional options: `comment`, `keyboardType`, `isSecure`, `autofill`, `returnKeyType`, `multiline`, `autocorrect`, `capitalization`, `focused`, `maxLength`, `font`, `minLines`, `maxLines`, `searchable`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: optional action block

Modifiers:

- `.onChange` (event actions, optional)

Reference: components.md#textinput

## Interactivity

### `Pressable`

Pressable region with tap, context-menu, drag, pinch, double-tap, and long-press actions

Signature: `Pressable(disabled:, haptic:, fillMaxSize:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Optional options: `disabled`, `haptic`, `fillMaxSize`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers:

- `.onTap` (actions, optional)
- `.onLongPress` (actions, optional)
- `.contextMenu` (nodes, optional)
- `.onDoubleTap` (actions, optional)
- `.onDrag` (event actions, optional)
- `.onPinch` (event actions, optional)

Reference: components.md#pressable

## Media

### `ContentUnavailable`

Shows a native empty or unavailable content state

Signature: `ContentUnavailable(title, icon, description, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `title`, `icon`, `description`

Optional options: `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#content-unavailable

### `Icon`

Displays one portable or platform-specific native system icon

Signature: `Icon(system:, sfsymbol:, materialsymbol:, description, size, tint, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `description`, `size`, `tint`

Optional options: `system`, `sfsymbol`, `materialsymbol`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Exactly one of: `system`, `sfsymbol`, `materialsymbol`

Children: none

Modifiers: none

Reference: components.md#icon

### `Image`

Displays a native image from an asset, URL, or local file

Signature: `Image(asset:, url:, file:, description, scale:, placeholder:, maxHeight:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `description`

Optional options: `asset`, `url`, `file`, `scale`, `placeholder`, `maxHeight`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Exactly one of: `asset`, `url`, `file`

Children: none

Modifiers:

- `.sharedElement` (no block, optional)

Reference: components.md#image

### `LinearGradient`

Draws a native linear color gradient

Signature: `LinearGradient(startColor, endColor, direction:, height:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `startColor`, `endColor`

Optional options: `direction`, `height`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#lineargradient

## Navigation

### `Link`

Opens a URL in the system browser

Signature: `Link(url, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `url`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#link

### `NavigationBack`

Pops the navigation stack with an optional label

Signature: `NavigationBack(label:, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Optional options: `label`, `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `NavigationLink`

Navigates to a declared screen destination

Signature: `NavigationLink(destination, when:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `destination`

Optional options: `when`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `NavigationSplitView`

Adaptive sidebar and detail navigation

Signature: `NavigationSplitView(detailVisible:) { Sidebar { ... } Detail { ... } }`

Required options: `detailVisible`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: sidebar and detail blocks

Modifiers: none

Reference: components.md#navigationsplitview

### `NavigationStack`

Navigation host rooted at a declared screen

Signature: `NavigationStack(root, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `root`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `PagePager`

Swipeable pages with a native page indicator, bound to a selected-page value

Signature: `PagePager(selected:) { Tab(..) ... }`

Required options: `selected`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: `Tab` declarations

Modifiers: none

Reference: components.md#pagepager

## Lifecycle

### `OnActive`

Lifecycle trigger executed when the app becomes active

Signature: `OnActive { ... }` actions

Children: action block

Modifiers: none

Reference: state-and-navigation.md#4-lifecycle-hooks

### `OnAppear`

Lifecycle trigger executed when the view appears

Signature: `OnAppear async { ... }` actions

Children: action block

Leading flags: `async`

Modifiers: none

Reference: state-and-navigation.md#4-lifecycle-hooks

### `OnBackground`

Lifecycle trigger executed when the app enters the background

Signature: `OnBackground { ... }` actions

Children: action block

Modifiers: none

Reference: state-and-navigation.md#4-lifecycle-hooks

### `OnDisappear`

Lifecycle trigger executed when the view disappears

Signature: `OnDisappear { ... }` actions

Children: action block

Modifiers: none

Reference: state-and-navigation.md#4-lifecycle-hooks

### `OnInactive`

Lifecycle trigger executed when the app becomes inactive

Signature: `OnInactive { ... }` actions

Children: action block

Modifiers: none

Reference: state-and-navigation.md#4-lifecycle-hooks

## Lists

### `FastList`

High-performance virtualized list view

Signature: `FastList(collection | count: | sections:, axis:, ...)` with `{ bindings in ... }` rows

Optional options: `axis`, `rowHeight`, `scrollPosition`, `reverseLayout`, `pageSnap`, `native`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Source forms: positional collection, `count`:, or `sections:` (exactly one); row-key option `key`.

Children: row bindings plus row body

Modifiers:

- `.onEndReached` (actions, optional)
- `.onScroll` (actions, optional)
- `.onMove` (event actions, optional)
- `.stickyHeader` (nodes, optional)
- `.sectionHeader` (nodes, optional)
- `.swipeActions` (nodes, optional)

Reference: components.md#fastlist

## Tabs

### `AppBottomBar`

Bottom tab bar bound to a selected-tab binding

Signature: `AppBottomBar(selected:) { Tab(..) ... }`

Required options: `selected`

Optional options: `tint`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: `Tab` declarations

Modifiers: none

Reference: components.md#appbottombar

### `Toolbar`

Places native actions in a navigation toolbar

Signature: `Toolbar(placement:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Optional options: `placement`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#toolbar

## Refresh

### `RefreshControl`

Pull-to-refresh wrapper with an onRefresh action

Signature: `RefreshControl(isRefreshing, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `isRefreshing`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers:

- `.onRefresh` (actions, required)

Reference: components.md#refreshcontrol

## Theming

### `Appearance`

Selects system, light, or dark appearance for its native content

Signature: `Appearance(mode) { ... }`

Required options: `mode`

Children: node block

Modifiers: none

Reference: components.md#appearance

### `StatusBar`

Status bar style, visibility, and background

Signature: `StatusBar(style:, hidden:, background:)`

Optional options: `style`, `hidden`, `background`

Children: none

Modifiers: none

Reference: components.md#statusbar

## Accessibility

### Component options

Pass accessibility options directly to a visual built-in, custom component, or qualified native plugin component.

Signature: `Component(..., accessibilityLabel: String, accessibilityHint: String, accessibilityValue: String, accessibilityRole: None|Button|Link|Header|Image)`

`accessibilityLabel` is required whenever any accessibility option is present. Labels, hints, and values accept typed `String` expressions; literal labels and hints must be non-empty. The options are optional and may be used with the component's normal children and modifiers.

Swift emits native accessibility modifiers. Android emits Compose semantics, including hint text and heading semantics for `Header`.

Reference: components.md#accessibility-options

## Overlays

### `BottomSheet`

Modal bottom sheet bound to a boolean binding

Signature: `BottomSheet(isPresented, partial:, largeOnly:, title:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `isPresented`

Optional options: `partial`, `largeOnly`, `title`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#bottomsheet

### `ConfirmationDialog`

Native confirmation dialog bound to a boolean binding

Signature: `ConfirmationDialog(isPresented, title, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `isPresented`, `title`

Optional options: `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#confirmationdialog

### `Dialog`

Native alert with button actions and optional single-line text input

Signature: `Dialog(isPresented, title, message, comment:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `isPresented`, `title`, `message`

Optional options: `comment`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#dialog

## Composition

### `Content`

Renders a custom component's content slot

Signature: `Content()`

Children: none

Modifiers: none

Reference: components.md#custom-components--content

## Vocabulary

Keywords: app, screen, component, state, let, fn, async, await, enum, struct, plugin, try, catch, case, return, if, else, while, for, in, break, continue

Types: String, Bool, Int8, Int16, Int32, Int64, UInt8, UInt16, UInt32, UInt64, Float32, Float64, Bytes, Result, Array, Map, Set, Regex, RegexMatch, Range, Pair, Triple, Void, Ok, Err

