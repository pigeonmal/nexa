
private enum NexaRemoteImageError: Error {
    case invalidURL
    case invalidResponse
}

private let nexaImageByteLimit = 64 * 1024 * 1024
private let nexaImagePixelLimit = 2048

private func nexaImageDecodeHeight(_ fitHeight: CGFloat?, displayScale: CGFloat) -> Int {
    guard let fitHeight, fitHeight > 0, displayScale > 0 else { return nexaImagePixelLimit }
    let pixels = (fitHeight * displayScale).rounded(.up)
    return Int(min(max(1, pixels), CGFloat(nexaImagePixelLimit)))
}

private func nexaThumbnailLongestEdge(_ source: CGImageSource, height: Int) -> Int {
    guard height < nexaImagePixelLimit,
          let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
          var width = properties[kCGImagePropertyPixelWidth] as? Double,
          var imageHeight = properties[kCGImagePropertyPixelHeight] as? Double,
          width.isFinite, imageHeight.isFinite, width > 0, imageHeight > 0
    else { return nexaImagePixelLimit }

    switch CGImagePropertyOrientation(rawValue: properties[kCGImagePropertyOrientation] as? UInt32 ?? 1) {
    case .left, .right, .leftMirrored, .rightMirrored:
        swap(&width, &imageHeight)
    default:
        break
    }
    let edge = (Double(height) * max(1, width / imageHeight)).rounded(.up)
    return Int(min(edge, Double(nexaImagePixelLimit)))
}

private func nexaDownsampleImage(_ data: Data, height: Int) throws -> UIImage {
    guard let source = CGImageSourceCreateWithData(
        data as CFData, [kCGImageSourceShouldCache: false] as CFDictionary
    ) else { throw NexaRemoteImageError.invalidResponse }
    let options: [CFString: Any] = [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceShouldCacheImmediately: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: nexaThumbnailLongestEdge(source, height: height)
    ]
    guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else {
        throw NexaRemoteImageError.invalidResponse
    }
    return UIImage(cgImage: image)
}

private func nexaReadImageFile(_ url: URL) throws -> Data {
    if let fileSize = try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize,
       fileSize > nexaImageByteLimit {
        throw NexaRemoteImageError.invalidResponse
    }
    let file = try FileHandle(forReadingFrom: url)
    defer { try? file.close() }
    var data = Data()
    while true {
        try Task.checkCancellation()
        guard let chunk = try file.read(upToCount: 256 * 1024), !chunk.isEmpty else { return data }
        guard chunk.count <= nexaImageByteLimit - data.count else {
            throw NexaRemoteImageError.invalidResponse
        }
        data.append(chunk)
    }
}

private func nexaReadImage(_ url: URL) async throws -> Data {
    try Task.checkCancellation()
    if url.isFileURL {
        return try nexaReadImageFile(url)
    }
    let (body, response) = try await NexaURLSessionSupport.collector.data(
        for: URLRequest(url: url), in: NexaURLSessionSupport.sharedSession, limit: nexaImageByteLimit
    )
    guard let response = response as? HTTPURLResponse, (200..<300).contains(response.statusCode) else {
        throw NexaRemoteImageError.invalidResponse
    }
    return body
}

private enum NexaLoadedImage: @unchecked Sendable {
    case staticImage(UIImage)
    case animated(Data, Int)
}

@MainActor
private final class NexaImageLoader {
    static let shared = NexaImageLoader()

    private struct Waiter {
        let height: Int
        let continuation: CheckedContinuation<NexaLoadedImage, any Error>
    }

    private struct Load {
        let id: UInt64
        var task: Task<Void, Never>?
        var waiters: [UInt64: Waiter]

        var largestHeight: Int? { waiters.values.lazy.map(\.height).max() }
    }

    private let read: @Sendable (URL) async throws -> Data
    private let decode: @Sendable (Data, Int) async throws -> UIImage
    private let localLoadLimit: Int
    private let cache: NSCache<NSString, UIImage> = {
        let cache = NSCache<NSString, UIImage>()
        cache.totalCostLimit = 64 * 1024 * 1024
        return cache
    }()
    private var inFlight: [URL: Load] = [:]
    private var activeFileLoads = 0
    // Two batches keep FIFO order without shifting every waiting URL on dequeue.
    private var pendingFiles: [URL] = []
    private var readyFiles: [URL] = []
    private var nextID: UInt64 = 0

    init(
        localLoadLimit: Int = min(6, ProcessInfo.processInfo.activeProcessorCount),
        read: @escaping @Sendable (URL) async throws -> Data = nexaReadImage,
        decode: @escaping @Sendable (Data, Int) async throws -> UIImage = nexaDownsampleImage
    ) {
        self.localLoadLimit = max(1, localLoadLimit)
        self.read = read
        self.decode = decode
    }

    private func cacheKey(url: URL, height: Int) -> NSString {
        "\(url.absoluteString)#\(height)" as NSString
    }

