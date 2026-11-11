// nexa-codegen: NexaMarqueeSurface
private final class NexaMarqueeSurface: UIView {
    override class var layerClass: AnyClass { CAReplicatorLayer.self }
    var onLayout: (() -> Void)?
    override func layoutSubviews() { super.layoutSubviews(); onLayout?() }
    override func didMoveToWindow() { super.didMoveToWindow(); onLayout?() }
}

// nexa-codegen: NexaMarqueeHostedContent
private struct NexaMarqueeHostedContent<Content: View>: View {
    let content: Content
    let environment: EnvironmentValues
    var body: some View {
        content
            .environment(\.colorScheme, environment.colorScheme)
            .environment(\.locale, environment.locale)
            .environment(\.layoutDirection, environment.layoutDirection)
            .environment(\.dynamicTypeSize, environment.dynamicTypeSize)
            .environment(\.font, environment.font)
            .environment(\.multilineTextAlignment, environment.multilineTextAlignment)
            .environment(\.isEnabled, environment.isEnabled)
            .environment(\.displayScale, environment.displayScale)
            .environment(\.nexaWindowMetrics, environment.nexaWindowMetrics)
            .environment(\.nexaViewportSize, environment.nexaViewportSize)
            .environment(\.nexaVisualSafeAreaInsets, environment.nexaVisualSafeAreaInsets)
            .environment(\.nexaWindowSafeAreaInsets, environment.nexaWindowSafeAreaInsets)
            .environment(\.nexaSafeAreaRootFillsWidth, false)
            .ignoresSafeArea(.container, edges: .all)
    }
}

// nexa-codegen: NexaMarqueeSourceController
@MainActor
private final class NexaMarqueeSourceController<Content: View>: UIHostingController<Content> {
    var parentUpdate = true
    var onSourceLayout: (() -> Void)?
    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        if parentUpdate { parentUpdate = false }
        else { onSourceLayout?() }
    }
}

// nexa-codegen: NexaMarqueeController
@MainActor
private final class NexaMarqueeController<Content: View>: UIViewController {
    let host: NexaMarqueeSourceController<NexaMarqueeHostedContent<Content>>
    let surface = NexaMarqueeSurface()
    var horizontal = true
    var fillsCrossAxis = false
    private var environment: EnvironmentValues?
    var speed = 32.0
    var gap = 24.0
    var initialDelay = 1.0
    var edgeFade = 0.0
    var playing = true
    var foreground = true
    var reducedMotion = false
    var rtl = false
    var revision: [String] = []
    private var sourceSize = CGSize.zero
    private var viewport = CGSize.zero
    private var elapsed = 0.0
    private var started: CFTimeInterval?
    private var delayRemaining = 1.0
    private var delayStarted: CFTimeInterval?
    private var delayedStart: DispatchWorkItem?
    private var notifications: [NSObjectProtocol] = []
    private var scrollObservers: [NSKeyValueObservation] = []
    private weak var observingWindow: UIWindow?
    private var layingOut = false
    private var disposed = false
    private var edgeMask: CAGradientLayer?
    private var appliedFadeLength: CGFloat = -1
    private var appliedFadeHorizontal = true

