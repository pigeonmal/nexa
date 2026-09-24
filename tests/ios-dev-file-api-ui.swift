import XCTest

final class NexaDevFileApiTests: XCTestCase {
    func testFileApisAddedByHotReloadUseNativeFileHelpers() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["File: FILE_OK"].waitForExistence(timeout: 60))
        XCTAssertTrue(app.staticTexts["Exists: true"].exists)
        XCTAssertFalse(app.staticTexts["File: ERROR"].exists)
    }
}
