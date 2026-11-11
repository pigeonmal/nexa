// Native transport regression. All responses are private URLProtocol streams;
// no server, external network request, device, or screenshot is involved.
private final class ProbeProtocol: URLProtocol, @unchecked Sendable {
    static let sent = OSAllocatedUnfairLock(initialState: [String: Int]())
    private let stopped = OSAllocatedUnfairLock(initialState: false)
    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
    override func stopLoading() { stopped.withLock { $0 = true } }
    override func startLoading() {
        DispatchQueue.global(qos: .utility).async { [weak self] in self?.deliver() }
    }
    private func deliver() {
        guard let url = request.url else { return }
        if url.path == "/error" {
            client?.urlProtocol(self, didFailWithError: URLError(.cannotConnectToHost)); return
        }
        let count = url.path == "/small" ? 2048 : url.path == "/empty" ? 0 : 65536
        let headers = url.path == "/declared" ? ["Content-Length": String(count)] : [:]
        guard let response = HTTPURLResponse(url: url, statusCode: url.path == "/status" ? 404 : 200,
                                            httpVersion: "HTTP/1.1", headerFields: headers) else { return }
        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        for offset in stride(from: 0, to: count, by: 1024) {
            if stopped.withLock({ $0 }) { return }
            let chunk = Data(repeating: 42, count: min(1024, count - offset))
            Self.sent.withLock { $0[url.absoluteString, default: 0] += chunk.count }
            client?.urlProtocol(self, didLoad: chunk)
            Thread.sleep(forTimeInterval: 0.002)
        }
        if !stopped.withLock({ $0 }) { client?.urlProtocolDidFinishLoading(self) }
    }
}

extension NexaBoundedDataDelegate {
    var testPendingCount: Int { pending.withLock { $0.count } }
    var testBufferedBytes: Int { pending.withLock { $0.values.reduce(0) { $0 + $1.data.count } } }
    var testPendingDownloadCount: Int { downloads.withLock { $0.count } }
    var testDownloadBytesWritten: Int {
        downloads.withLock { $0.values.reduce(0) { $0 + $1.file.writtenByteCount } }
    }
}

private func verify(_ condition: @autoclosure () -> Bool, _ message: String) throws {
    if !condition() { throw NSError(domain: "NexaNativeBoundedResponse", code: 1,
                                    userInfo: [NSLocalizedDescriptionKey: message]) }
}

