// Generated image decoder checks. Test images and reference pixels stay in
// private memory; the simulator screen is never captured and no PNG is saved.
private func encodedImage(width: Int, height: Int, png: Bool, orientation: Int) throws -> Data {
    guard let space = CGColorSpace(name: CGColorSpace.sRGB),
          let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
                                  bytesPerRow: width * 4, space: space,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
        throw NSError(domain: "Image context", code: 1)
    }
    context.setFillColor(CGColor(red: 0.2, green: 0.5, blue: 0.8, alpha: png ? 0.5 : 1))
    context.fill(CGRect(x: 0, y: 0, width: width, height: height))
    context.setFillColor(CGColor(red: 0.8, green: 0.3, blue: 0.1, alpha: 1))
    context.fill(CGRect(x: 0, y: 0, width: width / 2, height: height / 3))
    let data = NSMutableData()
    guard let image = context.makeImage(),
          let destination = CGImageDestinationCreateWithData(data, (png ? "public.png" : "public.jpeg") as CFString, 1, nil) else {
        throw NSError(domain: "Image destination", code: 1)
    }
    CGImageDestinationAddImage(destination, image, [kCGImagePropertyOrientation: orientation] as CFDictionary)
    guard CGImageDestinationFinalize(destination) else { throw NSError(domain: "Image encoding", code: 1) }
    return data as Data
}

private func referenceDecode(_ data: Data, limit: Int = 2048) -> CGImage? {
    guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
    return CGImageSourceCreateThumbnailAtIndex(source, 0, [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: limit,
    ] as CFDictionary)
}

private func pixels(_ image: CGImage) throws -> String {
    let space = CGColorSpaceCreateDeviceRGB()
    guard let context = CGContext(data: nil, width: image.width, height: image.height,
                                  bitsPerComponent: 8, bytesPerRow: image.width * 4, space: space,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue),
          let bytes = context.data else { throw NSError(domain: "Pixel context", code: 1) }
    context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
    return SHA256.hash(data: Data(bytes: bytes, count: image.width * image.height * 4)).map { String(format: "%02x", $0) }.joined()
}

private actor ImageLoadGate {
    let data: Data
    private(set) var urls: [URL] = []
    private(set) var canceled: Set<Int> = []
    private var suspended: [Int: CheckedContinuation<Void, Never>] = [:]
    var started: Int { urls.count }

    init(data: Data) { self.data = data }

    func load(_ url: URL) async throws -> Data {
        let id = started
        urls.append(url)
        await withCheckedContinuation { suspended[id] = $0 }
        if Task.isCancelled { canceled.insert(id) }
        try Task.checkCancellation()
        return data
    }

    func release(_ id: Int) { suspended.removeValue(forKey: id)?.resume() }
}

private actor ImageDecodeGate {
    private(set) var heights: [Int] = []
    private var suspended: CheckedContinuation<Void, Never>?

    func decode(_ data: Data, height: Int) async throws -> UIImage {
        heights.append(height)
        await withCheckedContinuation { suspended = $0 }
        return try nexaDownsampleImage(data, height: height)
    }

    func release() {
        let continuation = suspended
        suspended = nil
        continuation?.resume()
    }
}

@MainActor
private func expectCancellation(_ task: Task<UIImage, any Error>) async throws {
    task.cancel()
    do {
        _ = try await task.value
        throw NSError(domain: "Canceled waiter returned image", code: 1)
    } catch is CancellationError {}
}

@MainActor
private func verifySharedLoads(at url: URL) async throws {
    let data = try encodedImage(width: 384, height: 256, png: true, orientation: 1)
    let gate = ImageLoadGate(data: data)
    let decoder = ImageDecodeGate()
    let loader = NexaImageLoader(localLoadLimit: 2, read: gate.load, decode: decoder.decode)
    var started = 0
    func request(height: Int = 72) -> Task<UIImage, any Error> {
        Task { @MainActor in
            started += 1
            return try await loader.image(for: url, height: height)
        }
    }
    let first = request(height: 2048)
    let second = request(height: 64)
    let third = request(height: 96)
    while started < 3 { await Task.yield() }
    while await gate.started < 1 { await Task.yield() }
    try await expectCancellation(first)
    await gate.release(0)
    while await decoder.heights.isEmpty { await Task.yield() }
    let late = request(height: 160)
    while started < 4 { await Task.yield() }
    await decoder.release()
    let two = try await second.value
    let three = try await third.value
    while await decoder.heights.count < 2 { await Task.yield() }
    await decoder.release()
    let larger = try await late.value
    guard two === three, two.size.height == 96, larger.size.height == 160,
          await decoder.heights == [96, 160], await gate.started == 1, await gate.canceled.isEmpty
    else {
        throw NSError(domain: "Concurrent requests did not share the active load", code: 1)
    }

    let abandoned = request()
    while await gate.started < 2 { await Task.yield() }
    try await expectCancellation(abandoned)
    let replacement = request()
    while await gate.started < 3 { await Task.yield() }
    await gate.release(1)
    while !(await gate.canceled.contains(1)) { await Task.yield() }
    await gate.release(2)
    while await decoder.heights.count < 3 { await Task.yield() }
    await decoder.release()
    let replacementImage = try await replacement.value
    guard replacementImage !== two else {
        throw NSError(domain: "Replacement request returned stale image", code: 1)
    }
}

