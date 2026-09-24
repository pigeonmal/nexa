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
        let lists = app.scrollViews
        XCTAssertGreaterThanOrEqual(lists.count, 2)
        let lastGridRow = app.staticTexts["Grid 39"]
        for _ in 0..<20 {
            if lastGridRow.exists { break }
            lists.element(boundBy: 1).swipeUp()
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
        app.scrollViews.element(boundBy: 0).swipeDown()
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'refreshes 1'"))
            .firstMatch.waitForExistence(timeout: 15))
    }
}
