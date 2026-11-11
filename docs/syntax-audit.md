# Syntax Audit

| **Scope**: accepted `.nx` component grammar | **Source**: compiler catalog | **Status**: generated and parse-checked |

This reference lists every built-in component's accepted arguments, child blocks, and event, interaction, or composition modifiers. Visual styling is expressed through component arguments. The entries are generated from the parser catalog and checked by parser probes.

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
Column(gap: 8, padding: 16) {
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

Vertical layout container with configurable gap, padding, margin, and native Marquee motion

Signature: `Column(gap:, margin:, alignment:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, cornerRadiusBottom:, borderColor:, borderWidth:, opacity:, scale:, scaleY:, rotation:, offsetY:, shadow:, blur:, clip:, zIndex:, glass:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `gap`, `margin`, `alignment`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `cornerRadiusBottom`, `borderColor`, `borderWidth`, `opacity`, `scale`, `scaleY`, `rotation`, `offsetY`, `shadow`, `blur`, `clip`, `zIndex`, `glass`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#column

### `ContentUnavailable`

Shows a native empty or unavailable content state

Signature: `ContentUnavailable(title, icon, description:, titleColor:, descriptionColor:, iconTint:, background:, comment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `title`, `icon`

Optional options: `description`, `titleColor`, `descriptionColor`, `iconTint`, `background`, `comment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

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

Signature: `Divider(color, thickness, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `color`, `thickness`

Optional options: `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#divider

### `Form`

Native settings form with optional background and configurable row dividers

Signature: `Form(showDividers:, background:, hideHorizontalScrollbar:, hideVerticalScrollbar:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `showDividers`, `background`, `hideHorizontalScrollbar`, `hideVerticalScrollbar`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#form

### `KeyboardAware`

Keeps content at the window edge by default, with an option to avoid the software keyboard

Signature: `KeyboardAware(dismiss:, behavior:, hideHorizontalScrollbar:, hideVerticalScrollbar:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `dismiss`, `behavior`, `hideHorizontalScrollbar`, `hideVerticalScrollbar`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#keyboardaware

### `Row`

Horizontal layout container with configurable gap, padding, margin, and native Marquee motion

Signature: `Row(gap:, margin:, alignment:, horizontalArrangement:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, cornerRadiusBottom:, borderColor:, borderWidth:, opacity:, scale:, scaleY:, rotation:, offsetY:, shadow:, blur:, clip:, zIndex:, glass:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `gap`, `margin`, `alignment`, `horizontalArrangement`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `cornerRadiusBottom`, `borderColor`, `borderWidth`, `opacity`, `scale`, `scaleY`, `rotation`, `offsetY`, `shadow`, `blur`, `clip`, `zIndex`, `glass`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#row

### `ScrollView`

Explicit native scroll container for content that does not fit in its viewport

Signature: `ScrollView(axis:, ref:, hideHorizontalScrollbar:, hideVerticalScrollbar:, margin:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, cornerRadiusBottom:, borderColor:, borderWidth:, opacity:, scale:, scaleY:, rotation:, offsetY:, shadow:, blur:, clip:, zIndex:, glass:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `axis`, `ref`, `hideHorizontalScrollbar`, `hideVerticalScrollbar`, `margin`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `cornerRadiusBottom`, `borderColor`, `borderWidth`, `opacity`, `scale`, `scaleY`, `rotation`, `offsetY`, `shadow`, `blur`, `clip`, `zIndex`, `glass`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#scrollview

### `Section`

A titled or footnoted group with optional background and divider override

Signature: `Section(title:, footer:, comment:, showDividers:, background:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `title`, `footer`, `comment`, `showDividers`, `background`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#section

### `Shape`

Draws a filled native shape at an explicit size

Signature: `Shape(shape, width, height, fill, animation:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `shape`, `width`, `height`, `fill`

Optional options: `animation`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#shape

### `Spacer`

Expands along the parent layout axis or creates a fixed gap

Signature: `Spacer(width:, height:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Optional options: `width`, `height`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#spacer

### `Stack`

Overlapping layout container

Signature: `Stack(alignment:, margin:, padding:, width:, height:, minWidth:, maxWidth:, minHeight:, maxHeight:, background:, cornerRadius:, cornerRadiusBottom:, borderColor:, borderWidth:, opacity:, scale:, scaleY:, rotation:, offsetY:, shadow:, blur:, clip:, zIndex:, glass:, animation:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }` (parentheses optional)

Optional options: `alignment`, `margin`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `cornerRadiusBottom`, `borderColor`, `borderWidth`, `opacity`, `scale`, `scaleY`, `rotation`, `offsetY`, `shadow`, `blur`, `clip`, `zIndex`, `glass`, `animation`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#stack

## Typography

### `ContentUnavailable`

Shows a native empty or unavailable content state

Signature: `ContentUnavailable(title, icon, description:, titleColor:, descriptionColor:, iconTint:, background:, comment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `title`, `icon`

Optional options: `description`, `titleColor`, `descriptionColor`, `iconTint`, `background`, `comment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#content-unavailable

### `NumberFlow`

Formats a number and rolls each changing digit natively

Signature: `NumberFlow(value, format:, currencyCode:, minimumFractionDigits:, maximumFractionDigits:, animation:, comment:, color:, alignment:, fontSize:, fontStyle:, fontDesign:, fontFamily:, fontWeight:, padding:, opacity:, scale:, scaleY:, rotation:, shadow:, blur:, clip:, zIndex:, lineLimit:, lineHeight:, letterSpacing:, strikethrough:, italic:, underline:, selectable:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `value`

Optional options: `format`, `currencyCode`, `minimumFractionDigits`, `maximumFractionDigits`, `animation`, `comment`, `color`, `alignment`, `fontSize`, `fontStyle`, `fontDesign`, `fontFamily`, `fontWeight`, `padding`, `opacity`, `scale`, `scaleY`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `lineLimit`, `lineHeight`, `letterSpacing`, `strikethrough`, `italic`, `underline`, `selectable`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#numberflow

### `Text`

Displays formatted text

Signature: `Text(value, comment:, color:, alignment:, fontSize:, fontStyle:, fontDesign:, fontFamily:, fontWeight:, padding:, opacity:, scale:, scaleY:, rotation:, shadow:, blur:, clip:, zIndex:, lineLimit:, lineHeight:, letterSpacing:, strikethrough:, italic:, underline:, selectable:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Optional options: `comment`, `color`, `alignment`, `fontSize`, `fontStyle`, `fontDesign`, `fontFamily`, `fontWeight`, `padding`, `opacity`, `scale`, `scaleY`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `lineLimit`, `lineHeight`, `letterSpacing`, `strikethrough`, `italic`, `underline`, `selectable`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#text

### `TextReveal`

Reveals timed text segments using native text layout and animation

Signature: `TextReveal(value, segments, position, isActive, isPlaying, revealEnabled, seekRevision, revealOpacity:, leadTime:, targetOffset:, smoothingDuration:, comment:, color:, alignment:, fontSize:, fontStyle:, fontDesign:, fontFamily:, fontWeight:, padding:, opacity:, scale:, scaleY:, rotation:, shadow:, blur:, clip:, zIndex:, lineLimit:, lineHeight:, letterSpacing:, strikethrough:, italic:, underline:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `segments`, `position`, `isActive`, `isPlaying`, `revealEnabled`, `seekRevision`

Optional options: `revealOpacity`, `leadTime`, `targetOffset`, `smoothingDuration`, `comment`, `color`, `alignment`, `fontSize`, `fontStyle`, `fontDesign`, `fontFamily`, `fontWeight`, `padding`, `opacity`, `scale`, `scaleY`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `lineLimit`, `lineHeight`, `letterSpacing`, `strikethrough`, `italic`, `underline`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#text-reveal

## Controls

### `Button`

Interactive button with click action handler

Signature: `Button(value, comment:, icon:, loading:, disabled:, style:, size:, shape:, tint:, glass:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)` with optional trailing `{ ... }` actions

Optional options: `comment`, `icon`, `loading`, `disabled`, `style`, `size`, `shape`, `tint`, `glass`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: optional action block

Modifiers: none

Reference: components.md#button

### `Picker`

Selects one string option from a native menu

Signature: `Picker(items, selected, itemIcons:, icon:, label:, tint:, comment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `items`, `selected`

Optional options: `itemIcons`, `icon`, `label`, `tint`, `comment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#picker

### `PickerDate`

Selects a date, time, date-time, or date range

Signature: `PickerDate(type, style:, selection:, timestamp:, startTimestamp:, endTimestamp:, minimum:, maximum:, tint:, textColor:, backgroundColor:, pickerBackgroundColor:, borderColor:, borderWidth:, cornerRadius:, locale:, timeZone:, is24Hour:, enabled:, accessibilityLabel:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `type`

Optional options: `style`, `selection`, `timestamp`, `startTimestamp`, `endTimestamp`, `minimum`, `maximum`, `tint`, `textColor`, `backgroundColor`, `pickerBackgroundColor`, `borderColor`, `borderWidth`, `cornerRadius`, `locale`, `timeZone`, `is24Hour`, `enabled`, `accessibilityLabel`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#pickerdate

### `ProgressBar`

Shows customizable determinate or indeterminate linear progress

Signature: `ProgressBar(progress:, tint:, trackColor:, thickness:, cornerRadius:, androidWave:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Optional options: `progress`, `tint`, `trackColor`, `thickness`, `cornerRadius`, `androidWave`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#progressbar

### `ProgressRing`

Shows customizable determinate or indeterminate circular progress

Signature: `ProgressRing(progress:, tint:, trackColor:, size:, thickness:, androidWave:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Optional options: `progress`, `tint`, `trackColor`, `size`, `thickness`, `androidWave`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#progressring

### `SegmentedControl`

Selects one string option with customizable color and shape

Signature: `SegmentedControl(items, selected, animation:, comment:, tint:, background:, selectedTextColor:, textColor:, fontSize:, fontWeight:, height:, cornerRadius:, borderColor:, borderWidth:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `items`, `selected`

Optional options: `animation`, `comment`, `tint`, `background`, `selectedTextColor`, `textColor`, `fontSize`, `fontWeight`, `height`, `cornerRadius`, `borderColor`, `borderWidth`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#segmentedcontrol

### `Slider`

Adjusts a Float64 state value within a native, optionally stepped range

Signature: `Slider(value, min, max, step:, showStepIndicators:, tint:, trackColor:, thumbColor:, trackHeight:, thumbSize:, hitSlop:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `value`, `min`, `max`

Optional options: `step`, `showStepIndicators`, `tint`, `trackColor`, `thumbColor`, `trackHeight`, `thumbSize`, `hitSlop`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers:

- `.onChange` (event actions, optional)
- `.onEditingChanged` (event actions, optional)

Reference: components.md#slider

### `Switch`

Boolean toggle with a label and optional change action

Signature: `Switch(value, label, comment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `value`, `label`

Optional options: `comment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers:

- `.onChange` (event actions, optional)

Reference: components.md#switch

### `TextInput`

Text input control bound to mutable state

Signature: `TextInput(value, placeholder, comment:, keyboardType:, isSecure:, autofill:, returnKeyType:, multiline:, autocorrect:, capitalization:, focused:, ref:, maxLength:, font:, fontSize:, fontFamily:, textColor:, placeholderColor:, clearable:, minLines:, maxLines:, searchable:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)` with optional trailing `{ ... }` actions

Required options: `value`, `placeholder`

Optional options: `comment`, `keyboardType`, `isSecure`, `autofill`, `returnKeyType`, `multiline`, `autocorrect`, `capitalization`, `focused`, `ref`, `maxLength`, `font`, `fontSize`, `fontFamily`, `textColor`, `placeholderColor`, `clearable`, `minLines`, `maxLines`, `searchable`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: optional action block

Modifiers:

- `.onChange` (event actions, optional)
- `.onFocus` (actions, optional)

Reference: components.md#textinput

## Interactivity

### `Pressable`

Pressable region with tap, context-menu, drag, pinch, double-tap, and long-press actions

Signature: `Pressable(disabled:, haptic:, fillMaxSize:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Optional options: `disabled`, `haptic`, `fillMaxSize`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers:

- `.onTap` (actions, required)
- `.onLongPress` (actions, optional)
- `.contextMenu` (nodes, optional)
- `.onDoubleTap` (actions, optional)
- `.onDrag` (event actions, optional)
- `.onPinch` (event actions, optional)

Reference: components.md#pressable

## Media

### `ContentUnavailable`

Shows a native empty or unavailable content state

Signature: `ContentUnavailable(title, icon, description:, titleColor:, descriptionColor:, iconTint:, background:, comment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `title`, `icon`

Optional options: `description`, `titleColor`, `descriptionColor`, `iconTint`, `background`, `comment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#content-unavailable

### `Icon`

Displays a system symbol or a glyph from a registered font

Signature: `Icon(system:, sfsymbol:, materialsymbol:, fontFamily:, glyph:, description, size, tint, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `description`, `size`, `tint`

Optional options: `system`, `sfsymbol`, `materialsymbol`, `fontFamily`, `glyph`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Exactly one of: `system`, `sfsymbol`, `materialsymbol`, `glyph`

Children: none

Modifiers: none

Reference: components.md#icon

### `Image`

Displays a native image from an asset, URL, or local file

Signature: `Image(asset:, url:, file:, description, scale:, placeholder:, showLoadingIndicator:, maxHeight:, fillMaxSize:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `description`

Optional options: `asset`, `url`, `file`, `scale`, `placeholder`, `showLoadingIndicator`, `maxHeight`, `fillMaxSize`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Exactly one of: `asset`, `url`, `file`

Children: none

Modifiers:

- `.sharedElement` (no block, optional)

Reference: components.md#image

### `LinearGradient`

Draws a native linear color gradient

Signature: `LinearGradient(startColor, endColor, direction:, height:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `startColor`, `endColor`

Optional options: `direction`, `height`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#lineargradient

## Navigation

### `Link`

Opens a URL in the system browser

Signature: `Link(url, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `url`

Optional options: `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#link

### `NavigationBack`

Pops the navigation stack with an optional label

Signature: `NavigationBack(label:, comment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Optional options: `label`, `comment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `NavigationLink`

Navigates to a declared screen destination

Signature: `NavigationLink(destination, when:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `destination`

Optional options: `when`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `NavigationSplitView`

Adaptive sidebar and detail navigation

Signature: `NavigationSplitView(detailVisible:) { Sidebar { ... } Detail { ... } }`

Required options: `detailVisible`

Optional options: `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: sidebar and detail blocks

Modifiers: none

Reference: components.md#navigationsplitview

### `NavigationStack`

Navigation host rooted at a declared screen

Signature: `NavigationStack(root, showNavigationBar:, titleMode:, titleAlignment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:)`

Required options: `root`

Optional options: `showNavigationBar`, `titleMode`, `titleAlignment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: none

Modifiers: none

Reference: components.md#navigationstack--navigationlink

### `PagePager`

Swipeable pages with a native page indicator, bound to a selected-page value

Signature: `PagePager(selected:) { Tab(..) ... }`

Required options: `selected`

Optional options: `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

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

High-performance virtualized list with configurable dividers and responsive grids

Signature: `FastList(collection | count: | sections:, axis:, ...)` with `{ bindings in ... }` rows

Optional options: `axis`, `rowHeight`, `gap`, `scrollPosition`, `scrollEnabled`, `reverseLayout`, `pageSnap`, `showDividers`, `hideHorizontalScrollbar`, `hideVerticalScrollbar`, `ref`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Source forms: positional collection, `count`:, or `sections:` (exactly one); row-key option `key`.

Children: row bindings plus row body

Modifiers:

- `.onEndReached` (actions, optional)
- `.onScroll` (actions, optional)
- `.onScrollBeginDrag` (actions, optional)
- `.onMove` (event actions, optional)
- `.stickyHeader` (nodes, optional)
- `.sectionHeader` (nodes, optional)
- `.swipeActions` (nodes, optional)

Reference: components.md#fastlist

## Tabs

### `AppBottomBar`

Bottom tab bar bound to a selected-tab binding

Signature: `AppBottomBar(selected:) { Tab(..) ... TabBar { ... }? }`

Required options: `selected`

Optional options: `tint`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: `Tab` destinations and an optional custom `TabBar` block

Modifiers: none

Reference: components.md#appbottombar

### `Toolbar`

Places native actions in a navigation toolbar

Signature: `Toolbar(placement:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Optional options: `placement`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#toolbar

## Refresh

### `RefreshControl`

Pull-to-refresh wrapper with an onRefresh action

Signature: `RefreshControl(isRefreshing, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `isRefreshing`

Optional options: `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

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

Signature: `StatusBar(style:, hidden:, background:, navigationBarBackground:)`

Optional options: `style`, `hidden`, `background`, `navigationBarBackground`

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

Native modal sheet controlled through a typed component reference

Signature: `BottomSheet(ref, contentSized:, contentPadding:, cornerRadius:, detents:, initialDetent:, showDragIndicator:, allowsDismissal:, allowsBackgroundInteraction:, shouldDismissOnBackPress:, backgroundColor:, scrimColor:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `ref`

Optional options: `contentSized`, `contentPadding`, `cornerRadius`, `detents`, `initialDetent`, `showDragIndicator`, `allowsDismissal`, `allowsBackgroundInteraction`, `shouldDismissOnBackPress`, `backgroundColor`, `scrimColor`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers:

- `.onDismiss` (actions, optional)

Reference: components.md#bottomsheet

### `ConfirmationDialog`

Native confirmation dialog bound to a boolean binding

Signature: `ConfirmationDialog(isPresented, title, comment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `isPresented`, `title`

Optional options: `comment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

Children: node block

Modifiers: none

Reference: components.md#confirmationdialog

### `Dialog`

Native alert with button actions and optional single-line text input

Signature: `Dialog(isPresented, title, message, comment:, padding:, margin:, accessibilityLabel:, accessibilityHint:, accessibilityValue:, accessibilityRole:) { ... }`

Required options: `isPresented`, `title`, `message`

Optional options: `comment`, `padding`, `margin`, `accessibilityLabel`, `accessibilityHint`, `accessibilityValue`, `accessibilityRole`

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

## Shared view modifiers

These modifiers wrap an existing rendered view and keep layout policy separate from its component type.

### `.safeAreaInset`

Arguments: `edge: Top | Bottom | Leading | Trailing` (required).

Places its node block at the selected safe edge with no additional gap. The node block is measured natively; a uniquely inferred compatible scroll owner receives endpoint accommodation. Without an owner, placement still works.

Reference: components.md#safe-area-and-edge-attached-content

### `.scrollHeader`

Arguments: `behavior: Pinned | Collapse | HideOnScroll = Pinned`, `collapsedHeight: positive numeric literal = 56` for `Collapse`.

Attaches one reusable header to exactly one vertical scroll owner. `Pinned` keeps its measured height, `Collapse` interpolates its height toward `collapsedHeight`, and `HideOnScroll` hides on downward movement and reveals on upward movement. It does not change standard `NavigationStack` large-title behavior.

Reference: components.md#safe-area-and-edge-attached-content

## Vocabulary

Keywords: app, screen, component, state, let, fn, async, await, enum, struct, plugin, try, catch, case, return, if, else, while, for, in, break, continue

Types: String, Bool, Int8, Int16, Int32, Int64, UInt8, UInt16, UInt32, UInt64, Float32, Float64, Bytes, Result, Array, Map, Set, Regex, RegexMatch, Range, Pair, Triple, Void, Ok, Err

