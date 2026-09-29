# Nexa Language Guide

Nexa source files use the `.nx` extension. Nexa checks types at compile time and generates SwiftUI for iOS or Jetpack Compose for Android.

## Scalar types

The scalar types accepted in app and component declarations are:

| Nexa type | Swift type | Kotlin type |
|---|---|---|
| `String` | `String` | `String` |
| `Bool` | `Bool` | `Boolean` |
| `Int8` | `Int8` | `Byte` |
| `Int16` | `Int16` | `Short` |
| `Int32` | `Int32` | `Int` |
| `Int64` | `Int64` | `Long` |
| `UInt8` | `UInt8` | `UByte` |
| `UInt16` | `UInt16` | `UShort` |
| `UInt32` | `UInt32` | `UInt` |
| `UInt64` | `UInt64` | `ULong` |
| `Float32` | `Float` | `Float` |
| `Float64` | `Double` | `Double` |

In Nexa source, use `Float32` and `Float64`; `Float` and `Double` are native output names, not Nexa type names. Integer literals infer as `Int32`, and decimal literals infer as `Float64` when the expected type does not specify another numeric type.

`Void` is not a general app value type. It is used as the return type of void methods in native plugin contracts.

## Optional and collection types

- `T?` is an optional value.
- `Array<T>` is an ordered collection. It maps to a Swift array and a Kotlin `List<T>`.
- `Set<T>` is a set.
- `Map<K, V>` is a key-value map.
- `Pair<A, B>` and `Triple<A, B, C>` are fixed-size tuples.
- `Result<T, E>` represents a success value or a typed error value.

Set elements and map keys must be supported scalar/hashable types. Nexa collection syntax uses `Array<T>`, not `List<T>`; `List` is the Kotlin target type emitted for `Array<T>`.

## Apps, state, and conditions

An app has one `body` containing its view tree. Declare mutable UI state with `state`:

```nexa
app Counter {
    state count: Int32 = 0

    body {
        Column(spacing: 12) {
            Text("Count")
            Text(count)
            Button("Increment") {
                count = count + 1
            }
            if count > 10 {
                Text("More than ten")
            }
        }
    }
}
```

Swift output stores this with SwiftUI state. Kotlin output uses Compose state, selecting primitive state holders such as `mutableIntStateOf` for `Int32`. `let` declares an immutable binding; `state` declares a mutable binding. A binding can omit its type when its initializer determines one unambiguous type.

## Arithmetic

Numeric expressions support `+`, `-`, `*`, `/`, and `%`, plus unary negation. Multiplication, division, and remainder bind more tightly than addition and subtraction; parentheses can make grouping explicit. Integer division truncates toward zero. Fixed-width integer addition, subtraction, multiplication, and negation wrap on overflow.

The `+` operator also concatenates two `String` values. It emits the native Swift or Kotlin string operator. The unary `!` operator negates a `Bool`.

Arrays, sets, and maps expose `count` as `Int32` and `isEmpty` as `Bool`:

```nexa
app CollectionExample {
    state names: Array<String> = ["Nexa"]

    body {
        Column {
            Text(names.count)
            Text(names.isEmpty)
        }
    }
}
```

Arrays also support `random()`, `shuffled()`, `reverse()`, and `slice(range)`:

```nexa
app ArrayUtilities {
    state values: Array<Int32> = [10, 20, 30, 40]
    state selected: Int32? = values.random()
    state randomized: Array<Int32> = values.shuffled()
    state reversed: Array<Int32> = values.reverse()
    state middle: Array<Int32> = values.slice(1..<3)

    body {
        Text(selected ?? 0)
        Text(randomized.count)
        Text(reversed.count)
        Text(middle.count)
    }
}
```

`random()` returns an optional element because an empty array has no value to choose. `shuffled()` and `reverse()` return a new array. `slice` takes one unstepped range; `..` includes its upper index and `..<` excludes it. Indices must be valid for the source array.

