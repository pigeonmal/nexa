import XCTest

final class NexaFastListSmokeTests: XCTestCase {
    func testListRowsRemainAccessibleAfterHotReload() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Item 15"].waitForExistence(timeout: 45))
        let ready = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-fast-list-ready")
        try? "ready".write(to: ready, atomically: true, encoding: .utf8)

        XCTAssertTrue(app.staticTexts["Reloaded 15"].waitForExistence(timeout: 180))
        let completed = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-fast-list-completed")
        try? "completed".write(to: completed, atomically: true, encoding: .utf8)
    }
}
