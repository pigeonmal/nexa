import XCTest

final class NexaFastListModesTests: XCTestCase {
    func testSectionsHeadersAndGridRender() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Section 0"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.staticTexts["Section 0 row 0: A1"].exists)
        XCTAssertTrue(app.staticTexts["Section 1 row 0: B1"].exists)
        XCTAssertTrue(app.staticTexts["Grid 0"].exists)
        XCTAssertTrue(app.staticTexts["Grid 1"].exists)
    }
}
