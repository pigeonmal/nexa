// nexa-codegen: NexaScrollIndicatorVisibility
struct NexaScrollIndicatorVisibility: ViewModifier {
    let hideHorizontal: Bool
    let hideVertical: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if hideHorizontal && hideVertical {
            content.scrollIndicators(.hidden)
        } else if hideHorizontal {
            content.scrollIndicators(.hidden, axes: .horizontal)
        } else if hideVertical {
            content.scrollIndicators(.hidden, axes: .vertical)
        } else {
            content
        }
    }
}

// nexa-codegen: NexaScrollViewPrimitive
struct NexaScrollViewPrimitive<Content: View>: View {
    let axis: Axis.Set
    let hideHorizontalScrollbar: Bool
    let hideVerticalScrollbar: Bool
    let componentRef: NexaScrollViewRef?
    @ViewBuilder let content: () -> Content

    var body: some View {
        let showsIndicators = axis == .vertical
            ? !hideVerticalScrollbar
            : !hideHorizontalScrollbar
        ScrollView(axis, showsIndicators: showsIndicators) {
            content()
                .background {
                    if let componentRef {
                        NexaScrollViewProbe(reference: componentRef, axis: axis)
                            .frame(width: 0, height: 0)
                            .accessibilityHidden(true)
                    }
                }
        }
        .modifier(NexaScrollIndicatorVisibility(
            hideHorizontal: hideHorizontalScrollbar,
            hideVertical: hideVerticalScrollbar
        ))
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}

@MainActor
private struct NexaScrollViewProbe: UIViewRepresentable {
    let reference: NexaScrollViewRef
    let axis: Axis.Set

    func makeUIView(context: Context) -> NexaScrollViewProbeView {
        let view = NexaScrollViewProbeView()
        view.isUserInteractionEnabled = false
        view.backgroundColor = .clear
        view.reference = reference
        view.axis = axis
        return view
    }

    func updateUIView(_ view: NexaScrollViewProbeView, context: Context) {
        view.reference = reference
        view.axis = axis
        view.attachIfAvailable()
    }
}

@MainActor
private final class NexaScrollViewProbeView: UIView {
    var reference: NexaScrollViewRef?
    var axis: Axis.Set = .vertical
    private weak var attachedScrollView: UIScrollView?
    private weak var attachedReference: NexaScrollViewRef?
    private var attachedAxis: Axis.Set?
    private var attachmentGeneration: UInt64?

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil {
            detach()
        } else {
            attachIfAvailable()
        }
    }

    override func didMoveToSuperview() {
        super.didMoveToSuperview()
        attachIfAvailable()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        attachIfAvailable()
    }

    func attachIfAvailable() {
        guard let scrollView = findScrollView() else { return }
        guard let reference else {
            detach()
            return
        }
        guard attachedScrollView !== scrollView
            || attachedReference !== reference
            || attachedAxis != axis else { return }
        detach()
        let vertical = axis == .vertical
        attachedScrollView = scrollView
        attachedReference = reference
        attachedAxis = axis
        attachmentGeneration = reference.attach(
            view: scrollView,
            scrollToTop: { [weak scrollView] animated in
                guard let scrollView else { return }
                let inset = scrollView.adjustedContentInset
                if vertical {
                    scrollView.setContentOffset(CGPoint(x: scrollView.contentOffset.x, y: -inset.top), animated: animated)
                } else {
                    let x = Self.logicalStartX(for: scrollView, inset: inset)
                    scrollView.setContentOffset(CGPoint(x: x, y: scrollView.contentOffset.y), animated: animated)
                }
            },
            scrollToEnd: { [weak scrollView] animated in
                guard let scrollView else { return }
                let inset = scrollView.adjustedContentInset
                if vertical {
                    let minimum = -inset.top
                    let maximum = max(minimum, scrollView.contentSize.height - scrollView.bounds.height + inset.bottom)
                    scrollView.setContentOffset(CGPoint(x: scrollView.contentOffset.x, y: maximum), animated: animated)
                } else {
                    let minimum = -inset.left
                    let maximum = max(minimum, scrollView.contentSize.width - scrollView.bounds.width + inset.right)
                    let x = scrollView.effectiveUserInterfaceLayoutDirection == .rightToLeft ? minimum : maximum
                    scrollView.setContentOffset(CGPoint(x: x, y: scrollView.contentOffset.y), animated: animated)
                }
            },
            scrollToOffset: { [weak scrollView] offset, animated in
                guard let scrollView else { return }
                let inset = scrollView.adjustedContentInset
                let finiteOffset = CGFloat(offset.isFinite ? max(0, offset) : 0)
                if vertical {
                    let minimum = -inset.top
                    let maximum = max(minimum, scrollView.contentSize.height - scrollView.bounds.height + inset.bottom)
                    let y = (minimum + finiteOffset).clamped(to: minimum...maximum)
                    scrollView.setContentOffset(CGPoint(x: scrollView.contentOffset.x, y: y), animated: animated)
                } else {
                    let minimum = -inset.left
                    let maximum = max(minimum, scrollView.contentSize.width - scrollView.bounds.width + inset.right)
                    let start = scrollView.effectiveUserInterfaceLayoutDirection == .rightToLeft ? maximum : minimum
                    let target = scrollView.effectiveUserInterfaceLayoutDirection == .rightToLeft
                        ? start - finiteOffset
                        : start + finiteOffset
                    scrollView.setContentOffset(CGPoint(x: target.clamped(to: minimum...maximum), y: scrollView.contentOffset.y), animated: animated)
                }
            }
        )
    }

    private static func logicalStartX(for scrollView: UIScrollView, inset: UIEdgeInsets) -> CGFloat {
        if scrollView.effectiveUserInterfaceLayoutDirection == .rightToLeft {
            return max(-inset.left, scrollView.contentSize.width - scrollView.bounds.width + inset.right)
        }
        return -inset.left
    }

    private func findScrollView() -> UIScrollView? {
        var candidate = superview
        while let view = candidate {
            if let scrollView = view as? UIScrollView { return scrollView }
            candidate = view.superview
        }
        return nil
    }

    private func detach() {
        if let attachmentGeneration {
            attachedReference?.detach(generation: attachmentGeneration)
        }
        attachmentGeneration = nil
        attachedScrollView = nil
        attachedReference = nil
        attachedAxis = nil
    }
}

private extension CGFloat {
    func clamped(to range: ClosedRange<CGFloat>) -> CGFloat {
        Swift.min(Swift.max(self, range.lowerBound), range.upperBound)
    }
}
