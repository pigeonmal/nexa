//! Core secure string storage backed by Keychain generic-password items.

use nexa_codegen::SourceWriter;

use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let enabled = features.facts.capabilities.uses_secure_storage_api;
    imports.add(enabled, "Foundation");
    imports.add(enabled, "Security");
}

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// Core secure string storage backed by the system Keychain.

private enum NexaSecureStorageError: Error {
    case keychain(OSStatus)
    case invalidUTF8
}

enum NexaSecureStorage {
    private static let baseService = Bundle.main.bundleIdentifier ?? "dev.nexa.application"
    private static let scopedServicePrefix = "\(baseService).nexa.secure-storage.scope."

    private static func service(for scope: String?) -> String {
        guard let scope else { return baseService }
        return "\(scopedServicePrefix)\(scope.utf8.count):\(scope)"
    }

    private static func query(for key: String, scope: String? = nil) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service(for: scope),
            kSecAttrAccount as String: key,
        ]
    }

    private static func getSynchronously(_ key: String, scope: String?) throws -> String? {
        var request = query(for: key, scope: scope)
        request[kSecReturnData as String] = true
        request[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        let status = SecItemCopyMatching(request as CFDictionary, &item)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw NexaSecureStorageError.keychain(status) }
        guard let data = item as? Data,
              let value = String(data: data, encoding: .utf8)
        else { throw NexaSecureStorageError.invalidUTF8 }
        return value
    }

    private static func setSynchronously(_ value: String, for key: String, scope: String?) throws {
        let itemQuery = query(for: key, scope: scope)
        let data = Data(value.utf8)
        let update = [kSecValueData as String: data] as CFDictionary
        let updateStatus = SecItemUpdate(itemQuery as CFDictionary, update)
        if updateStatus == errSecSuccess { return }
        guard updateStatus == errSecItemNotFound else {
            throw NexaSecureStorageError.keychain(updateStatus)
        }

        var insertion = itemQuery
        insertion[kSecValueData as String] = data
        insertion[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        let addStatus = SecItemAdd(insertion as CFDictionary, nil)
        if addStatus == errSecSuccess { return }
        // A competing writer may have inserted the same account after the
        // initial update lookup. Retry the value-only update in that case.
        if addStatus == errSecDuplicateItem {
            let retryStatus = SecItemUpdate(itemQuery as CFDictionary, update)
            guard retryStatus == errSecSuccess else {
                throw NexaSecureStorageError.keychain(retryStatus)
            }
            return
        }
        throw NexaSecureStorageError.keychain(addStatus)
    }

    private static func deleteSynchronously(_ key: String, scope: String?) throws {
        let status = SecItemDelete(query(for: key, scope: scope) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw NexaSecureStorageError.keychain(status)
        }
    }

    private static func clearSynchronously(_ scope: String?) throws {
        var request: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service(for: scope),
        ]
        if scope != nil {
            let status = SecItemDelete(request as CFDictionary)
            guard status == errSecSuccess || status == errSecItemNotFound else {
                throw NexaSecureStorageError.keychain(status)
            }
            return
        }

        request.removeValue(forKey: kSecAttrService as String)
        request[kSecReturnAttributes as String] = true
        request[kSecMatchLimit as String] = kSecMatchLimitAll
        var result: CFTypeRef?
        let readStatus = SecItemCopyMatching(request as CFDictionary, &result)
        if readStatus == errSecItemNotFound { return }
        guard readStatus == errSecSuccess,
              let items = result as? [[String: Any]]
        else { throw NexaSecureStorageError.keychain(readStatus) }

        for item in items {
            guard let storedService = item[kSecAttrService as String] as? String,
                  storedService == baseService || storedService.hasPrefix(scopedServicePrefix),
                  let storedAccount = item[kSecAttrAccount as String] as? String
            else { continue }
            let itemQuery: [String: Any] = [
                kSecClass as String: kSecClassGenericPassword,
                kSecAttrService as String: storedService,
                kSecAttrAccount as String: storedAccount,
            ]
            let deleteStatus = SecItemDelete(itemQuery as CFDictionary)
            guard deleteStatus == errSecSuccess || deleteStatus == errSecItemNotFound else {
                throw NexaSecureStorageError.keychain(deleteStatus)
            }
        }
    }

    static func get(_ key: String, _ scope: String? = nil) async throws -> String? {
        try await Task.detached(priority: .utility) { try getSynchronously(key, scope: scope) }.value
    }

    static func set(_ key: String, _ value: String, _ scope: String? = nil) async throws {
        try await Task.detached(priority: .utility) { try setSynchronously(value, for: key, scope: scope) }.value
    }

    static func delete(_ key: String, _ scope: String? = nil) async throws {
        try await Task.detached(priority: .utility) { try deleteSynchronously(key, scope: scope) }.value
    }

    static func clear(_ scope: String? = nil) async throws {
        try await Task.detached(priority: .utility) { try clearSynchronously(scope) }.value
    }
}
"#,
    );
}
