import XCTest

final class NexaTextInputRefUITests: XCTestCase {
    func testMountedEmailRefSurvivesLayoutAndRepeatedFocusBlur() {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launch()
        let email = app.textFields["Search email"]
        if !email.exists {
            let route = app.descendants(matching: .any)
                .matching(NSPredicate(format: "label == %@", "Text input variants")).firstMatch
            for _ in 0..<20 {
                if route.exists && route.isHittable { break }
                app.swipeUp()
            }
            XCTAssertTrue(route.isHittable)
            route.tap()
        }
        XCTAssertTrue(email.waitForExistence(timeout: 10))
        app.buttons["Reset inputs"].tap()
        for index in 1...3 {
            let focus = app.buttons["Focus email"]
            XCTAssertTrue(focus.isHittable)
            focus.tap()
            XCTAssertTrue(app.keyboards.firstMatch.waitForExistence(timeout: 5))
            app.typeText("a")
            XCTAssertEqual(email.value as? String, String(repeating: "a", count: index))
            app.buttons["Blur email"].tap()
            XCTAssertTrue(app.keyboards.firstMatch.waitForNonExistence(timeout: 5))
        }
    }
}
