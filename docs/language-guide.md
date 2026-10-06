# Nexa Language Guide 📖

**Nexa** is a statically-typed, ahead-of-time (AOT) compiled language designed specifically for building cross-platform native mobile applications. It compiles `.nx` source directly into idiomatic **Swift (SwiftUI)** for iOS and **Kotlin (Jetpack Compose)** for Android with zero runtime reflection.

---

## 1. Type System

Nexa features a strict, sound static type system with type inference for local variables and state initializers.

### Primitive Scalar Types

| Nexa Type | Swift Target | Kotlin Target | Size / Representation | Example Literal |
|---|---|---|---|---|
| `String` | `String` | `String` | UTF-8 encoded text | `"Hello, Nexa!"` |
| `Bool` | `Bool` | `Boolean` | 1-byte logical boolean | `true`, `false` |
| `Int8` | `Int8` | `Byte` | 8-bit signed integer | `127` |
| `Int16` | `Int16` | `Short` | 16-bit signed integer | `32767` |
| `Int32` | `Int32` | `Int` | 32-bit signed integer (Default) | `42` |
| `Int64` | `Int64` | `Long` | 64-bit signed integer | `10000000000` |
| `UInt8` | `UInt8` | `UByte` | 8-bit unsigned integer | `255` |
| `UInt16` | `UInt16` | `UShort` | 16-bit unsigned integer | `65535` |
| `UInt32` | `UInt32` | `UInt` | 32-bit unsigned integer | `100000` |
| `UInt64` | `UInt64` | `ULong` | 64-bit unsigned integer | `18446744073709551615` |
| `Float32` | `Float` | `Float` | 32-bit IEEE 754 floating point | `3.14` |
| `Float64` | `Double` | `Double` | 64-bit IEEE 754 (Default float) | `0.000001` |
| `Bytes` | `Data` | `ByteArray` | Owned contiguous byte buffer | `Bytes([0x01, 0x02])` |
| `Void` | `Void` | `Unit` | Absence of value | Return type only |

> [!NOTE]
> Integer literals default to `Int32`, while decimal literals default to `Float64` unless an explicit type annotation is supplied.

---

## 2. Declarations

Nexa code is organized into top-level declarations: `app`, `screen`, `component`, `class`, `struct`, `enum`, and `fn`.

### 2.1 Structs (Value Types)

Structs are immutable, plain data transfer records with automatic field-wise equality and constructors:

```nexa
struct UserProfile {
    id: Int64,
    name: String,
    avatarUrl: String?,
    isVerified: Bool,
    reputation: Float64
}

let user = UserProfile(1, "Alex", null, true, 4.95)
let hasAvatar = user.avatarUrl != null
```

### 2.2 Enums & Tagged Variants

Enums declare distinct choices or tagged payload variants:

```nexa
enum Status {
    active,
    pending,
    archived
}

enum NetworkResponse {
    success(data: String),
    failure(code: Int32, message: String)
}

let current = Status.active
```

### 2.3 Classes (Reference Types)

Classes own long-lived native handles, encapsulate state, and group behavior:

```nexa
class SessionManager(val sessionId: String) {
    static let maxRetries: Int32 = 3
    let store = MMKV.MMKVStore(sessionId, null, false)

    fn getAuthToken() -> String? {
        return this.store.getString("auth_token")
    }

    static fn defaultSession() -> SessionManager {
        return SessionManager("default_session")
    }
}

let session = SessionManager.defaultSession()
let token = session.getAuthToken()
```

| Class Feature | Syntax | Target Swift Mapping | Target Kotlin Mapping |
|---|---|---|---|
| **Primary Constructor** | `class Name(val param: Type)` | `init(param: Type)` | `class Name(val param: Type)` |
| **Instance Property** | `let property: Type = init` | Stored `let` member | Stored `val` property |
| **Instance Method** | `fn method() -> Type { ... }` | Direct instance function | Direct member function |
| **Static Property** | `static let name: Type = init` | `static let name` | `companion object { const val }`|
| **Static Function** | `static fn name() -> Type` | `static func name()` | `companion object { fun name() }`|

### 2.4 State vs. Let

- `let`: Immutable local binding or constant. Assigned once, cannot be mutated.
- `state`: Reactive mutable state. Modifying a `state` variable automatically triggers fine-grained UI re-rendering.

```nexa
app CounterApp {
    state counter: Int32 = 0
    let maxLimit: Int32 = 100

    body {
        Button("Add") {
            if counter < maxLimit {
                counter = counter + 1
            }
        }
    }
}
```

---

## 3. Compound & Collection Types

Nexa provides a rich set of strongly typed collections with zero type erasure in generated Swift and Kotlin.

### 3.1 `Array<T>`

Ordered, dynamic sequence of values:

```nexa
let scores: Array<Int32> = [95, 88, 72, 100]
```

#### Array Operations Reference

