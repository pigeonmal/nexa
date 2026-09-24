import XCTest

final class NexaHotReloadRemoteImageTests: XCTestCase {
    func testRemoteImageRendersAndHotReloads() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.images["Remote image"].waitForExistence(timeout: 45))
        let ready = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-remote-image-ready")
        try "__READY_TOKEN__".write(to: ready, atomically: true, encoding: .utf8)

        let patched = FileManager.default.temporaryDirectory.appendingPathComponent("nexa-remote-image-patched")
        let deadline = Date().addingTimeInterval(180)
        while Date() < deadline {
            if (try? String(contentsOf: patched, encoding: .utf8)) == "__PATCHED_TOKEN__" { break }
            Thread.sleep(forTimeInterval: 0.1)
        }
        XCTAssertEqual(try? String(contentsOf: patched, encoding: .utf8), "__PATCHED_TOKEN__")
        XCTAssertTrue(app.images["Updated remote image"].waitForExistence(timeout: 30))
        XCTAssertFalse(app.images["Remote image"].exists)
    }
}
