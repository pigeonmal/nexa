// A shared URLSession delegate accumulates bounded bodies in native chunks.
// Locks protect cancellation versus delegate delivery; no continuation is
// resumed while a lock is held, and every terminal path removes its entry.
private enum NexaBoundedResponseError: Error { case responseTooLarge }

// Download chunks are written directly to a unique staging file. The lock
// serializes writes against cancellation/finish and tracks the exact byte cap.
private final class NexaBoundedDownloadFile: @unchecked Sendable {
    private struct State: @unchecked Sendable {
        var handle: FileHandle?
        var bytesWritten = 0
    }

    let url: URL
    private let state: OSAllocatedUnfairLock<State>

    init(url: URL) throws {
        guard FileManager.default.createFile(atPath: url.path, contents: nil) else {
            throw CocoaError(.fileWriteUnknown)
        }
        do {
            let handle = try FileHandle(forWritingTo: url)
            self.url = url
            state = OSAllocatedUnfairLock(initialState: State(handle: handle))
        } catch {
            try? FileManager.default.removeItem(at: url)
            throw error
        }
    }

    func append(_ data: Data, limit: Int) throws -> Bool {
        try state.withLock { value in
            guard data.count <= limit - value.bytesWritten else { return false }
            guard let handle = value.handle else { throw CancellationError() }
            try handle.write(contentsOf: data)
            value.bytesWritten += data.count
            return true
        }
    }

    func close() throws {
        try state.withLock { value in
            guard let handle = value.handle else { return }
            value.handle = nil
            try handle.close()
        }
    }

    func discard() {
        try? close()
        try? FileManager.default.removeItem(at: url)
    }

    var writtenByteCount: Int { state.withLock { $0.bytesWritten } }
}

private final class NexaBoundedDataDelegate: NSObject, URLSessionDataDelegate {
    private struct Pending: Sendable {
        let continuation: CheckedContinuation<(Data, URLResponse), Error>
        let limit: Int
        var data = Data()
        var response: URLResponse?
    }
    private struct Control: Sendable {
        var task: URLSessionDataTask?
        var cancelled = false
    }
    private struct PendingDownload: Sendable {
        let continuation: CheckedContinuation<(URL, URLResponse), Error>
        let limit: Int
        let file: NexaBoundedDownloadFile
        var response: URLResponse?
    }
    private let pending = OSAllocatedUnfairLock(initialState: [Int: Pending]())
    private let downloads = OSAllocatedUnfairLock(initialState: [Int: PendingDownload]())

    func download(for request: URLRequest, in session: URLSession, limit: Int,
                  destinationURL: URL) async throws -> (URL, URLResponse) {
        guard limit >= 0 else { throw NexaBoundedResponseError.responseTooLarge }
        let file = try NexaBoundedDownloadFile(url: destinationURL)
        let control = OSAllocatedUnfairLock(initialState: Control())
        do {
            return try await withTaskCancellationHandler {
                try Task.checkCancellation()
                return try await withCheckedThrowingContinuation { continuation in
                    let task = session.dataTask(with: request)
                    downloads.withLock {
                        $0[task.taskIdentifier] = PendingDownload(
                            continuation: continuation, limit: limit, file: file
                        )
                    }
                    let shouldStart = control.withLock { value in
                        guard !value.cancelled else { return false }
                        value.task = task
                        return true
                    }
                    if shouldStart { task.resume() }
                    else { cancel(task) }
                }
            } onCancel: {
                let task = control.withLock { value in
                    value.cancelled = true
                    return value.task
                }
                if let task { self.cancel(task) }
            }
        } catch {
            file.discard()
            throw error
        }
    }

    func data(for request: URLRequest, in session: URLSession, limit: Int,
              uploadFile: URL? = nil) async throws -> (Data, URLResponse) {
        guard limit >= 0 else { throw NexaBoundedResponseError.responseTooLarge }
        let control = OSAllocatedUnfairLock(initialState: Control())
        return try await withTaskCancellationHandler {
            try Task.checkCancellation()
            return try await withCheckedThrowingContinuation { continuation in
                let task = uploadFile.map { session.uploadTask(with: request, fromFile: $0) }
                    ?? session.dataTask(with: request)
                pending.withLock { $0[task.taskIdentifier] = Pending(continuation: continuation, limit: limit) }
                let shouldStart = control.withLock { value in
                    guard !value.cancelled else { return false }
                    value.task = task
                    return true
                }
                if shouldStart { task.resume() }
                else { cancel(task) }
            }
        } onCancel: {
            let task = control.withLock { value in
                value.cancelled = true
                return value.task
            }
            if let task { self.cancel(task) }
        }
    }

