import XCTest

final class NexaDevNetworkFetchTests: XCTestCase {
    func testAsyncNetworkFetchUsesNativeResponse() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["HTTP: 200"].waitForExistence(timeout: 45))
    }
}
