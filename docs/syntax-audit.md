# Nexa syntax audit

This audit records the syntax rules that keep Nexa source predictable for a
developer and deterministic for the Swift and Kotlin backends. Nexa is still
alpha, so the compiler does not keep compatibility aliases when a spelling is
replaced. A rejected spelling fails during parsing with an actionable
diagnostic instead of being silently reinterpreted.

## FastList is explicit

`FastList` has one source followed by named options and one explicit row
binding block. The source determines the binding arity:

```nexa
app ListPaginationDemo {
    state rows = ["First", "Second"]

    body {
        FastList(rows, key: .self, rowHeight: 56) { row, index in
            Text("\(index): \(row)")
        }.onEndReached {
            rows.append("More")
        }
    }
}
```

The supported source forms are:

| Source | Row bindings | Use |
| --- | --- | --- |
| `FastList(count: rowCount) { index in ... }` | one | Generate rows from an `Int32` count. |
| `FastList(rows, ...) { row, index in ... }` | two | Render an `Array<T>` without copying it. |
| `FastList(sections: groups, ...) { item, index, section in ... }` | three | Render `Array<Array<T>>` with native sections. |

The parser rejects a missing `in`, duplicate binding names, and the wrong
number of bindings before semantic lowering. Names are authored by the
developer, so the compiler never guesses whether a single value means an item
or an index.

## Options and modifiers

The list-specific options are:

- `axis: Vertical|Horizontal|Grid(columns)`;
- `rowHeight: positiveNumber`;
- `key: .self` or a one-member path such as `key: .id`;
- `scrollPosition: mutableInt32`.

Behavior is expressed as a dot modifier attached to the row block:

```nexa
FastList(rows) { row, index in
    Text(row)
}.onEndReached { ... }
 .onScroll { ... }
```

The compiler accepts `.onEndReached`, `.onScroll`, `.stickyHeader`, and
`.sectionHeader` only where the source and axis support them. Each modifier may
appear once. Dot syntax makes the callback's receiver unambiguous and prevents
a bare keyword elsewhere in a body from being mistaken for list configuration.

`key` is intentionally a restricted key path rather than a general expression.
`.self` means the item value; `.id` means the `id` member of the item binding.
The semantic pass resolves the path, requires a statically known scalar
hashable type, and lowers it directly to UIKit hosted-row identity or Compose
lazy-list keys. No key collection, reflection, or runtime lookup is generated.

## Rejected ambiguous forms

The following spellings are not aliases:

- `items:`, `itemExtent:`, and `id:`;
- row options named `item:`, `index:`, or `section:`;
- a bare `onEndReached`, `onScroll`, `stickyHeader`, or `sectionHeader` block;
- a list call that provides both `count:` and a collection source;
- a `key` expression such as `key: row` instead of `.self` or `.member`.

They produce parser diagnostics that point to the option or modifier and show
the current explicit form. Removing these aliases keeps the grammar small and
prevents two source forms from generating different native behavior.

## General language rules

- A component's primary value may be positional only when its component grammar
  declares that position. Configuration values remain named (`axis:`,
  `rowHeight:`, `disabled:`, and so on).
- Named options are checked against a closed allow-list and duplicate options
  are errors. Unknown names are not ignored or forwarded to native code.
- Binding declarations use `state` for mutable values and `let` for immutable
  values. Type inference is shared across both declarations and writes a
  platform-independent type into the typed IR.
- Platform-only widgets use `platform ios { ... }` or `platform android { ... }`;
  the compiler removes the other branch at build time instead of generating a
  runtime platform condition.
- Closure bindings are explicit wherever a callback has more than one value.
  Their names are compile-time symbols and do not allocate a closure object in
  the generated runtime beyond the native callback required by the platform.

## Built-in API audit

The current built-ins follow a small number of argument shapes. This keeps the
surface readable without making every value a verbose named option.

