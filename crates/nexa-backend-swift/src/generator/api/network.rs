use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;
/// Emits the feature-gated native networking, path, and file APIs.
///
/// URLSession owns connection pooling, HTTP caching, redirects, TLS, and
/// HTTP/2 negotiation. Nexa only adds the typed request options that generated
/// code needs; it does not introduce a cross-platform request runtime.
use nexa_codegen::SourceWriter;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_network_transport() || features.uses_path_api || features.uses_file_api,
        "Foundation",
    );
    imports.add(features.uses_network_api, "CryptoKit");
    imports.add(features.uses_network_api, "Security");
    imports.add(features.uses_remote_image, "ImageIO");
}

pub(crate) fn render(
    out: &mut SourceWriter,
    include_network: bool,
    include_image_support: bool,
    include_path: bool,
    include_file: bool,
    include_file_async: bool,
) {
    if include_network || include_image_support {
        out.push_str(
            r#"
private enum NexaURLSessionSupport {
    static let configuration: URLSessionConfiguration = {
        let configuration = URLSessionConfiguration.default
        configuration.urlCache = URLCache(
            memoryCapacity: 16 * 1024 * 1024,
            diskCapacity: 64 * 1024 * 1024
        )
        configuration.requestCachePolicy = .useProtocolCachePolicy
        return configuration
    }()
    static let sharedSession = URLSession(configuration: configuration)
}

"#,
        );
    }
    if include_network {
        out.push_str(
            r#"
public enum NexaNetworkError: Error {
    case invalidURL
    case invalidResponse
    case httpStatus(Int)
    case responseTooLarge
}

public struct NexaNetworkResponse {
    public let statusCode: Int32
    public let headers: [String: [String]]
    public let body: Data

    public var text: String { String(decoding: body, as: UTF8.self) }
}

private enum NexaCertificatePin {
    private struct DERElement {
        let tag: UInt8
        let contentStart: Int
        let contentEnd: Int
        let encodedRange: Range<Int>
    }

    static func sha256SPKI(for certificate: SecCertificate) -> String? {
        sha256SPKI(in: SecCertificateCopyData(certificate) as Data)
    }

    static func sha256SPKI(in certificateData: Data) -> String? {
        let bytes = [UInt8](certificateData)
        guard let range = subjectPublicKeyInfoRange(in: bytes) else { return nil }
        let digest = SHA256.hash(data: Data(bytes[range]))
        return digest.map { String(format: "%02x", $0) }.joined()
    }

    private static func subjectPublicKeyInfoRange(in bytes: [UInt8]) -> Range<Int>? {
        guard let certificate = element(in: bytes, at: 0, limit: bytes.count),
              certificate.tag == 0x30,
              certificate.contentEnd == bytes.count,
              let tbs = element(in: bytes, at: certificate.contentStart, limit: certificate.contentEnd),
              tbs.tag == 0x30 else { return nil }

        var offset = tbs.contentStart
        if let version = element(in: bytes, at: offset, limit: tbs.contentEnd), version.tag == 0xa0 {
            offset = version.contentEnd
        }
        for expectedTag: UInt8 in [0x02, 0x30, 0x30, 0x30, 0x30] {
            guard let field = element(in: bytes, at: offset, limit: tbs.contentEnd),
                  field.tag == expectedTag else { return nil }
            offset = field.contentEnd
        }
        guard let subjectPublicKeyInfo = element(in: bytes, at: offset, limit: tbs.contentEnd),
              subjectPublicKeyInfo.tag == 0x30 else { return nil }
        return subjectPublicKeyInfo.encodedRange
    }

    private static func element(in bytes: [UInt8], at offset: Int, limit: Int) -> DERElement? {
        guard offset >= 0, limit <= bytes.count, offset < limit, limit - offset >= 2 else { return nil }
        let tag = bytes[offset]
        guard tag & 0x1f != 0x1f else { return nil }
        var cursor = offset + 1
        let firstLengthByte = bytes[cursor]
        cursor += 1
        let length: Int
        if firstLengthByte & 0x80 == 0 {
            length = Int(firstLengthByte)
        } else {
            let lengthByteCount = Int(firstLengthByte & 0x7f)
            guard lengthByteCount > 0, lengthByteCount <= 4,
                  lengthByteCount <= limit - cursor,
                  bytes[cursor] != 0 else { return nil }
            var decodedLength = 0
            for _ in 0..<lengthByteCount {
                decodedLength = decodedLength * 256 + Int(bytes[cursor])
                cursor += 1
            }
            guard decodedLength >= 128 else { return nil }
            length = decodedLength
        }
        guard length <= limit - cursor else { return nil }
        let contentEnd = cursor + length
        return DERElement(tag: tag, contentStart: cursor, contentEnd: contentEnd, encodedRange: offset..<contentEnd)
    }
}

private final class NexaURLSessionDelegate: NSObject, URLSessionTaskDelegate {
    let followRedirects: Bool
    let certificatePins: Set<String>

    init(followRedirects: Bool, certificatePins: Set<String>) {
        self.followRedirects = followRedirects
        self.certificatePins = Set(certificatePins.map { $0.lowercased() })
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest,
        completionHandler: @escaping (URLRequest?) -> Void
    ) {
        completionHandler(followRedirects ? request : nil)
    }

    func urlSession(
        _ session: URLSession,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        guard challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
              let trust = challenge.protectionSpace.serverTrust,
              !certificatePins.isEmpty else {
            completionHandler(.performDefaultHandling, nil)
            return
        }

        guard SecTrustEvaluateWithError(trust, nil),
              let chain = SecTrustCopyCertificateChain(trust) as? [SecCertificate] else {
            completionHandler(.cancelAuthenticationChallenge, nil)
            return
        }
        let matchesPin = chain.contains { certificate in
            guard let digest = NexaCertificatePin.sha256SPKI(for: certificate) else { return false }
            return certificatePins.contains(digest)
        }
        if matchesPin {
            completionHandler(.useCredential, URLCredential(trust: trust))
        } else {
            completionHandler(.cancelAuthenticationChallenge, nil)
        }
    }
}

public enum NexaNetwork {
    public static func fetch(
        url: String,
        method: String = "GET",
        body: Data? = nil,
        headers: [String: String] = [:],
        timeout: TimeInterval = 30,
        useCache: Bool = true,
        followRedirects: Bool = true,
        maxResponseBytes: Int = 64 * 1024 * 1024,
        certificatePins: Set<String> = []
    ) async throws -> NexaNetworkResponse {
        guard let url = URL(string: url) else { throw NexaNetworkError.invalidURL }
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.httpBody = body
        request.timeoutInterval = timeout
        request.cachePolicy = useCache ? .useProtocolCachePolicy : .reloadIgnoringLocalCacheData
        for (name, value) in headers {
            request.setValue(value, forHTTPHeaderField: name)
        }

        let delegate: NexaURLSessionDelegate? = (followRedirects && certificatePins.isEmpty)
            ? nil
            : NexaURLSessionDelegate(
                followRedirects: followRedirects,
                certificatePins: certificatePins
            )
        let session = delegate == nil
            ? NexaURLSessionSupport.sharedSession
            : URLSession(configuration: NexaURLSessionSupport.configuration, delegate: delegate, delegateQueue: nil)
        defer {
            if delegate != nil { session.finishTasksAndInvalidate() }
        }
        let (data, response) = try await session.data(for: request)
        guard let response = response as? HTTPURLResponse else {
            throw NexaNetworkError.invalidResponse
        }
        if data.count > maxResponseBytes {
            throw NexaNetworkError.responseTooLarge
        }
        guard (200..<300).contains(response.statusCode) else {
            throw NexaNetworkError.httpStatus(response.statusCode)
        }
        let headers = Dictionary(grouping: response.allHeaderFields.compactMap { key, value -> (String, String)? in
            guard let key = key as? String else { return nil }
            return (key, String(describing: value))
        }, by: { $0.0 }).mapValues { $0.map(\.1) }
        return NexaNetworkResponse(
            statusCode: Int32(response.statusCode),
            headers: headers,
            body: data
        )
    }

    public static func download(
        url: String,
        destinationPath: String,
        method: String = "GET",
        body: Data? = nil,
        headers: [String: String] = [:],
        timeout: TimeInterval = 30,
        useCache: Bool = true,
        followRedirects: Bool = true,
        maxResponseBytes: Int = 64 * 1024 * 1024,
        certificatePins: Set<String> = []
    ) async throws -> Bool {
        guard let url = URL(string: url) else { throw NexaNetworkError.invalidURL }
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.httpBody = body
        request.timeoutInterval = timeout
        request.cachePolicy = useCache ? .useProtocolCachePolicy : .reloadIgnoringLocalCacheData
        for (name, value) in headers {
            request.setValue(value, forHTTPHeaderField: name)
        }
        let delegate: NexaURLSessionDelegate? = (followRedirects && certificatePins.isEmpty)
            ? nil
            : NexaURLSessionDelegate(
                followRedirects: followRedirects,
                certificatePins: certificatePins
            )
        let session = delegate == nil
            ? NexaURLSessionSupport.sharedSession
            : URLSession(configuration: NexaURLSessionSupport.configuration, delegate: delegate, delegateQueue: nil)
        defer {
            if delegate != nil { session.finishTasksAndInvalidate() }
        }
        let (temporaryURL, response) = try await session.download(for: request)
        guard let response = response as? HTTPURLResponse else {
            throw NexaNetworkError.invalidResponse
        }
        guard (200..<300).contains(response.statusCode) else {
            throw NexaNetworkError.httpStatus(response.statusCode)
        }
        let destination = URL(fileURLWithPath: destinationPath)
        if let size = try? FileManager.default.attributesOfItem(atPath: temporaryURL.path)[.size] as? NSNumber,
           size.intValue > maxResponseBytes {
            throw NexaNetworkError.responseTooLarge
        }
        try FileManager.default.createDirectory(
            at: destination.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        if FileManager.default.fileExists(atPath: destination.path) {
            try FileManager.default.removeItem(at: destination)
        }
        try FileManager.default.moveItem(at: temporaryURL, to: destination)
        return true
    }
}

"#,
        );
    }
    if include_path {
        out.push_str(
            r#"public enum NexaPath {
    public static func documents(_ components: String... ) -> String {
        append(components, to: FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0])
    }

    public static func caches(_ components: String... ) -> String {
        append(components, to: FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0])
    }

    public static func temporary(_ components: String... ) -> String {
        append(components, to: URL(fileURLWithPath: NSTemporaryDirectory()))
    }

    public static func appSupport(_ components: String... ) -> String {
        append(components, to: FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0])
    }

    public static func join(_ path: String, _ components: String...) -> String {
        append(components, to: URL(fileURLWithPath: path))
    }

    private static func append(_ components: [String], to url: URL) -> String {
        components.reduce(url) { $0.appendingPathComponent($1) }.path
    }
}

