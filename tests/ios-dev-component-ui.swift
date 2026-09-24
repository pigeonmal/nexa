import XCTest

final class NexaDevComponentTests: XCTestCase {
    func testReloadedComponentArgumentsAndProjectedContentRender() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Card: Reloaded"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.staticTexts["Projected child"].exists)
        XCTAssertTrue(app.staticTexts["Binary and: true"].exists)
        XCTAssertTrue(app.staticTexts["Contains: true"].exists)
        XCTAssertTrue(app.staticTexts["Index: 5"].exists)
        XCTAssertTrue(app.staticTexts["Coalesce: fallback"].exists)
        XCTAssertTrue(app.staticTexts["Pair: 7"].exists)
    }
}
