import Foundation
import QuartzCore
import SwiftUI
import UIKit

/// Performance metrics monitor tracking display link frames and FPS.
@MainActor
final class NexaDevPerformanceMonitor: NSObject, ObservableObject {
    @Published private(set) var fps = 0
    @Published private(set) var frameTimeMs = 0.0

    private var displayLink: CADisplayLink?
    private var frameCount = 0
    private var lastSampleTime = 0.0
    private var previousTimestamp = 0.0

    func setEnabled(_ enabled: Bool) {
        displayLink?.invalidate()
        displayLink = nil
        frameCount = 0
        lastSampleTime = 0.0
        previousTimestamp = 0.0
        guard enabled else {
            fps = 0
            frameTimeMs = 0.0
            return
        }
        let link = CADisplayLink(target: self, selector: #selector(handleFrame(_:)))
        link.add(to: .main, forMode: .common)
        displayLink = link
    }

    @objc private func handleFrame(_ link: CADisplayLink) {
        let timestamp = link.timestamp
        if previousTimestamp != 0.0 {
            frameCount += 1
            frameTimeMs = (timestamp - previousTimestamp) * 1000.0
        }
        if lastSampleTime == 0.0 {
            lastSampleTime = timestamp
        }
        let elapsed = timestamp - lastSampleTime
        if elapsed >= 1.0 {
            fps = Int(Double(frameCount) / elapsed)
            frameTimeMs = frameCount > 0 ? (elapsed * 1000.0) / Double(frameCount) : 0.0
            frameCount = 0
            lastSampleTime = timestamp
        }
        previousTimestamp = timestamp
    }
}

/// Floating HUD overlay displaying live FPS and frame duration.
struct NexaDevPerformanceOverlay: View {
    let fps: Int
    let frameTimeMs: Double

    var body: some View {
        HStack(spacing: 8) {
            Text("\(fps) FPS")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
            Text(String(format: "%.1f ms", frameTimeMs))
                .font(.system(size: 11, weight: .regular, design: .monospaced))
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Color.black.opacity(0.75))
        .foregroundStyle(.white)
        .clipShape(Capsule())
    }
}

/// Status bar visibility modifier for modern and legacy iOS versions.
struct NexaDevStatusBarVisibility: ViewModifier {
    let hidden: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 27.0, *) {
            content.toolbarVisibility(hidden ? .hidden : .visible, for: .statusBar)
        } else {
            content.statusBarHidden(hidden)
        }
    }
}

func devStatusBarColor(_ value: Any?, isDark: Bool) -> Color? {
    guard let tagged = value as? [String: Any] else { return nil }
    let payload: [String: Any]
    if let fixed = tagged["Static"] as? [String: Any] {
        payload = fixed
    } else if let adaptive = tagged["Adaptive"] as? [String: Any],
              let selected = adaptive[isDark ? "dark" : "light"] as? [String: Any] {
        payload = selected
    } else {
        return nil
    }
    guard let red = payload["red"] as? Double,
          let green = payload["green"] as? Double,
          let blue = payload["blue"] as? Double,
          let alpha = payload["alpha"] as? Double
    else { return nil }
    return Color(.sRGB, red: red / 255, green: green / 255, blue: blue / 255, opacity: alpha / 255)
}

func devColor(_ value: Any?, isDark: Bool) -> Color? {
    devStatusBarColor(value, isDark: isDark)
}
