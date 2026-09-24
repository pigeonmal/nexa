import XCTest

final class NexaDeepLinkTests: XCTestCase {
    func testCustomURLOpensTypedScreenRoute() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Home"].waitForExistence(timeout: 20))
        app.open(URL(string: "nexa://product-details/widget-17/5")!)
        XCTAssertTrue(app.staticTexts["Product widget-17, page 5"].waitForExistence(timeout: 10))
    }
}
