import XCTest

final class NexaHotReloadRefreshControlTests: XCTestCase {
    func testGenericRefreshControlRendersAndReloadsItsAction() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Before refresh"].waitForExistence(timeout: 45))
        let ready = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-refresh-control-ready")
        try "__READY_TOKEN__".write(to: ready, atomically: true, encoding: .utf8)

        XCTAssertTrue(app.staticTexts["After reload"].waitForExistence(timeout: 180))
        let patched = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-refresh-control-patched")
        try "__PATCHED_TOKEN__".write(to: patched, atomically: true, encoding: .utf8)

        let verified = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-refresh-control-verified")
        let deadline = Date().addingTimeInterval(30)
        while Date() < deadline {
            if (try? String(contentsOf: verified, encoding: .utf8)) == "__VERIFIED_TOKEN__" { break }
            Thread.sleep(forTimeInterval: 0.1)
        }
        XCTAssertEqual(try? String(contentsOf: verified, encoding: .utf8), "__VERIFIED_TOKEN__")

        let scrollView = app.scrollViews.firstMatch
        XCTAssertTrue(scrollView.exists, "RefreshControl should expose its scrollable content")
        let start = scrollView.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.1))
        let end = scrollView.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.9))
        start.press(forDuration: 0.1, thenDragTo: end)
        XCTAssertTrue(app.staticTexts["Refreshes: 10"].waitForExistence(timeout: 15))
    }
}
