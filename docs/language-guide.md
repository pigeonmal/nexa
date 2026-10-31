# Nexa Language Guide 📖

> [!NOTE]
> **Scope:** Core `.nx` language syntax and typed standard APIs. UI components are listed in the [component guide](components.md), animation is documented in the [animation guide](animations.md), and the full checked component catalog is in the [syntax audit](syntax-audit.md).

| **Scope**: language and standard APIs | **Targets**: iOS and Android | **Errors**: typed `Result` and `throws` |

**Quick start:** A Nexa app declares typed state and a `body`; UI values and actions compile to native SwiftUI and Jetpack Compose.

```nexa
app ReadingQueue {
    state count: Int32 = 0

    body {
        Column {
            Text("Books read: \(count)", fontSize: 24, fontWeight: Bold)
            Button("Finish a book") {
                count = count + 1
            }
        }
    }
}
```

**Nexa** is a statically-typed, ahead-of-time (AOT) compiled language designed specifically for building cross-platform native mobile applications. It compiles `.nx` source directly into idiomatic **Swift (SwiftUI)** for iOS and **Kotlin (Jetpack Compose)** for Android with zero runtime reflection.

---

## 1. Type System

Nexa features a strict, sound static type system with type inference for local variables and state initializers.

### Primitive Scalar Types

> The Swift and Kotlin columns are generated from the backends' own type functions
> (`swift_type` / `kotlin_type`), so they cannot disagree with the code the compiler emits.
> Run `cargo test -p nexa-cli --test language_reference` with `NEXA_UPDATE_SNAPSHOTS=1` to
> regenerate.

<!-- nexadoc:begin scalar-types -->
| Nexa Type | Swift Target | Kotlin Target | Size / Representation | Example Literal |
|---|---|---|---|---|
| `String` | `String` | `String` | UTF-8 encoded text | `"Hello, Nexa!"` |
| `Bool` | `Bool` | `Boolean` | 1-byte logical boolean | `true`, `false` |
| `Int8` | `Int8` | `Byte` | 8-bit signed integer | `127` |
| `Int16` | `Int16` | `Short` | 16-bit signed integer | `32767` |
| `Int32` | `Int32` | `Int` | 32-bit signed integer (default integer literal) | `42` |
| `Int64` | `Int64` | `Long` | 64-bit signed integer | `10000000000` |
| `UInt8` | `UInt8` | `UByte` | 8-bit unsigned integer | `255` |
| `UInt16` | `UInt16` | `UShort` | 16-bit unsigned integer | `65535` |
| `UInt32` | `UInt32` | `UInt` | 32-bit unsigned integer | `100000` |
| `UInt64` | `UInt64` | `ULong` | 64-bit unsigned integer | `18446744073709551615` |
| `Float32` | `Float` | `Float` | 32-bit IEEE 754 floating point | `3.14` |
| `Float64` | `Double` | `Double` | 64-bit IEEE 754 (default decimal literal) | `0.000001` |
| `Bytes` | `Data` | `ByteArray` | Owned contiguous byte buffer | `Bytes.fromText(text: "receipt")` |
| `Void` | `Void` | `Unit` | Absence of value | Return type only |
<!-- nexadoc:end scalar-types -->

> [!NOTE]
> Integer literals default to `Int32`, while decimal literals default to `Float64` unless an explicit type annotation is supplied.

### Numeric operation behavior

| Operation | Integer behavior | Floating-point behavior |
|---|---|---|
| `+`, `-`, `*`, unary `-` | Wrap at the declared bit width. | Use the native IEEE 754 operation. |
| `/` | Truncate toward zero. Signed minimum divided by `-1` wraps to the minimum value. | Use the native IEEE 754 operation. |
| `%` | Return the remainder after division truncates toward zero. Signed minimum remainder `-1` is `0`. | Return the truncating remainder, matching Kotlin's `%` behavior. |

For integer division and remainder, a zero divisor causes a runtime arithmetic failure. Floating-point operations follow IEEE 754, including its behavior for zero divisors.

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

app ProfileSummary {
    let user = UserProfile(1, "Alex", null, true, 4.95)
    let hasAvatar = user.avatarUrl != null

    body {
        Column {
            Text(user.name)
            if hasAvatar {
                Text("Profile photo available")
            } else {
                Text("No profile photo")
            }
        }
    }
}
```

### 2.2 Enums & Tagged Variants

Enums declare a fixed set of choices:

```nexa
enum Status {
    active,
    pending,
    archived,
}

app SubscriptionStatus {
    let current = Status.active

    body {
        if current == Status.active {
            Text("Subscription is active")
        }
    }
}
```

App enums are closed sets of cases and do not carry payloads. Typed plugin errors declared in `.nxid` may have payloads.

### 2.3 Classes (Reference Types)

Classes own long-lived native handles, encapsulate state, and group behavior:

```nexa
class GreetingRepository(val prefix: String) {
    fn greeting(name: String) -> String {
        return prefix + name
    }
}

app Welcome {
    let repository = GreetingRepository("Welcome, ")

    body {
        Text(repository.greeting("Ari"))
    }
}
```

| Class Feature | Syntax | Target Swift Mapping | Target Kotlin Mapping |
|---|---|---|---|
| **Primary Constructor** | `class Name(val param: Type)` | Immutable stored property | Immutable stored property |
| **Instance Property** | `let property: Type = init` | Stored `let` member | Stored `val` property |
| **Instance Method** | `fn method() -> Type { ... }` | Direct instance function | Direct member function |
| **Static Property** | `static let name: Type = init` | Static stored property | Companion object property |
| **Static Function** | `static fn name() -> Type` | Static function | Companion object function |

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

### 2.5 Functions, loops, and asynchronous work

Functions declare parameter and return types. They can be top-level helpers or members of an `app`, `screen`, `component`, or `class`. A `for` loop iterates typed arrays and sets; `return` can exit from inside the loop.

```nexa
struct Delivery { id: Int32, title: String, delivered: Bool }

fn firstPendingDelivery(deliveries: Array<Delivery>) -> Delivery? {
    for delivery in deliveries {
        if !delivery.delivered {
            return delivery
        }
    }
    return null
}