Text styles can be chained after a component call. For example, `Text("Hi").fontSize(18).bold().padding(12)` is equivalent to setting `fontSize: 18`, `fontWeight: Bold`, and `padding: 12` in the `Text` arguments. `Text`, `Column`, `Row`, and `Stack` also accept `.opacity(value)`, `.scale(value)`, `.rotation(degrees)`, `.shadow(radius: 8, x: 0, y: 4, color: "#00000040")`, `.blur(radius)`, `.clip(shape: Rounded(radius))`, and `.zIndex(value)`.

Layout `animation` accepts `Spring`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`. Customize spring response time in seconds and damping ratio with `Spring(response: 0.35, damping: 0.8)`; omitted values use `0.5` seconds and `0.825`. Response and damping must be finite and greater than zero. SwiftUI receives these values directly. Compose uses the damping ratio directly and maps response to a native spring stiffness, so the motion is platform-native and approximate rather than frame-for-frame identical.

Conditional view blocks accept `.transition(.fade)`, `.transition(.scale)`, or `.transition(.slide(from: .bottom))`. The transition runs when an `if` branch or `when` case changes. The current slide syntax supports `.bottom`:

```nexa
app ConditionalTransitions {
    state expanded: Bool = false

    body {
        Button("Toggle") {
            expanded = !expanded
        }
        if expanded {
            Text("Details")
        }.transition(.fade)
        when expanded {
            true: { Text("Expanded") }
            else: { Spacer() }
        }.transition(.slide(from: .bottom))
    }
}
```

These modifiers are part of the typed module and DevRuntime payload, so changing the transition remains compatible with hot reload after the app has a DevRuntime build.

`Spacer()` consumes remaining space on a `Row` or `Column` axis. `Divider(color: "#808080", thickness: 1)` inserts a horizontal separator.
`Slider(value: amount, min: 0.0, max: 1.0, step: 0.01)` binds a stepped native slider to mutable `Float64` state.
`ProgressBar(progress: amount)` and `ProgressRing(progress: amount)` render normalized linear and circular progress controls.
`SegmentedControl(items: filters, selected: selectedFilter)` binds a compact native selector to mutable `String` state.
`Picker(items: sizes, selected: selectedSize)` binds a native menu selector to mutable `String` state.
`Dialog(isPresented: show, title: "Confirm", message: "Continue?") { Button("OK") { show = false } }` presents a native alert tied to mutable Boolean state.

## Value-producing conditionals

An `if` expression returns the value of the branch that runs. You can also write the same conditional as `condition ? value : fallback`. Both branches must have the same type, except that a value and `null` produce an optional value:

```nexa
app ConditionalExample {
    state enabled: Bool = true
    state title: String = if enabled { "Ready" } else { "Waiting" }
    state selected: String? = if enabled { "Ready" } else { null }

    body {
        Text(title)
        Text(enabled ? "Ready" : "Waiting")
        Text(selected ?? "Unavailable")
    }
}
```

The condition must be a `Bool`. The compiler removes an unreachable branch when the condition is a constant.

Inside an action, mutable state supports compound assignments: `+=`, `-=`, `*=`, `/=`, and `%=`. Each is equivalent to assigning the current value combined with the right-hand value by the matching binary operator:

```nexa
app Counter {
    state count: Int32 = 0

    body {
        Button("Update") {
            count += 2 * 3
            count -= 1
            count *= 2
        }
    }
}
```

## Structs, enums, and functions

Value structs are declared at the top level. Closed enums and functions are declared inside an app; functions have explicit parameter and return types:

```nexa
struct UserProfile {
    id: Int64,
    username: String,
}

