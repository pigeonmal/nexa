import XCTest

final class NexaDevAsyncAppearTests: XCTestCase {
    func testAsyncAppAndScreenAppearanceActionsRun() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["App ready"].waitForExistence(timeout: 30))
        XCTAssertTrue(app.staticTexts["Screen ready: Nexa"].waitForExistence(timeout: 30))
    }
}
