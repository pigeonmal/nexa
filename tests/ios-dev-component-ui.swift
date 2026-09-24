import XCTest

final class NexaDevComponentTests: XCTestCase {
    func testReloadedComponentArgumentsAndProjectedContentRender() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Card: Reloaded"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.staticTexts["Projected child"].exists)
        XCTAssertTrue(app.staticTexts["Binary and: true"].exists)
        XCTAssertTrue(app.staticTexts["Contains: true"].exists)
        XCTAssertTrue(app.staticTexts["Index: 5"].exists)
        XCTAssertTrue(app.staticTexts["Coalesce: fallback"].exists)
        XCTAssertTrue(app.staticTexts["Pair: 7"].exists)
        XCTAssertTrue(app.staticTexts["Mapped: 6"].exists)
        XCTAssertTrue(app.staticTexts["Filtered: 8"].exists)
        XCTAssertTrue(app.staticTexts["Reduced: 16"].exists)
        XCTAssertTrue(app.staticTexts["Int8: 8, Int16: 16, Int64: 64"].exists)
        XCTAssertTrue(app.staticTexts["UInt8: 8, UInt16: 16, UInt32: 32, UInt64: 64"].exists)
        XCTAssertTrue(app.staticTexts["Float32: 3.5, Float64: 6.5"].exists)
        XCTAssertTrue(app.staticTexts["Set: true"].exists)
        XCTAssertTrue(app.staticTexts["Map: 42"].exists)
        XCTAssertTrue(app.staticTexts["Triple: 4"].exists)
        XCTAssertTrue(app.staticTexts["Enum: ready"].exists)
        XCTAssertTrue(app.staticTexts["Result: 42"].exists)
        XCTAssertTrue(app.staticTexts["Result error: null"].exists)
        XCTAssertTrue(app.staticTexts["Struct: Ada"].exists)
        XCTAssertTrue(app.staticTexts["Array mutation: 2"].exists)
        XCTAssertTrue(app.staticTexts["Set mutation: true"].exists)
        XCTAssertTrue(app.staticTexts["Map mutation: 2"].exists)
        XCTAssertTrue(app.staticTexts["For: 6, While: 3"].exists)
        XCTAssertTrue(app.staticTexts["Break: 2, Continue: 5"].exists)
        XCTAssertTrue(app.staticTexts["ForMap: 2, TryCatch: 1"].exists)
        XCTAssertTrue(app.staticTexts["Other comparisons: true, true, true, true, true, true"].exists)
        XCTAssertTrue(app.staticTexts["Size classes: false, true, true, false"].exists)
        XCTAssertTrue(app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Native path: ")
        ).firstMatch.exists)
    }
}
