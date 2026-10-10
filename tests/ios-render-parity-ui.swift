import XCTest
import UIKit

final class NexaRenderParityTests: XCTestCase {
    private func capture(_ phase: String, app: XCUIApplication) throws {
        // Filter in XCTest before materializing elements. A full `.any` walk
        // includes hundreds of unlabeled SwiftUI layout nodes, and every
        // property read crosses the simulator test-runner boundary.
        let elements = app.descendants(matching: .any)
            .matching(NSPredicate(format: "label != '' OR identifier != ''"))
            .allElementsBoundByIndex
        let snapshot = elements.compactMap { element -> [String: Any]? in
            let frame = element.frame
            guard frame.width > 0, frame.height > 0, app.frame.intersects(frame) else { return nil }
            let label = element.label
            let identifier = element.identifier
            return [
                "role": String(describing: element.elementType),
                "label": label,
                "identifier": identifier,
                "enabled": element.isEnabled,
                "hittable": element.isHittable,
                "selected": element.isSelected,
                "frame": [
                    "x": (frame.minX * 10).rounded() / 10,
                    "y": (frame.minY * 10).rounded() / 10,
                    "width": (frame.width * 10).rounded() / 10,
                    "height": (frame.height * 10).rounded() / 10,
                ],
            ]
        }

        let directory = FileManager.default.temporaryDirectory
        let json = try JSONSerialization.data(withJSONObject: snapshot, options: [.sortedKeys])
        try json.write(to: directory.appendingPathComponent("nexa-render-parity-\(phase).json"))
        guard let png = app.screenshot().image.pngData() else {
            throw NSError(domain: "NexaRenderParity", code: 1, userInfo: [NSLocalizedDescriptionKey: "Could not encode simulator screenshot"])
        }
        try png.write(to: directory.appendingPathComponent("nexa-render-parity-\(phase).png"))
    }

    private func element(_ label: String, app: XCUIApplication) -> XCUIElement {
        let matches = app.descendants(matching: .any).matching(NSPredicate(format: "label == %@", label))
        return matches.allElementsBoundByIndex.first(where: { $0.exists && $0.isHittable }) ?? matches.firstMatch
    }

