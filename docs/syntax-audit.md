# Syntax Audit

> Generated from `crates/nexa-syntax/src/catalog.rs` — do not edit by hand.
> Run `cargo test -p nexa-syntax` with `NEXA_UPDATE_SNAPSHOTS=1` to regenerate.
> Every entry mirrors a parser production: the catalog test suite parses
> each component probe, so an audit entry without a working probe fails.

Each component lists its canonical signature, argument requirements,
child-block model, trailing modifiers, and manual documentation anchor.
Required options are bare names; optional options carry a trailing colon.

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

Signature: `Column(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, scale:, rotation:, shadow:, blur:, clip:, zIndex:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#column

### `Direction`

Layout direction override (LTR or RTL)

Signature: `Direction(value)`

Required options: `value`

Children: none

Modifiers: none

Reference: components.md#direction

### `Divider`

Separates content with a native divider

Signature: `Divider(color, thickness, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `color`, `thickness`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#divider

### `KeyboardAware`

Adjusts layout for the software keyboard

Signature: `KeyboardAware(dismiss:, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `dismiss`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#keyboardaware

### `Row`

Horizontal layout container with optional spacing

Signature: `Row(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, scale:, rotation:, shadow:, blur:, clip:, zIndex:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#row

### `Spacer`

Expands along the parent layout axis

Signature: `Spacer(accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#spacer

### `Stack`

Overlapping layout container

Signature: `Stack(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, scale:, rotation:, shadow:, blur:, clip:, zIndex:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#stack

## Typography

### `Text`

Displays formatted text

Signature: `Text(value, color:, fontSize:, fontWeight:, padding:, opacity:, scale:, rotation:, shadow:, blur:, clip:, zIndex:, lineLimit:, lineHeight:, letterSpacing:, selectable:, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Optional options: `color`, `fontSize`, `fontWeight`, `padding`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `lineLimit`, `lineHeight`, `letterSpacing`, `selectable`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#text

## Controls

### `Button`

Interactive button with click action handler

Signature: `Button(value, icon:, loading:, disabled:, accessibilityLabel:, accessibilityHint:, accessibilityRole:)` with optional trailing `{ ... }` actions

Optional options: `icon`, `loading`, `disabled`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: optional action block

Modifiers: none

Reference: components.md#button

### `Picker`

Selects one string option from a native menu

Signature: `Picker(items, selected, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `items`, `selected`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#picker

### `ProgressBar`

Shows linear progress for a Float64 value

Signature: `ProgressBar(progress, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `progress`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#progressbar

### `ProgressRing`

Shows circular progress for a Float64 value

Signature: `ProgressRing(progress, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `progress`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#progressring

### `SegmentedControl`

Selects one string option from a compact segmented control

Signature: `SegmentedControl(items, selected, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `items`, `selected`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#segmentedcontrol

### `Slider`

Adjusts a Float64 state value within a stepped range

Signature: `Slider(value, min, max, step, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `value`, `min`, `max`, `step`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#slider

### `Switch`

Boolean toggle with a label

Signature: `Switch(value, label, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `value`, `label`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#switch

### `TextInput`

Text input control bound to mutable state

Signature: `TextInput(value, placeholder, keyboard:, secure:, multiline:, autocorrect:, capitalization:, focused:, maxLength:, accessibilityLabel:, accessibilityHint:, accessibilityRole:)` with optional trailing `{ ... }` actions

Required options: `value`, `placeholder`

Optional options: `keyboard`, `secure`, `multiline`, `autocorrect`, `capitalization`, `focused`, `maxLength`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: optional action block

Modifiers: none

Reference: components.md#textinput

## Interactivity

### `Pressable`

Pressable region with tap, double-tap, and long-press actions

Signature: `Pressable(disabled:, haptic:, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }`

Optional options: `disabled`, `haptic`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers:

- `.onPress` (actions, required)
- `.onLongPress` (actions, optional)
- `.onDoubleTap` (actions, optional)

Reference: components.md#pressable

## Media

### `Image`

Displays an asset or network image

Signature: `Image(asset:, url:, description, scale:, placeholder:, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `description`

Optional options: `asset`, `url`, `scale`, `placeholder`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Exactly one of: `asset`, `url`

Children: none

Modifiers: none

Reference: components.md#image

## Navigation

### `Link`

Opens a URL in the system browser

Signature: `Link(url, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }`

Required options: `url`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#link

### `NavigationBack`

Pops the navigation stack with an optional label

Signature: `NavigationBack(label:, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Optional options: `label`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `NavigationLink`

Navigates to a declared screen destination

Signature: `NavigationLink(destination, when:, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }`

Required options: `destination`

Optional options: `when`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `NavigationStack`

Navigation host rooted at a declared screen

Signature: `NavigationStack(root, accessibilityLabel:, accessibilityHint:, accessibilityRole:)`

Required options: `root`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#navigationstack--navigationlink

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

Optional options: `axis`, `rowHeight`, `scrollPosition`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Source forms: positional collection, `count`:, or `sections:` (exactly one); row-key option `key`.

Children: row bindings plus row body

Modifiers:

- `.onEndReached` (actions, optional)
- `.onScroll` (actions, optional)
- `.stickyHeader` (nodes, optional)
- `.sectionHeader` (nodes, optional)

Reference: components.md#fastlist

## Tabs

### `AppBottomBar`

Bottom tab bar bound to a selected-tab binding

Signature: `AppBottomBar(selected:) { Tab(..) ... }`

Required options: `selected`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: `Tab` declarations

Modifiers: none

Reference: components.md#appbottombar

## Refresh

### `RefreshControl`

Pull-to-refresh wrapper with an onRefresh action

Signature: `RefreshControl(isRefreshing, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }`

Required options: `isRefreshing`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers:

- `.onRefresh` (actions, required)

Reference: components.md#refreshcontrol

## Theming

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

Signature: `Component(..., accessibilityLabel: String, accessibilityHint: String, accessibilityRole: None|Button|Link|Header|Image)`

`accessibilityLabel` is required whenever any accessibility option is present. Labels and hints accept typed `String` expressions; literal values must be non-empty. The options are optional and may be used with the component's normal children and modifiers.

Swift emits native accessibility modifiers. Android emits Compose semantics, including hint text and heading semantics for `Header`.

Reference: components.md#accessibility-options

## Overlays

### `BottomSheet`

Modal bottom sheet bound to a boolean binding

Signature: `BottomSheet(isPresented, partial:, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }`

Required options: `isPresented`

Optional options: `partial`, `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#bottomsheet

### `Dialog`

Native modal alert with a boolean binding and action content

Signature: `Dialog(isPresented, title, message, accessibilityLabel:, accessibilityHint:, accessibilityRole:) { ... }`

Required options: `isPresented`, `title`, `message`

Optional options: `accessibilityLabel`, `accessibilityHint`, `accessibilityRole`

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

Types: String, Bool, Int8, Int16, Int32, Int64, UInt8, UInt16, UInt32, UInt64, Float32, Float64, Bytes, Result, Array, Map, Set, Pair, Triple, Void, Ok, Err

