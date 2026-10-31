//! The compiler-owned contract for built-in native APIs.
//!
//! Signatures in this catalog drive argument validation and the published API
//! reference. `native_plan` remains responsible for lowering APIs whose IR has
//! a specialized representation (for example network requests and file I/O).

use nexa_diagnostics::Span;
use nexa_ir::{NumericType, Type};
use nexa_syntax::ast;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiType {
    Void,
    String,
    Bool,
    Bytes,
    Int32,
    Int64,
    Float64,
    NetworkResponse,
    Permission,
    PermissionStatus,
    OptionalString,
    OptionalInt64,
    OptionalFloat64,
    OptionalBool,
    StringArray,
    UInt8Array,
    StringSet,
    StringMap,
}

impl ApiType {
    pub(crate) fn to_ir(self) -> Type {
        match self {
            Self::Void => Type::Void,
            Self::String => Type::String,
            Self::Bool => Type::Bool,
            Self::Bytes => Type::Bytes,
            Self::Int32 => Type::Numeric(NumericType::Int32),
            Self::Int64 => Type::Numeric(NumericType::Int64),
            Self::Float64 => Type::Numeric(NumericType::Float64),
            Self::NetworkResponse => Type::NetworkResponse,
            Self::Permission => Type::Enum("Permission".to_owned()),
            Self::PermissionStatus => Type::Enum("PermissionStatus".to_owned()),
            Self::OptionalString => Type::Optional(Box::new(Type::String)),
            Self::OptionalInt64 => Type::Optional(Box::new(Type::Numeric(NumericType::Int64))),
            Self::OptionalFloat64 => Type::Optional(Box::new(Type::Numeric(NumericType::Float64))),
            Self::OptionalBool => Type::Optional(Box::new(Type::Bool)),
            Self::StringArray => Type::Array(Box::new(Type::String)),
            Self::UInt8Array => Type::Array(Box::new(Type::Numeric(NumericType::UInt8))),
            Self::StringSet => Type::Set(Box::new(Type::String)),
            Self::StringMap => Type::Map(Box::new(Type::String), Box::new(Type::String)),
        }
    }

