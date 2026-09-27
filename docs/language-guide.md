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