app DeliveryQueue {
    let deliveries = [
        Delivery(301, "Pack the customer order", false),
        Delivery(302, "Send dispatch confirmation", true)
    ]
    let nextDelivery = firstPendingDelivery(deliveries)

    body {
        Text(nextDelivery?.title ?? "All deliveries are complete")
    }
}
```

Use `async fn`, `await`, and a task handle for work that outlives an event handler. This example requests a service response and updates UI state on the main executor; a thrown request failure follows the `catch` path.

```nexa
app WeatherRefresh {
    state message: String = "Not refreshed"
    state refreshTask: TaskHandle? = null

    body {
        Column {
            Text(message)
            Button("Refresh order status") {
                Task.launch(handle: refreshTask, executor: TaskExecutor.Main) {
                    try {
                        let response = await Network.fetch(url: "https://api.open-meteo.com/v1/forecast?latitude=48.8566&longitude=2.3522&current=temperature_2m")
                        message = "Weather service responded with HTTP \(response.statusCode)"
                    } catch {
                        message = "Could not refresh. Check your connection."
                    }
                }
            }
        }
    }
}
```

`Task.cancel(handle:)` cancels a stored task handle. Avoid blocking the UI thread while awaiting network, database, or media plugin work.

Tasks belong to the view that launches them. A `TaskHandle` lets code cancel a task explicitly; a launch without a handle is fire-and-forget. Nexa cancels either kind when its owning view disappears on both platforms. A task launched by `FastList.onEndReached` belongs to that list, so removing the list cancels its pagination work even if its screen stays visible. Cancellation is cooperative, so an in-flight native API or plugin must support cancellation to stop its underlying work.

Nexa async functions suspend in the task that called them; they do not create a second task. Cancellation therefore continues through awaited Nexa functions and native calls unless code catches and suppresses it. The native spelling and task launch mapping are:

| Nexa | iOS output | Android output |
|---|---|---|
| `async fn loadProfile() -> Void throws` | `@MainActor func loadProfile() async throws -> Void` | `suspend fun loadProfile()`; failures propagate as exceptions |
| `await loadProfile()` | `try await loadProfile()` when it can throw | `loadProfile()` inside a suspend function or coroutine |
| `Task.launch(handle: refreshTask, executor: TaskExecutor.Main)` | `Task { @MainActor in ... }` | `launch(Dispatchers.Main.immediate) { ... }` |
| `Task.launch(handle: refreshTask, executor: TaskExecutor.Background)` | `Task.detached { ... }` | `launch(Dispatchers.Default) { ... }` |
| `Task.cancel(handle: refreshTask)` or remove its owner | `Task.cancel()`; owner scope cancels on disappearance | `Job.cancel()`; owner coroutine scope cancels on disposal |

Use `TaskExecutor` to choose where a launched task starts. An `async fn` alone does not move CPU work off the UI executor; use a background task for CPU-heavy work and keep blocking I/O inside an API or plugin that handles its native I/O dispatcher.

Use `when` to show one UI branch for a value. A closed enum can list every case without `else`; a `Bool` can use both `true` and `false`. Partial matches, strings, and numbers need an explicit `else` because they may have values the listed cases do not cover.

```nexa
enum DeliveryStatus { preparing, shipped, delivered }

