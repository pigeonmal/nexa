// Only app-root environment plumbing is stubbed. The native controller and its
// Core Animation work execute unchanged. No screen is captured.
private struct TestMetricKey: EnvironmentKey { static let defaultValue = 0 }
private struct TestFillKey: EnvironmentKey { static let defaultValue = false }
extension EnvironmentValues {
    var nexaWindowMetrics: Int { get { self[TestMetricKey.self] } set { self[TestMetricKey.self] = newValue } }
    var nexaViewportSize: Int { get { self[TestMetricKey.self] } set { self[TestMetricKey.self] = newValue } }
    var nexaVisualSafeAreaInsets: Int { get { self[TestMetricKey.self] } set { self[TestMetricKey.self] = newValue } }
    var nexaWindowSafeAreaInsets: Int { get { self[TestMetricKey.self] } set { self[TestMetricKey.self] = newValue } }
    var nexaSafeAreaRootFillsWidth: Bool { get { self[TestFillKey.self] } set { self[TestFillKey.self] = newValue } }
}

extension NexaMarqueeController {
    var testRunning: Bool { started != nil || delayedStart != nil || host.view.layer.animation(forKey: "nexa.marquee") != nil }
    var testVisible: Bool { visibleInWindow() }
    var testObserverCount: Int { scrollObservers.count }
    var testResourcesReleased: Bool {
        notifications.isEmpty && scrollObservers.isEmpty && surface.onLayout == nil
            && host.onSourceLayout == nil && surface.layer.mask == nil
    }
    var testDiagnostics: String { "source=\(sourceSize), viewport=\(viewport), visible=\(visibleInWindow()), stationary=\(stationary), loaded=\(isViewLoaded), app=\(UIApplication.shared.applicationState.rawValue)" }
}

@MainActor
private func verify(_ condition: @autoclosure () -> Bool, _ message: String) {
    guard condition() else {
        FileHandle.standardError.write(Data((message + "\n").utf8)); exit(1)
    }
}

@MainActor
private func checkScrollOwnership(environment: EnvironmentValues, text: Text, window: UIWindow) -> Bool {
    let first = UIScrollView(frame: CGRect(x: 0, y: 0, width: 320, height: 200))
    let second = UIScrollView(frame: CGRect(x: 0, y: 240, width: 320, height: 200))
    for scroll in [first, second] {
        scroll.contentSize = CGSize(width: 320, height: 1000)
        window.addSubview(scroll)
    }
    let wrapper = UIView(frame: CGRect(x: 0, y: 0, width: 100, height: 40))
    first.addSubview(wrapper)
    let controller = NexaMarqueeController(content: text, environment: environment)
    controller.loadViewIfNeeded()
    wrapper.addSubview(controller.view)
    controller.view.frame = wrapper.bounds
    controller.configure(horizontal: true, fillsCrossAxis: false, speed: 32, gap: 24, delay: 0, edgeFade: 0,
                         playing: true, foreground: true, reducedMotion: false, rtl: false,
                         revision: ["scroll-owner"], content: text, environment: environment)
    controller.view.layoutIfNeeded()
    verify(controller.testRunning && controller.testVisible, "Initial scroll owner should show an animating marquee")
    // Move an ancestor, preserving the window and the surface's direct parent.
    second.addSubview(wrapper)
    controller.view.setNeedsLayout()
    controller.view.layoutIfNeeded()
    verify(controller.testRunning && controller.testVisible, "Moving within a window should keep visible playback")
    second.contentOffset.y = 200
    RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.03))
    verify(!controller.testVisible, "New scroll owner should clip the marquee offscreen")
    let stopped = !controller.testRunning
    controller.dispose()
    controller.view.removeFromSuperview()
    first.removeFromSuperview(); second.removeFromSuperview()
    return stopped
}

