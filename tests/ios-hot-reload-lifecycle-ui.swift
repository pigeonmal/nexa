import XCTest

final class NexaHotReloadLifecycleTests: XCTestCase {
    func testLifecycleEventsAndReloadedAction() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Appears: 1"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.staticTexts["Active: 1"].waitForExistence(timeout: 15))
        app.buttons["Open detail"].tap()
        XCTAssertTrue(app.staticTexts["Detail visits: 1"].waitForExistence(timeout: 15))
        app.navigationBars.buttons.firstMatch.tap()
        XCTAssertTrue(app.staticTexts["Detail leaves: 1"].waitForExistence(timeout: 15))
        let ready = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-lifecycle-ready")
        try "__READY_TOKEN__".write(to: ready, atomically: true, encoding: .utf8)

        let patched = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-lifecycle-patched")
        let deadline = Date().addingTimeInterval(180)
        while Date() < deadline {
            if (try? String(contentsOf: patched, encoding: .utf8)) == "__PATCHED_TOKEN__" { break }
            Thread.sleep(forTimeInterval: 0.1)
        }
        XCTAssertEqual(try? String(contentsOf: patched, encoding: .utf8), "__PATCHED_TOKEN__")

        XCUIDevice.shared.press(.home)
        XCTAssertTrue(app.staticTexts["Inactive: 1"].waitForExistence(timeout: 15))
        XCTAssertTrue(app.staticTexts["Background: 1"].waitForExistence(timeout: 15))
        app.activate()
        XCTAssertTrue(app.staticTexts["Active: 11"].waitForExistence(timeout: 20))
        let appearanceLabel = app.staticTexts["Appears: 1"]
        XCTAssertTrue(appearanceLabel.exists, "compatible hot reload must not fire OnAppear again")
    }
}