    init(content: Content, environment: EnvironmentValues) {
        host = NexaMarqueeSourceController(rootView: NexaMarqueeHostedContent(content: content, environment: environment))
        super.init(nibName: nil, bundle: nil)
    }
    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("init(coder:) is unavailable") }

    override func loadView() {
        view = surface
        surface.clipsToBounds = true
        surface.backgroundColor = .clear
        host.view.backgroundColor = .clear
        host.sizingOptions = [.intrinsicContentSize, .preferredContentSize]
        if #available(iOS 16.4, *) { host.safeAreaRegions = [] }
        addChild(host)
        surface.addSubview(host.view)
        host.didMove(toParent: self)
        host.onSourceLayout = { [weak self] in
            DispatchQueue.main.async { [weak self] in
                guard let self, !self.disposed else { return }
                // UIKit can lay out the host for accessibility queries or parent
                // updates without changing content. Geometry and typed revision
                // comparisons decide whether the animation actually restarts.
                self.surface.setNeedsLayout()
            }
        }
        surface.onLayout = { [weak self] in self?.layoutContent() }
        for name in [UIAccessibility.voiceOverStatusDidChangeNotification,
                     UIAccessibility.reduceMotionStatusDidChangeNotification,
                     UIApplication.didBecomeActiveNotification,
                     UIApplication.willResignActiveNotification] {
            notifications.append(NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated { self?.updatePlayback() }
            })
        }
    }

    override func preferredContentSizeDidChange(forChildContentContainer container: UIContentContainer) {
        super.preferredContentSizeDidChange(forChildContentContainer: container)
        view.setNeedsLayout()
        view.invalidateIntrinsicContentSize()
    }

    func idealSize(_ proposal: ProposedViewSize) -> CGSize {
        let limit = CGFloat.greatestFiniteMagnitude
        return host.sizeThatFits(in: CGSize(width: horizontal ? limit : (proposal.width ?? limit),
                                           height: horizontal ? (proposal.height ?? limit) : limit))
    }

    func configure(horizontal: Bool, fillsCrossAxis: Bool, speed: Double, gap: Double, delay: Double, edgeFade: Double,
                   playing: Bool, foreground: Bool, reducedMotion: Bool, rtl: Bool,
                   revision: [String], content: Content, environment: EnvironmentValues) {
        guard !disposed else { return }
        let environmentChanged = self.environment.map {
            $0.locale != environment.locale || $0.dynamicTypeSize != environment.dynamicTypeSize
                || $0.font != environment.font || $0.colorScheme != environment.colorScheme
                || $0.displayScale != environment.displayScale
        } ?? false
        let restart = environmentChanged || self.horizontal != horizontal || self.fillsCrossAxis != fillsCrossAxis || self.speed != speed || self.gap != gap
            || self.initialDelay != delay || self.rtl != rtl || self.revision != revision
        self.horizontal = horizontal
        self.fillsCrossAxis = fillsCrossAxis
        self.environment = environment
        self.speed = speed
        self.gap = gap
        self.initialDelay = delay
        self.edgeFade = edgeFade
        self.playing = playing
        self.foreground = foreground
        self.reducedMotion = reducedMotion
        self.rtl = rtl
        self.revision = revision
        host.parentUpdate = true
        host.rootView = NexaMarqueeHostedContent(content: content, environment: environment)
        if restart { reset() }
        if isViewLoaded { view.setNeedsLayout(); updatePlayback() }
    }

    private var period: Double { Double(horizontal ? sourceSize.width : sourceSize.height) + gap }
    private var overflows: Bool {
        let source = horizontal ? sourceSize.width : sourceSize.height
        let available = horizontal ? viewport.width : viewport.height
        return available > 0 && source > available && period.isFinite && period > 0
    }
    private var stationary: Bool {
        reducedMotion || UIAccessibility.isReduceMotionEnabled || UIAccessibility.isVoiceOverRunning || !UIView.areAnimationsEnabled
    }

    private func updateEdgeFade() {
        let extent = horizontal ? surface.bounds.width : surface.bounds.height
        guard edgeFade > 0, extent > 0, overflows, !stationary else {
            if surface.layer.mask != nil {
                CATransaction.begin()
                CATransaction.setDisableActions(true)
                surface.layer.mask = nil
                CATransaction.commit()
            }
            return
        }
        let length = min(CGFloat(edgeFade), extent / 2)
        if let edgeMask, surface.layer.mask === edgeMask,
           edgeMask.frame == surface.bounds, appliedFadeLength == length,
           appliedFadeHorizontal == horizontal { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        defer { CATransaction.commit() }
        let mask: CAGradientLayer
        if let edgeMask { mask = edgeMask }
        else {
            mask = CAGradientLayer()
            mask.colors = [UIColor.clear.cgColor, UIColor.black.cgColor,
                           UIColor.black.cgColor, UIColor.clear.cgColor]
            edgeMask = mask
        }
        if mask.frame != surface.bounds || appliedFadeLength != length || appliedFadeHorizontal != horizontal {
            mask.frame = surface.bounds
            mask.startPoint = .zero
            mask.endPoint = horizontal ? CGPoint(x: 1, y: 0) : CGPoint(x: 0, y: 1)
            let fraction = Double(length / extent)
            mask.locations = [0, NSNumber(value: fraction), NSNumber(value: 1 - fraction), 1]
            appliedFadeLength = length
            appliedFadeHorizontal = horizontal
        }
        if surface.layer.mask !== mask { surface.layer.mask = mask }
    }

    private func visibleInWindow() -> Bool {
        guard let window = surface.window, !surface.isHidden, surface.alpha > 0 else { return false }
        var visible = surface.convert(surface.bounds, to: window).intersection(window.bounds)
        var ancestor = surface.superview
        while let current = ancestor {
            if current.isHidden || current.alpha == 0 { return false }
            if current.clipsToBounds { visible = visible.intersection(current.convert(current.bounds, to: window)) }
            ancestor = current.superview
        }
        return !visible.isEmpty && !visible.isNull
    }

    private func observeScrolling() {
        guard surface.window !== observingWindow else { return }
        observingWindow = surface.window
        scrollObservers.removeAll()
        // Carry a typed actor-bound action across KVO's nonisolated callback,
        // rather than recapturing the generic controller inside that boundary.
        let update: @MainActor @Sendable () -> Void = { [weak self] in self?.updatePlayback() }
        var ancestor = surface.superview
        while let current = ancestor {
            if let scroll = current as? UIScrollView {
                scrollObservers.append(scroll.observe(\.contentOffset, options: [.new]) { _, _ in
                    DispatchQueue.main.async(execute: update)
                })
            }
            ancestor = current.superview
        }
    }

    private func layoutContent() {
        guard !disposed, !layingOut else { return }
        layingOut = true
        defer { layingOut = false }
        observeScrolling()
        let ideal = idealSize(ProposedViewSize(width: surface.bounds.width, height: surface.bounds.height))
        guard ideal.width.isFinite, ideal.height.isFinite else { return }
        let size = CGSize(width: max(0, !horizontal && fillsCrossAxis ? surface.bounds.width : ideal.width),
                          height: max(0, horizontal && fillsCrossAxis ? surface.bounds.height : ideal.height))
        if sourceSize != size || viewport != surface.bounds.size {
            pause()
            sourceSize = size
            viewport = surface.bounds.size
            reset()
        }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        host.view.bounds = CGRect(origin: .zero, size: size)
        host.view.layer.position = CGPoint(x: size.width / 2, y: size.height / 2)
        if let replicator = surface.layer as? CAReplicatorLayer {
            replicator.instanceCount = overflows && !stationary ? 2 : 1
            let distance = CGFloat(period) * (horizontal && rtl ? -1 : 1)
            replicator.instanceTransform = CATransform3DMakeTranslation(horizontal ? distance : 0, horizontal ? 0 : distance, 0)
        }
        applyPosition()
        CATransaction.commit()
        updatePlayback()
    }

    private var base: Double { horizontal && rtl ? Double(viewport.width - sourceSize.width) : 0 }
    private var direction: Double { horizontal && rtl ? 1 : -1 }
    private var animationKey: String { horizontal ? "transform.translation.x" : "transform.translation.y" }

    private func applyPosition() {
        let phase = overflows && !stationary ? (elapsed * speed).truncatingRemainder(dividingBy: period) : 0
        let offset = base + direction * phase
        host.view.layer.transform = CATransform3DMakeTranslation(horizontal ? CGFloat(offset) : 0, horizontal ? 0 : CGFloat(offset), 0)
    }

    private func pause() {
        let now = CACurrentMediaTime()
        if let started { elapsed += max(0, now - started) }
        started = nil
        if let delayStarted { delayRemaining = max(0, delayRemaining - (now - delayStarted)) }
        delayStarted = nil
        delayedStart?.cancel()
        delayedStart = nil
        host.view.layer.removeAnimation(forKey: "nexa.marquee")
        CATransaction.begin(); CATransaction.setDisableActions(true)
        applyPosition()
        CATransaction.commit()
    }

    private func reset() {
        pause()
        elapsed = 0
        delayRemaining = initialDelay
    }

    func updatePlayback() {
        guard !disposed, isViewLoaded else { return }
        updateEdgeFade()
        if stationary {
            pause()
            elapsed = 0
            (surface.layer as? CAReplicatorLayer)?.instanceCount = 1
            applyPosition()
            return
        }
        guard overflows, playing, foreground, UIApplication.shared.applicationState == .active, visibleInWindow() else { pause(); return }
        (surface.layer as? CAReplicatorLayer)?.instanceCount = 2
        guard started == nil, delayedStart == nil else { return }
        if delayRemaining > 0 {
            delayStarted = CACurrentMediaTime()
            let work = DispatchWorkItem { [weak self] in
                guard let self else { return }
                self.delayRemaining = 0
                self.delayStarted = nil
                self.delayedStart = nil
                self.updatePlayback()
            }
            delayedStart = work
            DispatchQueue.main.asyncAfter(deadline: .now() + min(delayRemaining, Double(Int.max) / 1_000_000_000), execute: work)
            return
        }
        let duration = period / speed
        guard duration.isFinite, duration > 0 else { return }
        let animation = CABasicAnimation(keyPath: animationKey)
        animation.fromValue = base
        animation.toValue = base + direction * period
        animation.duration = duration
        animation.repeatCount = .infinity
        animation.timingFunction = CAMediaTimingFunction(name: .linear)
        let now = CACurrentMediaTime()
        animation.beginTime = host.view.layer.convertTime(now, from: nil) - elapsed.truncatingRemainder(dividingBy: duration)
        host.view.layer.add(animation, forKey: "nexa.marquee")
        started = now
    }

    func dispose() {
        // Dismantled controllers can outlive SwiftUI's subtree during native
        // transitions. Queued scroll/layout callbacks must never restart them.
        guard !disposed else { return }
        disposed = true
        pause()
        notifications.forEach(NotificationCenter.default.removeObserver)
        notifications.removeAll()
        scrollObservers.removeAll()
        surface.onLayout = nil
        host.onSourceLayout = nil
        surface.layer.mask = nil
        edgeMask = nil
    }
}

