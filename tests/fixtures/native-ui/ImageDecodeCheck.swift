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

private func referenceDecode(_ data: Data) -> CGImage? {
    guard let source = CGImageSourceCreateWithData(data as CFData, nil) else { return nil }
    return CGImageSourceCreateThumbnailAtIndex(source, 0, [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: 2048,
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

@main struct Check {
    @MainActor static func main() async throws {
        var cases = 0
        for (width, height, png, orientation) in [(96, 64, true, 1), (3072, 2048, true, 1), (4096, 2048, false, 6)] {
            let data = try encodedImage(width: width, height: height, png: png, orientation: orientation)
            guard let actual = try await nexaDecodeImage(data)?.cgImage, let reference = referenceDecode(data) else {
                throw NSError(domain: "Valid image decoding", code: 1)
            }
            guard actual.width == reference.width, actual.height == reference.height,
                  max(actual.width, actual.height) <= 2048,
                  try pixels(actual) == pixels(reference) else { throw NSError(domain: "Image geometry or pixels changed", code: 1) }
            cases += 1
        }
        guard try await nexaDecodeImage(Data([0, 1, 2])) == nil else { throw NSError(domain: "Invalid image accepted", code: 1) }
        let large = try encodedImage(width: 6144, height: 4096, png: false, orientation: 1)
        var heartbeats = 0
        let heartbeat = Task { @MainActor in
            while !Task.isCancelled {
                heartbeats += 1
                try? await Task.sleep(for: .milliseconds(1))
            }
        }
        _ = try await nexaDecodeImage(large)
        heartbeat.cancel()
        await heartbeat.value
        guard heartbeats > 0 else { throw NSError(domain: "Main actor did not progress during decode", code: 1) }
        let canceled = Task { @MainActor in try await nexaDecodeImage(large) }
        canceled.cancel()
        do { _ = try await canceled.value; throw NSError(domain: "Canceled decode returned image", code: 1) }
        catch is CancellationError {}
        print("{\"pixelAndGeometryCases\":\(cases),\"invalidDataRejected\":true,\"mainActorProgressed\":true,\"cancellationPropagated\":true}")
    }
}
