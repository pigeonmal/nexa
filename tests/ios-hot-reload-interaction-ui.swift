import XCTest

final class NexaHotReloadInteractionTests: XCTestCase {
    func testUpdatedActionsRunAndNavigationSurvivesReload() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Home Screen"].waitForExistence(timeout: 45))
        let stackUnderlay = app.staticTexts["Stack Underlay"]
        let stackOverlay = app.staticTexts["Stack Overlay V1"]
        XCTAssertTrue(stackUnderlay.waitForExistence(timeout: 10))
        XCTAssertTrue(stackOverlay.waitForExistence(timeout: 10))
        XCTAssertEqual(stackUnderlay.frame.midX, stackOverlay.frame.midX, accuracy: 2, "Stack children should share their horizontal center")
        XCTAssertEqual(stackUnderlay.frame.midY, stackOverlay.frame.midY, accuracy: 2, "Stack children should share their vertical center")
        let accessibleButton = app.buttons["Accessible Action V1"]
        XCTAssertTrue(accessibleButton.waitForExistence(timeout: 10), "Accessibility role and label should expose a button")
        XCTAssertTrue(app.descendants(matching: .any).matching(identifier: "Accessible Link V1").firstMatch.waitForExistence(timeout: 10), "the link accessibility label should be discoverable")
        XCTAssertTrue(app.descendants(matching: .any).matching(identifier: "Accessible Header V1").firstMatch.waitForExistence(timeout: 10), "the header accessibility label should be discoverable")
        XCTAssertTrue(app.descendants(matching: .any).matching(identifier: "Accessible Image V1").firstMatch.waitForExistence(timeout: 10), "the image accessibility label should be discoverable")
        XCTAssertTrue(app.descendants(matching: .any).matching(identifier: "Accessible Plain V1").firstMatch.waitForExistence(timeout: 10), "Accessibility with no explicit role should remain discoverable")
        XCTAssertTrue(app.staticTexts["Count: 0"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["Presses: 0"].waitForExistence(timeout: 10))
        let increment = app.buttons["Increment"]
        XCTAssertTrue(increment.waitForExistence(timeout: 10))
        increment.tap()
        XCTAssertTrue(app.staticTexts["Count: 1"].waitForExistence(timeout: 10))
        let pressable = app.buttons["Press me"]
        XCTAssertTrue(pressable.waitForExistence(timeout: 10))
        pressable.tap()
        XCTAssertTrue(app.staticTexts["Presses: 1"].waitForExistence(timeout: 10))

        let openDetails = app.buttons["Open Details"]
        XCTAssertTrue(openDetails.waitForExistence(timeout: 10))
        openDetails.tap()
        XCTAssertTrue(app.staticTexts["Details V1"].waitForExistence(timeout: 10))

        let temporaryDirectory = FileManager.default.temporaryDirectory
        let readyPath = temporaryDirectory.appendingPathComponent("nexa-hot-reload-ready")
        try? "__READY_TOKEN__".write(to: readyPath, atomically: true, encoding: .utf8)
        XCTAssertTrue(app.staticTexts["Details V2"].waitForExistence(timeout: 180))
        let reloadReadyPath = temporaryDirectory.appendingPathComponent("nexa-hot-reload-patched")
        try? "__PATCHED_TOKEN__".write(to: reloadReadyPath, atomically: true, encoding: .utf8)
        let verifiedPath = temporaryDirectory.appendingPathComponent("nexa-hot-reload-verified")
        let verificationDeadline = Date().addingTimeInterval(30)
        while Date() < verificationDeadline {
            if let verified = try? String(contentsOf: verifiedPath, encoding: .utf8), verified == "__VERIFIED_TOKEN__" {
                break
            }
            Thread.sleep(forTimeInterval: 0.1)
        }
        XCTAssertEqual(try? String(contentsOf: verifiedPath, encoding: .utf8), "__VERIFIED_TOKEN__", "the shell harness should verify the patched screen before navigation continues")

        let goHome = app.buttons["Go Home"]
        XCTAssertTrue(goHome.waitForExistence(timeout: 10))
        goHome.tap()
        XCTAssertTrue(app.staticTexts["Home Screen"].waitForExistence(timeout: 10))
        let reloadedStackOverlay = app.staticTexts["Stack Overlay V2"]
        XCTAssertTrue(reloadedStackOverlay.waitForExistence(timeout: 10))
        XCTAssertEqual(stackUnderlay.frame.midX, reloadedStackOverlay.frame.midX, accuracy: 2, "updated Stack children should remain horizontally aligned")
        XCTAssertEqual(stackUnderlay.frame.midY, reloadedStackOverlay.frame.midY, accuracy: 2, "updated Stack children should remain vertically aligned")
        XCTAssertTrue(app.buttons["Accessible Action V2"].waitForExistence(timeout: 10), "the accessibility label should hot reload")
        XCTAssertTrue(app.descendants(matching: .any).matching(identifier: "Accessible Link V2").firstMatch.waitForExistence(timeout: 10), "the link label should hot reload")
        XCTAssertTrue(app.descendants(matching: .any).matching(identifier: "Accessible Header V2").firstMatch.waitForExistence(timeout: 10), "the header label should hot reload")
        XCTAssertTrue(app.descendants(matching: .any).matching(identifier: "Accessible Image V2").firstMatch.waitForExistence(timeout: 10), "the image label should hot reload")
        XCTAssertTrue(app.descendants(matching: .any).matching(identifier: "Accessible Plain V2").firstMatch.waitForExistence(timeout: 10), "the plain label should hot reload")
        XCTAssertTrue(app.staticTexts["Count: 1"].exists)
        XCTAssertTrue(app.staticTexts["Presses: 1"].exists)
        increment.tap()
        XCTAssertTrue(app.staticTexts["Count: 11"].waitForExistence(timeout: 10))
        pressable.tap()
        XCTAssertTrue(app.staticTexts["Presses: 11"].waitForExistence(timeout: 10))
    }
}
