---
name: nexa-framework
description: "Use when changing the Nexa Rust compiler, language, typed IR, CLI, native Swift or Kotlin code generators, core UI components, build workflow, or performance architecture."
---

# Nexa Framework Engineering Guide

Fast, token-efficient, authoritative architectural cheat sheet for engineering on the Nexa transpiler and core crates. **Consult this guide directly instead of grepping through 50,000+ lines of codebase.**

---

## 1. Strict Syntax Invariants

### 1.1 Dot-Modifiers vs Component Parameters (MANDATORY RULE)
* **DOT-MODIFIERS ARE RESERVED ONLY FOR EVENT CALLBACKS & SYSTEM TRIGGERS**:
  * Allowed dot-modifiers:
    * `RefreshControl(...) { ... }.onRefresh { ... }`
    * `Pressable(...) { ... }.onTap { ... }.onDoubleTap { ... }.onLongPress(durationMs: 500) { ... }.onDrag { ... }.onPinch { ... }`
    * `FastList(...) { ... }.onEndReached { ... }.onScroll { ... }.stickyHeader { ... }.sectionHeader { ... }`
* **ALL STYLING & LAYOUT MUST BE DIRECT COMPONENT PARAMETERS**:
  * NEVER use trailing dot-modifiers for styling (no `.padding()`, `.background()`, `.cornerRadius()`, `.glass()`, etc.).
  * ALL styling properties must be passed inside the component call parentheses:
    ```nx
    // CORRECT
    Column(spacing: 12, padding: 16, background: "#FFFFFF", cornerRadius: 12, glass: "capsule") { ... }
    Button("Submit", style: BorderedProminent, shape: Capsule, tint: "#0984E3", glass: true) { ... }
    Text("Title", fontSize: 24, fontWeight: Bold, color: "#111111")

    // INCORRECT (FORBIDDEN)
    Column { ... }.padding(16).background("#FFFFFF")
    Text("Title").fontSize(24).bold()
    ```

### 1.2 Native Code Generation Principles
1. **Zero Runtime Reflection or Boxing**: Statically typed native code only. No dynamic `AnyMap` or untyped reflection.
2. **Zero `unwrap()` in Production Paths**: Return `Result<T, CompileError>`. Never panic during user code compilation.
3. **No `AnyView` in Swift UI**: Use generic view specialization (`NexaFastList<RowContent: View, HeaderContent: View = EmptyView>`).
4. **Unboxed Primitives in Kotlin**: Use `mutableIntStateOf`, `mutableDoubleStateOf` instead of generic `mutableStateOf<T>`.
5. **Deterministic Scaffolding**: All CLI code-gen and project templates must be strictly deterministic and idempotent.

### 1.3 Generic Framework Boundaries
* Nexa core must support arbitrary apps. Do not add product-specific models, labels, behaviors, defaults, symbol switches, storage rules, or workflow assumptions to the parser, IR, compiler, backends, DevRuntime, or CLI.
* Keep shared cross-platform concepts in a typed, target-neutral IR/API and resolve them in the native backends. A symbol, control, or navigation primitive must not have independent app-specific or platform-specific lookup tables in unrelated generators.
* A native component may implement one reusable platform primitive (for example, a bottom navigation bar), but its generated behavior must not invent screen content, titles, storage, sorting, or task behavior. Tab labels belong to the tab bar; destinations own their content and title.
* Release output is AOT Swift/Kotlin. Resolve names and feature branches during compilation so generated apps do not ship a general interpreter, reflection, or dynamic lookup path. DevRuntime may interpret IR only in development builds and must track the same typed IR contract.
* When porting an app feature, first decide whether it is a general capability or app-owned composition. Implement reusable capabilities in Nexa; leave product-specific composition in `.nx` app source unless the user explicitly narrows the work to framework-only.
* Locale and localization are first-party framework APIs, not optional plugins. Keep locale calls statically typed and lower them to Foundation or Android `Locale` APIs; compile literal localization keys to native string resources, with no release-time key lookup or reflection.

---

## 2. Fast Codebase Routing Map

Use this index to go directly to the exact file without exploratory searches:

| Task / Responsibility | Key Files |
|---|---|
| **Lexer & Tokens** | `crates/nexa-syntax/src/lexer.rs` |
| **Parser & AST** | `crates/nexa-syntax/src/parser.rs`, `crates/nexa-syntax/src/ast.rs` |
| **Component Schemas & Catalog** | `crates/nexa-syntax/src/catalog.rs` (vocabulary, schemas, argument models) |
| **Typed IR & Enums** | `crates/nexa-ir/src/lib.rs` (`Node`, `Expr`, `Action`, `Type`, enums); `crates/nexa-ir/src/system_icons.rs` (shared icon catalog and native name resolution) |
| **IR Traversal & Capabilities** | `crates/nexa-ir/src/walk.rs`, `crates/nexa-ir/src/capabilities.rs` |
| **Semantic Lowering (Components)** | `crates/nexa-compiler/src/semantic/components.rs` |
| **Semantic Lowering (Expressions & Types)** | `crates/nexa-compiler/src/semantic/expressions.rs` |
| **Semantic Lowering (Styles & Effects)** | `crates/nexa-compiler/src/semantic/styles.rs` |
| **IR Optimization (Constant Folding)** | `crates/nexa-compiler/src/optimize.rs` |
| **Swift UI Controls & Layout** | `crates/nexa-backend-swift/src/generator/components/` (`controls.rs`, `layout.rs`, `bottom_bar.rs`, `sheets.rs`) |
| **Swift Virtualized Lists** | `crates/nexa-backend-swift/src/generator/components/lists.rs`, `list_runtime.rs` |
| **Swift Structs, Types & Expressions** | `crates/nexa-backend-swift/src/generator/engine/` (`structs.rs`, `types.rs`, `expressions.rs`) |
| **Kotlin Composables & Layout** | `crates/nexa-backend-kotlin/src/generator/components/` (`controls.rs`, `layout.rs`, `bottom_bar.rs`, `sheets.rs`) |
| **Kotlin Virtualized Lists** | `crates/nexa-backend-kotlin/src/generator/components/lists.rs` |
| **Kotlin Data Classes & Engine** | `crates/nexa-backend-kotlin/src/generator/engine/` (`structs.rs`, `types.rs`, `expressions.rs`) |
| **CLI & Project Generation** | `crates/nexa-cli/src/project.rs`, `crates/nexa-cli/src/commands/` |
| **Hot Reload & Dev Runtime** | `runtime/ios/NexaDevRenderer.swift`, `runtime/android/NexaDevRenderer.kt` |
| **Coverage Inventory Fixture** | `crates/nexa-cli/tests/fixtures/hot_reload_coverage.json` |