app DeliveryStatusCard {
    state status: DeliveryStatus = DeliveryStatus.preparing

    body {
        when status {
            DeliveryStatus.preparing: { Text("Your order is being packed") }
            DeliveryStatus.shipped: { Text("Your order is on the way") }
            DeliveryStatus.delivered: { Text("Your order arrived") }
        }
    }
}
```

When the cases are complete, Nexa emits an exhaustive native `switch` on iOS and `when` on Android. Add `else: { ... }` when a listed value set is incomplete.

---

## 3. Compound & Collection Types

Nexa provides a rich set of strongly typed collections with zero type erasure in generated Swift and Kotlin.

### 3.1 `Array<T>`

Ordered, dynamic sequence of values:

```nexa
app QuizResults {
    let scores: Array<Int32> = [95, 88, 72, 100]

    body {
        Text("Scores recorded: \(scores.count)")
    }
}
```

#### Array Operations Reference

| Property / Method | Signature | Description |
|---|---|---|
| `count` | `Int32` | Returns the number of elements in the array. |
| `isEmpty` | `Bool` | Returns `true` if `count == 0`. |
| `first()` | `() -> T?` | Returns the first element or `null` if empty. |
| `last()` | `() -> T?` | Returns the last element or `null` if empty. |
| `append(element)` | `(T) -> Void` | Appends an element to the end of a mutable state array. |
| `remove(index)` | `(Int32) -> Void` | Removes an element from a mutable state array by index. |
| `filter(predicate)` | `((T) -> Bool) -> Array<T>` | Returns elements satisfying the predicate. |
| `map(transform)` | `((T) -> R) -> Array<R>` | Transforms elements into a new typed array. |
| `sortedBy(selector)`| `((T) -> Comparable) -> Array<T>` | Returns a copy sorted by the selector value. |
| `reverse()` | `() -> Array<T>` | Returns a copy with elements in reverse order. |
| `in` | `T in Array<T>` | Checks whether an array contains a value. |
| `shuffled()` | `() -> Array<T>` | Returns a shuffled copy. |
| `random()` | `() -> T?` | Returns a random element or `null` for an empty array. |
| `slice(range)` | `(Range<Int32>) -> Array<T>` | Returns the selected range; use `..<` for an exclusive end. |
| `take(count)` | `(Int32) -> Array<T>` | Returns up to the requested number of leading elements. |
| `flatMap(transform)` | `((T) -> Array<R>) -> Array<R>` | Maps each item to an array and flattens the result. |
| `reduce(initial, combine)` | `(R, (R, T) -> R) -> R` | Folds an array into one value. |
| `groupedBy(selector)` | `((T) -> K) -> Array<Array<T>>` | Groups values by a string or numeric key. |
| `move(from, to)` | `(Int32, Int32) -> Void` | Moves element from one index to another. |
| `moveSubset(from, to, subset)` | `(Int32, Int32, Array<T>) -> Void` | Moves subset elements while preserving hidden filter rows. |

`count` and `isEmpty` are read-only properties. `append`, `remove`, `move`, and `moveSubset` mutate array state; the other operations return values.

On an empty array, `count` is `0`, `isEmpty` is `true`, and `first()`, `last()`, and `random()` return `null`.
`filter`, `map`, `sortedBy`, `reverse`, `shuffled`, `take`, `flatMap`, and `groupedBy` return an empty array.
`reduce(initial, combine)` returns `initial`; `slice` returns an empty array for a valid empty range. A negative `take` count is treated as zero.

### 3.2 `Set<T>`

Unordered collection of unique values:

```nexa
app TopicPicker {
    state tags: Set<String> = ["swift", "kotlin", "nexa"]

    body {
        Column {
            Text(if ("nexa" in tags) { "Nexa selected" } else { "Select Nexa" })
            Button("Add mobile") {
                tags.insert("mobile")
            }
            Button("Remove Swift") {
                tags.remove("swift")
            }
        }
    }
}
```

### 3.3 `Map<K, V>`

Key-value dictionary where keys must be hashable scalars:

```nexa
app CartQuantities {
    state quantities: Map<String, Int32> = ["sku-441": 2, "sku-807": 1]

    body {
        Column {
            Text("Insulated Bottle")
            Text(quantities["sku-441"] ?? 0)
            Button("Add one bottle") {
                quantities.set("sku-441", (quantities["sku-441"] ?? 0) + 1)
            }
            Button("Remove travel mug") {
                quantities.remove("sku-807")
            }
        }
    }
}
```

| Member | Signature | Behavior |
|---|---|---|
| `map[key]` | `K -> V?` | Returns the value for a key, or `null` when it is absent. |
| `get(key)` | `(K) -> V?` | Named equivalent of indexed lookup. |
| `contains(key)` | `(K) -> Bool` | Checks whether a key is present. |
| `keys` | `Array<K>` | Returns the current keys. |
| `values` | `Array<V>` | Returns the current values. |
| `keys()` / `values()` | `() -> Array<K>` / `() -> Array<V>` | Named equivalents of the `keys` and `values` properties. |
| `count` | `Int32` | Number of key-value entries. |
| `isEmpty` | `Bool` | Whether the map has no entries. |
| `set(key, value)` | `(K, V) -> Void` | Inserts or replaces one entry in mutable Map state. |
| `remove(key)` | `(K) -> Void` | Removes the entry for a key from mutable Map state. |
| `clear()` | `() -> Void` | Removes every entry from mutable Map state. |

Map mutations require mutable state. Keys must be hashable scalar values; map state compiles to Swift dictionaries on iOS and snapshot state maps on Android.

### 3.4 String methods

String values support the `trimmed` property and the `trim()`, `toLowercase()`, `toUppercase()`, `normalizeNFKD()`, `split(delimiter)`, `contains(value)`, `startsWith(prefix)`, and `endsWith(suffix)` methods. Cleanup and normalization methods return `String`; `split` returns `Array<String>`; the final three return `Bool`.

`normalizeNFKD()` applies Unicode compatibility decomposition without removing combining marks. On Swift it uses Foundation's compatibility decomposition; on Kotlin it uses `java.text.Normalizer.Form.NFKD`.

```nexa
fn normalizedTrackTitle(title: String) -> String {
    let decomposed = title.normalizeNFKD()
    let unaccented = Regex.replace(pattern: "[\\u0300-\\u036f]", text: decomposed, replacement: "")
    let lowercase = unaccented.toLowercase().trim()
    return Regex.replace(pattern: "[^\\p{Letter}\\p{Number}]+", text: lowercase, replacement: " ").trim()
}
```

### 3.5 Regular expressions

Create a reusable expression with `Regex(pattern:)` or the `/pattern/flags` literal form. Portable flags are `i` for case-insensitive matching, `m` for line anchors, and `s` for dot matching newlines. `matches` requires the whole input to match; `find` and `findAll` return typed match values. Captured groups omit the full match and retain `null` for an unmatched optional group.

```nexa
app OrderReferenceSearch {
    state expression: Regex = /([A-Z]{2})-([0-9]{4})/i
    state first: RegexMatch? = null
    state references: Array<RegexMatch> = []
    state summary: String = "Search has not run"

    body {
        Column(spacing: 12) {
            Text(summary)
            Button("Find order references") {
                let receipt = "Packed orders: AB-1042 and CD-7715"
                first = expression.find(text: receipt)
                references = expression.findAll(receipt)
                summary = "Found \(references.count) order references"
            }
            if first != null {
                Text("First reference: \(first?.value ?? "No order found")")
            }
        }
    }
}
```

| Member | Signature | Behavior |
|---|---|---|
| `Regex(pattern:)` | `(String) -> Regex` | Compiles a reusable native regular expression. Invalid patterns produce an inert expression: matches are false, searches are empty, and replacement returns the input unchanged. |
| `/pattern/ims` | `Regex` literal | Creates a reusable expression. `i`, `m`, and `s` enable case-insensitive matching, line anchors, and dot matching newlines; duplicate or unsupported flags are diagnosed. |
| `matches(text:)` | `(String) -> Bool` | Returns true only when the expression matches the entire string. |
| `find(text:)` | `(String) -> RegexMatch?` | Returns the first match, or `null`. |
| `findAll(text:)` | `(String) -> Array<RegexMatch>` | Returns each non-overlapping match in input order. |
| `replace(text:with:)` | `(String, String) -> String` | Replaces each match using the platform's native replacement syntax. |
| `RegexMatch.value` | `String` | Full matched substring. |
| `RegexMatch.range` | `Range` | Half-open UTF-16 code-unit offsets into the input. |
| `Range.lowerBound` | `Int64` | Inclusive start offset in UTF-16 code units. |
| `Range.upperBound` | `Int64` | Exclusive end offset in UTF-16 code units. |
| `RegexMatch.groups` | `Array<String?>` | Capturing groups, excluding the full match; unmatched optional groups are `null`. |

Use common regular-expression syntax supported by Foundation's `NSRegularExpression` and Kotlin's `Regex`. The portable flags map to each native engine's inline options; platform-specific extensions can still behave differently.

### 3.6 `Pair<A, B>` and `Triple<A, B, C>`

Fixed-size, strongly typed tuples:

```nexa
app OrderReceipt {
    let coordinate: Pair<Float64, Float64> = Pair(37.7749, -122.4194)
    let item: Triple<String, Int32, Bool> = Triple("Order #10", 42, true)
    let isPaid = item.third

    body {
        Column {
            Text("\(item.first): \(item.second) items")
            Text("Pickup latitude: \(coordinate.first)")
            if isPaid { Text("Payment complete") }
        }
    }
}
```

---

## 4. Null Safety & Optionals

All types in Nexa are non-nullable by default. Append `?` to declare an optional type (`Type?`):

```nexa
app OptionalName {
    state username: String? = null

    body {
        Text(username ?? "Guest")
    }
}
```

### Safe Navigation & Null Coalescing

```nexa
struct Reader { name: String }

app ProfileHeader {
    state reader: Reader? = Reader("Mina")
    state username: String? = null

    body {
        Column {
            Text(reader?.name ?? "Guest reader")
            Text(username ?? "Guest")
            if username != null { Text(username ?? "") }
        }
    }
}
```

---

## 5. Explicit Error Handling: `Result<T, E>` and `throws`

Choose the error style by the kind of failure. Nexa keeps both paths visible in
the function signature and requires the caller to handle failures:

| Use | For | Caller |
|---|---|---|
| `Result<T, E>` | Expected outcomes in your app, such as insufficient funds or invalid input | Return `Ok(value)` or `Err(error)`; use `?` inside another function that returns a compatible `Result` |
| `throws` | A native operation can fail because of the OS, network, filesystem, secure storage, or a plugin | Wrap the call in `try { ... } catch { ... }` and show or recover from the error at the action or task boundary |

Do not use both styles on the same component-ref method. The compiler rejects
that combination. A plugin `.nxid` method with `throws` names its typed native
error so the catch can match the declared cases.

> [!NOTE]
> Core APIs such as `Network.fetch`, `File.readText`, and `SecureStorage.get`
> use a catch-all for native failures; platform exception types are not part of
> Nexa's public API. Plugin errors can use typed cases. Cancellation always
> propagates, even through a catch-all block.

Use `Result` when failure is a normal answer to a business operation:

```nexa
enum PaymentError { insufficientFunds, cardExpired }

