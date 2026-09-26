import Foundation
import ImageIO
import Vision

/// Formatting strategy for Vision screenshot text extraction.
enum ScreenshotExtractionMode: String {
    case text
    case layout
    case geometry

    func format(text: String, observation: VNRecognizedTextObservation) -> String {
        switch self {
        case .text:
            return text
        case .layout:
            return "\(observation.boundingBox.midX)\t\(text)"
        case .geometry:
            return "\(observation.boundingBox.midX)\t\(1 - observation.boundingBox.midY)\t\(text)"
        }
    }
}

var mode: ScreenshotExtractionMode = .text
var imagePath: String?

var args = Array(CommandLine.arguments.dropFirst())
var index = 0
while index < args.count {
    let arg = args[index]
    if arg == "--mode" && index + 1 < args.count {
        index += 1
        if let parsed = ScreenshotExtractionMode(rawValue: args[index].lowercased()) {
            mode = parsed
        } else {
            fputs("error: invalid mode '\(args[index])'. Valid modes: text, layout, geometry\n", stderr)
            exit(2)
        }
    } else if let parsed = ScreenshotExtractionMode(rawValue: arg.lowercased()), imagePath != nil {
        mode = parsed
    } else if !arg.starts(with: "-") && imagePath == nil {
        imagePath = arg
    } else if imagePath != nil, let parsed = ScreenshotExtractionMode(rawValue: arg.lowercased()) {
        mode = parsed
    } else {
        fputs("usage: read-screenshot.swift [--mode text|layout|geometry] <image>\n", stderr)
        exit(2)
    }
    index += 1
}

guard let path = imagePath,
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
else {
    fputs("usage: read-screenshot.swift [--mode text|layout|geometry] <image>\n", stderr)
    exit(2)
}

let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
try VNImageRequestHandler(cgImage: image).perform([request])
for observation in request.results ?? [] {
    guard let text = observation.topCandidates(1).first?.string else { continue }
    print(mode.format(text: text, observation: observation))
}
