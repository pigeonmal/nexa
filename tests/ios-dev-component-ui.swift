import XCTest

final class NexaDevComponentTests: XCTestCase {
    func testReloadedComponentArgumentsAndProjectedContentRender() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Card: Reloaded"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.staticTexts["Projected child"].exists)
    }
}
