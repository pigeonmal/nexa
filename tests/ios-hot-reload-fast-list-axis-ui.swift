import XCTest

final class NexaFastListAxisSmokeTests: XCTestCase {
    func testHorizontalAndGridAxesRemainAccessibleAfterHotReload() {
        let app = XCUIApplication()
        app.launch()

        let horizontalFirst = app.staticTexts["Cell 0 0"]
        let horizontalSecond = app.staticTexts["Cell 1 1"]
        XCTAssertTrue(horizontalFirst.waitForExistence(timeout: 45))
        XCTAssertTrue(horizontalSecond.waitForExistence(timeout: 10))
        XCTAssertGreaterThan(horizontalSecond.frame.midX, horizontalFirst.frame.midX)
        XCTAssertEqual(horizontalSecond.frame.midY, horizontalFirst.frame.midY, accuracy: 2)
        let ready = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-fast-list-axis-ready")
        try? "ready".write(to: ready, atomically: true, encoding: .utf8)

        let gridFirst = app.staticTexts["Updated Cell 0 0"]
        let gridSecond = app.staticTexts["Updated Cell 1 1"]
        let gridThird = app.staticTexts["Updated Cell 2 2"]
        XCTAssertTrue(gridFirst.waitForExistence(timeout: 180))
        XCTAssertTrue(gridSecond.waitForExistence(timeout: 10))
        XCTAssertTrue(gridThird.waitForExistence(timeout: 10))
        XCTAssertGreaterThan(gridSecond.frame.midX, gridFirst.frame.midX)
        XCTAssertEqual(gridSecond.frame.midY, gridFirst.frame.midY, accuracy: 2)
        XCTAssertGreaterThan(gridThird.frame.midY, gridFirst.frame.midY + 2)
        let completed = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-fast-list-axis-completed")
        try? "completed".write(to: completed, atomically: true, encoding: .utf8)
    }
}
