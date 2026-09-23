# Nexa Operational Learnings & Gotchas (`learnings.md`)

This living document captures hard-won operational knowledge, subtle platform gotchas, and architectural insights across the Nexa compiler, native backends (SwiftUI & Jetpack Compose), and plugin bridges (C++, JNI, Objective-C++).

---

## 1. SwiftUI Runtime & Performance Gotchas

### Eliminating `AnyView` in List Components
- **The Issue**: Using `AnyView` around section headers, sticky headers, or list rows destroys SwiftUI's ability to maintain view identity across layout passes. It forces heap allocation on every scroll event and degrades frame rates.
- **The Solution**: Use Swift generic default parameters in container structs. For example:
  ```swift
  @available(iOS 16.0, *)
  private struct NexaFastSectionedList<HeaderContent: View = EmptyView>: UIViewControllerRepresentable {
      // ...
  }
  ```
  This allows both header-less lists and lists with concrete `VStack` headers to compile without type erasure or `AnyView`.

### Enum Error Conformance
- When using user-defined enums inside `Result<Success, Failure>` in Swift, Swift requires that `Failure: Error`.
- Every generated user enum should conform to `String, Error` (e.g., `private enum NexaAppError: String, Error`).

---

## 2. Jetpack Compose & Android Memory Gotchas

### Preventing Recomposition Churn on Sub-Pixel Scrolling
- **The Issue**: Reading `LazyListState.layoutInfo` inside `snapshotFlow` without filtering fires emissions on every sub-pixel scroll offset change, allocating new `LazyListItemInfo` lists and triggering GC churn.
- **The Solution**: Always append `.distinctUntilChanged()` when observing derived index properties:
  ```kotlin
  snapshotFlow { list_state.firstVisibleItemIndex }.distinctUntilChanged().collect { ... }
  ```

### Avoiding Primitive Box Allocations
- **The Issue**: Calling `mutableStateOf(0.0)` creates `MutableState<java.lang.Double>`, boxing 64-bit floats onto the JVM heap on every state mutation.
- **The Solution**: Use unboxed specialized primitives:
  - `mutableDoubleStateOf(0.0)` for `Float64`
  - `mutableIntStateOf(0)` for `Int32`
  - `mutableLongStateOf(0L)` for `Int64`

---

## 3. C++ / JNI Interop & Memory Lifecycles

### Android JNI Native Class Memory Leaks
- **The Issue**: When wrapping native C++ classes (`std::unique_ptr<NativeClass>`) in Kotlin via a pointer handle `val nativeHandle: Long`, if user code forgets to call `.dispose()`, the C++ heap memory leaks forever.
- **The Solution**:
  1. Have generated Kotlin implementation wrappers implement `java.lang.AutoCloseable` (`override fun close() = dispose()`).
  2. Implement a `finalize()` safety net that calls `dispose()` if the object is garbage-collected before explicit closure:
  ```kotlin
  @Suppress("deprecation")
  protected fun finalize() {
      if (nativeHandle != 0L) {
          dispose()
      }
  }
  ```

---

## 4. Parser & AST Semantic Lowering

### Postfix `?` Operator Disambiguation
- In `.nx` syntax, `?` serves multiple roles:
  - Optional chaining: `obj?.field`
  - Optional indexing: `arr?[index]`
  - Ternary Elvis/coalescing: `expr ?? default`
  - Result try propagation: `expr?`
- When parsing `?` in `nexa-syntax`, check if the next token is `.` or `[`. If neither, parse as `ast::Expr::Try`.
- In `nexa-compiler`, ensure that `ast::Expr::Try` validates that its inner expression has type `Type::Result(ok_type, err_type)` and lowers to `nexa_ir::Expr::Try` returning `ok_type`.

### Dead Code Elimination in Compiler Optimization
- `nexa-compiler` runs a pure dead-code pass in `optimize.rs`. Top-level `fn` declarations that are not called by UI event handlers, states, or reachable functions will be eliminated.
- When writing tests or examples, ensure function calls are rooted in active UI actions or state declarations.