app ProfileApp {
    enum LoadState {
        idle,
        loading,
        loaded
    }

    fn greeting(name: String) -> String {
        return name
    }

    body {
        Text(greeting("Nexa"))
    }
}
```

App-local functions currently use typed parameters, immutable local `let` bindings, and one return expression. Struct constructors take field values in declaration order.

## Results and postfix `?`

Use an enum for the error type so the result can map to Swift's `Result`, whose failure type must conform to `Error`:

```nexa
app ResultExample {
    enum AppError {
        unavailable
    }

    fn loadValue() -> Result<Int32, AppError> {
        return Ok(42)
    }

    fn useValue() -> Result<Int32, AppError> {
        let value = loadValue()?
        return Ok(value)
    }

    state result: Result<Int32, AppError> = Ok(0)

    body {
        Column {
            Text("Result example")
            Button("Load") {
                result = useValue()
            }
        }
    }
}
```

`Ok(value)` and `Err(error)` construct results. On Swift, postfix `?` generates `try result.get()`, which throws the typed failure. On Kotlin, Nexa emits the sealed `NexaResult<T, E>` type; postfix `?` currently calls `getOrThrow()`, which turns a failure into a `RuntimeException`. It does not return `nil` on either target.

## JSON

`Json.parse<T>(raw:)` decodes a concrete Nexa value type and returns `Result<T, JsonError>`. `Json.stringify(value:)` encodes a value to a JSON string. The type is resolved at compile time, so model decoding uses generated typed codecs rather than reflection or an untyped object map.

Supported values include strings, booleans, numbers, bytes, optionals, arrays, sets, string-keyed maps, pairs, triples, results, declared enums, and declared structs composed from those types. Bytes use Base64 strings; enums use their case names; pairs and triples use JSON arrays; results use a one-key `success` or `failure` object. Maps require `String` keys. Unknown struct fields are ignored while decoding, and missing required fields return `JsonError.missingField`.

```nexa
struct UserProfile {
    name: String,
    age: Int32,
}