| Property / Method | Signature | Description |
|---|---|---|
| `count` | `Int32` | Returns the number of elements in the array. |
| `isEmpty` | `Bool` | Returns `true` if `count == 0`. |
| `first()` | `() -> T?` | Returns the first element or `null` if empty. |
| `last()` | `() -> T?` | Returns the last element or `null` if empty. |
| `push(element)` | `(T) -> Void` | Appends an element to the end of a mutable state array. |
| `remove(index)` | `(Int32) -> T` | Removes and returns the element at the specified index. |
| `filter(predicate)` | `((T) -> Bool) -> Array<T>` | Returns elements satisfying the predicate. |
| `map(transform)` | `((T) -> R) -> Array<R>` | Transforms elements into a new typed array. |
| `sortedBy(selector)`| `((T) -> Comparable) -> Array<T>` | Returns a copy sorted by the selector value. |
| `reversed()` | `() -> Array<T>` | Returns elements in reverse order. |
| `contains(element)` | `(T) -> Bool` | Checks for element presence. |
| `move(from, to)` | `(Int32, Int32) -> Void` | Moves element from one index to another. |
| `moveSubset(from, to, subset)` | `(Int32, Int32, Array<T>) -> Void` | Moves subset elements while preserving hidden filter rows. |

### 3.2 `Set<T>`

Unordered collection of unique values:

```nexa
state tags: Set<String> = ["swift", "kotlin", "nexa"]

tags.insert("mobile")
tags.remove("swift")
let hasNexa = tags.contains("nexa")
```

### 3.3 `Map<K, V>`

Key-value dictionary where keys must be hashable scalars:

```nexa
state cache: Map<String, Int32> = ["item_1": 100, "item_2": 200]

let item = cache.get("item_1") // Optional Int32?
cache.set("item_3", 300)
cache.remove("item_2")
```

### 3.4 `Pair<A, B>` and `Triple<A, B, C>`

Fixed-size, strongly typed tuples:

```nexa
let coordinate: Pair<Float64, Float64> = Pair(37.7749, -122.4194)
let lat = coordinate.first
let lon = coordinate.second

let item: Triple<String, Int32, Bool> = Triple("Order #10", 42, true)
let isPaid = item.third
```

---

## 4. Null Safety & Optionals

All types in Nexa are non-nullable by default. Append `?` to declare an optional type (`Type?`):

```nexa
let username: String? = null
let validName: String? = "Alice"
```

### Safe Navigation & Null Coalescing

```nexa
// 1. Safe access on compound properties
let length = username?.length

// 2. Null coalescing fallback
let displayName: String = username ?? "Guest"

// 3. Explicit check
if username != null {
    Log.info(message: username)
}
```

---

## 5. Explicit Error Handling: `Result<T, E>`

Nexa rejects unchecked hidden exceptions. Functions that can fail return typed `Result<T, E>` values:

```nexa
enum PaymentError {
    insufficientFunds,
    cardExpired,
    networkError(message: String)
}

fn processPayment(amount: Float64, balance: Float64) -> Result<Float64, PaymentError> {
    if amount > balance {
        return Result.Err(PaymentError.insufficientFunds)
    }
    return Result.Ok(balance - amount)
}
```

### Error Recovery: `try / catch` and the `?` Operator

```nexa
// Postfix '?' operator automatically unwraps or returns early on failure
fn executeOrder(cost: Float64, balance: Float64) -> Result<Float64, PaymentError> {
    let remaining = processPayment(cost, balance)?
    return Result.Ok(remaining)
}

// Exhaustive pattern matching recovery
try {
    let newBalance = processPayment(100.0, 50.0)?
} catch {
    case PaymentError.insufficientFunds {
        Dialog.alert(title: "Payment Failed", message: "Insufficient balance.")
    }
    case PaymentError.networkError(message) {
        Dialog.alert(title: "Connection Error", message: message)
    }
    else {
        Dialog.alert(title: "Error", message: "Transaction failed.")
    }
}
```

---

## 6. Built-in Core APIs

Nexa bundles zero-cost core modules implemented directly on native platform APIs:

### 6.1 `Storage` (Lightweight Key-Value)

```nexa
Storage.setString("theme", value: "dark")
let theme: String? = Storage.getString("theme")
Storage.remove("theme")
Storage.clear()
```

### 6.2 `Crypto` (Platform Security APIs)

```nexa
let hash: String = Crypto.sha256("secure_password")
let signature: String = Crypto.hmacSha256("secret_key", message: "payload")
let randomToken: Bytes = Crypto.randomBytes(32)
```

### 6.3 `Clipboard` (System Pasteboard)

```nexa
Clipboard.copy("Copied share link")
let isAvailable: Bool = Clipboard.hasText()
let content: String? = Clipboard.paste()
```

### 6.4 `Time` & `Number` (Formatting & Locale)

```nexa
let nowTimestamp: Int64 = Time.now()
let formattedDate: String = Time.formatDate(nowTimestamp, format: "yyyy-MM-dd")
let dayStart: Int64 = Time.startOfDay(nowTimestamp)

let currency: String = Number.formatCurrency(1234.50, currencyCode: "USD") // "$1,234.50"
let decimal: String = Number.formatDecimal(3.14159, maxFractions: 2)     // "3.14"
```

---

## 7. In-Language Unit & Component Testing

Declare test suites directly in any `.nx` file:

```nexa
fn calculateDiscount(total: Float64, percent: Float64) -> Float64 {
    return total * (1.0 - (percent / 100.0))
}

test "cart discount calculation" {
    let discounted = calculateDiscount(100.0, 20.0)
    assert(discounted == 80.0)
    assert(discounted > 0.0, "Discounted total must be positive")
}
```

### In-Memory Component Interaction Tests

```nexa
test "increment button increases state" for CounterApp() {
    assertText("Count: 0")
    tap("Add")
    assertText("Count: 1")
}
```

Execute tests via CLI:

```bash
nexa test               # Runs tests and builds native test hosts
nexa test --unit-only   # Lightning-fast evaluation without native toolchains
```
