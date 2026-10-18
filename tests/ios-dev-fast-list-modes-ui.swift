import XCTest

final class NexaFastListModesTests: XCTestCase {
    func testSectionsHeadersGridAndListCallbacks() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Section 0"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.staticTexts["Section 0 row 0: A1"].exists)
        XCTAssertTrue(app.staticTexts["Section 1 row 0: B1"].exists)
        XCTAssertTrue(app.staticTexts["Grid 0"].exists)
        XCTAssertTrue(app.staticTexts["Grid 1"].exists)
        let initialStatus = app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'Events '")).firstMatch
        XCTAssertTrue(initialStatus.exists)
        let initialEvents = Int(initialStatus.label.split(separator: " ").dropFirst().first ?? "0") ?? 0
        XCTAssertTrue(initialStatus.label.contains("refreshing false"))
        let lastGridRow = app.staticTexts["Grid 39"]
        for _ in 0..<20 {
            if lastGridRow.exists { break }
            app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.84))
                .press(
                    forDuration: 0.05,
                    thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.2))
                )
        }
        XCTAssertTrue(lastGridRow.waitForExistence(timeout: 10))
        let completedStatus = app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'pages 1'"))
            .firstMatch
        XCTAssertTrue(completedStatus.waitForExistence(timeout: 10))
        let finalEvents = Int(completedStatus.label.split(separator: " ").dropFirst().first ?? "0") ?? 0
        XCTAssertGreaterThan(finalEvents, initialEvents)
    }

    func testRefreshControlAction() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Section 0 row 0: A1"].waitForExistence(timeout: 45))
        let header = app.staticTexts["Section 0"]
        XCTAssertTrue(header.exists)
        let appHeight = app.frame.height
        let start = app.coordinate(
            withNormalizedOffset: CGVector(dx: 0.5, dy: max(0, min(1, (header.frame.minY + 1) / appHeight)))
        )
        let end = app.coordinate(
            withNormalizedOffset: CGVector(dx: 0.5, dy: max(0, min(0.9, (header.frame.minY + 240) / appHeight)))
        )
        start.press(forDuration: 0.05, thenDragTo: end)
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'refreshes 1'"))
            .firstMatch.waitForExistence(timeout: 15))
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'refreshing true'"))
            .firstMatch.waitForExistence(timeout: 5))
    }
}
