import XCTest

final class NexaDevCustomComponentHotReloadTests: XCTestCase {
    func testAddingRenamingAndRemovingCustomComponentLive() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Initial"].waitForExistence(timeout: 45))
        print("NEXA_COMPONENT_STAGE_INITIAL")

        XCTAssertTrue(app.staticTexts["Card: Added"].waitForExistence(timeout: 90))
        XCTAssertTrue(app.staticTexts["Projected child"].exists)
        print("NEXA_COMPONENT_STAGE_ADDED")

        XCTAssertTrue(app.staticTexts["Product: Renamed"].waitForExistence(timeout: 90))
        XCTAssertFalse(app.staticTexts["Card: Added"].exists)
        XCTAssertTrue(app.staticTexts["Projected child"].exists)
        print("NEXA_COMPONENT_STAGE_RENAMED")

        XCTAssertTrue(app.staticTexts["After component removal"].waitForExistence(timeout: 90))
        XCTAssertFalse(app.staticTexts["Product: Renamed"].exists)
        XCTAssertFalse(app.staticTexts["Projected child"].exists)
        print("NEXA_COMPONENT_STAGE_REMOVED")
    }
}
