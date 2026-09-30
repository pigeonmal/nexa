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
    private static let defaults = UserDefaults.standard

    static func getString(_ key: String) -> String? {
        defaults.string(forKey: keyPrefix + key)
    }

    static func setString(_ key: String, _ value: String) {
        defaults.set(value, forKey: keyPrefix + key)
    }

    static func delete(_ key: String) {
        defaults.removeObject(forKey: keyPrefix + key)
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