"#,
        );
    }
    if include_file {
        out.push_str(
            r#"public enum NexaFile {
"#,
        );
        if include_file_async {
            out.push_str(
                r#"    public static func read(_ path: String) async throws -> Data {
        try await Task.detached(priority: .utility) {
            try Data(contentsOf: URL(fileURLWithPath: path), options: .mappedIfSafe)
        }.value
    }

    public static func write(_ data: Data, to path: String) async throws {
        try await Task.detached(priority: .utility) {
            let url = URL(fileURLWithPath: path)
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )
            try data.write(to: url, options: .atomic)
        }.value
    }

    public static func readText(_ path: String) async throws -> String {
        String(decoding: try await read(path), as: UTF8.self)
    }

    public static func writeText(_ text: String, to path: String) async throws -> Bool {
        try await write(Data(text.utf8), to: path)
        return true
    }

    public static func delete(_ path: String) async throws -> Bool {
        try await Task.detached(priority: .utility) {
            try FileManager.default.removeItem(atPath: path)
        }.value
        return true
    }

"#,
            );
        }
        out.push_str(
            r#"    public static func exists(_ path: String) -> Bool {
        FileManager.default.fileExists(atPath: path)
    }
}

"#,
        );
    }
    if include_image_support {
        out.push_str(
            r#"
private enum NexaRemoteImageError: Error {
    case invalidURL
    case invalidResponse
}

private func nexaDownsampleImage(_ data: Data, maxPixelSize: Int = 2048) -> UIImage? {
    guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
    let options: [CFString: Any] = [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: maxPixelSize
    ]
    guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else {
        return nil
    }
    return UIImage(cgImage: image)
}

private enum NexaImageScale {
    case fit
    case fill
}

private struct NexaRemoteImage: View {
    let url: String
    let scale: NexaImageScale
    let placeholder: String?
    @State private var image: Image?
    @State private var failed = false

    var body: some View {
        Group {
            if let image {
                image.resizable().aspectRatio(contentMode: scale == .fit ? .fit : .fill)
            } else if failed {
                if let placeholder {
                    Image(placeholder).resizable().aspectRatio(contentMode: scale == .fit ? .fit : .fill)
                } else {
                    Image(systemName: "photo")
                }
            } else {
                ProgressView()
            }
        }
        .task(id: url) {
            do {
                guard let parsedURL = URL(string: url), parsedURL.scheme?.lowercased() == "https" else {
                    throw NexaRemoteImageError.invalidURL
                }
                let (data, response) = try await NexaURLSessionSupport.sharedSession.data(from: parsedURL)
                guard let httpResponse = response as? HTTPURLResponse,
                      (200..<300).contains(httpResponse.statusCode),
                      data.count <= 64 * 1024 * 1024,
                      let decoded = nexaDownsampleImage(data) else {
                    throw NexaRemoteImageError.invalidResponse
                }
                image = Image(uiImage: decoded)
            } catch {
                failed = true
            }
        }
    }
}
"#,
        );
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;

    use super::render;

    #[test]
    fn network_pins_hash_the_trusted_chain_spki_instead_of_the_leaf_certificate() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false);

        assert!(output.contains("SecTrustEvaluateWithError(trust, nil)"));
        assert!(output.contains("SecTrustCopyCertificateChain(trust)"));
        assert!(output.contains("chain.contains { certificate in"));
        assert!(output.contains("subjectPublicKeyInfoRange(in: bytes)"));
        assert!(output.contains("SHA256.hash(data: Data(bytes[range]))"));
        assert!(output.contains("certificatePins.map { $0.lowercased() }"));
        assert!(!output.contains("SHA256.hash(data: certificateData)"));
    }
}
