# Nexa Language Guide

Nexa (`.nx`) is a strongly-typed, modern declarative programming language tailored for building high-performance native mobile user interfaces.

---

## 1. Type System

Nexa enforces strict static typing. All types are resolved at compile-time:

### Primitive Types
- `String`: UTF-8 text string (maps to Swift `String`, Kotlin `String`).
- `Int32`: 32-bit signed integer (maps to Swift `Int32`, Kotlin `Int`).
- `Int64`: 64-bit signed integer (maps to Swift `Int64`, Kotlin `Long`).
- `Float`: 32-bit floating point number (maps to Swift `Float`, Kotlin `Float`).
- `Double`: 64-bit floating point number (maps to Swift `Double`, Kotlin `Double`).
- `Bool`: Boolean (`true` or `false`).
- `Void`: Empty return type.

### Collections & Optionals
- `List<T>`: Ordered dynamic array.
- `Map<K, V>`: Key-value associative mapping.
- `T?`: Optional value representing either a value of type `T` or null.

### Custom Types: Structs & Enums

```nexa
struct UserProfile {
    id: Int64,
    username: String,
    avatarUrl: String?,
}

enum Status {
    idle,
    loading,
    success,
    error,
}
```

---

## 2. State & Mutability

In Nexa, dynamic reactive state is declared using `state`:

```nexa
app CounterApp {
    state counter: Int32 = 0
    state query: String = ""

    body {
        Column {
            Text(counter)
            Button("Increment") {
                counter = counter + 1
            }
        }
    }
}
```

- When compiled to **Swift**: Emitted as `@State private var counter: Int32 = 0`.
- When compiled to **Kotlin**: Emitted as `val counter = remember { mutableIntStateOf(0) }` (zero boxing).

---

## 3. Explicit Error Handling: `Result<T, E>` & `?` Operator

Nexa provides Rust-style explicit error handling with zero exception-handling overhead.

### Returning & Propagating Results

```nexa
fn parseQuantity(input: String) -> Result<Int32, String> {
    if input == "" {
        return Err("Input string cannot be empty")
    }
    return Ok(10)
}

fn processOrder(input: String) -> Result<Int32, String> {
    // The postfix ? operator unwraps Ok or propagates Err immediately
    let quantity = parseQuantity(input)?
    return Ok(quantity)
}
```

- **Swift Target**: Maps directly to native Swift `Result<T, E>` and `try expr.get()`.
- **Kotlin Target**: Maps directly to native `NexaResult<T, E>` with sealed classes `Success` and `Failure`.

---

## 4. Control Flow & Conditionals

```nexa
// Conditional rendering in UI
Column {
    if state.isLoading {
        Text("Loading...")
    } else if state.isError {
        Text("An error occurred")
    } else {
        Text("Loaded successfully!")
    }
}
```
