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

Signature: `Column(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, animation:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `animation`

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

### `KeyboardAware`

Adjusts layout for the software keyboard

Signature: `KeyboardAware(dismiss:) { ... }` (parentheses optional)

Optional options: `dismiss`

Children: node block

Modifiers: none

Reference: components.md#keyboardaware

### `Row`

Horizontal layout container with optional spacing

Signature: `Row(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, animation:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `animation`

Children: node block

Modifiers: none

Reference: components.md#row

### `Stack`

Overlapping layout container

Signature: `Stack(spacing:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, borderColor:, borderWidth:, opacity:, animation:) { ... }` (parentheses optional)

Optional options: `spacing`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `animation`

Children: node block

Modifiers: none

Reference: components.md#stack

## Typography

### `Text`

Displays formatted text

Signature: `Text(value, color:, fontSize:, fontWeight:, lineLimit:, lineHeight:, letterSpacing:, selectable:)`

Optional options: `color`, `fontSize`, `fontWeight`, `lineLimit`, `lineHeight`, `letterSpacing`, `selectable`

Children: none

Modifiers: none

Reference: components.md#text

## Controls

### `Button`

Interactive button with click action handler

Signature: `Button(value, icon:, loading:, disabled:)` with optional trailing `{ ... }` actions

Optional options: `icon`, `loading`, `disabled`

Children: optional action block

Modifiers: none

Reference: components.md#button

### `Switch`

Boolean toggle with a label

Signature: `Switch(value, label)`

Required options: `value`, `label`

Children: none

Modifiers: none

Reference: components.md#switch

### `TextInput`

Text input control bound to mutable state

Signature: `TextInput(value, placeholder, keyboard:, secure:, multiline:, autocorrect:, capitalization:, focused:, maxLength:)` with optional trailing `{ ... }` actions

Required options: `value`, `placeholder`

Optional options: `keyboard`, `secure`, `multiline`, `autocorrect`, `capitalization`, `focused`, `maxLength`

Children: optional action block

Modifiers: none

Reference: components.md#textinput

## Interactivity

### `Pressable`

Pressable region with onPress and onLongPress actions

Signature: `Pressable(disabled:, haptic:) { ... }`

Optional options: `disabled`, `haptic`

Children: node block

Modifiers:

- `.onPress` (actions, required)
- `.onLongPress` (actions, optional)

Reference: components.md#pressable

## Media

### `Image`

Displays an asset or network image

Signature: `Image(asset:, url:, description, scale:, placeholder:)`

Required options: `description`

Optional options: `asset`, `url`, `scale`, `placeholder`

Exactly one of: `asset`, `url`

Children: none

Modifiers: none

Reference: components.md#image

## Navigation

### `Link`

Opens a URL in the system browser

Signature: `Link(url) { ... }`

Required options: `url`

Children: node block

Modifiers: none

Reference: components.md#link

### `NavigationBack`

Pops the navigation stack with an optional label

Signature: `NavigationBack(label:)`

Optional options: `label`

Children: none

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `NavigationLink`

Navigates to a declared screen destination

Signature: `NavigationLink(destination, when:) { ... }`

Required options: `destination`

Optional options: `when`

Children: node block

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `NavigationStack`

Navigation host rooted at a declared screen

Signature: `NavigationStack(root)`

Required options: `root`

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

Optional options: `axis`, `rowHeight`, `scrollPosition`

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

Children: `Tab` declarations

Modifiers: none

Reference: components.md#appbottombar

## Refresh

### `RefreshControl`

Pull-to-refresh wrapper with an onRefresh action

Signature: `RefreshControl(isRefreshing) { ... }`

Required options: `isRefreshing`

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

### `Accessibility`

Accessibility label, hint, and role wrapper

Signature: `Accessibility(label, hint:, role:) { ... }`

Required options: `label`

Optional options: `hint`, `role`

Children: node block

Modifiers: none

Reference: components.md#accessibility

## Overlays

### `BottomSheet`

Modal bottom sheet bound to a boolean binding

Signature: `BottomSheet(isPresented, partial:) { ... }`

Required options: `isPresented`

Optional options: `partial`

Children: node block

Modifiers: none

Reference: components.md#bottomsheet

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