    private func tap(_ label: String, app: XCUIApplication, timeout: TimeInterval = 10) throws {
        let target = element(label, app: app)
        guard target.waitForExistence(timeout: timeout) else {
            throw NSError(domain: "NexaRenderParity", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "Missing control \(label). Hierarchy: \(app.debugDescription)",
            ])
        }
        guard target.isHittable else {
            throw NSError(domain: "NexaRenderParity", code: 3, userInfo: [
                NSLocalizedDescriptionKey: "Control is not hittable: \(label)",
            ])
        }
        target.tap()
    }

    private func scrollToBottom(_ app: XCUIApplication, swipes: Int = 8) {
        let probe = app.staticTexts["Archived tasks appear here."]
        for _ in 0..<swipes {
            if let scrollView = scrollView(containing: probe, app: app) {
                scrollView.swipeUp()
            } else {
                app.swipeUp()
            }
            waitForScrollToSettle(app)
        }
    }

    private func scrollToLabel(_ label: String, upwards: Bool = true, app: XCUIApplication) throws {
        let target = element(label, app: app)
        for _ in 0..<12 {
            if target.exists && target.isHittable { return }
            if let scrollView = scrollView(containing: target, app: app) {
                upwards ? scrollView.swipeUp() : scrollView.swipeDown()
            } else {
                upwards ? app.swipeUp() : app.swipeDown()
            }
            waitForScrollToSettle(app)
        }
        XCTAssertTrue(target.exists && target.isHittable, "Could not scroll to \(label)")
    }

    private func waitForScrollToSettle(_ app: XCUIApplication) {
        let probe = app.staticTexts["Archived tasks appear here."]
        guard probe.exists else { return }
        let deadline = Date().addingTimeInterval(6)
        var previousY = probe.frame.minY
        var stableSamples = 0

        // SwiftUI's large-title collapse and Form scroll settling can continue
        // briefly after the anchor stops moving for a single frame. Require a
        // longer stable window so paired AOT/Dev captures compare the same
        // settled scroll position.
        while Date() < deadline && stableSamples < 6 {
            RunLoop.current.run(until: Date().addingTimeInterval(0.15))
            guard probe.exists else { return }
            let currentY = probe.frame.minY
            if abs(currentY - previousY) < 0.25 {
                stableSamples += 1
            } else {
                stableSamples = 0
            }
            previousY = currentY
        }
    }

    private func alignScrollAnchor(
        _ app: XCUIApplication,
        label: String,
        targetY: CGFloat
    ) {
        let anchor = app.staticTexts[label]
        guard anchor.exists else { return }
        let scrollContainer: XCUIElement = scrollView(containing: anchor, app: app) ?? app

        for _ in 0..<4 {
            waitForScrollToSettle(app)
            let deltaY = targetY - anchor.frame.minY
            if abs(deltaY) < 0.25 { return }

            let start = scrollContainer.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.55))
            // Move the finger in the direction needed to move the content to targetY.
            let end = start.withOffset(CGVector(dx: 0, dy: deltaY))
            start.press(forDuration: 0.05, thenDragTo: end)
        }
        waitForScrollToSettle(app)
    }

    private func scrollView(containing anchor: XCUIElement, app: XCUIApplication) -> XCUIElement? {
        guard anchor.exists else { return nil }
        let candidates = app.scrollViews.allElementsBoundByIndex
            + app.tables.allElementsBoundByIndex
            + app.collectionViews.allElementsBoundByIndex
        return candidates
            .filter { $0.frame.contains(anchor.frame) }
            .min { lhs, rhs in
                lhs.frame.width * lhs.frame.height < rhs.frame.width * rhs.frame.height
            }
    }

    func testAotAndDevRuntimeRenderTheBuiltInComponentCatalog() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Parity heading"].waitForExistence(timeout: 60))
        try capture("initial", app: app)

        try tap("Toolbar action", app: app)
        XCTAssertTrue(app.staticTexts["Toolbar actions: 1"].waitForExistence(timeout: 5))
        try capture("toolbar", app: app)

        let increment = app.buttons["Increment: 0"]
        XCTAssertTrue(increment.waitForExistence(timeout: 10))
        increment.tap()
        XCTAssertTrue(app.buttons["Increment: 1"].waitForExistence(timeout: 5))
        try capture("button", app: app)

        let enabled = app.switches["Enabled"]
        XCTAssertTrue(enabled.waitForExistence(timeout: 5))
        enabled.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).tap()
        XCTAssertEqual(enabled.value as? String, "1")
        XCTAssertTrue(app.staticTexts["Enabled: true"].waitForExistence(timeout: 5))
        try capture("switch", app: app)

        try tap("Open text input screen", app: app)
        let title = app.textFields["Task title"]
        XCTAssertTrue(title.waitForExistence(timeout: 5))
        title.tap()
        title.typeText("NexaParity\n")
        XCTAssertTrue(app.staticTexts["Title: NexaParity"].waitForExistence(timeout: 5))
        try capture("text-entry", app: app)
        try tap("Back to catalog", app: app)

        let scrollAnchor = app.staticTexts["Archived tasks appear here."]
        if let scrollView = scrollView(containing: scrollAnchor, app: app) {
            scrollView.swipeUp()
        } else {
            app.swipeUp()
        }
        waitForScrollToSettle(app)
        app.swipeUp()
        waitForScrollToSettle(app)
        app.swipeUp()
        waitForScrollToSettle(app)
        try capture("controls-scroll-3", app: app)
        app.swipeUp()
        waitForScrollToSettle(app)
        app.swipeUp()
        waitForScrollToSettle(app)
        try capture("controls-scroll-5", app: app)
        for _ in 0..<5 { app.swipeDown() }
        try tap("Open detail screen", app: app)
        XCTAssertTrue(app.staticTexts["Navigation destination"].waitForExistence(timeout: 10))
        try capture("navigation", app: app)
        try tap("Back to catalog", app: app)

        try tap("Lists", app: app)
        XCTAssertTrue(app.staticTexts["Virtualized list"].waitForExistence(timeout: 10))
        try capture("lists", app: app)
        if element("Refresh now", app: app).isHittable {
            try tap("Refresh now", app: app)
            try capture("lists-refresh", app: app)
        } else {
            try capture("lists-refresh", app: app)
        }
        app.swipeUp()
        waitForScrollToSettle(app)
        try capture("lists-scroll-1", app: app)
        app.swipeUp()
        waitForScrollToSettle(app)
        app.swipeUp()
        waitForScrollToSettle(app)
        try capture("lists-scroll-3", app: app)
        try scrollToLabel("Open list parameters", upwards: false, app: app)
        try tap("Open list parameters", app: app)
        XCTAssertTrue(app.staticTexts["Page snap list parameters"].waitForExistence(timeout: 10))
        try capture("list-parameters", app: app)
        app.swipeUp()
        XCTAssertTrue(app.staticTexts["Page snap row 1"].waitForExistence(timeout: 5))
        try capture("list-parameters-next", app: app)
        try tap("Back to lists", app: app)

        try tap("Workspace", app: app)
        XCTAssertTrue(app.navigationBars["Workspace"].waitForExistence(timeout: 10))
        try capture("workspace", app: app)

        try tap("Pages", app: app)
        XCTAssertTrue(app.staticTexts["Pager page one"].waitForExistence(timeout: 10))
        try capture("pages", app: app)
        try tap("Next page", app: app)
        XCTAssertTrue(app.staticTexts["Pager page two"].waitForExistence(timeout: 5))
        try capture("pages-next", app: app)

        try tap("Controls", app: app)
        try scrollToLabel("Open style parameters", app: app)
        try tap("Open style parameters", app: app)
        XCTAssertTrue(app.staticTexts["Column style parameters"].waitForExistence(timeout: 10))
        try capture("style-layout", app: app)
        try scrollToLabel("Typography style parameters", app: app)
        try capture("style-typography", app: app)
        try scrollToLabel("Button style parameters", app: app)
        try capture("style-button", app: app)
        try tap("Enable configured button", app: app)
        try tap("Configured button", app: app)
        XCTAssertTrue(app.staticTexts["Configured state: loading=true, disabled=false"].waitForExistence(timeout: 5))
        try capture("style-button-interaction", app: app)
        try tap("Open large sheet", app: app)
        XCTAssertTrue(app.staticTexts["Large sheet content"].waitForExistence(timeout: 5))
        try capture("large-sheet", app: app)
        try tap("Close large sheet", app: app)
        try scrollToLabel("Text input parameters", app: app)
        try capture("style-input", app: app)
        try scrollToLabel("Image and icon parameters", app: app)
        alignScrollAnchor(app, label: "Image and icon parameters", targetY: 406)
        try capture("style-images", app: app)
        try scrollToLabel("Pressable parameters", app: app)
        alignScrollAnchor(app, label: "Pressable parameters", targetY: 800)
        try capture("style-pressable", app: app)
        try scrollToLabel("Back to catalog", upwards: false, app: app)
        try tap("Back to catalog", app: app)

        try tap("Controls", app: app)
        scrollToBottom(app)
        try tap("Open bottom sheet", app: app)
        XCTAssertTrue(app.staticTexts["Sheet content"].waitForExistence(timeout: 5))
        try capture("bottom-sheet", app: app)
        try tap("Close sheet", app: app)
        try tap("Open dialog", app: app)
        XCTAssertTrue(app.alerts["Parity dialog"].waitForExistence(timeout: 5))
        try capture("dialog", app: app)
        try tap("Dismiss dialog", app: app)
        try tap("Open confirmation", app: app)
        XCTAssertTrue(app.buttons["Confirm action"].waitForExistence(timeout: 5))
        try capture("confirmation", app: app)
        try tap("Confirm action", app: app)
    }
}
