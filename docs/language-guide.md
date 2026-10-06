# Nexa Language Guide 📖

> [!NOTE]
> **Scope:** Core `.nx` language syntax and typed standard APIs. UI components are listed in the [component guide](components.md); their full checked argument catalog is in the [syntax audit](syntax-audit.md).

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

### 3.4 `Pair<A, B>` and `Triple<A, B, C>`

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

## 5. Explicit Error Handling: `Result<T, E>`

Nexa rejects unchecked hidden exceptions. Functions that can fail return typed `Result<T, E>` values:

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
                paymentResult = Json.stringify(value: processPayment(25.0, 100.0))
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

`?` unwraps an `Ok` value or returns its `Err` from the current function. It is valid only when the current function also returns a compatible `Result`.

Throwing native plugin methods use `try`/`catch`; their typed error variants are declared by the plugin's `.nxid` contract. See [plugin error handling](plugins.md#async-throwing-calls) for a current end-to-end example.

## 6. Built-in Core APIs

Nexa maps these typed APIs to platform services. Calls that can fail are marked as throwing in the compiler and must be handled with `try`/`catch` or propagated from an async function.

### 6.1 `Storage` (Lightweight Key-Value)

```nexa
app ThemePreferences {
    state theme: String = Storage.getString(key: "reader.theme") ?? "system"

    body {
        Column {
            Text("Current theme: \(theme)")
            Button("Use dark theme") {
                Storage.setString(key: "reader.theme", value: "dark")
                theme = "dark"
            }
            Button("Reset saved theme") {
                Storage.delete(key: "reader.theme")
                theme = "system"
            }
        }
    }
}
```

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
    let total: String = Number.formatCurrency(amount: 1234.50, currencyCode: "USD")

    body {
        Column {
            Text("Order placed: \(formattedDate)")
            Text("Total: \(total)")
            Text("Local date: \(Time.localizedDate(timestamp: placedAt))")
        }
    }
}
```

The complete set of built-in method names and argument spellings is generated from the parser catalog in the [syntax audit](syntax-audit.md#vocabulary).

### Secure storage

Use `SecureStorage` for small secrets such as refresh tokens. Its asynchronous operations can fail and must be handled with `try`/`catch`.

```nexa
app SessionVault {
    state status: String = "No secure session loaded"
    state secureTask: TaskHandle? = null
    state sessionToken: String = ""

    body {
        Column(spacing: 12) {
            TextInput(value: sessionToken, placeholder: "Token returned by sign-in")
            Text(status)
            Button("Save session token") {
                Task.launch(handle: secureTask, executor: TaskExecutor.Main) {
                    try {
                        await SecureStorage.set(key: "reader.refresh-token", value: sessionToken)
                        status = "Session token saved securely"
                    } catch {
                        status = "Secure storage is unavailable on this device"
                    }
                }
            }
        }
    }
}
```

### Files and permissions

`File.readText`, `File.writeText`, and `File.delete` are asynchronous and can fail, so handle them in `try`/`catch`. Declare a user-facing purpose message in `nexa.config.nx` for each system permission your app requests.

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

| API | Signature | Behavior |
|---|---|---|
| `Path.documents()` | `() -> String` | User documents directory. |
| `Path.caches()` | `() -> String` | App cache directory. |
| `Path.temporary()` | `() -> String` | Temporary files directory. |
| `Path.appSupport()` | `() -> String` | App support directory. |
| `Path.join(path, component)` | `(String, String) -> String` | Joins one path component using native path rules. |
| `File.exists(path)` | `(String) -> Bool` | Checks whether a path exists. |
| `File.readText(path)` | `async -> String throws` | Reads UTF-8 text from a file. |
| `File.writeText(path, contents)` | `async -> Bool throws` | Writes UTF-8 text and reports success. |
| `File.delete(path)` | `async -> Bool throws` | Deletes a file and reports success. |
| `Network.fetch(url, method, body, headers, timeout, useCache, followRedirects, maxResponseBytes, certificatePins)` | `async -> NetworkResponse throws` | Sends an HTTP request and returns the status code, headers, and text body. `url` is required; the remaining options have defaults listed below. |
| `Network.download(url, destinationPath, method, body, headers, timeout, useCache, followRedirects, maxResponseBytes, certificatePins)` | `async -> Bool throws` | Downloads a response to a local path. `url` and `destinationPath` are required; the remaining options have defaults listed below. |
| `Network.upload(url, file, fields)` | `async -> NetworkResponse throws` | Uploads a local file with optional form fields. `fields` defaults to an empty map. |
| `Permissions.status(permission)` | `async -> PermissionStatus` | Reads current system authorization. |
| `Permissions.request(permission)` | `async -> PermissionStatus` | Requests system authorization. |
| `Bytes.fromText(text)` | `(String) -> Bytes` | Encodes UTF-8 text as owned bytes. |
| `Bytes.fromArray(values)` | `(Array<UInt8>) -> Bytes` | Copies unsigned byte values into an owned byte buffer. |
| `Bytes.count(bytes)` | `(Bytes) -> Int32` | Returns the byte length. |
| `SecureStorage.get(key)` | `async -> String? throws` | Reads a protected string value. |
| `SecureStorage.set(key, value)` | `async -> Void throws` | Stores a protected value. |
| `SecureStorage.delete(key)` | `async -> Void throws` | Deletes one protected value. |
| `SecureStorage.clear()` | `async -> Void throws` | Clears protected values managed by this API. |
| `Json.parse<T>(raw)` | `(String) -> Result<T, JsonError>` | Parses text as a declared Nexa value type. |
| `Json.stringify<T>(value)` | `(T) -> String` | Encodes a supported typed value as JSON text. |
| `Storage.getString(key)` | `(String) -> String?` | Reads a non-secret preference. |
| `Storage.setString(key, value)` | `(String, String) -> Void` | Writes a non-secret preference. |
| `Storage.delete(key)` | `(String) -> Void` | Deletes one preference. |
| `Storage.clear()` | `() -> Void` | Clears preferences managed by this API. |
| `Clipboard.setText(text)` | `(String) -> Void` | Replaces system clipboard text. |
| `Clipboard.getText()` | `() -> String?` | Reads clipboard text when present. |
| `Clipboard.hasText()` | `() -> Bool` | Checks whether clipboard text is available. |
| `Crypto.sha256(text)` / `Crypto.sha512(text)` | `(String) -> String` | Returns the digest encoded as lowercase hexadecimal. |
| `Crypto.hmacSha256(key, message)` | `(String, String) -> String` | Returns an HMAC-SHA-256 digest as lowercase hexadecimal. |
| `Crypto.randomBytes(count)` | `(Int32) -> String` | Generates secure random bytes and returns their lowercase hexadecimal encoding. |
| `Time.now()` | `() -> Int64` | Current Unix timestamp in milliseconds. |
| `Time.monotonic()` | `() -> Int64` | Monotonic clock value in nanoseconds; use only for elapsed-time measurement. |
| `Time.elapsed(since)` | `(Int64) -> Int64` | Nanoseconds elapsed from a monotonic reading. |
| `Time.sleep(milliseconds)` | `async (Int64) -> Void` | Suspends the current task for the requested duration. |
| `Time.iso8601(timestamp)` | `(Int64) -> String` | Formats a millisecond timestamp as UTC ISO 8601. |
| `Time.iso8601ToMillis(text)` | `(String) -> Int64?` | Parses the supported UTC ISO 8601 form. |
| `Time.startOfDay(timestamp)` | `(Int64) -> Int64` | Local calendar start of day in milliseconds. |
| `Time.addCalendarDays(timestamp, days)` | `(Int64, Int32) -> Int64` | Adds local calendar days and preserves wall-clock time across daylight-saving changes. |
| `Time.localizedDate(timestamp)` / `localizedTime(timestamp)` / `localizedDateTime(timestamp)` | `(Int64) -> String` | Formats using the user's locale and time zone. |
| `Time.format(timestamp, pattern)` | `(Int64, String) -> String` | Formats with a date pattern in the current locale and time zone. |
| `Number.formatCurrency(amount, currencyCode)` | `(Float64, String) -> String` | Formats currency for the current locale. |
| `Locale.currentLanguageCode()` | `() -> String` | Returns the current language code. |
| `Locale.preferredLanguageCodes()` | `() -> Array<String>` | Returns the device's preferred language codes. |
| `Locale.displayName(languageCode)` | `(String) -> String?` | Returns the localized language name when available. |
| `Locale.localized(sourceText)` | `(String) -> String` | Localizes a source-text literal or interpolation using generated catalogs. |
| `Keyboard.dismiss()` | `() -> Void` | Dismisses the active keyboard. |
| `Screen.lockOrientation(mode)` | `(String) -> Void` | Sets orientation to `Portrait`, `Landscape`, or `All`. |
| `AppIcon.set(name)` | `async (String?) -> Bool` | Sets the alternate app icon, or restores the default with `null`. |
| `Haptics.impact(style)` | `(Light | Medium | Heavy) -> Void` | Triggers an impact haptic. |
| `Haptics.notification(kind)` | `(Success | Error) -> Void` | Triggers a notification haptic. |
| `Haptics.selection()` | `() -> Void` | Triggers a selection haptic. |
| `Log.info(message)` / `Log.warning(message)` / `Log.error(message)` | `(String) -> Void` | Writes a typed diagnostic message to the native log. |

`Network.fetch` and `Network.download` accept these named options after their required arguments: `method: String = "GET"`, `body: String? = null`, `headers: Map<String, String> = [:]`, `timeout: Float64 = 30`, `useCache: Bool = true`, `followRedirects: Bool = true`, `maxResponseBytes: Int64 = 67108864`, and `certificatePins: Set<String> = []`. `Network.upload` accepts `fields: Map<String, String> = [:]`.

| Built-in type | Values or fields |
|---|---|
| `Permission` | `Camera`, `Microphone`, `Photos`, `Location`, `Notifications`, `Contacts`, `Calendar`, `Bluetooth`, `Motion` |
| `PermissionStatus` | `granted`, `denied`, `restricted`, `notDetermined` |
| `NetworkResponse` | `statusCode: Int32`, `headers: Map<String, String>`, `body: String` |

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