fn processPayment(amount: Float64, balance: Float64) -> Result<Float64, PaymentError> {
    if amount > balance {
        return Err(PaymentError.insufficientFunds)
    }
    return Ok(balance - amount)
}

app CheckoutPolicy {
    state paymentResult: String = "No payment checked"

    body {
        Column {
            Text(paymentResult)
            Button("Validate $25 order") {
                let result = processPayment(125.0, 100.0)
                paymentResult = result.isSuccess
                    ? "Paid: $\(result.value ?? 0.0)"
                    : (result.error == PaymentError.insufficientFunds
                        ? "Payment declined: insufficient funds"
                        : "Payment declined")
            }
        }
    }
}
```

### Propagating a `Result`: the `?` Operator

```nexa
// Postfix '?' operator automatically unwraps or returns early on failure
enum PaymentError { insufficientFunds, cardExpired }

fn processPayment(amount: Float64, balance: Float64) -> Result<Float64, PaymentError> {
    if amount > balance {
        return Err(PaymentError.insufficientFunds)
    }
    return Ok(balance - amount)
}

fn executeOrder(cost: Float64, balance: Float64) -> Result<Float64, PaymentError> {
    let remaining = processPayment(cost, balance)?
    return Ok(remaining)
}

app OrderPayment {
    state orderResult: String = "Ready to validate order"

    body {
        Column {
            Text(orderResult)
            Button("Submit $25 order") {
                orderResult = Json.stringify(value: executeOrder(25.0, 100.0))
            }
        }
    }
}
```

`?` returns the `Err` immediately, or unwraps the `Ok` value. It is valid only
inside a function that returns a compatible `Result`. At a UI boundary, use
`isSuccess` to choose the message, then read `value` or `error` as an optional
typed payload.

For external operations such as `Network.fetch` or `File.readText`, use the
throwing path and handle it where the user can recover:

```nexa
app ProfileRefresh {
    state refreshTask: TaskHandle? = null
    state profileText: String = "Profile is ready to refresh"

    body {
        Column {
            Button("Refresh profile") {
                Task.launch(handle: refreshTask, executor: TaskExecutor.Main) {
                    try {
                        let response = await Network.fetch(url: "https://api.example.com/profile")
                        profileText = response.body
                    } catch {
                        profileText = "Profile could not be loaded. Try again."
                    }
                }
            }
            Text(profileText)
        }
    }
}
```

Throwing native plugin methods use the same `try`/`catch` form; their typed
error variants come from the plugin's `.nxid` contract. See
[plugin error handling](plugins.md#async-throwing-calls) for a complete example.

## 6. Built-in Core APIs

Nexa maps these typed APIs to platform services. Calls that can fail are marked as throwing in the compiler and must be handled with `try`/`catch` or propagated from an async function.

### Responsive layout sizing

`Column`, `Row`, and `Stack` accept `screenWidthFraction: Float64` and `screenHeightFraction: Float64` from `0` through `1`. The native view uses those fractions of the current screen dimensions (points on iOS, `dp` on Android). Do not combine `screenWidthFraction` with `width` or `fillMaxWidth`, or `screenHeightFraction` with `height` or `fillMaxHeight`. `Layout.screenWidth` and `Layout.screenHeight` read the current app viewport in logical points or `dp`; `SafeArea.top` reads the remaining visual top inset in the same units. These values can drive animated style expressions. `Layout.isRegularWidth`, `Layout.isCompactWidth`, `Layout.isRegularHeight`, and `Layout.isCompactHeight` return `Bool` size-class checks.

This search overlay uses the viewport height and top inset to keep its collapsed shape aligned below the status area while it animates:

```nexa
app SearchOverlay {
    state progress: Float64 = 1.0
    body {
        Column(
            fillMaxWidth: true,
            fillMaxHeight: true,
            cornerRadius: (1.0 - progress) * 35.1,
            scaleY: (70.2 + progress * (Layout.screenHeight - 70.2)) / Layout.screenHeight,
            offsetY: (1.0 - progress) * (SafeArea.top - (Layout.screenHeight - 70.2) / 2.0)
        ) {
            Text("Search results")
        }
    }
}
```

```nexa
app ArtistHero {
    body {
        Stack(screenHeightFraction: 0.5, background: "#171717") {
            Image(
                url: "https://cdn.example.com/artists/artist-842/header.jpg",
                description: "Artist header artwork",
                scale: Fill,
                fillMaxSize: true
            )
        }
    }
}
```

### 6.1 `Storage` (Lightweight Key-Value)

`Storage` has direct getters and setters for common scalar values, plus `Storage.get<T>` and `Storage.set<T>` for any type supported by Nexa's generated JSON codecs, including structs and collections. `get<T>` returns `null` when the key is missing or the stored JSON cannot be decoded. A key stores one value type at a time; writing another type replaces its saved value. `Storage.delete` removes the key; `Storage.clear` removes every preference managed by Nexa.

See the complete [reader preferences example](../examples/typed_storage.nx) for optional defaults, typed writes, and deletion.

### 6.2 `Crypto` (Platform Security APIs)

```nexa
app ReceiptFingerprint {
    state fingerprint: String = "Not generated"
    state payloadByteCount: Int32 = 0

    body {
        Column {
            Text(fingerprint)
            Text("Receipt bytes: \(payloadByteCount)")
            Button("Fingerprint receipt text") {
                let payload: Bytes = Bytes.fromText(text: "Order 4481: 2 travel mugs")
                fingerprint = Crypto.sha256(text: "Order 4481: 2 travel mugs")
                payloadByteCount = Bytes.count(bytes: payload)
            }
        }
    }
}
```

### 6.3 `Clipboard` (System Pasteboard)

```nexa
app ShareInvite {
    state copiedText: String = "Nothing copied yet"

    body {
        Column {
            Text(copiedText)
            Button("Copy reading group invite") {
                Clipboard.setText(text: "https://example.com/reading-group/7F2A")
                copiedText = Clipboard.getText() ?? "Clipboard is empty"
            }
        }
    }
}
```

### 6.4 `Time` & `Number` (Formatting & Locale)

```nexa
app OrderSummary {
    let placedAt: Int64 = Time.now()
    let formattedDate: String = Time.format(timestamp: placedAt, pattern: "yyyy-MM-dd")
    let announcementAt: Int64 = Time.iso8601ToMillis(text: "2026-09-06T00:00:00Z") ?? placedAt
    let announcementDate: String = Time.localizedLongDate(timestamp: announcementAt, locale: Locale.currentLanguageCode())
    let trackLength: String = Time.formatDuration(seconds: 202.75)
    let total: String = Number.formatCurrency(amount: 1234.50, currencyCode: "USD")

    body {
        Column {
            Text("Order placed: \(formattedDate)")
            Text("Track length: \(trackLength)")
            Text("Total: \(total)")
            Text("Local date: \(Time.localizedDate(timestamp: placedAt))")
            Text("Announcement date: \(announcementDate)")
        }
    }
}
```

Time APIs use milliseconds for Unix timestamps. `Time.now()` is wall-clock time and may change when the system clock is corrected; use `Time.monotonic()` with `Time.elapsed(since:)` to measure durations.

`Time.iso8601()` always formats UTC as `yyyy-MM-ddTHH:mm:ss.SSSZ`. `Time.iso8601ToMillis()` accepts that format or the same format without the fractional part; it requires `Z`, rejects numeric offsets, and returns `null` for invalid dates.

`Time.startOfDay()`, `Time.addCalendarDays()`, and `Time.format()` use the device's current calendar and time zone. `Time.localizedLongDate(timestamp:locale:)` uses the supplied app language with the device calendar and time zone. Calendar days are not fixed 24-hour periods, so a day across a daylight-saving change can be 23 or 25 hours. `Time.format()` uses ICU date patterns such as `yyyy-MM-dd`; `MM` is the month and `mm` is the minute.

`Time.formatDuration(seconds:)` returns a fixed `mm:ss` string without locale-sensitive date formatting. It floors fractional seconds, pads single-digit minutes and seconds with zeroes, and returns `00:00` for zero, negative, or non-finite values.

The complete set of built-in method names and argument spellings is generated from the parser catalog in the [syntax audit](syntax-audit.md#vocabulary).

### Secure storage

Use `SecureStorage` for small secrets such as refresh tokens. Its asynchronous operations can fail and must be handled with `try`/`catch`. Pass `scope` to keep feature data separate; `clear(scope:)` removes only that feature's values, while `clear()` removes all secure values managed by the app. Omitting `scope` uses the existing app-wide key space.

```nexa
app SessionVault {
    state status: String = "No secure session loaded"
    state secureTask: TaskHandle? = null
    state sessionToken: String = ""

    body {
        Column {
            TextInput(value: sessionToken, placeholder: "Token returned by sign-in")
            Text(status)
            Button("Save session token") {
                Task.launch(handle: secureTask, executor: TaskExecutor.Main) {
                    try {
                        await SecureStorage.set(key: "refresh-token", value: sessionToken, scope: "auth")
                        status = "Session token saved securely"
                    } catch {
                        status = "Secure storage is unavailable on this device"
                    }
                }
            }
            Button("Sign out") {
                Task.launch(handle: secureTask, executor: TaskExecutor.Main) {
                    try {
                        await SecureStorage.clear(scope: "auth")
                        sessionToken = ""
                        status = "Authentication data removed"
                    } catch {
                        status = "Authentication data could not be removed"
                    }
                }
            }
        }
    }
}
```

### Files and permissions

`File.readText`, `File.writeText`, `File.readBytes`, `File.writeBytes`, and `File.delete` are asynchronous and can fail, so handle them in `try`/`catch`. Byte methods preserve file contents exactly; use them for attachments, images, and other binary data. Declare a user-facing purpose message in `nexa.config.nx` for each system permission your app requests.

```nexa
app ReadingNotes {
    state notes: String = ""
    state status: String = "No local note loaded"
    state saveTask: TaskHandle? = null

    body {
        Column(spacing: 12) {
            Text(status)
            Text(notes)
            Button("Save today's reading note") {
                Task.launch(handle: saveTask, executor: TaskExecutor.Main) {
                    try {
                        let path = Path.join(Path.documents(), "reading-note.txt")
                        let saved = await File.writeText(path: path, contents: "Next up: finish chapter 8")
                        if saved {
                            notes = await File.readText(path: path)
                            status = "Reading note saved"
                        } else {
                            status = "Could not save the note"
                        }
                    } catch {
                        status = "Could not read or write the note"
                    }
                }
            }
        }
    }
}
```

```nx
app CameraPermissionPrompt {
    state permissionStatus: PermissionStatus = PermissionStatus.notDetermined
    state permissionTask: TaskHandle? = null

    body {
        Column(spacing: 12) {
            Button("Enable order-label scanning") {
                Task.launch(handle: permissionTask, executor: TaskExecutor.Main) {
                    permissionStatus = await Permissions.request(permission: Camera)
                }
            }
            if permissionStatus == PermissionStatus.granted {
                Text("Camera access granted")
            } else {
                Text("Camera access is not available")
            }
        }
    }
}
```

Declare each permission used by the app in `nexa.config.nx`. The configured message becomes the iOS usage description; Nexa adds the matching Android manifest permission. A reachable native plugin can provide the platform declaration in its plugin manifest. `nexa check` reports missing declarations, warns with `NEXA-W1011` when a configured permission is unused, and warns with `NEXA-W1012` when the permission is selected at runtime and cannot be checked statically. Use `--deny-warnings` to make either warning fail the check.

`PermissionStatus` preserves the extra access states the operating system exposes:

| Value | Meaning |
|---|---|
| `granted` | Full access is available. |
| `limited` | Access is limited to selected photos or contacts. |
| `provisional` | iOS can deliver notifications quietly before the user makes a full choice. |
| `denied` | Access is denied and the system may still allow another request. |
| `permanentlyDenied` | Android has recorded at least two denials and the foreground activity reports that another prompt is unavailable; send the user to system Settings. |
| `restricted` | Device policy or parental controls block access. |
| `notDetermined` | The app has not received an authorization choice. |

`limited` is returned for iOS photo/contact access and Android 14 selected-photo access. `provisional` is returned for iOS notifications. Android reports `permanentlyDenied` after repeated denials when its foreground activity says another prompt is unavailable; iOS keeps its denied state as `denied` because users can change it in Settings.

### JSON encoding

`Json.parse<T>` returns `Result<T, JsonError>`; `Json.stringify<T>` produces a JSON string from a statically typed value.

```nexa
struct ReaderSettings { theme: String, dailyGoal: Int32 }

