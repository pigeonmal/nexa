---
name: nexa-framework
description: "Use when changing the Nexa Rust compiler, language, typed IR, CLI, native Swift or Kotlin code generators, core UI components, build workflow, or performance architecture."
---

# Nexa Framework Engineering

Routing map and non-negotiable invariants for the Rust compiler and both AOT backends.

## Rule 1: the catalog is the grammar

`crates/nexa-syntax/src/catalog.rs` is the single source of truth for the accepted language:
component vocabulary, argument schemas, child models, dot modifiers, keywords, and types.
`docs/syntax-audit.md` and `editors/vscode/syntaxes/nexa.tmLanguage.json` are generated from it
and asserted byte-for-byte by `cargo test -p nexa-syntax`.

When you add or change a component:

1. Add the `ComponentEntry` (vocabulary, summary, snippet, parse-checked `probe`) and the
   `ComponentSchema` (arguments, children, modifiers, flags).
2. Update the parser if the grammar really changed.
3. Run `NEXA_UPDATE_SNAPSHOTS=1 cargo test -p nexa-syntax` and commit the regenerated files.

Never hand-edit `docs/syntax-audit.md`. Never restate the catalog into a skill, doc, or comment —
point at it. A copied table has no compiler behind it and will drift.

## Rule 2: generated documentation is generated

Any Markdown table restating a machine-readable source of truth is rendered from that source and
gated by a test. `nexa_testkit::docs` provides `assert_snapshot` (whole file) and `assert_region`
(`<!-- nexadoc:begin NAME -->` fenced span, prose around it stays hand-written).

| Published table | Source of truth | Gate |
|---|---|---|
| `docs/syntax-audit.md` | `nexa_syntax::catalog` | `cargo test -p nexa-syntax` |
| `docs/system-icons.md#3-catalog-of-shared-icons` | `nexa_ir::system_icons::SHARED_ICONS` | `-p nexa-ir --test system_icon_docs` |
| `README.md#official-native-plugins`, `docs/plugins.md` (3 tables) | `plugins/*/plugin.config.nx` | `-p nexa-cli --test plugin_reference` |
| `docs/getting-started.md#configuration-options-reference` | `ProjectConfig::from_defaults` | `-p nexa-cli --test project_config_reference` |
| `docs/getting-started.md#cli-command-matrix` | the CLI's command and flag tables | same |
| `docs/language-guide.md#primitive-scalar-types` | `swift_type` / `kotlin_type` | `-p nexa-cli --test language_reference` |
| `docs/architecture-audit.md` newest row | `cache::CACHE_VERSION` | `-p nexa-cli --test cache_audit` |

Put each `render_*` in the crate that owns the data. Never generate prose: an editorial column
lives in a `&'static` table next to the code, with a test that fails when a real item has no entry.
CI runs these gates in the `docs` job.

## Non-negotiable invariants

- **No runtime reflection or boxing.** No `AnyMap`, untyped parameter maps, or string-keyed member
  lookup in generated output.
- **No `unwrap()` on a user compilation path.** Return `Result<T, CompileError>`; a user's `.nx`
  file must never panic the compiler.
- **No `AnyView` in Swift release view trees** or virtualized list cells. Specialize with generic
  parameters (`NexaSectionedList<RowContent: View, HeaderContent: View = EmptyView>`).
- **No boxed primitive state in Kotlin.** Use `mutableIntStateOf`, `mutableDoubleStateOf`,
  `mutableLongStateOf` — not `mutableStateOf<Double>`.
- **Deterministic scaffolding.** CLI templates and generated projects must be idempotent: the same
  input produces byte-identical output.

## Cross-platform parity is the default

Every new or changed shared feature and component must have matching behavior and visual defaults
on iOS and Android without the app author having to request parity. Use shared design tokens and
layout values, then compare the SwiftUI and Compose output (and both DevRuntime hosts when they
render the feature). Add a platform-specific variation only when the app author explicitly asks
for it or a native constraint requires it; document that difference and keep its effect narrow.

## Framework boundaries