app JsonExample {
    fn parseAndEncode(raw: String) -> String {
        let profile: Result<UserProfile, JsonError> = Json.parse<UserProfile>(raw: raw)
        return Json.stringify(value: profile)
    }

    state raw: String = Json.stringify(value: UserProfile("Ada", 37))
    state status: String = "Not parsed yet"

    body {
        Text(raw)
        Text(status)
        Button("Validate JSON") {
            status = parseAndEncode(raw)
        }
    }
}
```

Parse failures are returned as `JsonError.invalidJson`, `typeMismatch`, `missingField`, or `invalidValue`; callers can inspect the result or propagate it with `?` from a function that returns a compatible `Result`.

## Time

`Time` is a core API, available on every target without a plugin:

| Call | Returns | Meaning |
|---|---|---|
| `Time.now()` | `Int64` | Milliseconds since the Unix epoch, UTC |
| `Time.monotonic()` | `Int64` | Nanoseconds from a fixed origin; only differences are meaningful |
| `Time.elapsed(since: timestamp)` | `Int64` | Nanoseconds since a value returned by `Time.monotonic()` |
| `Time.sleep(milliseconds: Int64)` | `Void` | Suspends the caller |
| `Time.iso8601(timestamp: Int64)` | `String` | The instant as `yyyy-MM-ddTHH:mm:ss.SSSZ` in UTC |
| `Time.iso8601ToMillis(text: String)` | `Int64?` | The inverse, or `null` when the text is not that layout |

The two clocks are separate on purpose. `Time.now()` is a wall clock: it can jump when the system time is corrected, so it answers *when* something happened. `Time.monotonic()` never moves backwards, so it is the only one that can answer *how long* something took:

```nexa
app TimerExample {
    state startedAt: Int64 = 0
    state finishedAt: Int64 = 0
    state stamp: String = ""

    body {
        Column(spacing: 16) {
            Text(stamp)
        }

        OnAppear async {
            startedAt = Time.monotonic()
            stamp = Time.iso8601(timestamp: Time.now())
            try {
                await Time.sleep(milliseconds: 50)
            } catch {
            }
            finishedAt = Time.monotonic()
        }
    }
}
```

`Time.sleep` is the one clock call that can throw, because the surrounding task can be cancelled while it is suspended, so `await Time.sleep(...)` needs a `try { ... } catch { ... }` block or a throwing function, exactly like an awaited `File` or `Network` call. Awaiting a clock *reading* is an error: only `sleep` suspends.

Use `Time.elapsed(since:)` to measure durations without mixing the monotonic clock with wall-clock timestamps:

```nexa
let startedAt = Time.monotonic()
// Run the work being measured.
let elapsedNanoseconds = Time.elapsed(since: startedAt)
```

`Time.iso8601` and `Time.iso8601ToMillis` are hand-implemented on both platforms with the same leap-year rules rather than a formatter, so a timestamp written on one target reads on the other. The layout is UTC and locale-independent, and the fractional part is optional when parsing. This is deliberately not a display format: there is no localized or timezone-aware date formatting, because its output would differ between devices and between platforms.

## Currency formatting

`Number.formatCurrency(amount: Float64, currencyCode: String)` formats a value using the device's current locale and the requested ISO 4217 currency code. For example, a French device may display `1234.5` in EUR as `1 234,50 €`, while an English device uses its local grouping and symbol placement. An unrecognized currency code falls back to locale-aware decimal formatting rather than throwing.

```nexa
app PriceExample {
    let price: String = Number.formatCurrency(amount: 1234.5, currencyCode: "EUR")

    body {
        Text(price)
    }
}
```

The formatter uses the platform locale at the time of the call. Updating device locale or currency data can therefore change its output. The call is supported by the native AOT backends and DevRuntime.

## Cryptography

`Crypto` is a synchronous core API on iOS and Android. SHA digests and HMAC-SHA256 are returned as lowercase hexadecimal strings. `randomBytes` returns Base64 text generated by the platform's secure random source; counts at or below zero return an empty string.

```nexa
let digest: String = Crypto.sha256(text: "payload")
let signature: String = Crypto.hmacSha256(key: "secret", message: "payload")
let nonce: String = Crypto.randomBytes(count: 16)
```

The implementation uses CryptoKit and Security on iOS, and `MessageDigest`, `Mac`, and `SecureRandom` on Android. Calls use direct native helpers in release builds and are available to DevRuntime hot reload.

## Clipboard

`Clipboard.setText(text: String)` writes plain text to the system clipboard. `Clipboard.getText()` returns the current plain text as `String?`, and `Clipboard.hasText()` reports whether the clipboard contains text. The API uses `UIPasteboard` on iOS and the Android clipboard service; iOS may show its system paste permission prompt when an app reads clipboard content. Clipboard calls added to `.nx` code are available through DevRuntime hot reload.

## Haptics

Use `Haptics.impact(style:)` for light, medium, or heavy impact feedback; `Haptics.notification(kind:)` for success or error feedback; and `Haptics.selection()` for selection changes. The style values are `Light`, `Medium`, and `Heavy`; notification values are `Success` and `Error`.

```nexa
Button("Save") {
    Haptics.notification(kind: Success)
}
```

Release builds call UIKit feedback generators on iOS and Android view haptic feedback constants on Android. DevRuntime supports all three calls, so adding them to app source can hot reload without rebuilding the host. Android uses the foreground activity's decor view and does not request the vibration permission.

## Keyboard

`Keyboard.dismiss()` hides the software keyboard while leaving the focused input intact. Use `TextInput`'s `autofill` option to identify username, password, or one-time-code fields, and `returnKeyType` to choose Done, Search, Send, or Next. On Android the autofill hint is expressed through Compose semantics; on iOS it uses the native text content type.

## Logging

`Log` writes messages to the native platform log, visible in Android Logcat and the iOS Console:

```nexa
Log.info(message: "Loaded $count records")
Log.warning(message: "Using cached data")
Log.error(message: "The request failed")
```

## Platform-specific UI

Use `platform ios { ... }` and `platform android { ... }` for target-specific views. Nexa removes the inactive block before checking and generating the selected target:

```nexa
app PlatformExample {
    body {
        platform ios {
            Text("iOS view")
        }
        platform android {
            Text("Android view")
        }
    }
}
```

For supported components and their arguments, see [Components](components.md). For app and screen state and navigation, see [State and Navigation](state-and-navigation.md).