---

## 3. Core Component Catalog & Parameter Reference

| Component | Parameters | Allowed Children / Blocks |
|---|---|---|
| `Column` | `spacing: Int`, `padding: Int`, `alignment: Leading\|Center\|Trailing`, `background: Color`, `cornerRadius: Float`, `width: Float`, `height: Float`, `minWidth`, `maxWidth`, `glass: "circle"\|"capsule"\|"rounded(r)"` | UI nodes block |
| `Row` | `spacing: Int`, `padding: Int`, `alignment: Top\|Center\|Bottom`, `background: Color`, `cornerRadius: Float`, `width: Float`, `height: Float`, `glass: "circle"\|"capsule"\|"rounded(r)"` | UI nodes block |
| `Stack` | `alignment: Center\|TopLeading\|...`, `width: Float`, `height: Float`, `background: Color`, `cornerRadius: Float`, `glass: ...` | UI nodes block |
| `Text` | Positional text `String`, `fontSize: Int`, `fontWeight: Normal\|Medium\|Semibold\|Bold`, `color: Color`, `maxLines: Int` | None |
| `Button` | Positional title `String`, `icon: shared system icon name`, `loading: Bool`, `disabled: Bool`, `style`, `size`, `shape`, `tint`, `glass` | Action block `{ ... }` |
| `TextInput` | `value: mutableString`, `placeholder: String`, `fontSize: Int` | None |
| `Switch` | `value: mutableBool`, `label: String` | None |
| `SegmentedControl` | `items: Array<String>`, `selected: mutableString` | None |
| `Picker` | `items: Array<String>`, `selected: mutableString`, optional `icon: shared system icon name` | None |
| `Icon` | Exactly one of `system: sharedName`, `sfsymbol: iOSName`, or `materialsymbol: AndroidName`; `size`, `tint`, `description` | None |
| `Image` | `asset: String` or `url: String`, `description: String` | None |
| `Pressable` | None | UI nodes block + trailing `.onTap { ... }`, `.onLongPress { ... }`, `.contextMenu { Button(...) { ... } }`, `.onDoubleTap { ... }`, `.onDrag { ... }`, `.onPinch { ... }` |
| `FastList` | Collection `FastList(items, key: .id)` or `FastList(count: Int, rowHeight: Float)` | Row binding `{ item, index in ... }` + optional `.onEndReached`, `.onScroll`, `.stickyHeader`, `.sectionHeader` |
| `AppBottomBar`| `selected: mutableInt32` | `Tab(index: Int, label: String, icon: shared system icon name, badge: String?, role: "search"?) { ... }` |
| `BottomSheet` | `isPresented: mutableBool`, `partial: Bool` | UI nodes block |
| `Dialog` | `isPresented: mutableBool`, `title: String`, `message: String` | Button action nodes block |
| `RefreshControl`| `isRefreshing: mutableBool` | Content block + trailing `.onRefresh { ... }` |
| `Spacer` | None (expands along axis) | None |
| `Divider` | `color: Color`, `thickness: Float` | None |

---

## 4. State & Collection Semantics

* **Primitives**: `String`, `Bool`, `Int32`, `Float64`.
* **Collections**:
  * `Array<T>`: `.count`, `.append(value)`, `.remove(index)`, `.move(from, to)`.
  * `Set<T>`: `.insert(value)`, `.remove(value)`, `value in set`.
  * `Map<K, V>`: `.set(key, value)`, `.remove(key)`, `map[key]`.
* **Value Structs**:
  ```nx
  struct TaskItem {
      id: String,
      content: String,
      priority: Int32,
  }
  // Lowers to:
  // Swift:  struct TaskItem: Equatable { let id: String; let content: String; let priority: Int32 }
  // Kotlin: data class TaskItem(val id: String, val content: String, val priority: Int)
  ```
* **Assignment**: `Assign` is strictly for state variables: `variable = value`.

---

## 5. Verification Commands

Always run and verify before completing work:

```bash
# 1. Type check
cargo check --workspace --all-targets

# 2. Strict linter (must be 0 warnings)
cargo clippy --workspace --all-targets

# 3. Complete test suite
cargo test --workspace

# 4. Safe temp directories in tests (Dev-dependency only)
# Always use nexa_testkit::TempDir / VacantDir, NEVER SystemTime::now()
```
