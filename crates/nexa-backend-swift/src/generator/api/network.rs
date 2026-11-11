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
        features.uses_network_transport()
            || features.uses_network_connectivity
            || features.uses_path_api
            || features.uses_file_api,
        "Foundation",
    );
    imports.add(features.uses_network_transport(), "os");
    imports.add(features.uses_network_api, "CryptoKit");
    imports.add(features.uses_network_api, "Security");
    imports.add(features.uses_network_connectivity, "Network");
    imports.add(features.uses_remote_image, "ImageIO");
}

pub(crate) fn render(
    out: &mut SourceWriter,
    include_network: bool,
    include_image_support: bool,
    include_path: bool,
    include_file: bool,
    include_file_async: bool,
    include_network_connectivity: bool,
) {
    if include_network_connectivity {
        out.push_str(
            r#"
private final class NexaNetworkPathStatus: @unchecked Sendable {
    static let shared = NexaNetworkPathStatus()

    private let monitor = NWPathMonitor()
    private let lock = NSLock()
    private var online = false
    private var statusHandler: ((Bool) -> Void)?

    private init() {
        monitor.pathUpdateHandler = { [weak self] path in
            self?.setOnline(path.status == .satisfied)
        }
        monitor.start(queue: DispatchQueue(label: "dev.nexa.network-path-status"))
    }

    var isOnline: Bool {
        lock.lock()
        defer { lock.unlock() }
        return online
    }

    private func setOnline(_ value: Bool) {
        lock.lock()
        let changed = online != value
        online = value
        lock.unlock()
        if changed {
            DispatchQueue.main.async { [weak self] in
                self?.notifyStatusChange(value)
            }
        }
    }

    func setStatusHandler(_ handler: @escaping (Bool) -> Void) {
        lock.lock()
        statusHandler = handler
        lock.unlock()
    }

    private func notifyStatusChange(_ value: Bool) {
        lock.lock()
        let handler = statusHandler
        lock.unlock()
        handler?(value)
    }
}

"#,
        );
    }
    if include_network || include_image_support {
        out.push_str(include_str!("bounded_response.swift"));
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
    static let collector = NexaBoundedDataDelegate()
    static let sharedSession = URLSession(configuration: configuration, delegate: collector, delegateQueue: nil)
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
    case invalidUploadField
}

public struct NexaNetworkResponse {
    public let statusCode: Int32
    let headers: [(String, String)]
    public let body: Data

    public var text: String { String(decoding: body, as: UTF8.self) }

    public func getHeader(_ name: String) -> String? {
        headers.first(where: { $0.0.caseInsensitiveCompare(name) == .orderedSame })?.1
    }

    public func getHeaders(_ name: String) -> [String] {
        var values: [String] = []
        for header in headers where header.0.caseInsensitiveCompare(name) == .orderedSame {
            values.append(header.1)
        }
        return values
    }

    public func getCookie() -> String {
        getHeaders("Set-Cookie")
            .compactMap { value -> String? in
                guard let first = value
                    .split(separator: ";", maxSplits: 1, omittingEmptySubsequences: false)
                    .first else { return nil }
                let pair = String(first).trimmingCharacters(in: .whitespaces)
                guard let equals = pair.firstIndex(of: "="), equals != pair.startIndex else {
                    return nil
                }
                let cookieName = String(pair[..<equals]).trimmingCharacters(in: .whitespaces)
                let cookieValue = String(pair[pair.index(after: equals)...])
                    .trimmingCharacters(in: .whitespaces)
                guard !cookieName.isEmpty else { return nil }
                return "\(cookieName)=\(cookieValue)"
            }
            .joined(separator: "; ")
    }
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

private final class NexaURLSessionDelegate: NSObject, URLSessionDataDelegate {
    let collector = NexaBoundedDataDelegate()
    let followRedirects: Bool
    let certificatePins: Set<String>

    init(followRedirects: Bool, certificatePins: Set<String>) {
        self.followRedirects = followRedirects
        self.certificatePins = Set(certificatePins.map { $0.lowercased() })
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive response: URLResponse,
                    completionHandler: @escaping (URLSession.ResponseDisposition) -> Void) {
        collector.urlSession(session, dataTask: dataTask, didReceive: response, completionHandler: completionHandler)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        collector.urlSession(session, dataTask: dataTask, didReceive: data)
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        collector.urlSession(session, task: task, didCompleteWithError: error)
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
    /// Shared URLSession for native networking plugins in this app.
    public static var sharedURLSession: URLSession {
        NexaURLSessionSupport.sharedSession
    }

    private static func urlRequestCachePolicy(_ value: String) -> URLRequest.CachePolicy {
        switch value {
        case "Reload": return .reloadIgnoringLocalCacheData
        case "CacheOnly": return .returnCacheDataDontLoad
        default: return .useProtocolCachePolicy
        }
    }

    private static func makeResponse(
        data: Data,
        response: URLResponse,
        maxResponseBytes: Int
    ) throws -> NexaNetworkResponse {
        guard let response = response as? HTTPURLResponse else {
            throw NexaNetworkError.invalidResponse
        }
        if data.count > maxResponseBytes {
            throw NexaNetworkError.responseTooLarge
        }
        let headers = response.allHeaderFields.flatMap { key, value -> [(String, String)] in
            guard let name = key as? String else { return [] }
            if let values = value as? [String] {
                return values.map { (name, $0) }
            }
            return [(name, String(describing: value))]
        }
        return NexaNetworkResponse(
            statusCode: Int32(response.statusCode),
            headers: headers,
            body: data
        )
    }

    public static func fetch(
        url: String,
        method: String = "GET",
        body: Data? = nil,
        headers: [String: String] = [:],
        timeout: TimeInterval = 30,
        cachePolicy: String = "Default",
        followRedirects: Bool = true,
        maxResponseBytes: Int = 64 * 1024 * 1024,
        certificatePins: Set<String> = []
    ) async throws -> NexaNetworkResponse {
        guard let url = URL(string: url) else { throw NexaNetworkError.invalidURL }
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.httpBody = body
        request.timeoutInterval = timeout
        request.cachePolicy = urlRequestCachePolicy(cachePolicy)
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
        do {
            let collector = delegate?.collector ?? NexaURLSessionSupport.collector
            let (data, response) = try await collector.data(for: request, in: session, limit: maxResponseBytes)
            return try makeResponse(data: data, response: response, maxResponseBytes: maxResponseBytes)
        } catch NexaBoundedResponseError.responseTooLarge {
            throw NexaNetworkError.responseTooLarge
        }
    }

    public static func download(
        url: String,
        destinationPath: String,
        method: String = "GET",
        body: Data? = nil,
        headers: [String: String] = [:],
        timeout: TimeInterval = 30,
        cachePolicy: String = "Default",
        followRedirects: Bool = true,
        maxResponseBytes: Int = 64 * 1024 * 1024,
        certificatePins: Set<String> = []
    ) async throws -> Bool {
        guard let url = URL(string: url) else { throw NexaNetworkError.invalidURL }
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.httpBody = body
        request.timeoutInterval = timeout
        request.cachePolicy = urlRequestCachePolicy(cachePolicy)
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
        let destination = URL(fileURLWithPath: destinationPath)
        try FileManager.default.createDirectory(
            at: destination.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        let temporaryURL = destination.deletingLastPathComponent()
            .appendingPathComponent(".nexa-download-\(UUID().uuidString).tmp")
        defer { try? FileManager.default.removeItem(at: temporaryURL) }
        let collector = delegate?.collector ?? NexaURLSessionSupport.collector
        let response: URLResponse
        do {
            (_, response) = try await collector.download(
                for: request,
                in: session,
                limit: maxResponseBytes,
                destinationURL: temporaryURL
            )
        } catch NexaBoundedResponseError.responseTooLarge {
            throw NexaNetworkError.responseTooLarge
        }
        guard let response = response as? HTTPURLResponse else {
            throw NexaNetworkError.invalidResponse
        }
        guard (200..<300).contains(response.statusCode) else {
            throw NexaNetworkError.httpStatus(response.statusCode)
        }
        if FileManager.default.fileExists(atPath: destination.path) {
            _ = try FileManager.default.replaceItemAt(destination, withItemAt: temporaryURL)
        } else {
            try FileManager.default.moveItem(at: temporaryURL, to: destination)
        }
        return true
    }

    public static func upload(
        url: String,
        file: String,
        fields: [String: String] = [:],
        headers: [String: String] = [:],
        timeout: TimeInterval = 30,
        followRedirects: Bool = true,
        maxResponseBytes: Int = 64 * 1024 * 1024,
        certificatePins: Set<String> = []
    ) async throws -> NexaNetworkResponse {
        guard let url = URL(string: url) else { throw NexaNetworkError.invalidURL }
        let sourceURL = URL(fileURLWithPath: file)
        guard FileManager.default.fileExists(atPath: sourceURL.path) else {
            throw CocoaError(.fileNoSuchFile)
        }

        let boundary = "Nexa-\(UUID().uuidString)"
        let uploadURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("nexa-upload-\(UUID().uuidString).multipart")
        guard FileManager.default.createFile(atPath: uploadURL.path, contents: nil) else {
            throw CocoaError(.fileWriteUnknown)
        }
        defer { try? FileManager.default.removeItem(at: uploadURL) }

        let output = try FileHandle(forWritingTo: uploadURL)
        defer { try? output.close() }
        for (name, value) in fields.sorted(by: { $0.key < $1.key }) {
            try output.write(contentsOf: Data("--\(boundary)\r\n".utf8))
            try output.write(contentsOf: Data(
                "Content-Disposition: form-data; name=\"\(dispositionParameter(name))\"\r\n\r\n".utf8
            ))
            try output.write(contentsOf: Data(value.utf8))
            try output.write(contentsOf: Data("\r\n".utf8))
        }
        try output.write(contentsOf: Data("--\(boundary)\r\n".utf8))
        try output.write(contentsOf: Data(
            "Content-Disposition: form-data; name=\"file\"; filename=\"\(dispositionParameter(sourceURL.lastPathComponent))\"\r\n".utf8
        ))
        try output.write(contentsOf: Data("Content-Type: application/octet-stream\r\n\r\n".utf8))

        let input = try FileHandle(forReadingFrom: sourceURL)
        defer { try? input.close() }
        while let chunk = try input.read(upToCount: 1024 * 1024), !chunk.isEmpty {
            try output.write(contentsOf: chunk)
        }
        try output.write(contentsOf: Data("\r\n--\(boundary)--\r\n".utf8))
        try output.close()

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.timeoutInterval = timeout
        for (name, value) in headers {
            request.setValue(value, forHTTPHeaderField: name)
        }
        request.setValue("multipart/form-data; boundary=\(boundary)", forHTTPHeaderField: "Content-Type")
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
        do {
            let collector = delegate?.collector ?? NexaURLSessionSupport.collector
            let (data, response) = try await collector.data(for: request, in: session,
                limit: maxResponseBytes, uploadFile: uploadURL)
            return try makeResponse(data: data, response: response, maxResponseBytes: maxResponseBytes)
        } catch NexaBoundedResponseError.responseTooLarge {
            throw NexaNetworkError.responseTooLarge
        }
    }

    private static func dispositionParameter(_ value: String) -> String {
        value
            .replacingOccurrences(of: "\\", with: "%5C")
            .replacingOccurrences(of: "\"", with: "%22")
            .replacingOccurrences(of: "\r", with: "%0D")
            .replacingOccurrences(of: "\n", with: "%0A")
    }
}

"#,
        );
    }
    if include_network_connectivity && include_network {
        // The connectivity helper is emitted above and the networking API
        // already owns this enum, so attach the cheap synchronous getter here.
        // Insert before the transport operations by generating an extension.
        out.push_str(
            r#"
public extension NexaNetwork {
    static func isOnline() -> Bool {
        NexaNetworkPathStatus.shared.isOnline
    }

    static func onStatusChange(_ handler: @escaping (Bool) -> Void) {
        NexaNetworkPathStatus.shared.setStatusHandler(handler)
    }
}

"#,
        );
    } else if include_network_connectivity {
        out.push_str(
            r#"
public enum NexaNetwork {
    public static func isOnline() -> Bool {
        NexaNetworkPathStatus.shared.isOnline
    }

    public static func onStatusChange(_ handler: @escaping (Bool) -> Void) {
        NexaNetworkPathStatus.shared.setStatusHandler(handler)
    }
}

"#,
        );
    }
    if include_path {
        out.push_str(
            r#"public enum NexaPath {
    public static func documents() -> String {
        FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].path
    }

    public static func caches() -> String {
        FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0].path
    }

    public static func temporary() -> String {
        NSTemporaryDirectory()
    }

    public static func appSupport() -> String {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].path
    }

    public static func join(_ path: String, _ component: String) -> String {
        URL(fileURLWithPath: path).appendingPathComponent(component).path
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

    public static func write(_ data: Data, to path: String) async throws -> Bool {
        try await Task.detached(priority: .utility) {
            let url = URL(fileURLWithPath: path)
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )
            try data.write(to: url, options: .atomic)
        }.value
        return true
    }

    public static func readText(_ path: String) async throws -> String {
        String(decoding: try await read(path), as: UTF8.self)
    }

    public static func writeText(_ text: String, to path: String) async throws -> Bool {
        try await write(Data(text.utf8), to: path)
    }

    public static func delete(_ path: String) async throws -> Bool {
        try await Task.detached(priority: .utility) {
            let fileManager = FileManager.default
            guard fileManager.fileExists(atPath: path) else { return false }
            do {
                try fileManager.removeItem(atPath: path)
                return true
            } catch {
                guard !fileManager.fileExists(atPath: path) else { throw error }
                return false
            }
        }.value
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
    guard let source = CGImageSourceCreateWithData(
        data as CFData, [kCGImageSourceShouldCache: false] as CFDictionary
    ) else { return nil }
    let options: [CFString: Any] = [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceShouldCacheImmediately: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: maxPixelSize
    ]
    guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else {
        return nil
    }
    return UIImage(cgImage: image)
}

private func nexaDecodeImage(_ data: Data) async throws -> UIImage? {
    try Task.checkCancellation()
    let decodeTask = Task.detached(priority: .utility) {
        try Task.checkCancellation()
        return nexaDownsampleImage(data)
    }
    let decoded = try await withTaskCancellationHandler {
        try await decodeTask.value
    } onCancel: { decodeTask.cancel() }
    try Task.checkCancellation()
    return decoded
}

struct NexaRemoteImage: View {
    let url: String
    let scale: NexaImageScale
    let placeholder: String?
    let showLoadingIndicator: Bool
    let allowsFile: Bool
    @State private var image: Image?
    @State private var failed = false

    var body: some View {
        ZStack {
            Color.clear
                .frame(width: 1, height: 1)
                .accessibilityHidden(true)

            if let image {
                image.resizable().aspectRatio(contentMode: scale == .fit ? .fit : .fill)
            } else if failed {
                if let placeholder {
                    NexaImagePlaceholderPrimitive(name: placeholder, scale: scale)
                } else {
                    EmptyView()
                }
            } else if let placeholder {
                NexaImagePlaceholderPrimitive(name: placeholder, scale: scale)
            } else if showLoadingIndicator {
                ProgressView()
                    .controlSize(.small)
                    .frame(width: 20, height: 20)
            } else {
                EmptyView()
            }
        }
        .task(id: url) {
            image = nil
            failed = false
            do {
                guard let parsedURL = URL(string: url) else {
                    throw NexaRemoteImageError.invalidURL
                }
                guard parsedURL.isFileURL ? allowsFile : parsedURL.scheme?.lowercased() == "https" else {
                    throw NexaRemoteImageError.invalidURL
                }
                let data: Data
                if parsedURL.isFileURL {
                    if let resourceValues = try? parsedURL.resourceValues(forKeys: [.fileSizeKey]),
                       let fileSize = resourceValues.fileSize,
                       fileSize > 64 * 1024 * 1024 {
                        throw NexaRemoteImageError.invalidResponse
                    }
                    let localReadTask = Task.detached(priority: .utility) {
                        let fileHandle = try FileHandle(forReadingFrom: parsedURL)
                        defer { try? fileHandle.close() }

                        var data = Data()
                        while true {
                            try Task.checkCancellation()
                            guard let chunk = try fileHandle.read(upToCount: 256 * 1024),
                                  !chunk.isEmpty else {
                                break
                            }
                            guard chunk.count <= 64 * 1024 * 1024 - data.count else {
                                throw NexaRemoteImageError.invalidResponse
                            }
                            data.append(chunk)
                        }
                        return data
                    }
                    data = try await withTaskCancellationHandler {
                        try await localReadTask.value
                    } onCancel: {
                        localReadTask.cancel()
                    }
                } else {
                    let (remoteData, response) = try await NexaURLSessionSupport.collector.data(
                        for: URLRequest(url: parsedURL), in: NexaURLSessionSupport.sharedSession,
                        limit: 64 * 1024 * 1024)
                    guard let httpResponse = response as? HTTPURLResponse,
                          (200..<300).contains(httpResponse.statusCode) else {
                        throw NexaRemoteImageError.invalidResponse
                    }
                    data = remoteData
                }
                try Task.checkCancellation()
                guard data.count <= 64 * 1024 * 1024 else {
                    throw NexaRemoteImageError.invalidResponse
                }
                let decoded = try await nexaDecodeImage(data)
                try Task.checkCancellation()
                guard let decoded else { throw NexaRemoteImageError.invalidResponse }
                image = Image(uiImage: decoded)
            } catch is CancellationError {
                return
            } catch {
                guard !Task.isCancelled else { return }
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
    fn path_helpers_match_the_single_component_language_api() {
        let mut output = SourceWriter::new();
        render(&mut output, false, false, true, false, false, false);

        assert!(output.contains("public static func documents() -> String"));
        assert!(output.contains("public static func caches() -> String"));
        assert!(output.contains("public static func temporary() -> String"));
        assert!(output.contains("public static func appSupport() -> String"));
        assert!(
            output
                .contains("public static func join(_ path: String, _ component: String) -> String")
        );
        assert!(!output.contains("String..."));
        assert!(!output.contains("private static func append("));
    }

    #[test]
    fn outgoing_requests_accept_string_maps() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("headers: [String: String] = [:]"));
        assert!(output.contains("let headers: [(String, String)]"));
        assert!(!output.contains("NexaHttpHeader"));
        assert!(output.contains("public func getHeaders(_ name: String) -> [String]"));
        assert!(output.contains("public func getCookie() -> String"));
        assert!(output.contains("caseInsensitiveCompare(name) == .orderedSame"));
        assert!(output.contains("getHeaders(\"Set-Cookie\")"));
        assert!(output.contains("firstIndex(of: \"=\")"));
        assert!(output.contains("pair[pair.index(after: equals)...]"));
        assert!(output.contains(".split(separator: \";\", maxSplits: 1"));
        assert!(output.contains(".joined(separator: \"; \")"));
        assert!(output.contains("for (name, value) in headers"));
        assert!(output.contains("request.setValue(value, forHTTPHeaderField: name)"));
    }

    #[test]
    fn network_pins_hash_the_trusted_chain_spki_instead_of_the_leaf_certificate() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("SecTrustEvaluateWithError(trust, nil)"));
        assert!(output.contains("SecTrustCopyCertificateChain(trust)"));
        assert!(output.contains("chain.contains { certificate in"));
        assert!(output.contains("subjectPublicKeyInfoRange(in: bytes)"));
        assert!(output.contains("SHA256.hash(data: Data(bytes[range]))"));
        assert!(output.contains("certificatePins.map { $0.lowercased() }"));
        assert!(!output.contains("SHA256.hash(data: certificateData)"));
    }

    #[test]
    fn network_status_uses_a_shared_thread_safe_path_monitor() {
        let mut output = SourceWriter::new();
        render(&mut output, false, false, false, false, false, true);

        assert!(output.contains("private final class NexaNetworkPathStatus: @unchecked Sendable"));
        assert!(output.contains("monitor.pathUpdateHandler"));
        assert!(output.contains("monitor.start(queue:"));
        assert!(output.contains("lock.lock()"));
        assert!(output.contains("public static func isOnline() -> Bool"));
        assert!(output.contains("public static func onStatusChange("));
        assert!(!output.contains("public static func fetch("));
    }

    #[test]
    fn native_plugins_can_reuse_the_generated_urlsession() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("public static var sharedURLSession: URLSession"));
        assert!(output.contains("NexaURLSessionSupport.sharedSession"));

        let mut without_network = SourceWriter::new();
        render(
            &mut without_network,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert!(!without_network.contains("sharedURLSession"));
    }

    #[test]
    fn multipart_upload_uses_urlsession_file_upload_and_cleans_up_temporary_body() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);

        assert!(output.contains("private static func makeResponse("));
        assert_eq!(
            output.as_str().matches("return try makeResponse(").count(),
            2
        );
        assert_eq!(
            output
                .as_str()
                .matches("response.allHeaderFields.flatMap")
                .count(),
            1
        );
        assert!(output.contains("public static func upload("));
        assert!(output.contains("async throws -> NexaNetworkResponse"));
        assert!(output.contains("uploadFile: uploadURL"));
        assert!(output.contains("session.uploadTask(with: request, fromFile: $0)"));
        assert!(output.contains("defer { try? FileManager.default.removeItem(at: uploadURL) }"));
        assert!(output.contains("timeout: TimeInterval = 30"));
        assert!(output.contains("followRedirects: Bool = true"));
        assert!(output.contains("NexaNetworkError.responseTooLarge"));
        assert!(output.contains("certificatePins: Set<String> = []"));
        assert!(output.contains("request.setValue(\"multipart/form-data; boundary=\\(boundary)\""));
    }

    #[test]
    fn downloads_validate_before_replacing_an_existing_file() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);
        let download = output
            .as_str()
            .split_once("public static func download(")
            .expect("find generated download helper")
            .1
            .split_once("public static func upload(")
            .expect("find end of generated download helper")
            .0;

        assert!(download.contains("try await collector.download("));
        assert!(download.contains("limit: maxResponseBytes"));
        assert!(download.contains("destinationURL: temporaryURL"));
        assert!(!download.contains("session.download(for: request)"));
        assert!(download.contains("replaceItemAt(destination, withItemAt: temporaryURL)"));
        assert!(download.contains("moveItem(at: temporaryURL, to: destination)"));
        assert!(!download.contains("removeItem(at: destination)"));
        assert!(
            download
                .find("guard (200..<300)")
                .expect("validate HTTP status")
                < download
                    .find("replaceItemAt(destination")
                    .expect("replace after validation"),
            "failed HTTP responses must not replace the destination"
        );
    }

    #[test]
    fn remote_image_loading_cancels_local_reads_and_ignores_stale_results() {
        let mut output = SourceWriter::new();
        render(&mut output, false, true, false, false, false, false);

        assert!(output.contains(".task(id: url) {"));
        assert!(output.contains("let localReadTask = Task.detached(priority: .utility)"));
        assert!(output.contains("fileHandle.read(upToCount: 256 * 1024)"));
        assert!(output.contains("withTaskCancellationHandler {"));
        assert!(output.contains("localReadTask.cancel()"));
        assert!(output.contains("try Task.checkCancellation()"));
        assert!(output.contains("} catch is CancellationError {"));
        assert!(output.contains("guard !Task.isCancelled else { return }"));
    }

    #[test]
    fn file_writes_are_atomic_and_delete_missing_paths_returns_false() {
        let mut output = SourceWriter::new();
        render(&mut output, false, false, false, true, true, false);

        assert!(output.contains("try data.write(to: url, options: .atomic)"));
        assert!(
            output.contains("guard fileManager.fileExists(atPath: path) else { return false }")
        );
        assert!(output.contains("try fileManager.removeItem(atPath: path)"));
        assert!(
            output.contains("guard !fileManager.fileExists(atPath: path) else { throw error }")
        );
    }

    #[test]
    fn fetch_and_upload_return_http_error_responses_for_status_inspection() {
        let mut output = SourceWriter::new();
        render(&mut output, true, false, false, false, false, false);
        let source = output.as_str();
        let response_builder = source
            .split_once("private static func makeResponse(")
            .expect("find shared response builder")
            .1
            .split_once("public static func fetch(")
            .expect("find end of response builder")
            .0;
        assert!(!response_builder.contains("httpStatus"));
        assert!(response_builder.contains("Int32(response.statusCode)"));

        let fetch = source
            .split_once("public static func fetch(")
            .expect("find fetch helper")
            .1
            .split_once("public static func download(")
            .expect("find end of fetch helper")
            .0;
        assert!(!fetch.contains("200..<300"));

        let upload = source
            .split_once("public static func upload(")
            .expect("find upload helper")
            .1;
        assert!(!upload.contains("200..<300"));

        let download = source
            .split_once("public static func download(")
            .expect("find download helper")
            .1
            .split_once("public static func upload(")
            .expect("find end of download helper")
            .0;
        assert!(download.contains("guard (200..<300).contains(response.statusCode)"));
    }
}