@main struct Check {
    @MainActor static func main() {
        // A simctl-spawned helper has no foreground scene. Override only this
        // OS query within the test process so actual controller playback runs.
        let selector = #selector(getter: UIApplication.applicationState)
        let method = class_getInstanceMethod(UIApplication.self, selector)!
        let original = method_getImplementation(method)
        let active: @convention(c) (AnyObject, Selector) -> Int = { _, _ in UIApplication.State.active.rawValue }
        method_setImplementation(method, unsafeBitCast(active, to: IMP.self))
        defer { method_setImplementation(method, original) }
        verify(UIApplication.shared.applicationState == .active, "Test foreground adapter failed")
        let environment = EnvironmentValues()
        let text = Text("Now playing: an intentionally long media title with native marquee rendering")
        let controller = NexaMarqueeController(content: text, environment: environment)
        controller.loadViewIfNeeded()
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 440, height: 956))
        window.isHidden = false
        let scroll = UIScrollView(frame: CGRect(x: 0, y: 0, width: 320, height: 480))
        scroll.contentSize = CGSize(width: 320, height: 1000)
        window.addSubview(scroll)
        scroll.addSubview(controller.view)
        controller.view.frame = CGRect(x: 0, y: 0, width: 100, height: 40)
        controller.configure(horizontal: true, fillsCrossAxis: false, speed: 32, gap: 24, delay: 0, edgeFade: 8,
                             playing: true, foreground: true, reducedMotion: false, rtl: false,
                             revision: ["title"], content: text, environment: environment)
        controller.view.layoutIfNeeded()
        controller.updatePlayback()
        verify(controller.testRunning, "Overflowing visible content should animate before disposal: \(controller.testDiagnostics)")
        verify(controller.testObserverCount == 1, "Scroll observer should be installed before teardown")
        scroll.contentOffset.y = 1 // Queue a real observer callback before disposal.
        controller.dispose()
        verify(!controller.testRunning, "Disposal should cancel playback")
        // Mirror a previously queued scroll/notification callback arriving after
        // SwiftUI dismantles a controller still retained by a native transition.
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.02))
        controller.updatePlayback()
        let restarted = controller.testRunning
        let expectBug = CommandLine.arguments.contains("--expect-disposal-bug")
        verify(restarted == expectBug, expectBug ? "Expected to reproduce disposal restart" : "Disposed controller restarted native animation")
        controller.dispose()
        if !expectBug {
            verify(controller.testResourcesReleased, "Disposal should release observers, layout callbacks and fade mask")
            controller.configure(horizontal: false, fillsCrossAxis: false, speed: 24, gap: 12, delay: 0, edgeFade: 4,
                                 playing: true, foreground: true, reducedMotion: false, rtl: false,
                                 revision: ["late-update"], content: text, environment: environment)
            controller.view.setNeedsLayout()
            controller.view.layoutIfNeeded()
            controller.updatePlayback()
            verify(!controller.testRunning && controller.testResourcesReleased, "Late configuration/layout should not revive a disposed controller")
            let delayed = NexaMarqueeController(content: text, environment: environment)
            delayed.loadViewIfNeeded()
            scroll.addSubview(delayed.view)
            delayed.view.frame = CGRect(x: 0, y: 0, width: 100, height: 40)
            delayed.configure(horizontal: true, fillsCrossAxis: false, speed: 32, gap: 24, delay: 0.03, edgeFade: 8,
                              playing: true, foreground: true, reducedMotion: false, rtl: false,
                              revision: ["delay"], content: text, environment: environment)
            delayed.view.layoutIfNeeded()
            verify(delayed.testRunning, "Overflow should schedule the initial delay")
            delayed.dispose()
            RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.05))
            verify(!delayed.testRunning && delayed.testResourcesReleased, "Canceled initial delay must not start playback")
            delayed.view.removeFromSuperview()
        }
        controller.view.removeFromSuperview()
        let scrollOwnership = checkScrollOwnership(environment: environment, text: text, window: window)
        verify(scrollOwnership, "Moving to a new scroll owner must preserve offscreen suspension")
        window.isHidden = true
        withExtendedLifetime(window) {}
        print("{\"animationBeforeDisposal\":true,\"restartedAfterDisposal\":\(restarted),\"queuedScrollCallback\":true,\"pendingDelayCanceled\":\(!expectBug),\"lateConfigurationIgnored\":\(!expectBug),\"scrollOwnerStopped\":\(scrollOwnership)}")
    }
}
