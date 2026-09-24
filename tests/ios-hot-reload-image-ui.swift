import XCTest

final class NexaHotReloadImageTests: XCTestCase {
    func testBundledImageAssetsRenderAndHotReload() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.images["Fitted image"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.images["Filled image"].waitForExistence(timeout: 15))
        let ready = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-image-ready")
        try "__READY_TOKEN__".write(to: ready, atomically: true, encoding: .utf8)

        let patched = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-image-patched")
        let deadline = Date().addingTimeInterval(180)
        while Date() < deadline {
            if (try? String(contentsOf: patched, encoding: .utf8)) == "__PATCHED_TOKEN__" { break }
            Thread.sleep(forTimeInterval: 0.1)
        }
        XCTAssertEqual(try? String(contentsOf: patched, encoding: .utf8), "__PATCHED_TOKEN__")
        XCTAssertTrue(app.images["Updated fitted image"].waitForExistence(timeout: 30))
        XCTAssertFalse(app.images["Fitted image"].exists)
        XCTAssertTrue(app.images["Filled image"].exists)
    }
}
