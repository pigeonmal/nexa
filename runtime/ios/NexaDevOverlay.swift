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

struct NexaDevErrorOverlay: View {
    let diagnostics: [NexaDevDiagnostic]
    let onOpenInEditor: (NexaDevDiagnostic) -> Void

    @State private var detailsPresented = false

    private var firstDiagnostic: NexaDevDiagnostic? { diagnostics.first }

    var body: some View {
        if let diagnostic = firstDiagnostic {
            Button {
                detailsPresented = true
            } label: {
                VStack(alignment: .leading, spacing: 5) {
                    Text("Nexa development error")
                        .font(.subheadline.weight(.semibold))
                    Text(diagnostic.isRuntimeFailure ? "Runtime error · source mapping unavailable" : diagnostic.sourceLocation)
                        .font(.caption.monospaced())
                        .foregroundStyle(.red)
                    Text(diagnostic.message)
                        .font(.subheadline)
                        .lineLimit(2)
                    Text(diagnostics.count > 1 ? "\(diagnostics.count) errors · tap to inspect" : (diagnostic.isRuntimeFailure ? "Tap to inspect stack trace" : "Tap to inspect and open in editor"))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(14)
                .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
                .overlay {
                    RoundedRectangle(cornerRadius: 16)
                        .stroke(.red.opacity(0.35), lineWidth: 1)
                }
            }
            .buttonStyle(.plain)
            .sheet(isPresented: $detailsPresented) {
                NavigationStack {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 18) {
                            ForEach(diagnostics) { item in
                                VStack(alignment: .leading, spacing: 8) {
                                    Text(item.isRuntimeFailure ? "Runtime error · source mapping unavailable" : item.sourceLocation)
                                        .font(.caption.monospaced())
                                        .foregroundStyle(.red)
                                    Text(item.message)
                                        .font(.body)
                                    if item.isRuntimeFailure {
                                        if let stackTrace = item.stackTrace {
                                            Text(stackTrace)
                                                .font(.caption.monospaced())
                                                .textSelection(.enabled)
                                        }
                                    } else {
                                        Button {
                                            onOpenInEditor(item)
                                        } label: {
                                            Label("Open in editor", systemImage: "arrow.up.forward.app")
                                        }
                                        .buttonStyle(.bordered)
                                    }
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                            }
                        }
                        .padding()
                    }
                    .navigationTitle("Development diagnostics")
                    .navigationBarTitleDisplayMode(.inline)
                    .toolbar {
                        ToolbarItem(placement: .confirmationAction) {
                            Button("Done") { detailsPresented = false }
                        }
                    }
                }
                .presentationDetents([.medium, .large])
            }
        }
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
    if let wrapped = tagged["Static"] as? [String: Any],
       wrapped["Static"] != nil || wrapped["Adaptive"] != nil {
        return devStatusBarColor(wrapped, isDark: isDark)
    }
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

func devHexColor(_ value: String) -> Color? {
    let digits = value.hasPrefix("#") ? String(value.dropFirst()) : value
    guard let packed = UInt64(digits, radix: 16) else { return nil }
    switch digits.count {
    case 6:
        return Color(.sRGB, red: Double((packed >> 16) & 0xFF) / 255, green: Double((packed >> 8) & 0xFF) / 255, blue: Double(packed & 0xFF) / 255, opacity: 1)
    case 8:
        return Color(.sRGB, red: Double((packed >> 24) & 0xFF) / 255, green: Double((packed >> 16) & 0xFF) / 255, blue: Double((packed >> 8) & 0xFF) / 255, opacity: Double(packed & 0xFF) / 255)
    default:
        return nil
    }
}

func devColor(_ value: Any?, isDark: Bool) -> Color? {
    devStatusBarColor(value, isDark: isDark)
}
