//! App-private string storage backed by the iOS UserDefaults domain.

use nexa_codegen::SourceWriter;

use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.facts.capabilities.uses_storage_api, "Foundation");
}

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// App-private string storage. Keys are isolated from unrelated defaults.
enum NexaStorage {
    private static let keyPrefix = "dev.nexa.storage."
    // UserDefaults is thread-safe, but its reference type is not Sendable on
    // every supported Foundation version. The static binding is immutable;
    // this declaration avoids adding actor hops to the synchronous API. When
    // an App Group is configured, use that suite so extensions can read the
    // same small app settings; otherwise retain the standard app-private suite.
    private nonisolated(unsafe) static let defaults = {
        guard let group = Bundle.main.object(forInfoDictionaryKey: "NexaAppGroupIdentifier") as? String,
              let shared = UserDefaults(suiteName: group) else {
            return UserDefaults.standard
        }
        return shared
    }()

    private static func storageKey(_ key: String, type: String? = nil) -> String {
        keyPrefix + (type.map { $0 + "." } ?? "") + key
    }

    private static func removeOtherTypes(for key: String, keeping type: String?) {
        for candidate in [nil, "bool", "int64", "float64", "json"] as [String?] where candidate != type {
            defaults.removeObject(forKey: storageKey(key, type: candidate))
        }
    }

    static func getString(_ key: String) -> String? {
        defaults.string(forKey: storageKey(key))
    }

    static func setString(_ key: String, _ value: String) {
        removeOtherTypes(for: key, keeping: nil)
        defaults.set(value, forKey: storageKey(key))
    }

    static func getJson(_ key: String) -> String? {
        defaults.string(forKey: storageKey(key, type: "json"))
    }

    static func setJson(_ key: String, _ value: String) {
        removeOtherTypes(for: key, keeping: "json")
        defaults.set(value, forKey: storageKey(key, type: "json"))
    }

    static func getBool(_ key: String) -> Bool? {
        (defaults.object(forKey: storageKey(key, type: "bool")) as? NSNumber)?.boolValue
    }

    static func setBool(_ key: String, _ value: Bool) {
        removeOtherTypes(for: key, keeping: "bool")
        defaults.set(value, forKey: storageKey(key, type: "bool"))
    }

    static func getInt64(_ key: String) -> Int64? {
        (defaults.object(forKey: storageKey(key, type: "int64")) as? NSNumber)?.int64Value
    }

    static func setInt64(_ key: String, _ value: Int64) {
        removeOtherTypes(for: key, keeping: "int64")
        defaults.set(NSNumber(value: value), forKey: storageKey(key, type: "int64"))
    }

    static func getFloat64(_ key: String) -> Double? {
        (defaults.object(forKey: storageKey(key, type: "float64")) as? NSNumber)?.doubleValue
    }

    static func setFloat64(_ key: String, _ value: Double) {
        removeOtherTypes(for: key, keeping: "float64")
        defaults.set(NSNumber(value: value), forKey: storageKey(key, type: "float64"))
    }

    static func delete(_ key: String) {
        removeOtherTypes(for: key, keeping: nil)
        defaults.removeObject(forKey: storageKey(key))
    }

    static func clear() {
        for key in defaults.dictionaryRepresentation().keys where key.hasPrefix(keyPrefix) {
            defaults.removeObject(forKey: key)
        }
    }
}
"#,
    );
}