    private func cancel(_ task: URLSessionTask) {
        let download = downloads.withLock { $0.removeValue(forKey: task.taskIdentifier) }
        if let download {
            task.cancel()
            download.file.discard()
            download.continuation.resume(throwing: CancellationError())
            return
        }
        let request = pending.withLock { $0.removeValue(forKey: task.taskIdentifier) }
        task.cancel()
        request?.continuation.resume(throwing: CancellationError())
    }

    private func failDownload(_ task: URLSessionTask, error: Error) {
        let download = downloads.withLock { $0.removeValue(forKey: task.taskIdentifier) }
        guard let download else { return }
        task.cancel()
        download.file.discard()
        download.continuation.resume(throwing: error)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask,
                    didReceive response: URLResponse,
                    completionHandler: @escaping (URLSession.ResponseDisposition) -> Void) {
        let downloadResult = downloads.withLock { entries -> (Bool, Bool) in
            guard let request = entries[dataTask.taskIdentifier] else { return (false, false) }
            guard response.expectedContentLength <= Int64(request.limit) else { return (true, true) }
            entries[dataTask.taskIdentifier]?.response = response
            return (true, false)
        }
        if downloadResult.0 {
            completionHandler(downloadResult.1 ? .cancel : .allow)
            if downloadResult.1 {
                failDownload(dataTask, error: NexaBoundedResponseError.responseTooLarge)
            }
            return
        }
        let rejected = pending.withLock { entries -> Pending? in
            guard let limit = entries[dataTask.taskIdentifier]?.limit else { return nil }
            if response.expectedContentLength > Int64(limit) {
                return entries.removeValue(forKey: dataTask.taskIdentifier)
            }
            entries[dataTask.taskIdentifier]?.response = response
            return nil
        }
        completionHandler(rejected == nil ? .allow : .cancel)
        rejected?.continuation.resume(throwing: NexaBoundedResponseError.responseTooLarge)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        if let download = downloads.withLock({ $0[dataTask.taskIdentifier] }) {
            do {
                if try !download.file.append(data, limit: download.limit) {
                    failDownload(dataTask, error: NexaBoundedResponseError.responseTooLarge)
                }
            } catch {
                failDownload(dataTask, error: error)
            }
            return
        }
        let rejected = pending.withLock { entries -> Pending? in
            // Read scalar fields without holding a copy of Pending.data across
            // append. That keeps Data uniquely owned instead of copying the
            // complete accumulated body on every chunk.
            guard let limit = entries[dataTask.taskIdentifier]?.limit,
                  let count = entries[dataTask.taskIdentifier]?.data.count else { return nil }
            guard data.count <= limit - count else {
                return entries.removeValue(forKey: dataTask.taskIdentifier)
            }
            entries[dataTask.taskIdentifier]?.data.append(data)
            return nil
        }
        if let rejected {
            dataTask.cancel()
            rejected.continuation.resume(throwing: NexaBoundedResponseError.responseTooLarge)
        }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        if let download = downloads.withLock({ $0.removeValue(forKey: task.taskIdentifier) }) {
            if let error {
                download.file.discard()
                download.continuation.resume(throwing: error)
            } else if let response = download.response {
                do {
                    try download.file.close()
                    download.continuation.resume(returning: (download.file.url, response))
                } catch {
                    download.file.discard()
                    download.continuation.resume(throwing: error)
                }
            } else {
                download.file.discard()
                download.continuation.resume(throwing: URLError(.badServerResponse))
            }
            return
        }
        guard let request = pending.withLock({ $0.removeValue(forKey: task.taskIdentifier) }) else { return }
        if let error { request.continuation.resume(throwing: error) }
        else if let response = request.response {
            request.continuation.resume(returning: (request.data, response))
        } else { request.continuation.resume(throwing: URLError(.badServerResponse)) }
    }
}