@MainActor
private func verifyFileLoadScheduling(data: Data, baseURL: URL) async throws {
    let gate = ImageLoadGate(data: data)
    let loader = NexaImageLoader(localLoadLimit: 1, read: gate.load)
    var registered = 0
    func submit(_ url: URL) async -> Task<UIImage, any Error> {
        let expected = registered + 1
        let waiter = Task { @MainActor in
            registered += 1
            return try await loader.image(for: url, height: 32)
        }
        while registered < expected { await Task.yield() }
        return waiter
    }

    let first = baseURL.appendingPathComponent("first")
    let second = baseURL.appendingPathComponent("second")
    let third = baseURL.appendingPathComponent("third")
    let fourth = baseURL.appendingPathComponent("fourth")
    let dropped = baseURL.appendingPathComponent("canceled-before-rotation")
    let ready = baseURL.appendingPathComponent("canceled-after-rotation")
    let remote = URL(string: "https://example.test/cover.jpg")!

    let running = await submit(first)
    while await gate.started < 1 { await Task.yield() }
    let droppedWaiter = await submit(dropped)
    let secondWaiter = await submit(second)
    let secondDuplicate = await submit(second)
    let readyWaiter = await submit(ready)
    let thirdWaiter = await submit(third)
    let remoteWaiter = await submit(remote)
    while await gate.started < 2 { await Task.yield() }
    guard await gate.urls == [first, remote] else {
        throw NSError(domain: "File limit did not leave HTTPS independent", code: 1)
    }
    try await expectCancellation(droppedWaiter)
    try await expectCancellation(running)
    await gate.release(1)
    _ = try await remoteWaiter.value
    guard await gate.urls == [first, remote] else {
        throw NSError(domain: "Canceled native work released its slot before it exited", code: 1)
    }

    await gate.release(0)
    while await gate.started < 3 { await Task.yield() }
    let fourthWaiter = await submit(fourth)
    try await expectCancellation(readyWaiter)
    await gate.release(2)
    let decodedSecond = try await secondWaiter.value
    let duplicate = try await secondDuplicate.value
    guard decodedSecond === duplicate else {
        throw NSError(domain: "Queued duplicate launched a separate decode", code: 1)
    }
    while await gate.started < 4 { await Task.yield() }
    await gate.release(3)
    _ = try await thirdWaiter.value
    while await gate.started < 5 { await Task.yield() }
    await gate.release(4)
    _ = try await fourthWaiter.value
    guard await gate.urls == [first, remote, second, third, fourth], await gate.canceled == [0] else {
        throw NSError(domain: "File queue started canceled work or lost FIFO order", code: 1)
    }
}

@main struct Check {
    @MainActor static func main() async throws {
        let path = URL(fileURLWithPath: CommandLine.arguments[1])
        let loader = NexaImageLoader.shared
        guard nexaImageDecodeHeight(65, displayScale: 3) == 195,
              nexaImageDecodeHeight(0.1, displayScale: 3) == 1,
              nexaImageDecodeHeight(900, displayScale: 3) == 2048,
              nexaImageDecodeHeight(nil, displayScale: 3) == 2048
        else { throw NSError(domain: "Image quality did not map to display pixels", code: 1) }
        var cases = 0
        for (width, height, png, orientation) in [(96, 64, true, 1), (3072, 2048, true, 1), (4096, 2048, false, 6)] {
            let data = try encodedImage(width: width, height: height, png: png, orientation: orientation)
            try data.write(to: path, options: .atomic)
            guard let actual = try await loader.image(for: path).cgImage, let reference = referenceDecode(data) else {
                throw NSError(domain: "Valid image decoding", code: 1)
            }
            guard actual.width == reference.width, actual.height == reference.height,
                  max(actual.width, actual.height) <= 2048,
                  try pixels(actual) == pixels(reference) else { throw NSError(domain: "Image geometry or pixels changed", code: 1) }
            let expectedEdge = orientation == 6 ? 40 : 60
            guard let thumbnail = try await loader.image(for: path, height: 40).cgImage,
                  let thumbnailReference = referenceDecode(data, limit: expectedEdge),
                  thumbnail.width == (orientation == 6 ? 20 : 60), thumbnail.height == 40,
                  try pixels(thumbnail) == pixels(thumbnailReference)
            else { throw NSError(domain: "Sized image geometry or pixels changed", code: 1) }
            cases += 2
        }
        try Data([0, 1, 2]).write(to: path, options: .atomic)
        do {
            _ = try await loader.image(for: path)
            throw NSError(domain: "Invalid image accepted", code: 1)
        } catch NexaRemoteImageError.invalidResponse {}
        let oversized = try FileHandle(forWritingTo: path)
        try oversized.truncate(atOffset: UInt64(nexaImageByteLimit) + 1)
        try oversized.close()
        do {
            _ = try await loader.image(for: path)
            throw NSError(domain: "Oversized image accepted", code: 1)
        } catch NexaRemoteImageError.invalidResponse {}
        let large = try encodedImage(width: 6144, height: 4096, png: false, orientation: 1)
        try large.write(to: path, options: .atomic)
        var heartbeats = 0
        let heartbeat = Task { @MainActor in
            while !Task.isCancelled {
                heartbeats += 1
                try? await Task.sleep(for: .milliseconds(1))
            }
        }
        _ = try await loader.image(for: path)
        heartbeat.cancel()
        await heartbeat.value
        guard heartbeats > 0 else { throw NSError(domain: "Main actor did not progress during decode", code: 1) }
        let canceled = Task { @MainActor in try await loader.image(for: path) }
        canceled.cancel()
        do { _ = try await canceled.value; throw NSError(domain: "Canceled decode returned image", code: 1) }
        catch is CancellationError {}
        try await verifySharedLoads(at: path)
        try await verifyFileLoadScheduling(data: large, baseURL: path)
        print("{\"pixelAndGeometryCases\":\(cases),\"invalidDataRejected\":true,\"oversizedFileRejected\":true,\"mainActorProgressed\":true,\"sharedLoadAndCancellationVerified\":true,\"fileLoadSchedulingVerified\":true}")
    }
}
