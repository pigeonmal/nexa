/// Emits the feature-gated native networking, path, and file APIs.
///
/// URLSession owns connection pooling, HTTP caching, redirects, TLS, and
/// HTTP/2 negotiation. Nexa only adds the typed request options that generated
/// code needs; it does not introduce a cross-platform request runtime.
pub(super) fn render(
    out: &mut String,
    include_network: bool,
    include_image_support: bool,
    include_path: bool,
    include_file: bool,
    include_file_async: bool,
) {
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

private final class NexaURLSessionDelegate: NSObject, URLSessionTaskDelegate {
    let followRedirects: Bool
    let certificatePins: Set<String>

    init(followRedirects: Bool, certificatePins: Set<String>) {
        self.followRedirects = followRedirects
        self.certificatePins = certificatePins
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
              let certificate = (SecTrustCopyCertificateChain(trust) as? [SecCertificate])?.first else {
            completionHandler(.cancelAuthenticationChallenge, nil)
            return
        }
        let certificateData = SecCertificateCopyData(certificate) as Data
        let digest = SHA256.hash(data: certificateData)
            .map { String(format: "%02x", $0) }
            .joined()
        if certificatePins.contains(digest) {
            completionHandler(.useCredential, URLCredential(trust: trust))
        } else {
            completionHandler(.cancelAuthenticationChallenge, nil)
        }
    }
}

public enum NexaNetwork {
    private static let configuration: URLSessionConfiguration = {
        let configuration = URLSessionConfiguration.default
        configuration.urlCache = URLCache(
            memoryCapacity: 16 * 1024 * 1024,
            diskCapacity: 64 * 1024 * 1024
        )
        configuration.requestCachePolicy = .useProtocolCachePolicy
        return configuration
    }()
    private static let sharedSession = URLSession(configuration: configuration)

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
            ? sharedSession
            : URLSession(configuration: configuration, delegate: delegate, delegateQueue: nil)
        defer {
            if delegate != nil { session.finishTasksAndInvalidate() }
        }
        let (bytes, response) = try await session.bytes(for: request)
        guard let response = response as? HTTPURLResponse else {
            throw NexaNetworkError.invalidResponse
        }

        var data = Data()
        data.reserveCapacity(min(maxResponseBytes, 64 * 1024))
        for try await byte in bytes {
            if data.count >= maxResponseBytes {
                throw NexaNetworkError.responseTooLarge
            }
            data.append(byte)
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
            ? sharedSession
            : URLSession(configuration: configuration, delegate: delegate, delegateQueue: nil)
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
                    throw NexaNetworkError.invalidURL
                }
                let response = try await NexaNetwork.fetch(url: url)
                guard let decoded = UIImage(data: response.body) else {
                    throw NexaNetworkError.invalidResponse
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
