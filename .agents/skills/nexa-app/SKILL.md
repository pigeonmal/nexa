---
name: nexa-app
description: "Use when helping a Nexa user plan, author, structure, or troubleshoot a mobile app in `.nx`, especially when the user wants a complete app without writing Swift or Kotlin."
---

# Create an App with Nexa

Help people build an iOS and Android app from one `.nx` source without asking them to edit
native Swift or Kotlin. Start from the user's product goal, clarify only what the request leaves
genuinely open, and build a coherent app from the components the compiler actually accepts.

## The one rule that matters

**Never write `.nx` syntax from memory.** Every accepted component, argument, and modifier is in
[`docs/syntax-audit.md`](../../../docs/syntax-audit.md), which is generated from
`crates/nexa-syntax/src/catalog.rs` and parse-checked by `cargo test -p nexa-syntax`. Read the
relevant section before writing code. This skill deliberately does not restate those tables: a
copied table is a table that rots, and the audit is regenerated whenever the grammar changes.

Verify every snippet you hand the user:

```bash
nexa check            # type-checks both targets and reports grammar and type errors
```

`nexa check` is the arbiter. If it passes, the syntax is real; if it fails, the error names the
exact option or value that is wrong. Never claim a construct works without having run it.

## Cross-platform parity is implicit

Treat every shared `.nx` feature and component as an iOS/Android feature by default. Its behavior,
colors, typography, spacing, sizing, alignment, and interaction affordances should match across
both generated apps without the user having to ask. Preview or inspect both targets when changing
UI. Use `platform ios` / `platform android` only for a deliberate, requested difference or a real
native constraint, and keep that branch limited to the difference.

## Reading order

| Need | Read |
|---|---|
| Accepted components, arguments, modifiers | [`docs/syntax-audit.md`](../../../docs/syntax-audit.md) |
| Types, `Result<T, E>`, collections, core APIs | [`docs/language-guide.md`](../../../docs/language-guide.md) |
| State scopes, screens, navigation, lifecycle | [`docs/state-and-navigation.md`](../../../docs/state-and-navigation.md) |
| Component idioms and worked examples | [`docs/components.md`](../../../docs/components.md) |
| CLI flags and `nexa.config.nx` keys | [`docs/getting-started.md`](../../../docs/getting-started.md) |
| Runnable programs | `examples/*.nx`, `examples/archetypes/`, `examples/components/` |
| Native plugin APIs | `plugins/<name>/README.md` and its `native.nxid` |

## Layout and interaction

- `Column` is the vertical container, `Row` horizontal, `Stack` for overlays. Layout `alignment`
  takes `Start`, `Center`, or `End` — `Leading`/`Trailing` are `Text` values and are rejected on
  layout. `Stack` rejects `spacing`; the compiler says so explicitly.
- Styling and layout are named arguments inside the call: `Column(spacing: 12, padding: 16,
  background: "#FFFFFF", cornerRadius: 12)`. There are no `.padding()`-style layout modifiers.
- Dot modifiers are for event callbacks only — `.onTap`, `.onRefresh`, `.onScroll`,
  `.onEndReached`, `.contextMenu`, `.stickyHeader`. The tap handler is `.onTap`; `.onPress` is not
  accepted syntax.
- Bounds pair together: `minWidth`/`maxWidth`, `minHeight`/`maxHeight`, and `borderColor` with
  `borderWidth`. Each minimum must not exceed its maximum.
- Accessibility options (`accessibilityLabel`, `accessibilityHint`, `accessibilityValue`,
  `accessibilityRole`) go directly on any visual component. A label is required as soon as any
  accessibility option is used.

## State, binding, and mutability

- `state name: Type = init` is mutable; `let` is immutable. Nexa does not infer mutability from a
  later assignment.
- Integer literals default to `Int32`, decimal literals to `Float64`. Use an explicit annotation
  for anything narrower or for an empty collection.
- Bindings passed to a component (`Slider(value:)`, `AppBottomBar(selected:)`, `TextInput(value:)`)
  must be a **mutable state binding**, not a literal. `Slider(value: 0.5, ...)` is an error; declare
  `state level: Float64 = 0.5` and pass that.
- `FastList` takes its source positionally or as `count:` / `sections:` — never `items:`, which the
  compiler rejects by name. An array source binds **two** names: `{ item, index in ... }`. An
  unused binding is a warning, so use `nexa check --deny-warnings` when that matters.

## File architecture

For anything beyond a single screen, establish this before filling in screens:

```text
App.nx                 # the one `app` declaration, app-wide state, root navigation
screens/Home.nx        # one screen per file
components/TaskRow.nx  # reusable components
models/Task.nx         # structs and enums
helpers/DateFormat.nx  # pure functions
```

Join files with `import "relative/path.nx"`, resolved from the importing file. Component names
share one project-wide scope. These names are a useful starting shape, not a framework
requirement — a small app may stay in one file.

## Platform and permissions

- Use `platform ios { ... }` / `platform android { ... }` for target-only branches. `nexa check`
  validates both.
- Declare host permissions and purpose strings in `nexa.config.nx`, never in app source:
  `config { permissions { camera: "Scan a return label." } }`. The compiler writes the iOS
  `Info.plist` strings and Android manifest entries from that.
- Configure plugins in the same file: `config { plugins { Notifications { ... } } }`. Options are
  validated against the plugin's `native.nxid` at generation time.
- An app's `android.minSdk` must be at least every plugin's floor. The generated project raises the
  iOS deployment target to the highest plugin minimum; the published floors are in
  `README.md#official-native-plugins`.

## Working with users

- Translate a non-technical request into screens, state, navigation, and component behavior, and
  explain the structure in plain language.
- When a request exceeds the language, name the specific gap and offer the closest working
  core-only approach. Do not imply the project is store-ready when Nexa only emits source.
- Present `.nx` source and the build command. Never hand the user a step that requires editing
  generated Swift or Kotlin for something the language supports.

## Current CLI

```bash
nexa create <Name>        # scaffold App.nx, nexa.config.nx, .nexa/signing.properties, .gitignore, README.md
nexa check                # type-check both targets (--deny-warnings, --audit, --locked)
nexa dev                  # build, launch, hot reload (--ios, --android, --once, --compile-only)
nexa test                 # build and run (--unit-only for language tests only)
nexa release              # iOS archive or Android AAB
nexa audit App.nx         # capability reachability and generated dependencies
nexa doctor               # toolchain health
```

There is no `nexa build` and no `nexa generate` command. Full flag list:
[`docs/getting-started.md`](../../../docs/getting-started.md#cli-command-matrix).