// nexa-codegen: NexaMarqueeBridge
private struct NexaMarqueeBridge<Content: View>: UIViewControllerRepresentable {
    let horizontal: Bool
    let fillsCrossAxis: Bool
    let speed: Double
    let gap: Double
    let delay: Double
    let edgeFade: Double
    let playing: Bool
    let foreground: Bool
    let reducedMotion: Bool
    let rtl: Bool
    let revision: [String]
    let content: Content

    func makeUIViewController(context: Context) -> NexaMarqueeController<Content> {
        NexaMarqueeController(content: content, environment: context.environment)
    }
    func updateUIViewController(_ controller: NexaMarqueeController<Content>, context: Context) {
        controller.configure(horizontal: horizontal, fillsCrossAxis: fillsCrossAxis, speed: speed, gap: gap, delay: delay, edgeFade: edgeFade, playing: playing,
                             foreground: foreground, reducedMotion: reducedMotion, rtl: rtl, revision: revision, content: content, environment: context.environment)
    }
    func sizeThatFits(_ proposal: ProposedViewSize, uiViewController: NexaMarqueeController<Content>, context: Context) -> CGSize? {
        let ideal = uiViewController.idealSize(proposal)
        let width = horizontal || fillsCrossAxis ? (proposal.width.flatMap { $0.isFinite ? $0 : nil } ?? ideal.width) : ideal.width
        let height = !horizontal || fillsCrossAxis ? (proposal.height.flatMap { $0.isFinite ? $0 : nil } ?? ideal.height) : ideal.height
        return CGSize(width: max(0, width), height: max(0, height))
    }
    static func dismantleUIViewController(_ controller: NexaMarqueeController<Content>, coordinator: ()) { controller.dispose() }
}

// nexa-codegen: NexaMarqueePrimitive
struct NexaMarqueePrimitive<Content: View>: View {
    let horizontal: Bool
    let fillsCrossAxis: Bool
    let speed: Double
    let gap: Double
    let delay: Double
    let edgeFade: Double
    let playing: Bool
    let revision: [String]
    @ViewBuilder let content: () -> Content
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.accessibilityReduceMotion) private var reducedMotion
    @Environment(\.layoutDirection) private var layoutDirection

    var body: some View {
        NexaMarqueeBridge(horizontal: horizontal, fillsCrossAxis: fillsCrossAxis, speed: speed, gap: gap, delay: delay, edgeFade: edgeFade,
                          playing: playing, foreground: scenePhase == .active, reducedMotion: reducedMotion,
                          rtl: layoutDirection == .rightToLeft, revision: revision,
                          content: content().fixedSize(horizontal: horizontal, vertical: !horizontal))
    }
}