    fn spelling(self) -> &'static str {
        match self {
            Self::Void => "Void",
            Self::String => "String",
            Self::Bool => "Bool",
            Self::Bytes => "Bytes",
            Self::Int32 => "Int32",
            Self::Int64 => "Int64",
            Self::Float64 => "Float64",
            Self::NetworkResponse => "NetworkResponse",
            Self::Permission => "Permission",
            Self::PermissionStatus => "PermissionStatus",
            Self::OptionalString => "String?",
            Self::OptionalInt64 => "Int64?",
            Self::OptionalFloat64 => "Float64?",
            Self::OptionalBool => "Bool?",
            Self::StringArray => "Array<String>",
            Self::UInt8Array => "Array<UInt8>",
            Self::StringSet => "Set<String>",
            Self::StringMap => "Map<String, String>",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiDefault {
    None,
    Null,
    String(&'static str),
    Number(&'static str),
    Bool(bool),
    EmptySet,
    EmptyMap,
}

impl ApiDefault {
    pub(crate) fn to_ast(self, span: Span) -> Option<ast::Expr> {
        Some(match self {
            Self::None => return None,
            Self::Null => ast::Expr::Null(span),
            Self::String(value) => ast::Expr::String(value.to_owned(), span),
            Self::Number(value) => ast::Expr::Number(value.to_owned(), span),
            Self::Bool(value) => ast::Expr::Bool(value, span),
            Self::EmptySet => ast::Expr::Array(Vec::new(), span),
            Self::EmptyMap => ast::Expr::Map(Vec::new(), span),
        })
    }

    fn spelling(self) -> Option<String> {
        match self {
            Self::None => None,
            Self::Null => Some("null".to_owned()),
            Self::String(value) => Some(format!("\"{value}\"")),
            Self::Number(value) => Some(value.to_owned()),
            Self::Bool(value) => Some(value.to_string()),
            Self::EmptySet => Some("[]".to_owned()),
            Self::EmptyMap => Some("[:]".to_owned()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiEffect {
    Pure,
    Environment,
    Ui,
    PersistentStorage,
    ProtectedStorage,
    Filesystem,
    Network,
    Permission,
    Haptics,
    Logging,
    Crypto,
    TaskScheduling,
}

impl ApiEffect {
    fn label(self) -> &'static str {
        match self {
            Self::Pure => "pure computation",
            Self::Environment => "reads device environment",
            Self::Ui => "UI side effect",
            Self::PersistentStorage => "persistent app storage",
            Self::ProtectedStorage => "protected OS storage",
            Self::Filesystem => "filesystem I/O",
            Self::Network => "network I/O",
            Self::Permission => "system permission prompt or query",
            Self::Haptics => "device haptic feedback",
            Self::Logging => "native logging side effect",
            Self::Crypto => "cryptographic operation",
            Self::TaskScheduling => "suspends the current foreground task",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CoreApiArgument {
    pub name: &'static str,
    pub ty: ApiType,
    pub default: ApiDefault,
    pub allowed_values: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub struct CoreApiDefinition {
    pub qualified_name: &'static str,
    pub receiver: Option<ApiType>,
    pub result: ApiType,
    pub asynchronous: bool,
    pub throwing: bool,
    pub effect: ApiEffect,
    pub summary: &'static str,
    pub arguments: &'static [CoreApiArgument],
}

const NO_VALUES: &[&str] = &[];
macro_rules! arg {
    ($name:literal, $ty:ident) => {
        CoreApiArgument {
            name: $name,
            ty: ApiType::$ty,
            default: ApiDefault::None,
            allowed_values: NO_VALUES,
        }
    };
    ($name:literal, $ty:ident, values($values:expr)) => {
        CoreApiArgument {
            name: $name,
            ty: ApiType::$ty,
            default: ApiDefault::None,
            allowed_values: $values,
        }
    };
    ($name:literal, $ty:ident, $default:ident($value:expr)) => {
        CoreApiArgument {
            name: $name,
            ty: ApiType::$ty,
            default: ApiDefault::$default($value),
            allowed_values: NO_VALUES,
        }
    };
    ($name:literal, $ty:ident, $default:ident($value:expr), values($values:expr)) => {
        CoreApiArgument {
            name: $name,
            ty: ApiType::$ty,
            default: ApiDefault::$default($value),
            allowed_values: $values,
        }
    };
    ($name:literal, $ty:ident, $default:ident) => {
        CoreApiArgument {
            name: $name,
            ty: ApiType::$ty,
            default: ApiDefault::$default,
            allowed_values: NO_VALUES,
        }
    };
}
macro_rules! api {
    ($name:literal, $result:ident, $async:literal, $throws:literal, $effect:ident, $summary:literal, [$($arg:expr),* $(,)?]) => {
        CoreApiDefinition { qualified_name: $name, receiver: None, result: ApiType::$result, asynchronous: $async, throwing: $throws, effect: ApiEffect::$effect, summary: $summary, arguments: &[$($arg),*] }
    };
}
macro_rules! method {
    ($receiver:ident, $name:literal, $result:ident, $async:literal, $throws:literal, $effect:ident, $summary:literal, [$($arg:expr),* $(,)?]) => {
        CoreApiDefinition { qualified_name: $name, receiver: Some(ApiType::$receiver), result: ApiType::$result, asynchronous: $async, throwing: $throws, effect: ApiEffect::$effect, summary: $summary, arguments: &[$($arg),*] }
    };
}

const IMPACT_STYLES: &[&str] = &["Light", "Medium", "Heavy"];
const NOTIFICATION_KINDS: &[&str] = &["Success", "Error"];
const ORIENTATIONS: &[&str] = &["Portrait", "Landscape", "All"];
const NETWORK_CACHE_POLICIES: &[&str] = &["Default", "Reload", "CacheOnly"];

const SPECIAL_API_ROWS: &[(&str, &str, &str)] = &[
    (
        "Result<T, E> members",
        "`isSuccess: Bool`, `value: T?`, `error: E?`",
        "Checks whether the result is successful and returns its success or failure payload when present.",
    ),
    (
        "Network.isOnline",
        "`Bool`",
        "Reads the current native network-connectivity state.",
    ),
    (
        "Network.onStatusChange",
        "`(online: Bool) -> Void event subscription`",
        "Registers a native connectivity listener and calls the action when the status changes.",
    ),
    (
        "Json.parse<T>",
        "`(raw: String) -> Result<T, JsonError>`",
        "Typed JSON decoding; `T` is inferred from context or supplied explicitly.",
    ),
    (
        "Json.stringify<T>",
        "`(value: T) -> String`",
        "Typed JSON encoding for supported Nexa values.",
    ),
    (
        "Storage.get<T>",
        "`(key: String) -> T?`",
        "Reads a typed JSON value from app storage; missing or invalid data returns null.",
    ),
    (
        "Storage.set<T>",
        "`(key: String, value: T) -> Void`",
        "Saves a typed JSON value using Nexa's generated codec for T.",
    ),
    (
        "Locale.displayName",
        "`(languageCode: String) -> String?`",
        "Returns the localized language name when available.",
    ),
    (
        "Locale.localized",
        "`(sourceText: String) -> String`",
        "Localizes source text or a source-text interpolation.",
    ),
    (
        "Locale.setLanguageOverride",
        "`(languageCode: String?) -> Void`",
        "Persists an app language override; null returns localization to the device language.",
    ),
    (
        "Navigation.push",
        "`(ScreenName(arguments...)) -> Void`",
        "Pushes a declared screen using its statically checked route parameters.",
    ),
    (
        "Navigation.back",
        "`() -> Void`",
        "Pops one route; does nothing at the navigation root.",
    ),
    (
        "ComponentRef<T>",
        "`ComponentRef<T>() -> ComponentRef<T>`",
        "Creates an identity-stable handle for a built-in or native plugin component, or a reusable `.nx` component that declares ref methods.",
    ),
    (
        "ComponentRef<FastList>.scrollToIndex",
        "`(index: Int32, animated: Bool = true, viewPosition: Float64 = 0.0) -> Result<Void, ComponentRefError<Void>>`",
        "Scrolls to an item; `viewPosition` is clamped from viewport start (`0`) through center (`0.5`) to end (`1`).",
    ),
    (
        "ComponentRef<FastList>.scrollToTop",
        "`(animated: Bool = true) -> Result<Void, ComponentRefError<Void>>`",
        "Scrolls to the first row when the list is attached.",
    ),
    (
        "ComponentRef<FastList>.scrollToEnd",
        "`(animated: Bool = true) -> Result<Void, ComponentRefError<Void>>`",
        "Scrolls to the final row when the list is attached.",
    ),
    (
        "ComponentRef<FastList>.scrollToOffset",
        "`(offset: Float64, animated: Bool = true) -> Result<Void, ComponentRefError<Void>>`",
        "Scrolls to an absolute logical offset when the list is attached.",
    ),
    (
        "ComponentRef<BottomSheet>.present",
        "`() -> Result<Void, ComponentRefError<Void>>`",
        "Presents the attached sheet; returns `notAttached` when its view is not mounted.",
    ),
    (
        "ComponentRef<BottomSheet>.dismiss",
        "`() -> Result<Void, ComponentRefError<Void>>`",
        "Dismisses the attached sheet; returns `notAttached` when its view is not mounted.",
    ),
    (
        "Reusable and plugin component ref methods",
        "`(parameters...) -> Result<T, ComponentRefError<Failure>>`; async methods require `await`",
        "Reusable `.nx` methods can return `Result<T, AppError>` with an app enum or propagate a plugin error with `throws Namespace.ErrorType`; ref calls flatten either failure into `ComponentRefError<Failure>`. Plugin native methods use their IDL signatures. Detached calls return `notAttached`.",
    ),
];

/// Every fixed-signature core API or built-in method accepted by semantic lowering.
pub const CORE_APIS: &[CoreApiDefinition] = &[
    method!(
        NetworkResponse,
        "NetworkResponse.getHeader",
        OptionalString,
        false,
        false,
        Pure,
        "Matches header names case-insensitively and returns the first value, or null when none match.",
        [arg!("name", String)]
    ),
    method!(
        NetworkResponse,
        "NetworkResponse.getHeaders",
        StringArray,
        false,
        false,
        Pure,
        "Matches header names case-insensitively and returns every value in response order; returns an empty array when none match.",
        [arg!("name", String)]
    ),
    method!(
        NetworkResponse,
        "NetworkResponse.getCookie",
        String,
        false,
        false,
        Pure,
        "Reads every Set-Cookie header, keeps each name and value before the first semicolon, preserves equals signs and quoted characters in values, and joins valid pairs with `; `. Cookie attributes are omitted; returns an empty string when no valid pair exists.",
        []
    ),
    api!(
        "Network.fetch",
        NetworkResponse,
        true,
        true,
        Network,
        "Returns every HTTP status for inspection; throws for invalid URLs, transport errors, timeouts, cancellation, response size limits, and certificate-pin failures. `timeoutSeconds` is measured in seconds. `cachePolicy` defaults to `Default` (honor HTTP cache headers); `Reload` skips the local cache; `CacheOnly` uses a cached response without contacting the server and fails on a cache miss.",
        [
            arg!("url", String),
            arg!("method", String, String("GET")),
            arg!("body", OptionalString, Null),
            arg!("headers", StringMap, EmptyMap),
            arg!("timeoutSeconds", Float64, Number("30")),
            arg!(
                "cachePolicy",
                String,
                String("Default"),
                values(NETWORK_CACHE_POLICIES)
            ),
            arg!("followRedirects", Bool, Bool(true)),
            arg!("maxResponseBytes", Int64, Number("67108864")),
            arg!("certificatePins", StringSet, EmptySet),
        ]
    ),
    api!(
        "Network.download",
        Bool,
        true,
        true,
        Network,
        "Downloads to temporary storage and replaces the destination only for a 2xx response; other HTTP statuses throw and preserve the destination. `timeoutSeconds` is measured in seconds. `cachePolicy` defaults to `Default` (honor HTTP cache headers); `Reload` skips the local cache; `CacheOnly` uses a cached response without contacting the server and fails on a cache miss.",
        [
            arg!("url", String),
            arg!("destinationPath", String),
            arg!("method", String, String("GET")),
            arg!("body", OptionalString, Null),
            arg!("headers", StringMap, EmptyMap),
            arg!("timeoutSeconds", Float64, Number("30")),
            arg!(
                "cachePolicy",
                String,
                String("Default"),
                values(NETWORK_CACHE_POLICIES)
            ),
            arg!("followRedirects", Bool, Bool(true)),
            arg!("maxResponseBytes", Int64, Number("67108864")),
            arg!("certificatePins", StringSet, EmptySet),
        ]
    ),
    api!(
        "Network.upload",
        NetworkResponse,
        true,
        true,
        Network,
        "Uploads a local file as multipart/form-data and returns every HTTP status for inspection; request and transport failures throw. `timeoutSeconds` is measured in seconds. Nexa generates the multipart Content-Type boundary, replacing any supplied Content-Type header.",
        [
            arg!("url", String),
            arg!("file", String),
            arg!("fields", StringMap, EmptyMap),
            arg!("headers", StringMap, EmptyMap),
            arg!("timeoutSeconds", Float64, Number("30")),
            arg!("followRedirects", Bool, Bool(true)),
            arg!("maxResponseBytes", Int64, Number("67108864")),
            arg!("certificatePins", StringSet, EmptySet),
        ]
    ),
    api!(
        "Path.documents",
        String,
        false,
        false,
        Environment,
        "Returns the documents directory.",
        []
    ),
    api!(
        "Path.caches",
        String,
        false,
        false,
        Environment,
        "Returns the caches directory.",
        []
    ),
    api!(
        "Path.temporary",
        String,
        false,
        false,
        Environment,
        "Returns the temporary directory.",
        []
    ),
    api!(
        "Path.appSupport",
        String,
        false,
        false,
        Environment,
        "Returns the app-support directory.",
        []
    ),
    api!(
        "Path.join",
        String,
        false,
        false,
        Pure,
        "Joins one path component using native path rules.",
        [arg!("path", String), arg!("component", String)]
    ),
    api!(
        "File.exists",
        Bool,
        false,
        false,
        Filesystem,
        "Checks whether a path exists.",
        [arg!("path", String)]
    ),
    api!(
        "File.readText",
        String,
        true,
        true,
        Filesystem,
        "Reads UTF-8 text from a file.",
        [arg!("path", String)]
    ),
    api!(
        "File.readBytes",
        Bytes,
        true,
        true,
        Filesystem,
        "Reads the exact bytes from a file.",
        [arg!("path", String)]
    ),
    api!(
        "File.writeText",
        Bool,
        true,
        true,
        Filesystem,
        "Writes UTF-8 text atomically and reports success.",
        [arg!("path", String), arg!("contents", String)]
    ),
    api!(
        "File.writeBytes",
        Bool,
        true,
        true,
        Filesystem,
        "Writes bytes atomically and reports success.",
        [arg!("path", String), arg!("contents", Bytes)]
    ),
    api!(
        "File.delete",
        Bool,
        true,
        true,
        Filesystem,
        "Deletes a file; returns false when missing and throws on other I/O failures.",
        [arg!("path", String)]
    ),
    api!(
        "Keyboard.dismiss",
        Void,
        false,
        false,
        Ui,
        "Dismisses the active keyboard.",
        []
    ),
    api!(
        "Screen.lockOrientation",
        Void,
        false,
        false,
        Ui,
        "Sets the supported screen orientation.",
        [arg!("mode", String, values(ORIENTATIONS))]
    ),
    api!(
        "Bytes.fromText",
        Bytes,
        false,
        false,
        Pure,
        "Encodes UTF-8 text as owned bytes.",
        [arg!("text", String)]
    ),
    api!(
        "Bytes.fromArray",
        Bytes,
        false,
        false,
        Pure,
        "Copies unsigned byte values into an owned byte buffer.",
        [arg!("values", UInt8Array)]
    ),
    api!(
        "Bytes.count",
        Int32,
        false,
        false,
        Pure,
        "Returns the byte length.",
        [arg!("bytes", Bytes)]
    ),
    api!(
        "Permissions.status",
        PermissionStatus,
        true,
        false,
        Permission,
        "Reads current system authorization.",
        [arg!("permission", Permission)]
    ),
    api!(
        "Permissions.request",
        PermissionStatus,
        true,
        false,
        Permission,
        "Requests system authorization.",
        [arg!("permission", Permission)]
    ),
    api!(
        "Time.now",
        Int64,
        false,
        false,
        Environment,
        "Returns Unix time in milliseconds.",
        []
    ),
    api!(
        "Time.monotonic",
        Int64,
        false,
        false,
        Environment,
        "Returns a monotonic clock value in nanoseconds.",
        []
    ),
    api!(
        "Time.elapsed",
        Int64,
        false,
        false,
        Pure,
        "Returns nanoseconds elapsed from a monotonic reading.",
        [arg!("since", Int64)]
    ),
    api!(
        "Time.sleep",
        Void,
        true,
        false,
        TaskScheduling,
        "Suspends the current task for the requested duration.",
        [arg!("milliseconds", Int64)]
    ),
    api!(
        "Time.iso8601",
        String,
        false,
        false,
        Pure,
        "Formats a timestamp in UTC as yyyy-MM-ddTHH:mm:ss.SSSZ.",
        [arg!("timestamp", Int64)]
    ),
    api!(
        "Time.iso8601ToMillis",
        OptionalInt64,
        false,
        false,
        Pure,
        "Parses UTC yyyy-MM-ddTHH:mm:ssZ with a required Z and optional three-digit millisecond fraction; rejects offsets and invalid dates.",
        [arg!("text", String)]
    ),
    api!(
        "Time.startOfDay",
        Int64,
        false,
        false,
        Environment,
        "Returns the start of day in the device's current calendar and time zone, in milliseconds.",
        [arg!("timestamp", Int64)]
    ),
    api!(
        "Time.addCalendarDays",
        Int64,
        false,
        false,
        Environment,
        "Adds calendar days in the device's current calendar and time zone; daylight-saving changes can make the elapsed duration differ from 24 hours.",
        [arg!("timestamp", Int64), arg!("days", Int32)]
    ),
    api!(
        "Time.localizedDate",
        String,
        false,
        false,
        Environment,
        "Formats a date using the device's current locale, calendar, and time zone.",
        [arg!("timestamp", Int64)]
    ),
    api!(
        "Time.localizedLongDate",
        String,
        false,
        false,
        Environment,
        "Formats a long date using the specified app language and the device calendar and time zone.",
        [arg!("timestamp", Int64), arg!("locale", String)]
    ),
    api!(
        "Time.localizedTime",
        String,
        false,
        false,
        Environment,
        "Formats a time using the device's current locale, calendar, and time zone.",
        [arg!("timestamp", Int64)]
    ),
    api!(
        "Time.localizedDateTime",
        String,
        false,
        false,
        Environment,
        "Formats a date and time using the device's current locale, calendar, and time zone.",
        [arg!("timestamp", Int64)]
    ),
    api!(
        "Time.format",
        String,
        false,
        false,
        Environment,
        "Formats a timestamp using the device's current locale, calendar, and time zone with an ICU date pattern.",
        [arg!("timestamp", Int64), arg!("pattern", String)]
    ),
    api!(
        "Time.formatDuration",
        String,
        false,
        false,
        Pure,
        "Formats a non-negative duration as zero-padded minutes and seconds (mm:ss).",
        [arg!("seconds", Float64)]
    ),
    api!(
        "AppIcon.set",
        Bool,
        true,
        false,
        Ui,
        "Sets an alternate app icon, or restores the default with null.",
        [arg!("name", OptionalString)]
    ),
    api!(
        "Number.formatCurrency",
        String,
        false,
        false,
        Environment,
        "Formats currency for the current locale.",
        [arg!("amount", Float64), arg!("currencyCode", String)]
    ),
    api!(
        "Locale.currentLanguageCode",
        String,
        false,
        false,
        Environment,
        "Returns the selected app language code, or the current device language when no override is set.",
        []
    ),
    api!(
        "Locale.setLanguageOverride",
        Void,
        false,
        false,
        PersistentStorage,
        "Persists a language override for localized text and returns to the device language when null is supplied.",
        [arg!("languageCode", OptionalString)]
    ),
    api!(
        "Locale.preferredLanguageCodes",
        StringArray,
        false,
        false,
        Environment,
        "Returns the selected app language first, followed by the device's other preferred language codes.",
        []
    ),
    api!(
        "Crypto.sha1",
        String,
        false,
        false,
        Crypto,
        "Returns a SHA-1 digest encoded as lowercase hexadecimal for protocol compatibility.",
        [arg!("text", String)]
    ),
    api!(
        "Crypto.sha256",
        String,
        false,
        false,
        Crypto,
        "Returns a SHA-256 digest encoded as lowercase hexadecimal.",
        [arg!("text", String)]
    ),
    api!(
        "Crypto.sha512",
        String,
        false,
        false,
        Crypto,
        "Returns a SHA-512 digest encoded as lowercase hexadecimal.",
        [arg!("text", String)]
    ),
    api!(
        "Crypto.hmacSha256",
        String,
        false,
        false,
        Crypto,
        "Returns an HMAC-SHA-256 digest as lowercase hexadecimal.",
        [arg!("key", String), arg!("message", String)]
    ),
    api!(
        "Crypto.randomBytes",
        String,
        false,
        false,
        Crypto,
        "Generates secure random bytes and returns lowercase hexadecimal.",
        [arg!("count", Int32)]
    ),
    api!(
        "SecureStorage.get",
        OptionalString,
        true,
        true,
        ProtectedStorage,
        "Reads a protected string value; an optional scope keeps feature data separate.",
        [arg!("key", String), arg!("scope", OptionalString, Null)]
    ),
    api!(
        "SecureStorage.set",
        Void,
        true,
        true,
        ProtectedStorage,
        "Stores a protected value in an optional feature scope.",
        [
            arg!("key", String),
            arg!("value", String),
            arg!("scope", OptionalString, Null)
        ]
    ),
    api!(
        "SecureStorage.delete",
        Void,
        true,
        true,
        ProtectedStorage,
        "Deletes one protected value from an optional feature scope.",
        [arg!("key", String), arg!("scope", OptionalString, Null)]
    ),
    api!(
        "SecureStorage.clear",
        Void,
        true,
        true,
        ProtectedStorage,
        "Clears all protected values, or only those in the requested feature scope.",
        [arg!("scope", OptionalString, Null)]
    ),
    api!(
        "Storage.getString",
        OptionalString,
        false,
        false,
        PersistentStorage,
        "Reads a non-secret preference.",
        [arg!("key", String)]
    ),
    api!(
        "Storage.setString",
        Void,
        false,
        false,
        PersistentStorage,
        "Writes a non-secret preference.",
        [arg!("key", String), arg!("value", String)]
    ),
    api!(
        "Storage.getBool",
        OptionalBool,
        false,
        false,
        PersistentStorage,
        "Reads a saved Boolean value, or null when the key is missing.",
        [arg!("key", String)]
    ),
    api!(
        "Storage.setBool",
        Void,
        false,
        false,
        PersistentStorage,
        "Saves a Boolean value.",
        [arg!("key", String), arg!("value", Bool)]
    ),
    api!(
        "Storage.getInt64",
        OptionalInt64,
        false,
        false,
        PersistentStorage,
        "Reads a saved 64-bit integer, or null when the key is missing.",
        [arg!("key", String)]
    ),
    api!(
        "Storage.setInt64",
        Void,
        false,
        false,
        PersistentStorage,
        "Saves a 64-bit integer.",
        [arg!("key", String), arg!("value", Int64)]
    ),
    api!(
        "Storage.getFloat64",
        OptionalFloat64,
        false,
        false,
        PersistentStorage,
        "Reads a saved 64-bit floating-point value, or null when the key is missing.",
        [arg!("key", String)]
    ),
    api!(
        "Storage.setFloat64",
        Void,
        false,
        false,
        PersistentStorage,
        "Saves a 64-bit floating-point value.",
        [arg!("key", String), arg!("value", Float64)]
    ),
    api!(
        "Storage.delete",
        Void,
        false,
        false,
        PersistentStorage,
        "Deletes one preference.",
        [arg!("key", String)]
    ),
    api!(
        "Storage.clear",
        Void,
        false,
        false,
        PersistentStorage,
        "Clears preferences managed by this API.",
        []
    ),
    api!(
        "Clipboard.setText",
        Void,
        false,
        false,
        Ui,
        "Replaces system clipboard text.",
        [arg!("text", String)]
    ),
    api!(
        "Clipboard.getText",
        OptionalString,
        false,
        false,
        Ui,
        "Reads clipboard text when present.",
        []
    ),
    api!(
        "Clipboard.hasText",
        Bool,
        false,
        false,
        Ui,
        "Checks whether clipboard text is available.",
        []
    ),
    api!(
        "Haptics.impact",
        Void,
        false,
        false,
        Haptics,
        "Triggers an impact haptic.",
        [arg!("style", String, values(IMPACT_STYLES))]
    ),
    api!(
        "Haptics.notification",
        Void,
        false,
        false,
        Haptics,
        "Triggers a notification haptic.",
        [arg!("kind", String, values(NOTIFICATION_KINDS))]
    ),
    api!(
        "Haptics.selection",
        Void,
        false,
        false,
        Haptics,
        "Triggers a selection haptic.",
        []
    ),
    api!(
        "Log.info",
        Void,
        false,
        false,
        Logging,
        "Writes an informational native log entry.",
        [arg!("message", String)]
    ),
    api!(
        "Log.warning",
        Void,
        false,
        false,
        Logging,
        "Writes a warning native log entry.",
        [arg!("message", String)]
    ),
    api!(
        "Log.error",
        Void,
        false,
        false,
        Logging,
        "Writes an error native log entry.",
        [arg!("message", String)]
    ),
    api!(
        "Regex.isMatch",
        Bool,
        false,
        false,
        Pure,
        "Returns whether a pattern occurs in the input.",
        [arg!("pattern", String), arg!("text", String)]
    ),
    api!(
        "Regex.matches",
        StringArray,
        false,
        false,
        Pure,
        "Returns all non-overlapping matched substrings.",
        [arg!("pattern", String), arg!("text", String)]
    ),
    api!(
        "Regex.replace",
        String,
        false,
        false,
        Pure,
        "Replaces all matches using native replacement syntax.",
        [
            arg!("pattern", String),
            arg!("text", String),
            arg!("replacement", String)
        ]
    ),
];

pub(crate) fn lookup(qualified_name: &str) -> Option<&'static CoreApiDefinition> {
    CORE_APIS
        .iter()
        .find(|api| api.qualified_name == qualified_name)
}

pub(crate) fn lookup_method(receiver: ApiType, name: &str) -> Option<&'static CoreApiDefinition> {
    CORE_APIS.iter().find(|api| {
        api.receiver == Some(receiver) && api.qualified_name.rsplit('.').next() == Some(name)
    })
}

pub(crate) fn is_core_namespace(namespace: &str) -> bool {
    matches!(
        namespace,
        "Network"
            | "Path"
            | "File"
            | "Permissions"
            | "Bytes"
            | "Time"
            | "Log"
            | "Keyboard"
            | "Number"
            | "Json"
            | "Crypto"
            | "SecureStorage"
            | "Storage"
            | "Clipboard"
            | "Haptics"
            | "Regex"
            | "Locale"
            | "Screen"
            | "AppIcon"
            | "Navigation"
            | "ComponentRef"
    ) || CORE_APIS.iter().any(|api| {
        api.receiver.is_none() && api.qualified_name.starts_with(&format!("{namespace}."))
    })
}

pub(crate) fn render_reference() -> String {
    let mut output = String::from(
        "All callable built-in APIs below execute at runtime. Compile-time values come from declarations and project configuration rather than API calls.\n\n| API | Typed signature | Effect and behavior |\n|---|---|---|\n",
    );
    for api in CORE_APIS {
        let arguments = api
            .arguments
            .iter()
            .map(|argument| {
                let default = argument
                    .default
                    .spelling()
                    .map(|value| format!(" = {value}"))
                    .unwrap_or_default();
                let allowed_values = if argument.allowed_values.is_empty() {
                    String::new()
                } else {
                    format!(" {{{}}}", argument.allowed_values.join(" | "))
                };
                format!(
                    "{}: {}{default}{allowed_values}",
                    argument.name,
                    argument.ty.spelling()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let asynchronous = if api.asynchronous { "async " } else { "" };
        let throwing = if api.throwing { " throws" } else { "" };
        output.push_str(&format!(
            "| `{}` | `{asynchronous}({arguments}) -> {}{throwing}` | {} — {} |\n",
            api.qualified_name,
            api.result.spelling(),
            api.effect.label(),
            api.summary
        ));
    }
    for (name, signature, behavior) in SPECIAL_API_ROWS {
        output.push_str(&format!("| `{name}` | {signature} | {behavior} |\n"));
    }
    output.push_str(
        "\n| Built-in type | Values or fields |\n|---|---|\n\
         | `Permission` | `Camera`, `Microphone`, `Photos`, `Location`, `Notifications`, `Contacts`, `Calendar`, `Bluetooth`, `Motion` |\n\
         | `PermissionStatus` | `granted`, `limited`, `provisional`, `denied`, `permanentlyDenied`, `restricted`, `notDetermined` |\n\
         | `NetworkResponse` | `statusCode: Int32`, `body: String`, `getHeader(name: String): String?`, `getHeaders(name: String): Array<String>`, `getCookie(): String` |\n",
    );
    output.push_str(
        "| `JsonError` | `invalidJson`, `typeMismatch`, `missingField`, `invalidValue` |\n\
         | `ComponentRefError<Failure>` | `notAttached`, `failure(Failure)` |\n",
    );
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_unique_names_and_valid_parameter_defaults() {
        let mut names = std::collections::HashSet::new();
        for api in CORE_APIS {
            assert!(
                names.insert(api.qualified_name),
                "duplicate API {}",
                api.qualified_name
            );
            let mut parameters = std::collections::HashSet::new();
            for argument in api.arguments {
                assert!(
                    parameters.insert(argument.name),
                    "duplicate argument {} on {}",
                    argument.name,
                    api.qualified_name
                );
                if !argument.allowed_values.is_empty() {
                    assert_eq!(argument.ty, ApiType::String);
                }
                if argument.default != ApiDefault::None {
                    assert!(argument.default.to_ast(Span::default()).is_some());
                }
            }
        }
    }

    #[test]
    fn core_api_catalog_is_the_return_type_source_for_inference() {
        let symbols = std::collections::HashMap::new();
        let functions = super::super::expressions::FunctionSignatures::new();
        for api in CORE_APIS {
            if api.receiver.is_some() {
                continue;
            }
            let (namespace, name) = api
                .qualified_name
                .split_once('.')
                .expect("core API names are qualified");
            let expression = ast::Expr::QualifiedCall {
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                type_arguments: Vec::new(),
                arguments: Vec::new(),
                named_arguments: std::collections::BTreeMap::new(),
                span: Span::default(),
            };
            assert_eq!(
                super::super::expressions::infer_expr_type(&expression, &symbols, &functions),
                Some(api.result.to_ir()),
                "{} should infer from its catalog result type",
                api.qualified_name,
            );
        }
    }

    #[test]
    fn network_response_method_signatures_come_from_the_core_api_catalog() {
        let get_header = lookup_method(ApiType::NetworkResponse, "getHeader").unwrap();
        assert_eq!(get_header.arguments.len(), 1);
        assert_eq!(get_header.arguments[0].name, "name");
        assert_eq!(get_header.arguments[0].ty, ApiType::String);
        assert_eq!(get_header.result, ApiType::OptionalString);

        let get_headers = lookup_method(ApiType::NetworkResponse, "getHeaders").unwrap();
        assert_eq!(get_headers.arguments.len(), 1);
        assert_eq!(get_headers.arguments[0].name, "name");
        assert_eq!(get_headers.arguments[0].ty, ApiType::String);
        assert_eq!(get_headers.result, ApiType::StringArray);

        let get_cookie = lookup_method(ApiType::NetworkResponse, "getCookie").unwrap();
        assert!(get_cookie.arguments.is_empty());
        assert_eq!(get_cookie.result, ApiType::String);
        assert!(lookup_method(ApiType::NetworkResponse, "unknown").is_none());
    }

    #[test]
    fn reference_publishes_typed_network_header_values() {
        let rendered = render_reference();
        assert!(rendered.contains("`NetworkResponse` | `statusCode: Int32`, `body: String`"));
        assert!(rendered.contains("`NetworkResponse.getHeader`"));
        assert!(!rendered.contains("HttpHeader"));
    }

    #[test]
    fn reference_publishes_specialized_network_json_and_component_ref_contracts() {
        let rendered = render_reference();
        for expected in [
            "`NetworkResponse.getHeaders`",
            "`NetworkResponse.getCookie`",
            "`(name: String) -> Array<String>`",
            "empty array when none match",
            "empty string when no valid pair exists",
            "`Network.isOnline`",
            "`Network.onStatusChange`",
            "`JsonError` | `invalidJson`, `typeMismatch`, `missingField`, `invalidValue`",
            "`ComponentRefError<Failure>` | `notAttached`, `failure(Failure)`",
        ] {
            assert!(
                rendered.contains(expected),
                "API reference is missing `{expected}`"
            );
        }
        assert!(!rendered.contains("`JSON.parse<T>`"));
        assert!(!rendered.contains("`JSON.stringify<T>`"));
    }

    #[test]
    fn bottom_sheet_presentation_is_documented_as_typed_component_ref_methods() {
        let rendered = render_reference();
        for expected in [
            "`ComponentRef<BottomSheet>.present`",
            "`ComponentRef<BottomSheet>.dismiss`",
            "`notAttached` when its view is not mounted",
        ] {
            assert!(
                rendered.contains(expected),
                "API reference is missing `{expected}`"
            );
        }
        assert!(!rendered.contains("`BottomSheet.snapTo`"));
    }
}