- Core must serve arbitrary apps. No product-specific models, labels, storage rules, or workflow
  assumptions in the parser, IR, compiler, backends, DevRuntime, or CLI.
- Shared concepts live in the typed, target-neutral IR and resolve in each backend. Never give one
  primitive independent lookup tables in unrelated generators.
- A native component may implement one reusable platform primitive (a bottom bar), but must not
  invent screen content, titles, or behavior. Tab labels belong to the tab bar; destinations own
  their content.
- Release output is AOT. Resolve names and branches during compilation — no interpreter,
  reflection, or dynamic lookup ships. DevRuntime may interpret IR in debug builds only and must
  track the same typed contract.
- Porting an app feature: decide first whether it is a general capability or app-owned composition.
  Implement capabilities in Nexa; leave product composition in `.nx`.

## Routing map

| Responsibility | File |
|---|---|
| Lexer / parser / AST | `crates/nexa-syntax/src/{lexer,parser,ast}.rs` |
| **Component catalog (grammar source of truth)** | `crates/nexa-syntax/src/catalog.rs` |
| Typed IR, `Type`, `NumericType`, enums | `crates/nexa-ir/src/lib.rs` |
| Shared system icon catalog | `crates/nexa-ir/src/system_icons.rs` |
| IR walk / capability reachability | `crates/nexa-ir/src/{walk,capabilities}.rs` |
| Semantic lowering: components | `crates/nexa-compiler/src/semantic/components.rs` |
| Semantic lowering: expressions, core APIs, types | `crates/nexa-compiler/src/semantic/expressions.rs` |
| Semantic lowering: styles, alignment, effects | `crates/nexa-compiler/src/semantic/styles.rs` |
| Constant folding / dead code | `crates/nexa-compiler/src/optimize.rs` |
| Plugin IDL + manifest parsing | `crates/nexa-plugin-idl/src/{lib,manifest}.rs` |
| Plugin binding generation | `crates/nexa-codegen/src/plugin/` |
| Swift components / lists | `crates/nexa-backend-swift/src/generator/components/` |
| Swift types / structs / expressions | `crates/nexa-backend-swift/src/generator/engine/` |
| Kotlin components / lists | `crates/nexa-backend-kotlin/src/generator/components/` |
| Kotlin types / structs / expressions | `crates/nexa-backend-kotlin/src/generator/engine/` |
| CLI commands and flags | `crates/nexa-cli/src/commands.rs` |
| Project generation, templates | `crates/nexa-cli/src/project.rs`, `src/project/templates.rs` |
| Project config defaults | `crates/nexa-cli/src/config.rs` |
| Generated-doc renderers (CLI-side) | `crates/nexa-cli/src/docs.rs` |
| Build cache and `CACHE_VERSION` | `crates/nexa-cli/src/cache.rs` |
| `nexa plugin` subcommands | `crates/nexa-cli/src/plugin_cli.rs` |
| DevRuntime (iOS / Android) | `runtime/ios/`, `runtime/android/` |
| Shared test support | `crates/nexa-testkit/src/` |

## Testing rules

- Never name a temp directory from a clock. `SystemTime::now().as_nanos()` repeats under
  concurrency and `create_dir_all` is a no-op on an existing path, so two tests silently share a
  tree. Use `nexa_testkit::TempDir::new` / `VacantDir::new`, which claim a name atomically.
- `nexa-testkit` is a `dev-dependency` only and depends on nothing, so any crate can use it without
  inverting the layering.

## Verification

```bash
export CARGO_TARGET_DIR=/tmp/nexa-verification-target   # reuse artifacts; do not delete

cargo check --workspace --all-targets
cargo clippy --workspace --all-targets                 # must be 0 warnings
cargo test --workspace
```

Run the narrowest useful command while iterating (`cargo check -p nexa-compiler`,
`cargo test -p nexa-syntax --test catalog_tests`), then the three full gates once at the end.
After changing anything that creates temporary directories, also run the collision gate:
`cargo test -p nexa-testkit -- --ignored`.
