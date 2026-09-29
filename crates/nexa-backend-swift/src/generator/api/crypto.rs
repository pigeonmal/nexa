//! Core cryptographic helpers implemented with CryptoKit and Security.

use nexa_codegen::SourceWriter;

use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let enabled = features.facts.capabilities.uses_crypto_api;
    imports.add(enabled, "CryptoKit");
    imports.add(enabled, "Foundation");
    imports.add(enabled, "Security");
}

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"// Core cryptographic helpers.

private func nexaCryptoHex<D: Sequence>(_ bytes: D) -> String where D.Element == UInt8 {
    let digits = Array("0123456789abcdef".utf8)
    var encoded: [UInt8] = []
    encoded.reserveCapacity(bytes.underestimatedCount * 2)
    for byte in bytes {
        encoded.append(digits[Int(byte >> 4)])
        encoded.append(digits[Int(byte & 0x0f)])
    }
    return String(decoding: encoded, as: UTF8.self)
}

func nexaCryptoSha256(_ text: String) -> String {
    nexaCryptoHex(SHA256.hash(data: Data(text.utf8)))
}

func nexaCryptoSha512(_ text: String) -> String {
    nexaCryptoHex(SHA512.hash(data: Data(text.utf8)))
}

func nexaCryptoHmacSha256(_ key: String, _ message: String) -> String {
    let key = SymmetricKey(data: Data(key.utf8))
    let code = HMAC<SHA256>.authenticationCode(for: Data(message.utf8), using: key)
    return nexaCryptoHex(code)
}

func nexaCryptoRandomBytes(_ count: Int32) -> String {
    guard count > 0 else { return "" }
    var bytes = Data(count: Int(count))
    let status = bytes.withUnsafeMutableBytes { buffer in
        guard let address = buffer.baseAddress else { return errSecParam }
        return SecRandomCopyBytes(kSecRandomDefault, buffer.count, address)
    }
    guard status == errSecSuccess else { return "" }
    return bytes.base64EncodedString()
}
"#,
    );
}