    func image(for url: URL, height: Int = nexaImagePixelLimit) async throws -> NexaLoadedImage {
        if let cached = cache.object(forKey: cacheKey(url: url, height: height)) {
            return .staticImage(cached)
        }
        nextID &+= 1
        let id = nextID
        let loaded = try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<NexaLoadedImage, any Error>) in
                guard !Task.isCancelled else {
                    continuation.resume(throwing: CancellationError())
                    return
                }
                let waiter = Waiter(height: height, continuation: continuation)
                if inFlight[url] != nil {
                    inFlight[url]?.waiters[id] = waiter
                    return
                }
                inFlight[url] = Load(id: id, task: nil, waiters: [id: waiter])
                if url.isFileURL && activeFileLoads >= localLoadLimit {
                    pendingFiles.append(url)
                } else {
                    start(url, loadID: id)
                }
            }
        } onCancel: {
            Task { @MainActor in
                self.cancel(url, waiter: id)
            }
        }
        try Task.checkCancellation()
        return loaded
    }

    private func start(_ url: URL, loadID id: UInt64) {
        let isFile = url.isFileURL
        if isFile { activeFileLoads += 1 }
        inFlight[url]?.task = Task.detached(priority: .utility) { [weak self, read, decode] in
            do {
                try Task.checkCancellation()
                let data = try await read(url)
                if let source = CGImageSourceCreateWithData(
                    data as CFData, [kCGImageSourceShouldCache: false] as CFDictionary
                ), CGImageSourceGetCount(source) > 1 {
                    let initialHeight = await self?.largestHeight(for: url, loadID: id) ?? nexaImagePixelLimit
                    _ = await self?.resolve(
                        url, loadID: id, through: nexaImagePixelLimit, result: .success(.animated(data, initialHeight))
                    )
                } else {
                    var nextHeight = await self?.largestHeight(for: url, loadID: id)
                    while let height = nextHeight {
                        try Task.checkCancellation()
                        let decoded = try await decode(data, height)
                        await self?.storeCached(decoded, url: url, height: height)
                        nextHeight = await self?.resolve(
                            url, loadID: id, through: height, result: .success(.staticImage(decoded))
                        )
                    }
                }
            } catch {
                _ = await self?.resolve(url, loadID: id, through: nexaImagePixelLimit, result: .failure(error))
            }
            // A canceled decode can still be in ImageIO. Release its slot after it exits.
            if isFile { await self?.finishFileLoad() }
        }
    }

    private func storeCached(_ image: UIImage, url: URL, height: Int) {
        let cost = image.cgImage.map { $0.bytesPerRow * $0.height } ?? 0
        cache.setObject(image, forKey: cacheKey(url: url, height: height), cost: cost)
    }

    private func finishFileLoad() {
        activeFileLoads -= 1
        if readyFiles.isEmpty {
            readyFiles = pendingFiles.reversed()
            pendingFiles.removeAll(keepingCapacity: true)
        }
        if let next = readyFiles.popLast(), let id = inFlight[next]?.id {
            start(next, loadID: id)
        }
    }

    private func cancel(_ url: URL, waiter id: UInt64) {
        guard let waiter = inFlight[url]?.waiters.removeValue(forKey: id) else { return }
        if inFlight[url]?.waiters.isEmpty == true {
            let load = inFlight.removeValue(forKey: url)
            if let task = load?.task {
                task.cancel()
            } else {
                pendingFiles.removeAll { $0 == url }
                readyFiles.removeAll { $0 == url }
            }
        }
        waiter.continuation.resume(throwing: CancellationError())
    }

    private func largestHeight(for url: URL, loadID: UInt64) -> Int? {
        guard let load = inFlight[url], load.id == loadID else { return nil }
        return load.largestHeight
    }

    private func resolve(
        _ url: URL, loadID: UInt64, through height: Int, result: Result<NexaLoadedImage, any Error>
    ) -> Int? {
        // A canceled task may finish after a replacement request has started.
        guard let load = inFlight[url], load.id == loadID, let largest = load.largestHeight else { return nil }
        if largest <= height {
            inFlight.removeValue(forKey: url)
            for waiter in load.waiters.values {
                waiter.continuation.resume(with: result)
            }
            return nil
        }
        // A larger request joined during decoding. Keep the same encoded body for it.
        for (id, waiter) in load.waiters where waiter.height <= height {
            inFlight[url]?.waiters.removeValue(forKey: id)
            waiter.continuation.resume(with: result)
        }
        return largest
    }
}

struct NexaRemoteImage: View {
    private struct Request: Equatable {
        let url: String
        let allowsFile: Bool
        let height: Int

        func validatedURL() throws -> URL {
            guard let parsed = URL(string: url),
                  parsed.isFileURL ? allowsFile : parsed.scheme?.lowercased() == "https"
            else { throw NexaRemoteImageError.invalidURL }
            return parsed
        }
    }

    let url: String
    let scale: NexaImageScale
    let placeholder: String?
    let showLoadingIndicator: Bool
    let autoReplay: Bool
    let allowsFile: Bool
    let fitHeight: CGFloat?
    @Environment(\.displayScale) private var displayScale
    @State private var image: Image?
    @State private var animatedData: Data?
    @State private var failed = false

    var body: some View {
        let request = Request(url: url, allowsFile: allowsFile, height: nexaImageDecodeHeight(fitHeight, displayScale: displayScale))
        ZStack {
            Color.clear
                .frame(width: 0, height: 0)
                .accessibilityHidden(true)

            if let animatedData {
                NexaAnimatedGifView(
                    data: animatedData,
                    scale: scale,
                    autoReplay: autoReplay,
                    maxPixelHeight: request.height
                )
            } else if let image {
                image.resizable().aspectRatio(contentMode: scale == .fit ? .fit : .fill)
            } else if let placeholder {
                NexaImagePlaceholderPrimitive(name: placeholder, scale: scale, autoReplay: autoReplay)
            } else if !failed && showLoadingIndicator {
                ProgressView()
                    .controlSize(.small)
                    .frame(width: 20, height: 20)
            }
        }
        .task(id: request) {
            image = nil
            animatedData = nil
            failed = false
            do {
                let loaded = try await NexaImageLoader.shared.image(for: request.validatedURL(), height: request.height)
                try Task.checkCancellation()
                switch loaded {
                case .staticImage(let uiImage):
                    image = Image(uiImage: uiImage)
                case .animated(let data, _):
                    animatedData = data
                }
            } catch is CancellationError {
                return
            } catch {
                guard !Task.isCancelled else { return }
                failed = true
            }
        }
    }
}