app SettingsBackup {
    state raw: String = "{\"theme\":\"dark\",\"dailyGoal\":30}"
    state status: String = "Ready to validate settings"

    body {
        Column(spacing: 12) {
            Text(status)
            Button("Validate and re-encode") {
                let parsed: Result<ReaderSettings, JsonError> = Json.parse<ReaderSettings>(raw: raw)
                raw = Json.stringify(value: parsed)
                status = "Typed parse result encoded as JSON"
            }
        }
    }
}
```

`JsonError` cases are `invalidJson`, `typeMismatch`, `missingField`, and `invalidValue`.

### Core API reference

These calls are built into the compiler and need no plugin declaration. Asynchronous calls require `await`; `throws` calls require `try`/`catch` or propagation.

Pass request headers as a simple `Map<String, String>`. Use `getHeader(name:)` for the first matching value (`String?`, or `null` when missing), and `getHeaders(name:)` when repeated headers matter. Both match names without regard to case.

`Network.fetch` and `Network.upload` return `NetworkResponse` for every HTTP status, including 4xx and 5xx; inspect `response.statusCode` and read the response body yourself. Their `try`/`catch` path is for invalid requests and transport failures such as connection errors, timeouts, cancellation, or certificate-pin failures. `Network.download` only replaces its destination for a 2xx response; other HTTP statuses throw and leave the existing file unchanged.

```nexa
app ProfileScreen {
    state accessToken: String = "session-token"
    state profileJson: String = ""
    state cookieHeader: String = ""
    state contentType: String? = null
    state refreshTask: TaskHandle? = null

    body {
        Column {
            Button("Refresh profile") {
                Task.launch(handle: refreshTask, executor: TaskExecutor.Main) {
                    try {
                        let requestHeaders: Map<String, String> = [
                            "Authorization": "Bearer \(accessToken)",
                            "Accept": "application/json"
                        ]
                        let response = await Network.fetch(
                            url: "https://api.example.com/v1/profile",
                            headers: requestHeaders
                        )
                        profileJson = "HTTP \(response.statusCode): \(response.body)"
                        contentType = response.getHeader(name: "Content-Type")
                        cookieHeader = response.getCookie()
                    } catch {
                        profileJson = "Could not load profile"
                    }
                }
            }
            Text(profileJson)
            Text(contentType ?? "No content type")
        }
    }
}
```

`getCookie()` reads every `Set-Cookie` value and keeps the cookie pair before its first semicolon, dropping attributes such as `Path` and `HttpOnly`. It splits each pair at the first `=` so values may contain additional equals signs or quoted characters, then joins pairs with `; `. If no valid cookie pair exists, it returns `""`. Reuse it in a later request with `headers: ["Cookie": cookieHeader]`.

`cachePolicy` accepts `Default` (follow HTTP cache headers), `Reload` (skip cached data), or `CacheOnly` (use cached data without contacting the server; fail if there is no cached response). See the complete [cached reading-list example](../examples/network_caching.nx).

<!-- nexadoc:begin core-api-reference -->
All callable built-in APIs below execute at runtime. Compile-time values come from declarations and project configuration rather than API calls.

| API | Typed signature | Effect and behavior |
|---|---|---|
| `NetworkResponse.getHeader` | `(name: String) -> String?` | pure computation — Matches header names case-insensitively and returns the first value, or null when none match. |
| `NetworkResponse.getHeaders` | `(name: String) -> Array<String>` | pure computation — Matches header names case-insensitively and returns every value in response order; returns an empty array when none match. |
| `NetworkResponse.getCookie` | `() -> String` | pure computation — Reads every Set-Cookie header, keeps each name and value before the first semicolon, preserves equals signs and quoted characters in values, and joins valid pairs with `; `. Cookie attributes are omitted; returns an empty string when no valid pair exists. |
| `Network.fetch` | `async (url: String, method: String = "GET", body: String? = null, headers: Map<String, String> = [:], timeoutSeconds: Float64 = 30, cachePolicy: String = "Default" {Default | Reload | CacheOnly}, followRedirects: Bool = true, maxResponseBytes: Int64 = 67108864, certificatePins: Set<String> = []) -> NetworkResponse throws` | network I/O — Returns every HTTP status for inspection; throws for invalid URLs, transport errors, timeouts, cancellation, response size limits, and certificate-pin failures. `timeoutSeconds` is measured in seconds. `cachePolicy` defaults to `Default` (honor HTTP cache headers); `Reload` skips the local cache; `CacheOnly` uses a cached response without contacting the server and fails on a cache miss. |
| `Network.download` | `async (url: String, destinationPath: String, method: String = "GET", body: String? = null, headers: Map<String, String> = [:], timeoutSeconds: Float64 = 30, cachePolicy: String = "Default" {Default | Reload | CacheOnly}, followRedirects: Bool = true, maxResponseBytes: Int64 = 67108864, certificatePins: Set<String> = []) -> Bool throws` | network I/O — Downloads to temporary storage and replaces the destination only for a 2xx response; other HTTP statuses throw and preserve the destination. `timeoutSeconds` is measured in seconds. `cachePolicy` defaults to `Default` (honor HTTP cache headers); `Reload` skips the local cache; `CacheOnly` uses a cached response without contacting the server and fails on a cache miss. |
| `Network.upload` | `async (url: String, file: String, fields: Map<String, String> = [:], headers: Map<String, String> = [:], timeoutSeconds: Float64 = 30, followRedirects: Bool = true, maxResponseBytes: Int64 = 67108864, certificatePins: Set<String> = []) -> NetworkResponse throws` | network I/O — Uploads a local file as multipart/form-data and returns every HTTP status for inspection; request and transport failures throw. `timeoutSeconds` is measured in seconds. Nexa generates the multipart Content-Type boundary, replacing any supplied Content-Type header. |
| `Path.documents` | `() -> String` | reads device environment — Returns the documents directory. |
| `Path.caches` | `() -> String` | reads device environment — Returns the caches directory. |
| `Path.temporary` | `() -> String` | reads device environment — Returns the temporary directory. |
| `Path.appSupport` | `() -> String` | reads device environment — Returns the app-support directory. |
| `Path.join` | `(path: String, component: String) -> String` | pure computation — Joins one path component using native path rules. |
| `File.exists` | `(path: String) -> Bool` | filesystem I/O — Checks whether a path exists. |
| `File.readText` | `async (path: String) -> String throws` | filesystem I/O — Reads UTF-8 text from a file. |
| `File.readBytes` | `async (path: String) -> Bytes throws` | filesystem I/O — Reads the exact bytes from a file. |
| `File.writeText` | `async (path: String, contents: String) -> Bool throws` | filesystem I/O — Writes UTF-8 text atomically and reports success. |
| `File.writeBytes` | `async (path: String, contents: Bytes) -> Bool throws` | filesystem I/O — Writes bytes atomically and reports success. |
| `File.delete` | `async (path: String) -> Bool throws` | filesystem I/O — Deletes a file; returns false when missing and throws on other I/O failures. |
| `Keyboard.dismiss` | `() -> Void` | UI side effect — Dismisses the active keyboard. |
| `Screen.lockOrientation` | `(mode: String {Portrait | Landscape | All}) -> Void` | UI side effect — Sets the supported screen orientation. |
| `Bytes.fromText` | `(text: String) -> Bytes` | pure computation — Encodes UTF-8 text as owned bytes. |
| `Bytes.fromArray` | `(values: Array<UInt8>) -> Bytes` | pure computation — Copies unsigned byte values into an owned byte buffer. |
| `Bytes.count` | `(bytes: Bytes) -> Int32` | pure computation — Returns the byte length. |
| `Permissions.status` | `async (permission: Permission) -> PermissionStatus` | system permission prompt or query — Reads current system authorization. |
| `Permissions.request` | `async (permission: Permission) -> PermissionStatus` | system permission prompt or query — Requests system authorization. |
| `Time.now` | `() -> Int64` | reads device environment — Returns Unix time in milliseconds. |
| `Time.monotonic` | `() -> Int64` | reads device environment — Returns a monotonic clock value in nanoseconds. |
| `Time.elapsed` | `(since: Int64) -> Int64` | pure computation — Returns nanoseconds elapsed from a monotonic reading. |
| `Time.sleep` | `async (milliseconds: Int64) -> Void` | suspends the current foreground task — Suspends the current task for the requested duration. |
| `Time.iso8601` | `(timestamp: Int64) -> String` | pure computation — Formats a timestamp in UTC as yyyy-MM-ddTHH:mm:ss.SSSZ. |
| `Time.iso8601ToMillis` | `(text: String) -> Int64?` | pure computation — Parses UTC yyyy-MM-ddTHH:mm:ssZ with a required Z and optional three-digit millisecond fraction; rejects offsets and invalid dates. |
| `Time.startOfDay` | `(timestamp: Int64) -> Int64` | reads device environment — Returns the start of day in the device's current calendar and time zone, in milliseconds. |
| `Time.addCalendarDays` | `(timestamp: Int64, days: Int32) -> Int64` | reads device environment — Adds calendar days in the device's current calendar and time zone; daylight-saving changes can make the elapsed duration differ from 24 hours. |
| `Time.localizedDate` | `(timestamp: Int64) -> String` | reads device environment — Formats a date using the device's current locale, calendar, and time zone. |
| `Time.localizedLongDate` | `(timestamp: Int64, locale: String) -> String` | reads device environment — Formats a long date using the specified app language and the device calendar and time zone. |
| `Time.localizedTime` | `(timestamp: Int64) -> String` | reads device environment — Formats a time using the device's current locale, calendar, and time zone. |
| `Time.localizedDateTime` | `(timestamp: Int64) -> String` | reads device environment — Formats a date and time using the device's current locale, calendar, and time zone. |
| `Time.format` | `(timestamp: Int64, pattern: String) -> String` | reads device environment — Formats a timestamp using the device's current locale, calendar, and time zone with an ICU date pattern. |
| `Time.formatDuration` | `(seconds: Float64) -> String` | pure computation — Formats a non-negative duration as zero-padded minutes and seconds (mm:ss). |
| `AppIcon.set` | `async (name: String?) -> Bool` | UI side effect — Sets an alternate app icon, or restores the default with null. |
| `Number.formatCurrency` | `(amount: Float64, currencyCode: String) -> String` | reads device environment — Formats currency for the current locale. |
| `Locale.currentLanguageCode` | `() -> String` | reads device environment — Returns the selected app language code, or the current device language when no override is set. |
| `Locale.setLanguageOverride` | `(languageCode: String?) -> Void` | persistent app storage — Persists a language override for localized text and returns to the device language when null is supplied. |
| `Locale.preferredLanguageCodes` | `() -> Array<String>` | reads device environment — Returns the selected app language first, followed by the device's other preferred language codes. |
| `Crypto.sha1` | `(text: String) -> String` | cryptographic operation — Returns a SHA-1 digest encoded as lowercase hexadecimal for protocol compatibility. |
| `Crypto.sha256` | `(text: String) -> String` | cryptographic operation — Returns a SHA-256 digest encoded as lowercase hexadecimal. |
| `Crypto.sha512` | `(text: String) -> String` | cryptographic operation — Returns a SHA-512 digest encoded as lowercase hexadecimal. |
| `Crypto.hmacSha256` | `(key: String, message: String) -> String` | cryptographic operation — Returns an HMAC-SHA-256 digest as lowercase hexadecimal. |
| `Crypto.randomBytes` | `(count: Int32) -> String` | cryptographic operation — Generates secure random bytes and returns lowercase hexadecimal. |
| `SecureStorage.get` | `async (key: String, scope: String? = null) -> String? throws` | protected OS storage — Reads a protected string value; an optional scope keeps feature data separate. |
| `SecureStorage.set` | `async (key: String, value: String, scope: String? = null) -> Void throws` | protected OS storage — Stores a protected value in an optional feature scope. |
| `SecureStorage.delete` | `async (key: String, scope: String? = null) -> Void throws` | protected OS storage — Deletes one protected value from an optional feature scope. |
| `SecureStorage.clear` | `async (scope: String? = null) -> Void throws` | protected OS storage — Clears all protected values, or only those in the requested feature scope. |
| `Storage.getString` | `(key: String) -> String?` | persistent app storage — Reads a non-secret preference. |
| `Storage.setString` | `(key: String, value: String) -> Void` | persistent app storage — Writes a non-secret preference. |
| `Storage.getBool` | `(key: String) -> Bool?` | persistent app storage — Reads a saved Boolean value, or null when the key is missing. |
| `Storage.setBool` | `(key: String, value: Bool) -> Void` | persistent app storage — Saves a Boolean value. |
| `Storage.getInt64` | `(key: String) -> Int64?` | persistent app storage — Reads a saved 64-bit integer, or null when the key is missing. |
| `Storage.setInt64` | `(key: String, value: Int64) -> Void` | persistent app storage — Saves a 64-bit integer. |
| `Storage.getFloat64` | `(key: String) -> Float64?` | persistent app storage — Reads a saved 64-bit floating-point value, or null when the key is missing. |
| `Storage.setFloat64` | `(key: String, value: Float64) -> Void` | persistent app storage — Saves a 64-bit floating-point value. |
| `Storage.delete` | `(key: String) -> Void` | persistent app storage — Deletes one preference. |
| `Storage.clear` | `() -> Void` | persistent app storage — Clears preferences managed by this API. |
| `Clipboard.setText` | `(text: String) -> Void` | UI side effect — Replaces system clipboard text. |
| `Clipboard.getText` | `() -> String?` | UI side effect — Reads clipboard text when present. |
| `Clipboard.hasText` | `() -> Bool` | UI side effect — Checks whether clipboard text is available. |
| `Haptics.impact` | `(style: String {Light | Medium | Heavy}) -> Void` | device haptic feedback — Triggers an impact haptic. |
| `Haptics.notification` | `(kind: String {Success | Error}) -> Void` | device haptic feedback — Triggers a notification haptic. |
| `Haptics.selection` | `() -> Void` | device haptic feedback — Triggers a selection haptic. |
| `Log.info` | `(message: String) -> Void` | native logging side effect — Writes an informational native log entry. |
| `Log.warning` | `(message: String) -> Void` | native logging side effect — Writes a warning native log entry. |
| `Log.error` | `(message: String) -> Void` | native logging side effect — Writes an error native log entry. |
| `Regex.isMatch` | `(pattern: String, text: String) -> Bool` | pure computation — Returns whether a pattern occurs in the input. |
| `Regex.matches` | `(pattern: String, text: String) -> Array<String>` | pure computation — Returns all non-overlapping matched substrings. |
| `Regex.replace` | `(pattern: String, text: String, replacement: String) -> String` | pure computation — Replaces all matches using native replacement syntax. |
| `Result<T, E> members` | `isSuccess: Bool`, `value: T?`, `error: E?` | Checks whether the result is successful and returns its success or failure payload when present. |
| `Network.isOnline` | `Bool` | Reads the current native network-connectivity state. |
| `Network.onStatusChange` | `(online: Bool) -> Void event subscription` | Registers a native connectivity listener and calls the action when the status changes. |
| `Json.parse<T>` | `(raw: String) -> Result<T, JsonError>` | Typed JSON decoding; `T` is inferred from context or supplied explicitly. |
| `Json.stringify<T>` | `(value: T) -> String` | Typed JSON encoding for supported Nexa values. |
| `Storage.get<T>` | `(key: String) -> T?` | Reads a typed JSON value from app storage; missing or invalid data returns null. |
| `Storage.set<T>` | `(key: String, value: T) -> Void` | Saves a typed JSON value using Nexa's generated codec for T. |
| `Locale.displayName` | `(languageCode: String) -> String?` | Returns the localized language name when available. |
| `Locale.localized` | `(sourceText: String) -> String` | Localizes source text or a source-text interpolation. |
| `Locale.setLanguageOverride` | `(languageCode: String?) -> Void` | Persists an app language override; null returns localization to the device language. |
| `Navigation.push` | `(ScreenName(arguments...)) -> Void` | Pushes a declared screen using its statically checked route parameters. |
| `Navigation.back` | `() -> Void` | Pops one route; does nothing at the navigation root. |
| `ComponentRef<T>` | `ComponentRef<T>() -> ComponentRef<T>` | Creates an identity-stable handle for a built-in or native plugin component, or a reusable `.nx` component that declares ref methods. |
| `ComponentRef<FastList>.scrollToIndex` | `(index: Int32, animated: Bool = true, viewPosition: Float64 = 0.0) -> Result<Void, ComponentRefError<Void>>` | Scrolls to an item; `viewPosition` is clamped from viewport start (`0`) through center (`0.5`) to end (`1`). |
| `ComponentRef<FastList>.scrollToTop` | `(animated: Bool = true) -> Result<Void, ComponentRefError<Void>>` | Scrolls to the first row when the list is attached. |
| `ComponentRef<FastList>.scrollToEnd` | `(animated: Bool = true) -> Result<Void, ComponentRefError<Void>>` | Scrolls to the final row when the list is attached. |
| `ComponentRef<FastList>.scrollToOffset` | `(offset: Float64, animated: Bool = true) -> Result<Void, ComponentRefError<Void>>` | Scrolls to an absolute logical offset when the list is attached. |
| `ComponentRef<BottomSheet>.present` | `() -> Result<Void, ComponentRefError<Void>>` | Presents the attached sheet; returns `notAttached` when its view is not mounted. |
| `ComponentRef<BottomSheet>.dismiss` | `() -> Result<Void, ComponentRefError<Void>>` | Dismisses the attached sheet; returns `notAttached` when its view is not mounted. |
| `Reusable and plugin component ref methods` | `(parameters...) -> Result<T, ComponentRefError<Failure>>`; async methods require `await` | Reusable `.nx` methods can return `Result<T, AppError>` with an app enum or propagate a plugin error with `throws Namespace.ErrorType`; ref calls flatten either failure into `ComponentRefError<Failure>`. Plugin native methods use their IDL signatures. Detached calls return `notAttached`. |

| Built-in type | Values or fields |
|---|---|
| `Permission` | `Camera`, `Microphone`, `Photos`, `Location`, `Notifications`, `Contacts`, `Calendar`, `Bluetooth`, `Motion` |
| `PermissionStatus` | `granted`, `limited`, `provisional`, `denied`, `permanentlyDenied`, `restricted`, `notDetermined` |
| `NetworkResponse` | `statusCode: Int32`, `body: String`, `getHeader(name: String): String?`, `getHeaders(name: String): Array<String>`, `getCookie(): String` |
| `JsonError` | `invalidJson`, `typeMismatch`, `missingField`, `invalidValue` |
| `ComponentRefError<Failure>` | `notAttached`, `failure(Failure)` |
<!-- nexadoc:end core-api-reference -->

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

app DiscountCalculator {
    body {
        Text("Checkout applies the verified discount calculation")
    }
}
```