| API family | Value arguments | Child/action blocks | Ambiguity rule |
| --- | --- | --- | --- |
| `Column`, `Row`, `Stack` | Named layout options only | One child block | `Stack` rejects `spacing`; style names are closed and duplicate names fail. |
| `StatusBar` | Optional named `style:`, `hidden:`, and `background:` | None | Defaults are explicit and the declaration is allowed only once at an app or screen root. |
| `Direction` | Required named `value: LTR|RTL` | None | The direction value is a closed choice and the declaration is top-level only. |
| `OnAppear`, `OnDisappear`, `OnActive`, `OnInactive`, `OnBackground` | `OnAppear` may add the `async` marker; lifecycle callbacks take no value arguments | One action block | `OnAppear`/`OnDisappear` are allowed once on an app or named screen; app active-state callbacks are allowed once only at app-body top level. `await` is restricted to `OnAppear async`. |
| Named `screen` declarations | Screen-local `state`/`let` declarations must precede UI nodes | Screen UI nodes and lifecycle blocks | A screen state name must be unique across the app so generated native state storage stays direct and deterministic. |
| `Text`, `Button` | One positional primary value, then named options | Optional action block for `Button` | The first value is always the label/text; all configuration is named. |
| `TextInput`, `Switch`, `NavigationStack`, `NavigationLink`, `Link`, `Accessibility`, `BottomSheet` | Named options only; `Accessibility` accepts required `label` and optional `hint`/`role` | A fixed content block where applicable | Required options and unknown names are checked before lowering; accessibility labels and hints are typed `String` expressions and literal values cannot be empty. Navigation targets use a declared screen name or `ScreenName(value, ...)`; route values are positional and checked against the screen parameter list. |
| `NavigationBack` | Optional named `label: String` (defaults to `"Back"`) | None | It is valid only inside a declared screen; the label is type checked as `String` and lowering uses the native dismiss/pop-back-stack operation. |
| `Image` | Named `asset:` or `url:` source plus named options | None | Exactly one source is required; providing both is an error. |
| `Pressable` | Named `disabled:` and `haptic:` | Content followed by `.onPress { ... }` and optional `.onLongPress { ... }` modifiers | Action names identify their gesture; duplicate modifiers and missing `.onPress` are errors. |
| `RefreshControl` | Named `isRefreshing:` | Content followed by `.onRefresh { ... }` | The direct `FastList` child is recognized structurally for native refresh integration. |
| `FastList` | One source plus closed named options | Explicit row block followed by dot modifiers | Source shape fixes row binding arity; see the detailed rules above. |
| `AppBottomBar` and `Tab` | Named selection/tab options | Tab declarations with one content block each | Tab indexes are static, unique, and non-negative. |
| `If`, `When`, `Platform` | One condition/value or closed platform target | Explicit branch blocks | `When` requires a typed scalar and `else`; `Platform` removes the inactive branch during target lowering. |
| `Theme` and `Layout` predicates | Named token/style values or qualified predicate names | Theme has no child block; predicates are expressions | Token kinds, style names, and predicate names are closed; unsupported placement is rejected before generation. |
| `Content()` and custom components | `Content()` has zero values; custom component properties are named | Optional custom content slot | Custom calls reject missing, unknown, or duplicate properties during semantic analysis. |
| `Network`, `Path`, `File`, `Permissions` | Qualified calls use named options; `await` is explicit for async calls | None | The native-call specification owns the closed option list and defaults. |

The only intentionally positional component values are the primary text/button
value and the `FastList` array source. Collection mutation and ordinary
function calls remain positional because their arity and types are fixed by the
method/function declaration. This prevents a named option from being confused
with a row binding, while keeping common UI calls short.

### Non-intuitive forms found and resolved

- `FastList` previously combined implicit row names, row options, and bare
  callback words. It now requires explicit bindings, `key:`, `rowHeight:`, and
  dot modifiers.
- `Image` has two source modes, so the parser now requires exactly one of
  `asset:` and `url:` instead of choosing one silently.
- `Pressable` and `RefreshControl` previously used adjacent anonymous action
  blocks. They now use named dot modifiers (`.onPress`, `.onLongPress`, and
  `.onRefresh`) so a block cannot be mistaken for a different gesture or
  refresh callback. The parser rejects the old adjacent-block form.
- Every nested `FastList` slot is part of semantic traversal. Row content,
  `stickyHeader`, and `sectionHeader` all participate in component reachability,
  `Content()` validation, top-level-only declarations, and unused-binding
  diagnostics. A component used only by a section header is retained, and an
  invalid `Content()` in an app-level section header is rejected at compile time.
- Native calls use named arguments and an explicit `await`, which keeps method,
  body, headers, timeout, cache, redirect, response-limit, and pinning options
  distinguishable across Swift and Kotlin.
- `state` and `let` share deterministic inference (`Int32` for integer literals,
  `Float64` for decimal literals) while mutability remains explicit. The
  compiler never changes `let` into `state` or vice versa.

These rules keep syntax diagnostics in the parser/semantic layers while the
backends remain responsible only for direct native Swift and Kotlin emission.
