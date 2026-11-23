@MainActor
private func runHorizontalListViewportChecks() {
    let view = NexaFastHorizontalListView(frame: .zero, collectionViewLayout: UICollectionViewFlowLayout())
    let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 400, height: 800))
    var deliveries = 0
    view.onNextViewportLayout = { deliveries += 1 }
    window.addSubview(view)
    view.layoutSubviews()
    RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.01))
    verify(deliveries == 0, "A prefetched zero-sized list must not position its content")

    view.frame.size = CGSize(width: 350, height: 96)
    view.layoutSubviews()
    view.removeFromSuperview()
    RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.01))
    verify(deliveries == 0, "A list detached before the layout settles must keep its pending position")

    window.addSubview(view)
    view.layoutSubviews()
    view.layoutSubviews()
    verify(deliveries == 0, "Initial positioning must run outside layoutSubviews")
    RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.01))
    verify(deliveries == 1, "The first usable viewport must deliver the pending position once")

    let cell = NexaFastHorizontalListCell(frame: .zero)
    cell.itemExtent = 96
    let attributes = UICollectionViewLayoutAttributes(forCellWith: IndexPath(item: 0, section: 0))
    attributes.size = .zero
    verify(cell.preferredLayoutAttributesFitting(attributes) === attributes,
        "A zero-sized proposal must not trigger self-sizing invalidation")

    let scroll = UIScrollView(frame: CGRect(x: 0, y: 0, width: 350, height: 96))
    scroll.contentInsetAdjustmentBehavior = .never
    scroll.contentInset = UIEdgeInsets(top: 0, left: 12, bottom: 0, right: 24)
    scroll.contentSize = CGSize(width: 638, height: 96)
    nexaScrollFastListToOffset(in: scroll, horizontal: true, fromMaximum: true, offset: 0, animated: false)
    verify(scroll.contentOffset.x == 312, "The end command includes trailing content padding")

    nexaScrollFastListToOffset(in: scroll, horizontal: true, fromMaximum: false, offset: 160, animated: false)
    verify(scroll.contentOffset.x == 148, "The offset is relative to the inset start")

    nexaScrollFastListToOffset(in: scroll, horizontal: true, fromMaximum: false, offset: 0, animated: false)
    verify(scroll.contentOffset.x == -12, "The top command includes leading content padding")
}