### In-Memory Component Interaction Tests

```nexa
component CounterControl() {
    state count: Int32 = 0

    body {
        Column {
            Text("Count: \(count)")
            Button("Add") { count = count + 1 }
        }
    }
}

app CounterDemo {
    body { CounterControl() }
}

test "increment button increases state" for CounterControl() {
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

## 8. Compiler Warning Codes

Warnings use stable `NEXA-W` codes in CLI output and LSP diagnostics. Use the code to identify a warning in editor settings or CI; the message can become more specific without changing the code.

<!-- nexadoc:begin warning-codes -->
| Code | Meaning |
|---|---|
| `NEXA-W0000` | Warning without a more specific code. |
| `NEXA-W1001` | Function is never used and will be removed from generated code. |
| `NEXA-W1002` | Test local is never used. |
| `NEXA-W1003` | Function local is never used. |
| `NEXA-W1004` | Function parameter is never used. |
| `NEXA-W1005` | State or constant is never used and will be removed from generated code. |
| `NEXA-W1006` | Component parameter is never used. |
| `NEXA-W1007` | FastList row, section, or index binding is never used. |
| `NEXA-W1008` | Loop binding is never used. |
| `NEXA-W1009` | Condition is always true or false, so one branch is unreachable. |
| `NEXA-W1010` | Collection rows use positional identity because FastList has no key. |
| `NEXA-W1011` | Permission metadata is not used by app permission APIs or a reachable native plugin. |
| `NEXA-W1012` | Permission values are selected dynamically, so native permission declarations cannot be checked statically. |
| `NEXA-W9000` | Plugin analyzer reported a warning. |
<!-- nexadoc:end warning-codes -->
