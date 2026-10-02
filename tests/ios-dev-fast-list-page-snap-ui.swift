import XCTest

final class NexaFastListPageSnapTests: XCTestCase {
    func testVerticalFeedSettlesOnOneFullPageAndReportsSettledPage() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Current page 0"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.staticTexts["Feed page 0"].exists)

        let eventStatus = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Settled events ")
        ).firstMatch
        XCTAssertTrue(eventStatus.exists)
        let initialEvents = Int(eventStatus.label.split(separator: " ").last ?? "0") ?? 0

        let list = app.tables.firstMatch.exists ? app.tables.firstMatch : app.scrollViews.firstMatch
        XCTAssertTrue(list.waitForExistence(timeout: 10))
        let initialOffset = app.staticTexts["Feed page 0"].frame.minY - list.frame.minY
        list.swipeUp()

        XCTAssertTrue(app.staticTexts["Current page 1"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Feed page 1"].exists)
        let settledOffset = app.staticTexts["Feed page 1"].frame.minY - list.frame.minY
        XCTAssertEqual(settledOffset, initialOffset, accuracy: 2)
        let settledEvents = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Settled events ")
        ).firstMatch
        XCTAssertTrue(settledEvents.waitForExistence(timeout: 10))
        let finalEvents = Int(settledEvents.label.split(separator: " ").last ?? "0") ?? 0
        XCTAssertEqual(finalEvents, initialEvents + 1)

        print("NEXA_PAGE_SNAP iOS page=1 rowViewportOffset=\(settledOffset)pt settledEvents=\(finalEvents)")
    }
}
