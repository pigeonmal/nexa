# Language design decisions

Nexa borrows useful ideas from Kotlin and Swift, but keeps one source language and one typed IR. Syntax is selected for cross-platform clarity; Swift and Kotlin spelling differences are handled by the backends. Features enter the language when their type and runtime behavior are defined well enough to generate direct native code.

| Concept | Decision | Nexa design and status |
| --- | --- | --- |
| Condition (`if / else`) | Integrate now | Lowers directly to native branches for UI and event actions. |
| Multiple choice (`when / switch`) | Adopt after enums | Use one exhaustive `match` construct and emit Kotlin `when` or Swift `switch`; avoid separate source syntaxes and non-exhaustive UI states. |
| AND / OR / NOT | Integrate now | `&&`, `||`, `!`; native short-circuit operators. |
| Equality and comparison | Integrate now | `==`, `!=` for scalar values; numeric `<`, `<=`, `>`, `>=`. Exact types are required; no runtime conversions. |
| Mutable variable / constant | Keep Nexa terms | `state` means observed, mutable UI state; `let` means immutable. `var`/`val` aliases would blur state lifetime and add duplicate syntax. |
| Function / return | Adopt | Typed, statically resolved functions; avoid dynamic function registries. Not implemented yet. |
| Nullable / optional | Adopt | Explicit `T?` with checked unwrap semantics, mapped to Swift `Optional` and Kotlin nullable types. Not implemented yet. |
| Default value if null | Adopt one spelling | Prefer `??` in Nexa and lower to native short-circuit fallback; do not make Kotlin's `?:` the shared spelling. Depends on optionals. |
| Safe access | Adopt | `?.` with compile-time member/type checking; depends on optionals and user-defined types. |
| Loops | Adopt | Direct native `for`/`while` control flow. For UI collections, use `FastList` so large lists stay virtualized. Not implemented yet. |
| `break` / `continue` | Adopt with loops | Direct loop control, validated by the parser/type checker. |
| Range | Adopt with loops | A compact start/end/step IR, without eagerly materializing an array. Not implemented yet. |
| Anonymous function / closure | Adopt selectively | Permit closures at callback boundaries after capture and escape rules are explicit; avoid boxing or heap allocation for non-escaping callbacks. |
| `map / filter / reduce` | Adopt with optimization rules | Fuse non-escaping transforms into a single pass where possible; do not blindly emit allocation-heavy chained collection calls. |
| Array / List | Already supported | `Array<T>` lowers to Swift `Array<T>` and Kotlin `List<T>`; mutable collection semantics are not implied. |
| Dictionary / Map | Defer | Useful, but needs explicit key constraints, ordering, and mutation semantics before choosing native representations. |
| Set | Defer | Same key/equality questions as maps; use native set storage when introduced. |
| Class | Limit to identity cases | Prefer value types for ordinary models. Add references only where stable identity or native handles require them. |
| Constructor | Adopt with value types | Generate direct Swift initializers and Kotlin constructors; define initialization and mutability before exposing user types. |
| Inheritance | Avoid in core | Use composition and statically dispatched interfaces; class inheritance brings dynamic dispatch and fragile shared behavior. |
| Interface / protocol | Adopt as static constraints | Resolve implementations statically where possible; avoid implicit existential boxes in hot paths. |
| Enum | Adopt before matching | Closed, typed value cases enable efficient native enums/sealed representations and exhaustive `match`. |
| Extension | Defer as syntax sugar | Can desugar at compile time, but requires clear member lookup and conflict rules. |
| Generics | Adopt with monomorphization | Specialize statically to avoid runtime generic dispatch; track binary-size growth. |
| Type check (`is`) | Avoid general runtime checks | Prefer exhaustive matching on closed enums; no dynamic object hierarchy is planned for ordinary app models. |
| Type cast (`as / as?`) | Avoid unchecked casts | Add only explicit, type-checked conversions or optional casts when a real interop case needs them. |
| Exceptions | Prefer typed errors | Model recoverable cross-platform failures with `Result<T, E>`/typed effects; catch platform exceptions at native/plugin boundaries. |
| Access control | Adopt with modules | Enforce visibility at compile time; imports and namespaces need explicit module semantics first. |
| String interpolation | Adopt | Parse interpolation into typed expression segments and emit native string construction; not implemented yet. |
| Getter / setter | Defer | Computed properties can hide work or state changes; begin with explicit functions and state bindings, then add visible compile-time accessors if needed. |

The current operator and conditional syntax is demonstrated in [conditional-logic.nx](../examples/conditional-logic.nx). Component-local state and multi-file imports are described in the [language guide](language.md). This decision table is a design direction, not a claim that deferred features already compile.
