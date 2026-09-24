import Foundation
import ImageIO
import Vision

guard CommandLine.arguments.count == 2,
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: CommandLine.arguments[1]) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
else {
    fputs("usage: read-screenshot-layout.swift <image>\n", stderr)
    exit(2)
}

let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
try VNImageRequestHandler(cgImage: image).perform([request])
for observation in request.results ?? [] {
    guard let text = observation.topCandidates(1).first?.string else { continue }
    print("\(observation.boundingBox.midX)\t\(text)")
}