@main struct Check {
    static func main() async throws {
        let collector = NexaBoundedDataDelegate()
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [ProbeProtocol.self]
        configuration.urlCache = nil
        let session = URLSession(configuration: configuration, delegate: collector, delegateQueue: nil)
        defer { session.invalidateAndCancel() }
        func request(_ path: String) -> URLRequest {
            URLRequest(url: URL(string: "https://nexa-native.invalid" + path + "?id=" + UUID().uuidString)!, timeoutInterval: 5)
        }
        let reference = request("/unknown")
        let (oldData, _) = try await session.data(for: reference)
        try verify(oldData.count == 65536, "Late-check reference must consume the entire response")
        var results: [[String: Any]] = []
        for path in ["/declared", "/unknown"] {
            let input = request(path)
            let finished = OSAllocatedUnfairLock(initialState: false)
            let operation = Task {
                defer { finished.withLock { $0 = true } }
                return try await collector.data(for: input, in: session, limit: 4096)
            }
            var peakBuffered = 0
            while !finished.withLock({ $0 }) {
                peakBuffered = max(peakBuffered, collector.testBufferedBytes)
                try await Task.sleep(for: .milliseconds(1))
            }
            do {
                _ = try await operation.value
                throw NSError(domain: "Expected size rejection", code: 1)
            } catch NexaBoundedResponseError.responseTooLarge {}
            let received = ProbeProtocol.sent.withLock { $0[input.url!.absoluteString] ?? 0 }
            try verify(received < 65536, "Oversized transfer should cancel before completion")
            try verify(peakBuffered <= 4096, "Application buffer must not exceed its limit")
            try verify(collector.testPendingCount == 0, "Rejected request must release its continuation and body")
            results.append(["path": path, "emittedBytes": received, "peakObservedBufferedBytes": peakBuffered])
        }
        let (exact, _) = try await collector.data(for: request("/small"), in: session, limit: 2048)
        try verify(exact == Data(repeating: 42, count: 2048), "Exact-limit body should be preserved")
        let (empty, _) = try await collector.data(for: request("/empty"), in: session, limit: 0)
        try verify(empty.isEmpty, "Zero-byte budget should allow empty responses")
        var downloadResults: [[String: Any]] = []
        for path in ["/declared", "/unknown"] {
            let input = request(path)
            let destination = FileManager.default.temporaryDirectory
                .appendingPathComponent("nexa-bounded-download-\(UUID().uuidString).tmp")
            let finished = OSAllocatedUnfairLock(initialState: false)
            let operation = Task {
                defer { finished.withLock { $0 = true } }
                return try await collector.download(
                    for: input, in: session, limit: 4096, destinationURL: destination
                )
            }
            var peakWritten = 0
            while !finished.withLock({ $0 }) {
                peakWritten = max(peakWritten, collector.testDownloadBytesWritten)
                try await Task.sleep(for: .milliseconds(1))
            }
            do {
                _ = try await operation.value
                throw NSError(domain: "Expected bounded download rejection", code: 1)
            } catch NexaBoundedResponseError.responseTooLarge {}
            let received = ProbeProtocol.sent.withLock { $0[input.url!.absoluteString] ?? 0 }
            try verify(received < 65536, "Oversized download should stop before response completion")
            try verify(peakWritten <= 4096, "Staged download file must never exceed its byte limit")
            try verify(!FileManager.default.fileExists(atPath: destination.path), "Rejected download must remove its staging file")
            try verify(collector.testPendingDownloadCount == 0, "Rejected download must release pending state")
            downloadResults.append(["path": path, "emittedBytes": received, "peakWrittenBytes": peakWritten])
        }
        let exactDestination = FileManager.default.temporaryDirectory
            .appendingPathComponent("nexa-bounded-download-exact-\(UUID().uuidString).tmp")
        let (downloadedFile, _) = try await collector.download(
            for: request("/small"), in: session, limit: 2048, destinationURL: exactDestination
        )
        let downloadedBytes = try Data(contentsOf: downloadedFile)
        try verify(downloadedBytes == Data(repeating: 42, count: 2048),
                   "Exact-limit download should preserve the complete body")
        try FileManager.default.removeItem(at: downloadedFile)
        let emptyDestination = FileManager.default.temporaryDirectory
            .appendingPathComponent("nexa-bounded-download-empty-\(UUID().uuidString).tmp")
        let (emptyFile, _) = try await collector.download(
            for: request("/empty"), in: session, limit: 0, destinationURL: emptyDestination
        )
        let emptyBytes = try Data(contentsOf: emptyFile)
        try verify(emptyBytes.isEmpty, "Zero-byte download budget should allow an empty file")
        try FileManager.default.removeItem(at: emptyFile)
        let (_, status) = try await collector.data(for: request("/status"), in: session, limit: 65536)
        try verify((status as? HTTPURLResponse)?.statusCode == 404, "Network.fetch must retain HTTP status semantics")
        do { _ = try await collector.data(for: request("/error"), in: session, limit: 4096)
            throw NSError(domain: "Expected transport failure", code: 1)
        } catch let error as URLError { try verify(error.code == .cannotConnectToHost, "Transport error should propagate") }
        let inFlightRequest = request("/in-flight-cancel")
        let inFlight = Task {
            try await collector.data(for: inFlightRequest, in: session, limit: 65536)
        }
        while ProbeProtocol.sent.withLock({ $0[inFlightRequest.url!.absoluteString] ?? 0 }) < 1024 {
            try await Task.sleep(for: .milliseconds(1))
        }
        inFlight.cancel()
        do { _ = try await inFlight.value; throw NSError(domain: "Expected in-flight cancellation", code: 1) }
        catch is CancellationError {}
        let canceledBytes = ProbeProtocol.sent.withLock { $0[inFlightRequest.url!.absoluteString] ?? 0 }
        try verify(canceledBytes < 65536, "In-flight cancellation must stop response production")
        try verify(collector.testPendingCount == 0, "In-flight cancellation must release the body accumulator")
        let canceledDownloadRequest = request("/download-in-flight-cancel")
        let canceledDownloadURL = FileManager.default.temporaryDirectory
            .appendingPathComponent("nexa-bounded-download-cancel-\(UUID().uuidString).tmp")
        let canceledDownload = Task {
            try await collector.download(
                for: canceledDownloadRequest, in: session, limit: 65536,
                destinationURL: canceledDownloadURL
            )
        }
        while ProbeProtocol.sent.withLock({ $0[canceledDownloadRequest.url!.absoluteString] ?? 0 }) < 1024 {
            try await Task.sleep(for: .milliseconds(1))
        }
        canceledDownload.cancel()
        do { _ = try await canceledDownload.value; throw NSError(domain: "Expected download cancellation", code: 1) }
        catch is CancellationError {}
        try verify(!FileManager.default.fileExists(atPath: canceledDownloadURL.path), "Canceled download must remove staged bytes")
        try verify(collector.testPendingDownloadCount == 0, "Canceled download must release pending state")
        let uploadFile = FileManager.default.temporaryDirectory
            .appendingPathComponent("nexa-bounded-upload-\(UUID().uuidString)")
        try Data(repeating: 7, count: 1024).write(to: uploadFile)
        defer { try? FileManager.default.removeItem(at: uploadFile) }
        let uploadRequest = request("/upload")
        do {
            _ = try await collector.data(for: uploadRequest, in: session, limit: 4096, uploadFile: uploadFile)
            throw NSError(domain: "Expected upload response size rejection", code: 1)
        } catch NexaBoundedResponseError.responseTooLarge {}
        try verify(collector.testPendingCount == 0, "Oversized upload response must release its accumulator")
        for _ in 0..<50 {
            let operation = Task { try await collector.data(for: request("/unknown"), in: session, limit: 65536) }
            operation.cancel()
            do { _ = try await operation.value; throw NSError(domain: "Expected cancellation", code: 1) }
            catch is CancellationError {}
        }
        let concurrent = try await withThrowingTaskGroup(of: Int.self) { group in
            for _ in 0..<32 {
                group.addTask { try await collector.data(for: request("/small"), in: session, limit: 2048).0.count }
            }
            var total = 0
            for try await count in group { total += count }
            return total
        }
        try verify(concurrent == 32 * 2048, "Concurrent bodies must remain isolated")
        try verify(collector.testPendingCount == 0, "Completed and canceled requests must leave no pending entries")
        let output: [String: Any] = ["referenceConsumedBytes": oldData.count, "limit": 4096,
            "oversize": results, "cancellationCases": 50, "inFlightCancellationBytes": canceledBytes,
            "boundedDownloads": downloadResults, "exactLimitDownload": true,
            "canceledDownload": true,
            "boundedUploadResponse": true,
            "concurrentRequests": 32,
            "pendingAfterCompletion": collector.testPendingCount,
            "pendingDownloadsAfterCompletion": collector.testPendingDownloadCount]
        print(String(data: try JSONSerialization.data(withJSONObject: output, options: [.sortedKeys]), encoding: .utf8)!)
    }
}
